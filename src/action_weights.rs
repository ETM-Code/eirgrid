use std::collections::HashMap;
use rand::Rng;
use serde::{Serialize, Deserialize};
use crate::generator::GeneratorType;
use crate::constants::{MAX_ACCEPTABLE_EMISSIONS, MAX_ACCEPTABLE_COST};
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
const DIVERGENCE_FOR_NEGATIVE_WEIGHT: f64 = 0.03; //The difference of improvement necessary for a negative weight

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum GridAction {
    AddGenerator(GeneratorType),
    UpgradeEfficiency(String),  // Generator ID
    AdjustOperation(String, u8),  // Generator ID, percentage (0-100)
    AddCarbonOffset(String),  // Offset type
    CloseGenerator(String),  // Generator ID
    DoNothing, // New no-op action
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
            GridAction::DoNothing => SerializableAction {
                action_type: "DoNothing".to_string(),
                generator_type: None,
                generator_id: None,
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
    best_actions: Option<HashMap<u32, Vec<SerializableAction>>>,
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
    best_actions: Option<HashMap<u32, Vec<GridAction>>>, // Store actions from the best run
    iteration_count: u32,
    exploration_rate: f64,
    current_run_actions: HashMap<u32, Vec<GridAction>>, // Track actions in the current run
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
            
            // Initialize DoNothing action weight (base value can be tuned)
            year_weights.insert(GridAction::DoNothing, 0.1);
            
            // Add year's weights to the map
            weights.insert(year, year_weights);

            // Initialize action count weights for this year with bias towards fewer actions
            let mut count_weights = HashMap::new();
            let decay_rate = 0.4; // Controls how quickly the probability decreases
            let mut total_weight = 0.0;
            
            // Calculate weights with exponential decay
            for count in 0..=20 {
                let weight = (-decay_rate * count as f64).exp();
                count_weights.insert(count, weight);
                total_weight += weight;
            }
            
            // Normalize weights to sum to 1.0
            for weight in count_weights.values_mut() {
                *weight /= total_weight;
            }
            
            action_count_weights.insert(year, count_weights);
        }
        
        Self {
            weights,
            action_count_weights,
            learning_rate: 0.1,
            best_metrics: None,
            best_weights: None,
            best_actions: None,
            iteration_count: 0,
            exploration_rate: 0.2,
            current_run_actions: HashMap::new(),
        }
    }

    pub fn start_new_iteration(&mut self) {
        self.iteration_count += 1;
        // Decay exploration rate over time
        self.exploration_rate = 0.2 * (1.0 / (1.0 + 0.1 * self.iteration_count as f64));
        // Clear actions from the previous run
        self.current_run_actions.clear();
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
        // Initialize DoNothing with a base weight
        year_weights.insert(GridAction::DoNothing, 0.1);
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
        
        // If this was a bad outcome, slightly increase weights of other actions.
        if combined_improvement < 0.0 {
            let boost_factor = 1.0 + (self.learning_rate * 0.1); // Small boost to alternatives
            for (other_action, weight) in year_weights.iter_mut() {
                if other_action != action {
                    *weight = (*weight * boost_factor).min(MAX_WEIGHT);
                }
            }
            // If we've achieved net zero but are suffering from high costs, further boost DoNothing.
            if self.best_metrics.as_ref().map(|m| m.final_net_emissions <= 0.0 && m.total_cost > MAX_ACCEPTABLE_COST * 8.0).unwrap_or(false) {
                if let Some(noop_weight) = year_weights.get_mut(&GridAction::DoNothing) {
                    *noop_weight = (*noop_weight * (1.0 + self.learning_rate * 0.2)).min(MAX_WEIGHT);
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
                let best_score = score_metrics(best);
                let improvement = ((current_score - best_score) / best_score * 100.0).abs();
                
                // Create a VERY visible message with details about the improvement
                println!("\n\n");
                println!("{}", "🌟".repeat(40));
                println!("{}", "=".repeat(80));
                println!("🎉🎉🎉  MAJOR STRATEGY IMPROVEMENT FOUND!  🎉🎉🎉");
                println!("{}", "=".repeat(80));
                println!("{}", "🌟".repeat(40));
                println!("\nScore improved by {:.2}%", improvement);
                println!("Previous best score: {:.4} → New best score: {:.4}", best_score, current_score);

                // Add more detailed metrics information with better formatting
                println!("\n📊 DETAILED METRICS COMPARISON:");
                
                // Net emissions comparison with appropriate emoji
                let emissions_change = metrics.final_net_emissions - best.final_net_emissions;
                let emissions_emoji = if emissions_change <= 0.0 { "✅" } else { "⚠️" };
                println!("  {} Net emissions: {:.2} → {:.2} ({:+.2})", 
                        emissions_emoji, best.final_net_emissions, metrics.final_net_emissions, emissions_change);
                
                // Cost comparison with appropriate emoji
                let cost_change = metrics.total_cost - best.total_cost;
                let cost_change_pct = (cost_change / best.total_cost) * 100.0;
                let cost_emoji = if cost_change <= 0.0 { "✅" } else { "⚠️" };
                println!("  {} Total cost: €{:.2}B → €{:.2}B ({:+.2}%, {:+.2}B)", 
                        cost_emoji, 
                        best.total_cost / 1_000_000_000.0, 
                        metrics.total_cost / 1_000_000_000.0,
                        cost_change_pct,
                        cost_change / 1_000_000_000.0);
                
                // Public opinion comparison with appropriate emoji
                let opinion_change = (metrics.average_public_opinion - best.average_public_opinion) * 100.0;
                let opinion_emoji = if opinion_change >= 0.0 { "✅" } else { "⚠️" };
                println!("  {} Public opinion: {:.1}% → {:.1}% ({:+.1}%)", 
                        opinion_emoji,
                        best.average_public_opinion * 100.0, 
                        metrics.average_public_opinion * 100.0,
                        opinion_change);
                
                // Power reliability comparison with appropriate emoji
                let reliability_change = (metrics.power_reliability - best.power_reliability) * 100.0;
                let reliability_emoji = if reliability_change >= 0.0 { "✅" } else { "⚠️" };
                println!("  {} Power reliability: {:.1}% → {:.1}% ({:+.1}%)", 
                        reliability_emoji,
                        best.power_reliability * 100.0, 
                        metrics.power_reliability * 100.0,
                        reliability_change);
                
                println!("{}", "=".repeat(80));
                println!("{}", "🌟".repeat(40));
                println!("\n");
            } else {
                // First successful strategy found - make this VERY visible too
                println!("\n\n");
                println!("{}", "🌟".repeat(40));
                println!("{}", "=".repeat(80));
                println!("🎉🎉🎉  FIRST SUCCESSFUL STRATEGY FOUND!  🎉🎉🎉");
                println!("{}", "=".repeat(80));
                println!("{}", "🌟".repeat(40));
                println!("\nInitial score: {:.4}", current_score);
                
                // Add detailed metrics for the first strategy
                println!("\n📊 INITIAL METRICS:");
                println!("  Net emissions: {:.2} tonnes", metrics.final_net_emissions);
                println!("  Total cost: €{:.2}B/year", metrics.total_cost / 1_000_000_000.0);
                println!("  Public opinion: {:.1}%", metrics.average_public_opinion * 100.0);
                println!("  Power reliability: {:.1}%", metrics.power_reliability * 100.0);
                
                println!("{}", "=".repeat(80));
                println!("{}", "🌟".repeat(40));
                println!("\n");
            }
            self.best_metrics = Some(metrics);
            self.best_weights = Some(self.weights.clone());
            // Store the current run's actions as the best actions
            self.best_actions = Some(self.current_run_actions.clone());
        } else {
            // If this was significantly worse than our best, apply contrast learning
            self.apply_contrast_learning(&metrics);
            
            // Also print a warning if significantly worse
            if let Some(best) = &self.best_metrics {
                let best_score = score_metrics(best);
                let deterioration = (best_score - current_score) / best_score;
                if deterioration > 0.1 { // More than 10% worse
                    println!("\nWarning: Current strategy performing poorly. Score: {:.4} (Best: {:.4}, {:.1}% worse)", 
                            current_score, best_score, deterioration * 100.0);
                }
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
            best_actions: self.best_actions.clone(),
            iteration_count: self.iteration_count,
            exploration_rate: self.exploration_rate,
            current_run_actions: HashMap::new(),
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
        let mut serializable_weights = HashMap::new();
        for (year, year_weights) in &self.weights {
            let mut serializable_year_weights = Vec::new();
            for (action, &weight) in year_weights {
                match action {
                    GridAction::AddGenerator(gen_type) => {
                        serializable_year_weights.push((
                            SerializableAction {
                                action_type: "AddGenerator".to_string(),
                                generator_type: Some(gen_type.to_string()),
                                generator_id: None,
                                operation_percentage: None,
                                offset_type: None,
                            },
                            weight,
                        ));
                    },
                    GridAction::UpgradeEfficiency(id) => {
                        serializable_year_weights.push((
                            SerializableAction {
                                action_type: "UpgradeEfficiency".to_string(),
                                generator_type: None,
                                generator_id: Some(id.clone()),
                                operation_percentage: None,
                                offset_type: None,
                            },
                            weight,
                        ));
                    },
                    GridAction::AdjustOperation(id, percentage) => {
                        serializable_year_weights.push((
                            SerializableAction {
                                action_type: "AdjustOperation".to_string(),
                                generator_type: None,
                                generator_id: Some(id.clone()),
                                operation_percentage: Some(*percentage),
                                offset_type: None,
                            },
                            weight,
                        ));
                    },
                    GridAction::AddCarbonOffset(offset_type) => {
                        serializable_year_weights.push((
                            SerializableAction {
                                action_type: "AddCarbonOffset".to_string(),
                                generator_type: None,
                                generator_id: None,
                                operation_percentage: None,
                                offset_type: Some(offset_type.clone()),
                            },
                            weight,
                        ));
                    },
                    GridAction::CloseGenerator(id) => {
                        serializable_year_weights.push((
                            SerializableAction {
                                action_type: "CloseGenerator".to_string(),
                                generator_type: None,
                                generator_id: Some(id.clone()),
                                operation_percentage: None,
                                offset_type: None,
                            },
                            weight,
                        ));
                    },
                    GridAction::DoNothing => {
                        serializable_year_weights.push((
                            SerializableAction {
                                action_type: "DoNothing".to_string(),
                                generator_type: None,
                                generator_id: None,
                                operation_percentage: None,
                                offset_type: None,
                            },
                            weight,
                        ));
                    },
                }
            }
            serializable_weights.insert(*year, serializable_year_weights);
        }
        
        // Convert best weights to serializable format
        let serializable_best_weights = self.best_weights.as_ref().map(|best_weights| {
            let mut serializable = HashMap::new();
            for (year, year_weights) in best_weights {
                let mut serializable_year_weights = Vec::new();
                for (action, &weight) in year_weights {
                    match action {
                        GridAction::AddGenerator(gen_type) => {
                            serializable_year_weights.push((
                                SerializableAction {
                                    action_type: "AddGenerator".to_string(),
                                    generator_type: Some(gen_type.to_string()),
                                    generator_id: None,
                                    operation_percentage: None,
                                    offset_type: None,
                                },
                                weight,
                            ));
                        },
                        GridAction::UpgradeEfficiency(id) => {
                            serializable_year_weights.push((
                                SerializableAction {
                                    action_type: "UpgradeEfficiency".to_string(),
                                    generator_type: None,
                                    generator_id: Some(id.clone()),
                                    operation_percentage: None,
                                    offset_type: None,
                                },
                                weight,
                            ));
                        },
                        GridAction::AdjustOperation(id, percentage) => {
                            serializable_year_weights.push((
                                SerializableAction {
                                    action_type: "AdjustOperation".to_string(),
                                    generator_type: None,
                                    generator_id: Some(id.clone()),
                                    operation_percentage: Some(*percentage),
                                    offset_type: None,
                                },
                                weight,
                            ));
                        },
                        GridAction::AddCarbonOffset(offset_type) => {
                            serializable_year_weights.push((
                                SerializableAction {
                                    action_type: "AddCarbonOffset".to_string(),
                                    generator_type: None,
                                    generator_id: None,
                                    operation_percentage: None,
                                    offset_type: Some(offset_type.clone()),
                                },
                                weight,
                            ));
                        },
                        GridAction::CloseGenerator(id) => {
                            serializable_year_weights.push((
                                SerializableAction {
                                    action_type: "CloseGenerator".to_string(),
                                    generator_type: None,
                                    generator_id: Some(id.clone()),
                                    operation_percentage: None,
                                    offset_type: None,
                                },
                                weight,
                            ));
                        },
                        GridAction::DoNothing => {
                            serializable_year_weights.push((
                                SerializableAction {
                                    action_type: "DoNothing".to_string(),
                                    generator_type: None,
                                    generator_id: None,
                                    operation_percentage: None,
                                    offset_type: None,
                                },
                                weight,
                            ));
                        },
                    }
                }
                serializable.insert(*year, serializable_year_weights);
            }
            serializable
        });
        
        // Convert best actions to serializable format
        let serializable_best_actions = self.best_actions.as_ref().map(|best_actions| {
            let mut serializable = HashMap::new();
            for (year, actions) in best_actions {
                let mut serializable_actions = Vec::new();
                for action in actions {
                    match action {
                        GridAction::AddGenerator(gen_type) => {
                            serializable_actions.push(
                                SerializableAction {
                                    action_type: "AddGenerator".to_string(),
                                    generator_type: Some(gen_type.to_string()),
                                    generator_id: None,
                                    operation_percentage: None,
                                    offset_type: None,
                                }
                            );
                        },
                        GridAction::UpgradeEfficiency(id) => {
                            serializable_actions.push(
                                SerializableAction {
                                    action_type: "UpgradeEfficiency".to_string(),
                                    generator_type: None,
                                    generator_id: Some(id.clone()),
                                    operation_percentage: None,
                                    offset_type: None,
                                }
                            );
                        },
                        GridAction::AdjustOperation(id, percentage) => {
                            serializable_actions.push(
                                SerializableAction {
                                    action_type: "AdjustOperation".to_string(),
                                    generator_type: None,
                                    generator_id: Some(id.clone()),
                                    operation_percentage: Some(*percentage),
                                    offset_type: None,
                                }
                            );
                        },
                        GridAction::AddCarbonOffset(offset_type) => {
                            serializable_actions.push(
                                SerializableAction {
                                    action_type: "AddCarbonOffset".to_string(),
                                    generator_type: None,
                                    generator_id: None,
                                    operation_percentage: None,
                                    offset_type: Some(offset_type.clone()),
                                }
                            );
                        },
                        GridAction::CloseGenerator(id) => {
                            serializable_actions.push(
                                SerializableAction {
                                    action_type: "CloseGenerator".to_string(),
                                    generator_type: None,
                                    generator_id: Some(id.clone()),
                                    operation_percentage: None,
                                    offset_type: None,
                                }
                            );
                        },
                        GridAction::DoNothing => {
                            serializable_actions.push(
                                SerializableAction {
                                    action_type: "DoNothing".to_string(),
                                    generator_type: None,
                                    generator_id: None,
                                    operation_percentage: None,
                                    offset_type: None,
                                }
                            );
                        },
                    }
                }
                serializable.insert(*year, serializable_actions);
            }
            serializable
        });

        let serializable = SerializableWeights {
            weights: serializable_weights,
            learning_rate: self.learning_rate,
            best_metrics: self.best_metrics.clone(),
            best_weights: serializable_best_weights,
            best_actions: serializable_best_actions,
            iteration_count: self.iteration_count,
            exploration_rate: self.exploration_rate,
        };
        
        let json = serde_json::to_string_pretty(&serializable)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        std::fs::write(path, json)?;
        
        Ok(())
    }
    
    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        let _lock = FILE_MUTEX.lock().map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to acquire file lock for loading: {}", e))
        })?;
        
        let json = std::fs::read_to_string(path)?;
        let serializable: SerializableWeights = serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        // Convert serializable weights to actual weights
        let mut weights = HashMap::new();
        for (year, serializable_year_weights) in &serializable.weights {
            let mut year_weights = HashMap::new();
            for (serializable_action, weight) in serializable_year_weights {
                let action = match serializable_action.action_type.as_str() {
                    "AddGenerator" => {
                        if let Some(gen_type_str) = &serializable_action.generator_type {
                            let gen_type = GeneratorType::from_str(gen_type_str)
                                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                            GridAction::AddGenerator(gen_type)
                        } else {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "AddGenerator action missing generator_type",
                            ));
                        }
                    },
                    "UpgradeEfficiency" => {
                        if let Some(id) = &serializable_action.generator_id {
                            GridAction::UpgradeEfficiency(id.clone())
                        } else {
                            GridAction::UpgradeEfficiency(String::new())
                        }
                    },
                    "AdjustOperation" => {
                        let id = serializable_action.generator_id.clone().unwrap_or_default();
                        let percentage = serializable_action.operation_percentage.unwrap_or(0);
                        GridAction::AdjustOperation(id, percentage)
                    },
                    "AddCarbonOffset" => {
                        if let Some(offset_type) = &serializable_action.offset_type {
                            GridAction::AddCarbonOffset(offset_type.clone())
                        } else {
                            GridAction::AddCarbonOffset("Forest".to_string())
                        }
                    },
                    "CloseGenerator" => {
                        if let Some(id) = &serializable_action.generator_id {
                            GridAction::CloseGenerator(id.clone())
                        } else {
                            GridAction::CloseGenerator(String::new())
                        }
                    },
                    "DoNothing" => GridAction::DoNothing,
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Unknown action type: {}", serializable_action.action_type),
                        ));
                    }
                };
                year_weights.insert(action, *weight);
            }
            weights.insert(*year, year_weights);
        }
        
        // Convert serializable best weights to actual best weights
        let best_weights = serializable.best_weights.map(|serializable_best_weights| {
            let mut best_weights = HashMap::new();
            for (year, serializable_year_weights) in &serializable_best_weights {
                let mut year_weights = HashMap::new();
                for (serializable_action, weight) in serializable_year_weights {
                    let action = match serializable_action.action_type.as_str() {
                        "AddGenerator" => {
                            if let Some(gen_type_str) = &serializable_action.generator_type {
                                match GeneratorType::from_str(gen_type_str) {
                                    Ok(gen_type) => GridAction::AddGenerator(gen_type),
                                    Err(_) => continue, // Skip invalid generator types
                                }
                            } else {
                                continue;
                            }
                        },
                        "UpgradeEfficiency" => {
                            GridAction::UpgradeEfficiency(serializable_action.generator_id.clone().unwrap_or_default())
                        },
                        "AdjustOperation" => {
                            let id = serializable_action.generator_id.clone().unwrap_or_default();
                            let percentage = serializable_action.operation_percentage.unwrap_or(0);
                            GridAction::AdjustOperation(id, percentage)
                        },
                        "AddCarbonOffset" => {
                            GridAction::AddCarbonOffset(serializable_action.offset_type.clone().unwrap_or_else(|| "Forest".to_string()))
                        },
                        "CloseGenerator" => {
                            GridAction::CloseGenerator(serializable_action.generator_id.clone().unwrap_or_default())
                        },
                        "DoNothing" => GridAction::DoNothing,
                        _ => continue, // Skip unknown action types
                    };
                    year_weights.insert(action, *weight);
                }
                best_weights.insert(*year, year_weights);
            }
            best_weights
        });
        
        // Convert serializable best actions to actual best actions
        let best_actions = serializable.best_actions.map(|serializable_best_actions| {
            let mut best_actions = HashMap::new();
            for (year, serializable_actions) in &serializable_best_actions {
                let mut actions = Vec::new();
                for serializable_action in serializable_actions {
                    let action = match serializable_action.action_type.as_str() {
                        "AddGenerator" => {
                            if let Some(gen_type_str) = &serializable_action.generator_type {
                                match GeneratorType::from_str(gen_type_str) {
                                    Ok(gen_type) => GridAction::AddGenerator(gen_type),
                                    Err(_) => continue, // Skip invalid generator types
                                }
                            } else {
                                continue;
                            }
                        },
                        "UpgradeEfficiency" => {
                            GridAction::UpgradeEfficiency(serializable_action.generator_id.clone().unwrap_or_default())
                        },
                        "AdjustOperation" => {
                            let id = serializable_action.generator_id.clone().unwrap_or_default();
                            let percentage = serializable_action.operation_percentage.unwrap_or(0);
                            GridAction::AdjustOperation(id, percentage)
                        },
                        "AddCarbonOffset" => {
                            GridAction::AddCarbonOffset(serializable_action.offset_type.clone().unwrap_or_else(|| "Forest".to_string()))
                        },
                        "CloseGenerator" => {
                            GridAction::CloseGenerator(serializable_action.generator_id.clone().unwrap_or_default())
                        },
                        "DoNothing" => GridAction::DoNothing,
                        _ => continue, // Skip unknown action types
                    };
                    actions.push(action);
                }
                best_actions.insert(*year, actions);
            }
            best_actions
        });

        Ok(Self {
            weights,
            action_count_weights: HashMap::new(),
            learning_rate: serializable.learning_rate,
            best_metrics: serializable.best_metrics,
            best_weights,
            best_actions,
            iteration_count: serializable.iteration_count,
            exploration_rate: serializable.exploration_rate,
            current_run_actions: HashMap::new(),
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
                    self.best_actions = other.best_actions.clone();
                }
            } else {
                self.best_metrics = Some(other_metrics.clone());
                self.best_weights = other.best_weights.clone();
                self.best_actions = other.best_actions.clone();
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

    // Add a method to record an action in the current run
    pub fn record_action(&mut self, year: u32, action: GridAction) {
        self.current_run_actions.entry(year)
            .or_insert_with(Vec::new)
            .push(action);
    }

    // Method to apply contrast learning - penalize actions that differ from the best run
    pub fn apply_contrast_learning(&mut self, current_metrics: &SimulationMetrics) {
        // Only apply contrast learning if we have a best run to compare against
        if let (Some(best_metrics), Some(best_actions)) = (&self.best_metrics, &self.best_actions) {
            let best_score = score_metrics(best_metrics);
            let current_score = score_metrics(current_metrics);
            
            // Calculate how much worse the current run is compared to the best
            let deterioration = if best_score > 0.0 {
                (best_score - current_score) / best_score
            } else {
                0.0
            };
            
            // Only apply contrast learning if the current run is significantly worse (>5%)
            if deterioration > DIVERGENCE_FOR_NEGATIVE_WEIGHT {
                // Log the contrast learning application
                println!("\n🔄 Applying contrast learning - current run is {:.1}% worse than best", deterioration * 100.0);
                
                // Calculate the penalty factor - more severe for worse runs
                let penalty_factor = 1.0 / (1.0 + self.learning_rate * 2.0 * deterioration);
                
                // For each year, compare actions in the current run with the best run
                for (year, best_year_actions) in best_actions {
                    let current_year_actions = self.current_run_actions.get(year).cloned().unwrap_or_default();
                    
                    // Identify actions in the current run that differ from the best run
                    if let Some(year_weights) = self.weights.get_mut(year) {
                        // For each action in the current year
                        for (i, current_action) in current_year_actions.iter().enumerate() {
                            // Check if this action differs from the corresponding action in the best run
                            let differs = if i < best_year_actions.len() {
                                current_action != &best_year_actions[i]
                            } else {
                                // Extra actions not present in the best run - penalize
                                true
                            };
                            
                            if differs {
                                // Apply penalty to the different action
                                if let Some(weight) = year_weights.get_mut(current_action) {
                                    *weight = (*weight * penalty_factor).max(MIN_WEIGHT);
                                }
                                
                                // Boost the corresponding best action if available
                                if i < best_year_actions.len() {
                                    let best_action = &best_year_actions[i];
                                    if let Some(weight) = year_weights.get_mut(best_action) {
                                        *weight = (*weight * (1.0 + self.learning_rate * deterioration)).min(MAX_WEIGHT);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Get the best actions for a particular year
    pub fn get_best_actions_for_year(&self, year: u32) -> Option<&Vec<GridAction>> {
        self.best_actions.as_ref().and_then(|actions| actions.get(&year))
    }
    
    // Check if we have best actions available to replay
    pub fn has_best_actions(&self) -> bool {
        self.best_actions.is_some() && !self.best_actions.as_ref().unwrap().is_empty()
    }
}

pub fn score_metrics(metrics: &SimulationMetrics) -> f64 {
    // First priority: Reach net zero emissions
    if metrics.final_net_emissions > 0.0 {
        // If we haven't achieved net zero, only focus on reducing emissions
        1.0 - (metrics.final_net_emissions / MAX_ACCEPTABLE_EMISSIONS).min(1.0)
    }
    // Second priority: Optimize costs after achieving net zero
    else {
        // Base score of 1.0 for achieving net zero
        let base_score = 1.0;
        
        // Cost component - normalized and inverted so lower costs give higher scores
        // Use log scale to differentiate between very high costs
        let normalized_cost = (metrics.total_cost / MAX_ACCEPTABLE_COST).max(1.0);
        let log_cost = normalized_cost.ln();
        let max_expected_log_cost = (MAX_ACCEPTABLE_COST * 100.0 / MAX_ACCEPTABLE_COST).ln(); // Assume 100x budget is max
        let cost_score = 1.0 - (log_cost / max_expected_log_cost).min(1.0);
        
        // Public opinion component
        let opinion_score = metrics.average_public_opinion;
        
        // Combine scores with appropriate weights
        // Cost is higher priority until it's reasonable
        let cost_weight = if normalized_cost > 8.0 { 0.8 } else { 0.5 };
        let opinion_weight = 1.0 - cost_weight;
        
        base_score + (cost_score * cost_weight + opinion_score * opinion_weight)
    }
}

#[derive(Debug)]
pub struct ActionResult {
    pub net_emissions: f64,
    pub public_opinion: f64,
    pub power_balance: f64,
    pub total_cost: f64,
}

pub fn evaluate_action_impact(
    current_state: &ActionResult,
    new_state: &ActionResult,
) -> f64 {
    // Calculate immediate impact score based on priorities
    if current_state.net_emissions > 0.0 {
        // First priority: If we haven't achieved net zero, only consider emissions
        let emissions_improvement = (current_state.net_emissions - new_state.net_emissions) / 
                                  current_state.net_emissions.abs().max(1.0);
        emissions_improvement
    }
    else {
        // If we've achieved net zero, consider both cost and opinion improvements
        
        // Cost improvement (negative is better)
        let cost_change = new_state.total_cost - current_state.total_cost;
        let cost_improvement = -cost_change / current_state.total_cost.abs().max(1.0);
        
        // Opinion improvement
        let opinion_improvement = (new_state.public_opinion - current_state.public_opinion) /
                                current_state.public_opinion.abs().max(1.0);
        
        // Weight cost more heavily if it's very high
        let cost_weight = if current_state.total_cost > MAX_ACCEPTABLE_COST * 8.0 { 0.8 } else { 0.5 };
        let opinion_weight = 1.0 - cost_weight;
        
        // Combined improvement score
        cost_improvement * cost_weight + opinion_improvement * opinion_weight
    }
}
