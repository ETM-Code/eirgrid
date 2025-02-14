use std::collections::HashMap;
use rand::Rng;
use serde::{Serialize, Deserialize};
use crate::generator::GeneratorType;
use crate::constants::MAX_ACCEPTABLE_EMISSIONS;
use std::path::Path;
use lazy_static::lazy_static;
use std::str::FromStr;
use std::sync::Mutex;

lazy_static! {
    static ref FILE_MUTEX: Mutex<()> = Mutex::new(());
}

const DEFAULT_WEIGHT: f64 = 0.5;
const MIN_WEIGHT: f64 = 0.001;  // Ensure weight doesn't go too close to zero
const MAX_WEIGHT: f64 = 0.95;  // Ensure weight doesn't dominate completely

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum GridAction {
    AddGenerator(GeneratorType),
    UpgradeEfficiency(String),  // Generator ID
    AdjustOperation(String, u8),  // Generator ID, percentage (0-100)
    AddCarbonOffset(String),  // Offset type
    CloseGenerator(String),  // Generator ID
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationMetrics {
    pub final_net_emissions: f64,
    pub average_public_opinion: f64,
    pub total_cost: f64,
    pub power_reliability: f64,
}

// Helper struct for serialization
#[derive(Serialize, Deserialize)]
struct SerializableAction {
    action_type: String,
    generator_type: Option<String>,
    generator_id: Option<String>,
    operation_percentage: Option<u8>,
    offset_type: Option<String>,
}

impl From<&GridAction> for SerializableAction {
    fn from(action: &GridAction) -> Self {
        match action {
            GridAction::AddGenerator(gen_type) => SerializableAction {
                action_type: "AddGenerator".to_string(),
                generator_type: Some(gen_type.to_string()),
                generator_id: None,
                operation_percentage: None,
                offset_type: None,
            },
            GridAction::UpgradeEfficiency(id) => SerializableAction {
                action_type: "UpgradeEfficiency".to_string(),
                generator_type: None,
                generator_id: Some(id.clone()),
                operation_percentage: None,
                offset_type: None,
            },
            GridAction::AdjustOperation(id, percentage) => SerializableAction {
                action_type: "AdjustOperation".to_string(),
                generator_type: None,
                generator_id: Some(id.clone()),
                operation_percentage: Some(*percentage),
                offset_type: None,
            },
            GridAction::AddCarbonOffset(offset_type) => SerializableAction {
                action_type: "AddCarbonOffset".to_string(),
                generator_type: None,
                generator_id: None,
                operation_percentage: None,
                offset_type: Some(offset_type.clone()),
            },
            GridAction::CloseGenerator(id) => SerializableAction {
                action_type: "CloseGenerator".to_string(),
                generator_type: None,
                generator_id: Some(id.clone()),
                operation_percentage: None,
                offset_type: None,
            },
        }
    }
}

// Helper struct for serializing the entire weights map
#[derive(Serialize, Deserialize)]
struct SerializableWeights {
    weights: HashMap<u32, Vec<(SerializableAction, f64)>>,
    learning_rate: f64,
    best_metrics: Option<SimulationMetrics>,
    best_weights: Option<HashMap<u32, Vec<(SerializableAction, f64)>>>,
    iteration_count: u32,
    exploration_rate: f64,
}

#[derive(Debug, Clone)]
pub struct ActionWeights {
    weights: HashMap<u32, HashMap<GridAction, f64>>,
    action_count_weights: HashMap<u32, HashMap<u32, f64>>, // Maps year -> (action_count -> weight)
    learning_rate: f64,
    best_metrics: Option<SimulationMetrics>,
    best_weights: Option<HashMap<u32, HashMap<GridAction, f64>>>,
    iteration_count: u32,
    exploration_rate: f64,
}

impl ActionWeights {
    pub fn new() -> Self {
        let mut weights = HashMap::new();
        let mut action_count_weights = HashMap::new();
        
        // Initialize weights for each year from 2025 to 2050
        for year in 2025..=2050 {
            let mut year_weights = HashMap::new();
            
            // Initialize wind generator weights
            year_weights.insert(GridAction::AddGenerator(GeneratorType::OnshoreWind), 0.08);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::OffshoreWind), 0.08);
            
            // Initialize solar generator weights
            year_weights.insert(GridAction::AddGenerator(GeneratorType::DomesticSolar), 0.05);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::CommercialSolar), 0.05);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::UtilitySolar), 0.08);
            
            // Initialize nuclear and fossil fuel generator weights
            year_weights.insert(GridAction::AddGenerator(GeneratorType::Nuclear), 0.03);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::CoalPlant), 0.04);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::GasCombinedCycle), 0.06);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::GasPeaker), 0.02);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::Biomass), 0.04);
            
            // Initialize hydro and storage generator weights
            year_weights.insert(GridAction::AddGenerator(GeneratorType::HydroDam), 0.06);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::PumpedStorage), 0.06);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::BatteryStorage), 0.07);
            
            // Initialize marine generator weights
            year_weights.insert(GridAction::AddGenerator(GeneratorType::TidalGenerator), 0.05);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::WaveEnergy), 0.05);
            
            // Initialize other action weights
            year_weights.insert(GridAction::UpgradeEfficiency(String::new()), 0.04);
            year_weights.insert(GridAction::AdjustOperation(String::new(), 0), 0.04);
            
            // Initialize carbon offset weights
            year_weights.insert(GridAction::AddCarbonOffset("Forest".to_string()), 0.02);
            year_weights.insert(GridAction::AddCarbonOffset("Wetland".to_string()), 0.02);
            year_weights.insert(GridAction::AddCarbonOffset("ActiveCapture".to_string()), 0.02);
            year_weights.insert(GridAction::AddCarbonOffset("CarbonCredit".to_string()), 0.02);
            
            year_weights.insert(GridAction::CloseGenerator(String::new()), 0.02);
            
            // Add year's weights to the map
            weights.insert(year, year_weights);

            // Initialize action count weights for this year
            let mut count_weights = HashMap::new();
            for count in 0..=20 {
                count_weights.insert(count, 1.0 / 21.0); // Equal initial probability for 0-20 actions
            }
            action_count_weights.insert(year, count_weights);
        }
        
        Self {
            weights,
            action_count_weights,
            learning_rate: 0.1,
            best_metrics: None,
            best_weights: None,
            iteration_count: 0,
            exploration_rate: 0.2,
        }
    }

    pub fn start_new_iteration(&mut self) {
        self.iteration_count += 1;
        // Decay exploration rate over time
        self.exploration_rate = 0.2 * (1.0 / (1.0 + 0.1 * self.iteration_count as f64));
    }
    
    pub fn sample_action(&self, year: u32) -> GridAction {
        let year_weights = self.weights.get(&year).expect("Year weights not found");
        let mut rng = rand::thread_rng();

        // Epsilon-greedy exploration
        if rng.gen::<f64>() < self.exploration_rate {
            // Random exploration
            let actions: Vec<_> = year_weights.keys().collect();
            if actions.is_empty() {
                // Fallback to a safe default action if no actions are available
                return GridAction::AddGenerator(GeneratorType::GasPeaker);
            }
            let random_idx = rng.gen_range(0..actions.len());
            return actions[random_idx].clone();
        }

        // Exploitation - weighted selection
        let total_weight: f64 = year_weights.values().sum();
        if total_weight <= 0.0 {
            // If all weights are zero or negative, fall back to a safe default
            return GridAction::AddGenerator(GeneratorType::GasPeaker);
        }

        let mut random_val = rng.gen::<f64>() * total_weight;
        
        for (action, weight) in year_weights {
            random_val -= weight;
            if random_val <= 0.0 {
                return action.clone();
            }
        }
        
        // Fallback to a safe default if no action was selected
        GridAction::AddGenerator(GeneratorType::GasPeaker)
    }
    
    // Initialize weights for a single year
    fn initialize_weights(&self) -> HashMap<GridAction, f64> {
        let mut year_weights = HashMap::new();
        year_weights.insert(GridAction::AddGenerator(GeneratorType::OnshoreWind), 0.08);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::OffshoreWind), 0.08);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::DomesticSolar), 0.05);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::CommercialSolar), 0.05);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::UtilitySolar), 0.08);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::Nuclear), 0.03);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::CoalPlant), 0.04);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::GasCombinedCycle), 0.06);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::GasPeaker), 0.02);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::Biomass), 0.04);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::HydroDam), 0.06);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::PumpedStorage), 0.06);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::BatteryStorage), 0.07);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::TidalGenerator), 0.05);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::WaveEnergy), 0.05);
        year_weights.insert(GridAction::UpgradeEfficiency(String::new()), 0.04);
        year_weights.insert(GridAction::AdjustOperation(String::new(), 0), 0.04);
        year_weights.insert(GridAction::AddCarbonOffset("Forest".to_string()), 0.02);
        year_weights.insert(GridAction::AddCarbonOffset("Wetland".to_string()), 0.02);
        year_weights.insert(GridAction::AddCarbonOffset("ActiveCapture".to_string()), 0.02);
        year_weights.insert(GridAction::AddCarbonOffset("CarbonCredit".to_string()), 0.02);
        year_weights.insert(GridAction::CloseGenerator(String::new()), 0.02);
        year_weights
    }

    pub fn update_weights(&mut self, action: &GridAction, year: u32, improvement: f64) {
        // Ensure we have weights for this year
        if !self.weights.contains_key(&year) {
            self.weights.insert(year, self.initialize_weights());
        }
        
        let year_weights = self.weights.get_mut(&year).expect("Year weights not found");
        
        // If the action doesn't exist in weights, initialize it
        if !year_weights.contains_key(action) {
            year_weights.insert(action.clone(), DEFAULT_WEIGHT);
        }
        
        let current_weight = year_weights.get(action).expect("Weight should exist");
        
        // Get the final 2050 impact score from best metrics if available
        let final_impact_score = self.best_metrics.as_ref().map_or(0.0, |metrics| score_metrics(metrics));
        
        // Calculate the relative improvement compared to the best score
        let relative_improvement = if let Some(best) = &self.best_metrics {
            let best_score = score_metrics(best);
            if best_score > 0.0 {
                (final_impact_score - best_score) / best_score
            } else {
                final_impact_score
            }
        } else {
            final_impact_score
        };

        // Combine immediate and final impacts with adaptive weighting
        // If we're doing better than our best, weight immediate impact more
        // If we're doing worse, weight final impact more to encourage exploration
        let immediate_weight = if relative_improvement > 0.0 { 0.7 } else { 0.3 };
        let combined_improvement = immediate_weight * improvement + (1.0 - immediate_weight) * relative_improvement;
        
        // Calculate weight adjustment
        let adjustment_factor = if combined_improvement > 0.0 {
            // For improvements, increase weight proportionally to the improvement
            1.0 + (self.learning_rate * combined_improvement)
        } else {
            // For deteriorations, decrease weight proportionally to how bad it was
            1.0 / (1.0 + (self.learning_rate * combined_improvement.abs()))
        };
        
        // Apply the adjustment with bounds
        let new_weight = (current_weight * adjustment_factor)
            .max(MIN_WEIGHT)
            .min(MAX_WEIGHT);
        
        year_weights.insert(action.clone(), new_weight);
        
        // If this was a bad outcome, slightly increase weights of other actions
        if combined_improvement < 0.0 {
            let boost_factor = 1.0 + (self.learning_rate * 0.1); // Small boost to alternatives
            for (other_action, weight) in year_weights.iter_mut() {
                if other_action != action {
                    *weight = (*weight * boost_factor).min(MAX_WEIGHT);
                }
            }
        }
    }

    pub fn update_best_strategy(&mut self, metrics: SimulationMetrics) {
        let current_score = score_metrics(&metrics);
        
        let should_update = match &self.best_metrics {
            None => true,
            Some(best) => {
                let best_score = score_metrics(best);
                current_score > best_score
            }
        };

        if should_update {
            // Only print improvement message if we actually had a previous best
            if let Some(best) = &self.best_metrics {
                println!("\nNew best strategy found! Score improved from {:.4} to {:.4}", 
                        score_metrics(best),
                        current_score);
            }
            self.best_metrics = Some(metrics);
            self.best_weights = Some(self.weights.clone());
        } else if let Some(best) = &self.best_metrics {
            // If this was significantly worse than our best, print a warning
            let best_score = score_metrics(best);
            let deterioration = (best_score - current_score) / best_score;
            if deterioration > 0.1 { // More than 10% worse
                println!("\nWarning: Current strategy performing poorly. Score: {:.4} (Best: {:.4})", 
                        current_score, best_score);
            }
        }
    }

    pub fn clone_with_best(&self) -> Self {
        Self {
            weights: self.weights.clone(),
            action_count_weights: self.action_count_weights.clone(),
            learning_rate: self.learning_rate,
            best_metrics: self.best_metrics.clone(),
            best_weights: self.best_weights.clone(),
            iteration_count: self.iteration_count,
            exploration_rate: self.exploration_rate,
        }
    }

    pub fn load_best_strategy(&mut self) {
        if let Some(best_weights) = &self.best_weights {
            self.weights = best_weights.clone();
        }
    }
    
    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        // Acquire lock for file operations
        let _lock = FILE_MUTEX.lock().map_err(|e| {
            println!("Error acquiring file lock for saving: {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, "Failed to acquire file lock for saving")
        })?;
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = Path::new(path).parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                println!("Error creating directory {}: {}", parent.display(), e);
                return Err(e);
            }
        }

        // Convert to serializable format
        let serializable = SerializableWeights {
            weights: self.weights.iter().map(|(year, weights)| {
                (*year, weights.iter().map(|(action, weight)| {
                    (SerializableAction::from(action), *weight)
                }).collect())
            }).collect(),
            learning_rate: self.learning_rate,
            best_metrics: self.best_metrics.clone(),
            best_weights: self.best_weights.as_ref().map(|weights| {
                weights.iter().map(|(year, weights)| {
                    (*year, weights.iter().map(|(action, weight)| {
                        (SerializableAction::from(action), *weight)
                    }).collect())
                }).collect()
            }),
            iteration_count: self.iteration_count,
            exploration_rate: self.exploration_rate,
        };

        // Serialize to JSON with pretty printing
        let json = match serde_json::to_string_pretty(&serializable) {
            Ok(json) => json,
            Err(e) => {
                println!("Error serializing weights to JSON: {}", e);
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("JSON serialization error: {}", e)));
            }
        };

        // Write to file
        match std::fs::write(path, &json) {
            Ok(_) => {
                println!("Successfully saved weights to {}", path);
                Ok(())
            },
            Err(e) => {
                println!("Error writing weights to {}: {}", path, e);
                Err(e)
            }
        }
    }
    
    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        println!("Attempting to load weights from: {}", path);
        // Acquire lock for file operations
        let _lock = FILE_MUTEX.lock().map_err(|e| {
            println!("Error acquiring file lock: {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, "Failed to acquire file lock")
        })?;
        
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                println!("Error reading file: {}", e);
                return Err(e);
            }
        };
        
        let serializable: SerializableWeights = match serde_json::from_str(&content) {
            Ok(weights) => weights,
            Err(e) => {
                println!("Error parsing weights JSON: {}. Content starts with: {:?}", e, content.chars().take(50).collect::<String>());
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("JSON parse error: {}", e)));
            }
        };
        
        // Convert back to internal format
        let weights = serializable.weights.into_iter().map(|(year, actions)| {
            let mut year_weights = HashMap::new();
            for (action, weight) in actions {
                let grid_action = match action.action_type.as_str() {
                    "AddGenerator" => {
                        match action.generator_type {
                            Some(gen_type) => match GeneratorType::from_str(&gen_type) {
                                Ok(gen) => GridAction::AddGenerator(gen),
                                Err(e) => {
                                    println!("Error parsing generator type '{}': {}", gen_type, e);
                                    continue;
                                }
                            },
                            None => {
                                println!("Missing generator type for AddGenerator action");
                                continue;
                            }
                        }
                    },
                    "UpgradeEfficiency" => match action.generator_id {
                        Some(id) => GridAction::UpgradeEfficiency(id),
                        None => {
                            println!("Missing generator ID for UpgradeEfficiency action");
                            continue;
                        }
                    },
                    "AdjustOperation" => {
                        match (action.generator_id, action.operation_percentage) {
                            (Some(id), Some(percentage)) => GridAction::AdjustOperation(id, percentage),
                            _ => {
                                println!("Missing generator ID or percentage for AdjustOperation action");
                                continue;
                            }
                        }
                    },
                    "AddCarbonOffset" => match action.offset_type {
                        Some(offset_type) => GridAction::AddCarbonOffset(offset_type),
                        None => {
                            println!("Missing offset type for AddCarbonOffset action");
                            continue;
                        }
                    },
                    "CloseGenerator" => match action.generator_id {
                        Some(id) => GridAction::CloseGenerator(id),
                        None => {
                            println!("Missing generator ID for CloseGenerator action");
                            continue;
                        }
                    },
                    _ => {
                        println!("Unknown action type: {}", action.action_type);
                        continue;
                    }
                };
                year_weights.insert(grid_action, weight);
            }
            (year, year_weights)
        }).collect();

        let best_weights = serializable.best_weights.map(|weights| {
            weights.into_iter().map(|(year, actions)| {
                let mut year_weights = HashMap::new();
                for (action, weight) in actions {
                    let grid_action = match action.action_type.as_str() {
                        "AddGenerator" => {
                            match action.generator_type {
                                Some(gen_type) => match GeneratorType::from_str(&gen_type) {
                                    Ok(gen) => GridAction::AddGenerator(gen),
                                    Err(e) => {
                                        println!("Error parsing generator type '{}': {}", gen_type, e);
                                        continue;
                                    }
                                },
                                None => {
                                    println!("Missing generator type for AddGenerator action");
                                    continue;
                                }
                            }
                        },
                        "UpgradeEfficiency" => match action.generator_id {
                            Some(id) => GridAction::UpgradeEfficiency(id),
                            None => {
                                println!("Missing generator ID for UpgradeEfficiency action");
                                continue;
                            }
                        },
                        "AdjustOperation" => {
                            match (action.generator_id, action.operation_percentage) {
                                (Some(id), Some(percentage)) => GridAction::AdjustOperation(id, percentage),
                                _ => {
                                    println!("Missing generator ID or percentage for AdjustOperation action");
                                    continue;
                                }
                            }
                        },
                        "AddCarbonOffset" => match action.offset_type {
                            Some(offset_type) => GridAction::AddCarbonOffset(offset_type),
                            None => {
                                println!("Missing offset type for AddCarbonOffset action");
                                continue;
                            }
                        },
                        "CloseGenerator" => match action.generator_id {
                            Some(id) => GridAction::CloseGenerator(id),
                            None => {
                                println!("Missing generator ID for CloseGenerator action");
                                continue;
                            }
                        },
                        _ => {
                            println!("Unknown action type: {}", action.action_type);
                            continue;
                        }
                    };
                    year_weights.insert(grid_action, weight);
                }
                (year, year_weights)
            }).collect()
        });

        Ok(Self {
            weights,
            action_count_weights: HashMap::new(),
            learning_rate: serializable.learning_rate,
            best_metrics: serializable.best_metrics,
            best_weights,
            iteration_count: serializable.iteration_count,
            exploration_rate: serializable.exploration_rate,
        })
    }
    
    // Get a reference to the weights for a specific year
    pub fn get_year_weights(&self, year: u32) -> Option<&HashMap<GridAction, f64>> {
        self.weights.get(&year)
    }
    
    // Print the top N actions for a given year
    pub fn print_top_actions(&self, year: u32, n: usize) {
        if let Some(year_weights) = self.weights.get(&year) {
            let mut actions: Vec<_> = year_weights.iter().collect();
            actions.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
            
            println!("\nTop {} actions for year {}:", n, year);
            for (i, (action, weight)) in actions.iter().take(n).enumerate() {
                println!("{}. {:?}: {:.3}", i + 1, action, weight);
            }
        }
    }

    pub fn update_weights_from(&mut self, other: &ActionWeights) {
        // Merge weights with exponential moving average
        let alpha = 0.3; // Learning rate for weight merging
        
        for (year, year_weights) in &mut self.weights {
            if let Some(other_year_weights) = other.weights.get(year) {
                for (action, weight) in year_weights.iter_mut() {
                    if let Some(other_weight) = other_year_weights.get(action) {
                        *weight = *weight * (1.0 - alpha) + other_weight * alpha;
                    }
                }
            }
        }
        
        // Update best metrics if the other instance has better results
        if let Some(other_metrics) = &other.best_metrics {
            if let Some(current_metrics) = &self.best_metrics {
                if score_metrics(other_metrics) > score_metrics(current_metrics) {
                    self.best_metrics = Some(other_metrics.clone());
                    self.best_weights = other.best_weights.clone();
                }
            } else {
                self.best_metrics = Some(other_metrics.clone());
                self.best_weights = other.best_weights.clone();
            }
        }
    }

    // New method to update action count weights
    pub fn update_action_count_weights(&mut self, year: u32, action_count: u32, improvement: f64) {
        if let Some(year_counts) = self.action_count_weights.get_mut(&year) {
            if let Some(weight) = year_counts.get_mut(&action_count) {
                // Similar to action weight updates
                let adjustment_factor = if improvement > 0.0 {
                    1.0 + (self.learning_rate * improvement)
                } else {
                    1.0 / (1.0 + (self.learning_rate * improvement.abs()))
                };
                
                *weight = (*weight * adjustment_factor).max(0.01).min(1.0);
                
                // Normalize weights
                let total: f64 = year_counts.values().sum();
                for w in year_counts.values_mut() {
                    *w /= total;
                }
            }
        }
    }

    // Updated method to sample number of actions
    pub fn sample_additional_actions(&self, year: u32) -> u32 {
        let mut rng = rand::thread_rng();
        
        if let Some(year_counts) = self.action_count_weights.get(&year) {
            // Use exploration rate to decide between exploration and exploitation
            if rng.gen::<f64>() < self.exploration_rate {
                // Explore: random number of actions
                rng.gen_range(0..=20)
            } else {
                // Exploit: weighted random selection
                let total: f64 = year_counts.values().sum();
                let mut random_value = rng.gen::<f64>() * total;
                
                for (count, weight) in year_counts {
                    random_value -= weight;
                    if random_value <= 0.0 {
                        return *count;
                    }
                }
                10 // Fallback value
            }
        } else {
            10 // Fallback value
        }
    }

    pub fn get_best_metrics(&self) -> Option<(f64, bool)> {
        self.best_metrics.as_ref().map(|metrics| {
            (score_metrics(metrics), metrics.final_net_emissions <= 0.0)
        })
    }

    pub fn get_simulation_metrics(&self) -> Option<&SimulationMetrics> {
        self.best_metrics.as_ref()
    }
}

