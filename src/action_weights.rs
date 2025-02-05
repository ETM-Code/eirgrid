use std::collections::HashMap;
use rand::Rng;
use serde::{Serialize, Deserialize};
use crate::generator::GeneratorType;
use crate::constants::{MAX_ACCEPTABLE_EMISSIONS, MAX_ACCEPTABLE_COST};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionWeights {
    // Map of year to action weights
    weights: HashMap<u32, HashMap<GridAction, f64>>,
    learning_rate: f64,
    // New fields for optimization
    best_metrics: Option<SimulationMetrics>,
    best_weights: Option<HashMap<u32, HashMap<GridAction, f64>>>,
    iteration_count: u32,
    exploration_rate: f64,
}

impl ActionWeights {
    pub fn new() -> Self {
        let mut weights = HashMap::new();
        
        // Initialize weights for each year from 2025 to 2050
        for year in 2025..=2050 {
            let mut year_weights = HashMap::new();
            
            // Initialize generator addition weights
            year_weights.insert(GridAction::AddGenerator(GeneratorType::OnshoreWind), 0.1);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::OffshoreWind), 0.1);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::UtilitySolar), 0.1);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::Nuclear), 0.1);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::GasCombinedCycle), 0.1);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::HydroDam), 0.1);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::PumpedStorage), 0.1);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::TidalGenerator), 0.1);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::WaveEnergy), 0.1);
            
            // Initialize other action weights
            year_weights.insert(GridAction::UpgradeEfficiency(String::new()), 0.05);
            year_weights.insert(GridAction::AdjustOperation(String::new(), 0), 0.05);
            year_weights.insert(GridAction::AddCarbonOffset(String::new()), 0.05);
            year_weights.insert(GridAction::CloseGenerator(String::new()), 0.05);
            
            // Add year's weights to the map
            weights.insert(year, year_weights);
        }
        
        Self {
            weights,
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
            let random_idx = rng.gen_range(0..actions.len());
            return actions[random_idx].clone();
        }

        // Exploitation - weighted selection
        let total_weight: f64 = year_weights.values().sum();
        let mut random_val = rng.gen::<f64>() * total_weight;
        
        for (action, weight) in year_weights {
            random_val -= weight;
            if random_val <= 0.0 {
                return action.clone();
            }
        }
        
        // Fallback to first action (shouldn't happen with proper weights)
        year_weights.keys().next().unwrap().clone()
    }
    
    pub fn update_weights(&mut self, action: &GridAction, year: u32, improvement: f64) {
        let year_weights = self.weights.get_mut(&year).expect("Year weights not found");
        let current_weight = year_weights.get(action).unwrap();
        
        // Apply learning rate with momentum
        let momentum = 0.9;
        let new_weight = (current_weight * momentum + 
            self.learning_rate * improvement * (1.0 - momentum))
            .max(0.01)  // Ensure weight doesn't go too close to zero
            .min(0.85);  // Ensure weight doesn't dominate completely
        
        year_weights.insert(action.clone(), new_weight);
        
        // Normalize weights to sum to 1 for this year
        let total: f64 = year_weights.values().sum();
        for weight in year_weights.values_mut() {
            *weight /= total;
        }
    }

    pub fn update_best_strategy(&mut self, metrics: SimulationMetrics) {
        let should_update = match &self.best_metrics {
            None => true,
            Some(best) => {
                // Complex scoring function considering multiple factors
                let current_score = score_metrics(&metrics);
                let best_score = score_metrics(best);
                current_score > best_score
            }
        };

        if should_update {
            self.best_metrics = Some(metrics);
            self.best_weights = Some(self.weights.clone());
        }
    }

    pub fn load_best_strategy(&mut self) {
        if let Some(best_weights) = &self.best_weights {
            self.weights = best_weights.clone();
        }
    }
    
    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }
    
    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let weights: ActionWeights = serde_json::from_str(&content)?;
        Ok(weights)
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

    // New method to sample additional actions based on reinforcement learning parameters
    pub fn sample_additional_actions(&self, _year: u32) -> u32 {
        let mut rng = rand::thread_rng();
        let extra = self.iteration_count / 50;
        rng.gen_range(0..=20 + extra)
    }
}

pub fn score_metrics(metrics: &SimulationMetrics) -> f64 {
    // Normalize each metric to a 0-1 scale and weight them
    let emissions_score = 1.0 - (metrics.final_net_emissions / MAX_ACCEPTABLE_EMISSIONS).min(1.0);
    let opinion_score = metrics.average_public_opinion;
    let cost_score = 1.0 - (metrics.total_cost / MAX_ACCEPTABLE_COST).min(1.0);
    let reliability_score = metrics.power_reliability;
    
    // Weight the scores (adjust weights as needed)
    emissions_score * 0.4 +
    opinion_score * 0.2 +
    cost_score * 0.2 +
    reliability_score * 0.2
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
    year: u32,
) -> f64 {
    let emissions_weight = if year >= 2045 { 0.5 } else { 0.3 };
    let opinion_weight = 0.3;
    let power_weight = if year >= 2045 { 0.2 } else { 0.4 };
    
    let emissions_improvement = (current_state.net_emissions - new_state.net_emissions) / 
                              current_state.net_emissions.abs().max(1.0);
    let opinion_improvement = (new_state.public_opinion - current_state.public_opinion) /
                            current_state.public_opinion.abs().max(1.0);
    let power_improvement = if new_state.power_balance >= 0.0 {
        1.0
    } else {
        -1.0
    };
    
    emissions_weight * emissions_improvement +
    opinion_weight * opinion_improvement +
    power_weight * power_improvement
} 