pub fn score_metrics(metrics: &SimulationMetrics) -> f64 {
    // If we haven't achieved net zero emissions, only consider emissions
    if metrics.final_net_emissions > 0.0 {
        // Normalize emissions to 0-1 scale and invert (0 emissions = 1.0 score)
        1.0 - (metrics.final_net_emissions / MAX_ACCEPTABLE_EMISSIONS).min(1.0)
    } else {
        // After achieving net zero, only consider public opinion
        metrics.average_public_opinion
    }
}

#[derive(Debug)]
pub struct ActionResult {
    pub net_emissions: f64,
    pub public_opinion: f64,
    pub power_balance: f64,
}

pub fn evaluate_action_impact(
    current_state: &ActionResult,
    new_state: &ActionResult,
) -> f64 {
    // Calculate immediate impact score
    let immediate_score = if current_state.net_emissions > 0.0 {
        // If we haven't achieved net zero, only consider emissions
        let emissions_improvement = (current_state.net_emissions - new_state.net_emissions) / 
                                  current_state.net_emissions.abs().max(1.0);
        emissions_improvement
    } else {
        // If we've achieved net zero, only consider opinion
        (new_state.public_opinion - current_state.public_opinion) /
        current_state.public_opinion.abs().max(1.0)
    };

    // The final 2050 impact is handled through the SimulationMetrics scoring
    // in update_best_strategy, which affects 70% of the weight updates
    
    // Return immediate score which affects 30% of weight updates
    immediate_score
} 