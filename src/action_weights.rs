use std::collections::HashMap;
use rand::Rng;
use rand::rngs::StdRng;
use rand::SeedableRng;
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
const MIN_WEIGHT: f64 = 0.0001;  // Ensure weight doesn't go too close to zero
const MAX_WEIGHT: f64 = 0.999;  // Ensure weight doesn't dominate completely
const DIVERGENCE_FOR_NEGATIVE_WEIGHT: f64 = 0.03; // The difference of improvement necessary for a negative weight
const DIVERGENCE_EXPONENT: f64 = 0.3; // How rapidly to increase penalty with worse divergence (lower = more severe for values < 1)
const STAGNATION_PENALTY_FACTOR: f64 = 0.2; // Base factor for stagnation penalty
const STAGNATION_EXPONENT: f64 = 1.8; // How rapidly to increase penalty with more iterations without improvement
const MATCHING_ACTION_REWARD: f64 = 0.1; // Additional reward factor for actions that match the best strategy
const FORCE_REPLAY_THRESHOLD: u32 = 1000; // After this many iterations without improvement, start forcing replay
const ITERATIONS_FOR_RANDOMIZATION: u32 = 1000; // After this many iterations without improvement, apply randomization

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
    iterations_without_improvement: u32,
    exploration_rate: f64,
    deficit_weights: HashMap<u32, Vec<(SerializableAction, f64)>>,
    best_deficit_actions: Option<HashMap<u32, Vec<SerializableAction>>>,
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
    iterations_without_improvement: u32, // Track iterations without improvement
    exploration_rate: f64,
    current_run_actions: HashMap<u32, Vec<GridAction>>, // Track actions in the current run
    force_best_actions: bool, // Flag to force replay of best actions
    deficit_weights: HashMap<u32, HashMap<GridAction, f64>>, // Store weights specifically for deficit handling
    current_deficit_actions: HashMap<u32, Vec<GridAction>>, // Track deficit actions in the current run
    best_deficit_actions: Option<HashMap<u32, Vec<GridAction>>>, // Store deficit actions from the best run
    deterministic_rng: Option<StdRng>, // Optional deterministic RNG for reproducible runs
    guaranteed_best_actions: bool, // Flag to force replay of best actions with 100% probability
}

impl ActionWeights {
    pub fn new() -> Self {
        let mut weights = HashMap::new();
        let mut action_count_weights = HashMap::new();
        let mut deficit_weights = HashMap::new();
        
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

            // Initialize deficit handling weights with a separate set of weights
            // focused on reliable power generation options
            let mut deficit_year_weights = HashMap::new();
            
            // For deficit handling, prioritize fast-responding and reliable generators
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::GasPeaker), 0.15);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::GasCombinedCycle), 0.15);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::BatteryStorage), 0.15);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::PumpedStorage), 0.10);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::Biomass), 0.10);
            
            // Include renewables with lower initial weights for deficit handling
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::OnshoreWind), 0.07);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::OffshoreWind), 0.07);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::UtilitySolar), 0.06);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::HydroDam), 0.06);
            
            // Include nuclear with a lower weight due to long build time
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::Nuclear), 0.05);
            
            // Add other types with minimal weights
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::DomesticSolar), 0.01);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::CommercialSolar), 0.01);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::TidalGenerator), 0.01);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::WaveEnergy), 0.01);
            
            // DoNothing should have very low weight for deficit handling
            deficit_year_weights.insert(GridAction::DoNothing, 0.001);
            
            // Add deficit weights for this year
            deficit_weights.insert(year, deficit_year_weights);

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
            iterations_without_improvement: 0,
            exploration_rate: 0.2,
            current_run_actions: HashMap::new(),
            force_best_actions: false,
            deficit_weights,
            current_deficit_actions: HashMap::new(),
            best_deficit_actions: None,
            deterministic_rng: None,
            guaranteed_best_actions: false,
        }
    }

    pub fn set_rng(&mut self, rng: StdRng) {
        self.deterministic_rng = Some(rng);
    }
    
    pub fn start_new_iteration(&mut self) {
        self.iteration_count += 1;
        // Decay exploration rate over time
        self.exploration_rate = 0.2 * (1.0 / (1.0 + 0.1 * self.iteration_count as f64));
        // Clear actions from the previous run
        self.current_run_actions.clear();
        self.current_deficit_actions.clear();
        
        // Don't override force_best_actions if guaranteed_best_actions is set
        if self.guaranteed_best_actions {
            self.force_best_actions = true;
            return;
        }
        
        // Add logging for debugging if we have many iterations without improvement
        if self.iterations_without_improvement > 0 {
            if self.iterations_without_improvement % 10 == 0 {
                println!("⚠️ Currently at {} iterations without improvement", self.iterations_without_improvement);
            }
            
            // Occasionally restore best weights when stuck for a long time
            if self.iterations_without_improvement > 800 && self.iterations_without_improvement % 100 == 0 {
                if let Some(best_weights) = &self.best_weights {
                    println!("🔄 RESTORING BEST WEIGHTS after {} iterations without improvement", 
                            self.iterations_without_improvement);
                    // Create a partial copy of the best weights (75%) mixed with current weights (25%)
                    self.restore_best_weights(0.75);
                }
            }
            
            // Calculate probability of forcing replay based on stagnation
            if self.iterations_without_improvement > FORCE_REPLAY_THRESHOLD {
                let force_replay_probability = ((self.iterations_without_improvement - FORCE_REPLAY_THRESHOLD) as f64 / 500.0).min(0.9);
                
                let random_val = match &mut self.deterministic_rng {
                    Some(rng) => rng.gen::<f64>(),
                    None => rand::thread_rng().gen::<f64>(),
                };
                
                if random_val < force_replay_probability {
                    println!("🔄 FORCE REPLAY: Directly using best known actions (probability: {:.1}%)", 
                            force_replay_probability * 100.0);
                    self.force_best_actions = true;
                    return;
                }
            }
        }
        
        // Default behavior
        self.force_best_actions = false;
    }
    
    pub fn sample_action(&mut self, year: u32) -> GridAction {
        // If we're forcing replay of best actions and we have them, use those
        if self.force_best_actions {
            if let Some(best_actions) = &self.best_actions {
                if let Some(year_actions) = best_actions.get(&year) {
                    let current_index = self.current_run_actions.get(&year).map_or(0, |v| v.len());
                    if current_index < year_actions.len() {
                        let action = year_actions[current_index].clone();
                        println!("🔄 REPLAY: Using best action #{} for year {}: {:?}", 
                                current_index + 1, year, action);
                        
                        // Make sure to record the replayed action in the current run
                        self.current_run_actions.entry(year)
                            .or_insert_with(Vec::new)
                            .push(action.clone());
                        
                        return action;
                    } else {
                        println!("⚠️ REPLAY FALLBACK: Ran out of best actions for year {} (needed action #{}, have {})", 
                                year, current_index + 1, year_actions.len());
                        
                        // Add smart fallback for when we run out of actions
                        let fallback_action = self.generate_smart_fallback_action(year, "ran out of best actions");
                        
                        // Also record this fallback action in the current run
                        self.current_run_actions.entry(year)
                            .or_insert_with(Vec::new)
                            .push(fallback_action.clone());
                        
                        return fallback_action;
                    }
                } else {
                    println!("⚠️ REPLAY FALLBACK: No best actions recorded for year {}", year);
                    
                    // Add smart fallback for when no actions exist for this year
                    let fallback_action = self.generate_smart_fallback_action(year, "no best actions for year");
                    
                    // Also record this fallback action in the current run
                    self.current_run_actions.entry(year)
                        .or_insert_with(Vec::new)
                        .push(fallback_action.clone());
                    
                    return fallback_action;
                }
            } else {
                println!("⚠️ REPLAY FALLBACK: No best actions recorded for any year");
                
                // Add smart fallback for when no best actions exist at all
                let fallback_action = self.generate_smart_fallback_action(year, "no best actions at all");
                
                // Also record this fallback action in the current run
                self.current_run_actions.entry(year)
                    .or_insert_with(Vec::new)
                    .push(fallback_action.clone());
                
                return fallback_action;
            }
        }
        
        // Normal action selection logic
        let year_weights = self.weights.get(&year).expect("Year weights not found");

        // Calculate a dynamic exploration rate that decreases when we're stuck
        let current_exploration = if self.iterations_without_improvement > 100 {
            // Reduce exploration drastically after being stuck for a while to focus on best known actions
            self.exploration_rate * (1.0 / (1.0 + 0.01 * self.iterations_without_improvement as f64))
        } else {
            self.exploration_rate
        };

        // Determine if we should explore based on the deterministic RNG or thread_rng
        let should_explore = match &mut self.deterministic_rng {
            Some(rng) => rng.gen::<f64>() < current_exploration,
            None => rand::thread_rng().gen::<f64>() < current_exploration,
        };

        // Epsilon-greedy exploration with dynamic rate
        if should_explore {
            // Random exploration
            let actions: Vec<_> = year_weights.keys().collect();
            if actions.is_empty() {
                // Fallback to a safe default action if no actions are available
                return GridAction::AddGenerator(GeneratorType::GasPeaker);
            }
            
            let random_idx = match &mut self.deterministic_rng {
                Some(rng) => rng.gen_range(0..actions.len()),
                None => rand::thread_rng().gen_range(0..actions.len()),
            };
            
            return actions[random_idx].clone();
        }

        // Exploitation - weighted selection
        let total_weight: f64 = year_weights.values().sum();
        if total_weight <= 0.0 {
            // If all weights are zero or negative, fall back to a safe default
            return GridAction::AddGenerator(GeneratorType::GasPeaker);
        }

        // When stuck for many iterations, use a more aggressive selection strategy
        // by applying a power scaling to the weights, making higher weights even more likely
        if self.iterations_without_improvement > 500 {
            // Extract actions and weights
            let mut actions_with_weights: Vec<_> = year_weights.iter().collect();
            // Sort by weight in descending order
            actions_with_weights.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
            
            // Calculate how aggressive the selection should be based on stagnation
            let stagnation_factor = (self.iterations_without_improvement as f64 / 1000.0).min(1.0);
            let power_scaling = 1.0 + (2.0 * stagnation_factor); // Ranges from 1.0 to 3.0
            
            // Use exponentiated weights for selection
            let total_scaled_weight: f64 = actions_with_weights.iter()
                .map(|(_, &w)| w.powf(power_scaling))
                .sum();
            
            let mut random_val = match &mut self.deterministic_rng {
                Some(rng) => rng.gen::<f64>() * total_scaled_weight,
                None => rand::thread_rng().gen::<f64>() * total_scaled_weight,
            };
            
            for (action, &weight) in &actions_with_weights {
                let scaled_weight = weight.powf(power_scaling);
                random_val -= scaled_weight;
                if random_val <= 0.0 {
                    return (*action).clone();
                }
            }
            
            // Fallback to the highest weight action
            return actions_with_weights.first().map(|(a, _)| (*a).clone())
                .unwrap_or(GridAction::AddGenerator(GeneratorType::GasPeaker));
        } else {
            // Standard weighted selection for normal operation
            let mut random_val = match &mut self.deterministic_rng {
                Some(rng) => rng.gen::<f64>() * total_weight,
                None => rand::thread_rng().gen::<f64>() * total_weight,
            };
            
            for (action, weight) in year_weights {
                random_val -= weight;
                if random_val <= 0.0 {
                    return action.clone();
                }
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
        
        // Debug: Print current_run_actions info with more detailed breakdown
        let total_curr_actions = self.current_run_actions.values().map(|v| v.len()).sum::<usize>();
        let years_with_curr_actions = self.current_run_actions.values().filter(|v| !v.is_empty()).count();
        println!("DEBUG: Before update - Current run has {} actions across {} years", 
                total_curr_actions, years_with_curr_actions);
        
        // More detailed per-year breakdown for the current run
        println!("Current run actions per year:");
        for year in 2025..=2050 {
            if let Some(actions) = self.current_run_actions.get(&year) {
                if !actions.is_empty() {
                    println!("  Year {}: {} actions", year, actions.len());
                }
            }
        }
        
        // If we have empty current_run_actions but non-empty best actions, something's wrong
        if total_curr_actions == 0 && self.best_actions.is_some() {
            println!("⚠️ WARNING: Attempting to update best strategy with 0 actions in current run!");
            println!("This suggests actions aren't being recorded properly during simulation");
        }
        
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
                println!("Found after {} iterations without improvement", self.iterations_without_improvement);

                // Add more detailed metrics information with better formatting
                println!("\n📊 DETAILED METRICS COMPARISON:");
                
                // Net emissions comparison with appropriate emoji
                let emissions_change = metrics.final_net_emissions - best.final_net_emissions;
                let emissions_emoji = if emissions_change <= 0.0 { "✅" } else { "⚠️" };
                println!("  {} Net emissions: {:.2} → {:.2} ({:+.2})", 
                        emissions_emoji, best.final_net_emissions, metrics.final_net_emissions, emissions_change);
                
                // Net zero status comparison
                let old_net_zero = best.final_net_emissions <= 0.0;
                let new_net_zero = metrics.final_net_emissions <= 0.0;
                let net_zero_emoji = if new_net_zero { "✅" } else { "⚠️" };
                println!("  {} Net zero: {} → {}", 
                        net_zero_emoji, 
                        if old_net_zero { "YES" } else { "NO" }, 
                        if new_net_zero { "YES" } else { "NO" });
                
                // Total cost comparison
                let cost_change = metrics.total_cost - best.total_cost;
                let cost_emoji = if cost_change <= 0.0 { "✅" } else { "⚠️" };
                println!("  {} Total cost: €{:.2}B → €{:.2}B ({:+.2}B)", 
                        cost_emoji, 
                        best.total_cost / 1_000_000_000.0, 
                        metrics.total_cost / 1_000_000_000.0, 
                        cost_change / 1_000_000_000.0);
                
                // Public opinion comparison
                let opinion_change = metrics.average_public_opinion - best.average_public_opinion;
                let opinion_emoji = if opinion_change >= 0.0 { "✅" } else { "⚠️" };
                println!("  {} Public opinion: {:.1}% → {:.1}% ({:+.1}%)", 
                        opinion_emoji, 
                        best.average_public_opinion * 100.0, 
                        metrics.average_public_opinion * 100.0, 
                        opinion_change * 100.0);
                
                // Power reliability comparison
                let reliability_change = metrics.power_reliability - best.power_reliability;
                let reliability_emoji = if reliability_change >= 0.0 { "✅" } else { "⚠️" };
                println!("  {} Power reliability: {:.1}% → {:.1}% ({:+.1}%)", 
                        reliability_emoji, 
                        best.power_reliability * 100.0, 
                        metrics.power_reliability * 100.0, 
                        reliability_change * 100.0);
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
                
                // Log information about the actions being recorded for the first time
                let total_actions = self.current_run_actions.values().map(|v| v.len()).sum::<usize>();
                let deficit_actions = self.current_deficit_actions.values().map(|v| v.len()).sum::<usize>();
                println!("Recording first strategy with {} regular actions and {} deficit actions", 
                        total_actions, deficit_actions);
            }
            
            self.best_metrics = Some(metrics);
            self.best_weights = Some(self.weights.clone());
            
            // Make sure we have entries for each year even if they're empty
            let mut complete_actions = HashMap::new();
            let mut complete_deficit_actions = HashMap::new();
            
            // Initialize empty action lists for all years
            for year in 2025..=2050 {
                complete_actions.insert(year, Vec::new());
                complete_deficit_actions.insert(year, Vec::new());
            }
            
            // Then copy over any actions we actually have
            for (year, actions) in &self.current_run_actions {
                if !actions.is_empty() {
                    println!("DEBUG: Copying {} actions for year {} to best_actions", actions.len(), year);
                    complete_actions.insert(*year, actions.clone());
                }
            }
            
            for (year, actions) in &self.current_deficit_actions {
                if !actions.is_empty() {
                    println!("DEBUG: Copying {} deficit actions for year {} to best_deficit_actions", actions.len(), year);
                    complete_deficit_actions.insert(*year, actions.clone());
                }
            }
            
            // Debug: Check if we're actually capturing any actions
            let total_complete_actions = complete_actions.values().map(|v| v.len()).sum::<usize>();
            let years_with_complete_actions = complete_actions.values().filter(|v| !v.is_empty()).count();
            println!("DEBUG: Created best_actions map with {} actions across {} years", 
                    total_complete_actions, years_with_complete_actions);
            
            // More detailed per-year breakdown for complete_actions
            println!("Complete actions per year (to be stored as best):");
            for year in 2025..=2050 {
                if let Some(actions) = complete_actions.get(&year) {
                    if !actions.is_empty() {
                        println!("  Year {}: {} actions", year, actions.len());
                    }
                }
            }
            
            // Store the complete maps
            self.best_actions = Some(complete_actions);
            self.best_deficit_actions = Some(complete_deficit_actions);
            
            // Debug: Check the best_actions we just stored
            if let Some(ref best_actions) = self.best_actions {
                let total_best_actions = best_actions.values().map(|v| v.len()).sum::<usize>();
                let years_with_best_actions = best_actions.values().filter(|v| !v.is_empty()).count();
                println!("DEBUG: After update - best_actions now has {} actions across {} years", 
                        total_best_actions, years_with_best_actions);
                
                // Detailed per-year breakdown of best actions
                println!("Best actions per year after storage:");
                for year in 2025..=2050 {
                    if let Some(actions) = best_actions.get(&year) {
                        if !actions.is_empty() {
                            println!("  Year {}: {} best actions", year, actions.len());
                        }
                    }
                }
            }
            
            // Reset iterations without improvement counter when we find a better strategy
            self.iterations_without_improvement = 0;
        } else {
            // Track iterations without improvement
            self.iterations_without_improvement += 1;
            
            // Occasionally log if we have many iterations without improvement
            if self.iterations_without_improvement % 100 == 0 {
                println!("⏳ {} iterations without finding a better strategy", 
                        self.iterations_without_improvement);
            }
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
                serializable_year_weights.push((
                    SerializableAction::from(action),
                    weight,
                ));
            }
            serializable_weights.insert(*year, serializable_year_weights);
        }
        
        // Convert deficit weights to serializable format
        let mut serializable_deficit_weights = HashMap::new();
        for (year, year_weights) in &self.deficit_weights {
            let mut serializable_year_weights = Vec::new();
            for (action, &weight) in year_weights {
                serializable_year_weights.push((
                    SerializableAction::from(action),
                    weight,
                ));
            }
            serializable_deficit_weights.insert(*year, serializable_year_weights);
        }
        
        // Convert best weights to serializable format
        let serializable_best_weights = self.best_weights.as_ref().map(|best_weights| {
            let mut serializable = HashMap::new();
            for (year, year_weights) in best_weights {
                let mut serializable_year_weights = Vec::new();
                for (action, &weight) in year_weights {
                    serializable_year_weights.push((
                        SerializableAction::from(action),
                        weight,
                    ));
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
                    serializable_actions.push(SerializableAction::from(action));
                }
                serializable.insert(*year, serializable_actions);
            }
            serializable
        });

        // Convert best deficit actions to serializable format
        let serializable_best_deficit_actions = self.best_deficit_actions.as_ref().map(|best_actions| {
            let mut serializable = HashMap::new();
            for (year, actions) in best_actions {
                let mut serializable_actions = Vec::new();
                for action in actions {
                    serializable_actions.push(SerializableAction::from(action));
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
            iterations_without_improvement: self.iterations_without_improvement,
            exploration_rate: self.exploration_rate,
            deficit_weights: serializable_deficit_weights,
            best_deficit_actions: serializable_best_deficit_actions,
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
        
        // Convert serializable deficit weights to actual deficit weights
        let mut deficit_weights = HashMap::new();
        for (year, serializable_year_weights) in &serializable.deficit_weights {
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
            deficit_weights.insert(*year, year_weights);
        }
        
        // If no deficit weights were found in the file, initialize them with defaults
        if deficit_weights.is_empty() {
            for year in 2025..=2050 {
                let mut deficit_year_weights = HashMap::new();
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::GasPeaker), 0.15);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::GasCombinedCycle), 0.15);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::BatteryStorage), 0.15);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::PumpedStorage), 0.10);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::Biomass), 0.10);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::OnshoreWind), 0.07);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::OffshoreWind), 0.07);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::UtilitySolar), 0.06);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::HydroDam), 0.06);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::Nuclear), 0.05);
                deficit_year_weights.insert(GridAction::DoNothing, 0.001);
                deficit_weights.insert(year, deficit_year_weights);
            }
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

        // Convert serializable best deficit actions to actual best deficit actions
        let best_deficit_actions = serializable.best_deficit_actions.map(|serializable_best_actions| {
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
            iterations_without_improvement: serializable.iterations_without_improvement,
            exploration_rate: serializable.exploration_rate,
            current_run_actions: HashMap::new(),
            force_best_actions: false,
            deficit_weights,
            current_deficit_actions: HashMap::new(),
            best_deficit_actions,
            deterministic_rng: None,
            guaranteed_best_actions: false,
        })
    }
    #[allow(dead_code)]
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
        // Copy weights
        for (year, other_year_weights) in &other.weights {
            let year_weights = self.weights.entry(*year).or_insert_with(HashMap::new);
            for (action, weight) in other_year_weights {
                year_weights.insert(action.clone(), *weight);
            }
        }
        
        // Copy deficit weights
        for (year, other_deficit_weights) in &other.deficit_weights {
            let deficit_weights = self.deficit_weights.entry(*year).or_insert_with(HashMap::new);
            for (action, weight) in other_deficit_weights {
                deficit_weights.insert(action.clone(), *weight);
            }
        }
        
        // Copy action count weights
        for (year, other_count_weights) in &other.action_count_weights {
            let count_weights = self.action_count_weights.entry(*year).or_insert_with(HashMap::new);
            for (count, weight) in other_count_weights {
                count_weights.insert(*count, *weight);
            }
        }
        
        // Update performance counters
        self.iteration_count = std::cmp::max(self.iteration_count, other.iteration_count);
        
        // Transfer recorded actions from the other instance
        self.transfer_recorded_actions_from(other);
    }

    // New method to transfer recorded actions from another ActionWeights instance
    pub fn transfer_recorded_actions_from(&mut self, other: &ActionWeights) {
        // Clear our current run actions first
        self.current_run_actions.clear();
        
        // Copy all actions from the other instance's current run
        for (year, actions) in &other.current_run_actions {
            if !actions.is_empty() {
                // Clone the vector of actions for the current year
                self.current_run_actions.insert(*year, actions.clone());
            }
        }
        
        // Do the same for deficit actions
        self.current_deficit_actions.clear();
        for (year, actions) in &other.current_deficit_actions {
            if !actions.is_empty() {
                self.current_deficit_actions.insert(*year, actions.clone());
            }
        }
        
        // Debug output to verify actions were transferred
        let total_actions = self.current_run_actions.values().map(|v| v.len()).sum::<usize>();
        let years_with_actions = self.current_run_actions.values().filter(|v| !v.is_empty()).count();
        println!("DEBUG: Transferred {} actions across {} years from local weights", 
                 total_actions, years_with_actions);
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
    pub fn sample_additional_actions(&mut self, year: u32) -> u32 {
        let random_val = match &mut self.deterministic_rng {
            Some(rng) => rng.gen::<f64>(),
            None => rand::thread_rng().gen::<f64>(),
        };
        
        if let Some(year_counts) = self.action_count_weights.get(&year) {
            // Use weighted sampling based on historical data
            let total_weight: f64 = year_counts.values().sum();
            if total_weight <= 0.0 {
                return 0;
            }
            
            let mut random_choice = random_val * total_weight;
            
            for (count, weight) in year_counts {
                random_choice -= weight;
                if random_choice <= 0.0 {
                    return *count;
                }
            }
            
            // Fallback to a reasonable default if sampling fails
            return 5;
        } else {
            // Fallback to simple heuristic if no historical data
            let scaled_exploration = self.exploration_rate.powf(0.5); // Square root to increase base value
            let min_actions = (2.0 / scaled_exploration).round() as u32;
            let max_actions = (12.0 / scaled_exploration).round() as u32;
            
            match &mut self.deterministic_rng {
                Some(rng) => rng.gen_range(min_actions..=max_actions),
                None => rand::thread_rng().gen_range(min_actions..=max_actions),
            }
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
            
            // Only apply contrast learning if the current run is significantly worse (>3%)
            if deterioration > DIVERGENCE_FOR_NEGATIVE_WEIGHT {
                // Calculate stagnation penalty with exponential scaling
                // For stagnation, we want more iterations to have a stronger effect, so we use a power > 1
                let stagnation_iterations = self.iterations_without_improvement as f64 / 10.0;
                let stagnation_factor = 1.0 + (STAGNATION_PENALTY_FACTOR * stagnation_iterations.powf(STAGNATION_EXPONENT));
                
                // Fix the divergence scaling - for values between 0 and 1, using a power < 1 makes them larger
                // This ensures that worse divergence (higher values) results in stronger penalties
                let scaled_deterioration = deterioration.powf(DIVERGENCE_EXPONENT);
                
                // Calculate the combined penalty multiplier
                let combined_penalty = scaled_deterioration * stagnation_factor;
                
                // Enhanced adaptive learning rate based on stagnation and performance degradation
                let adaptive_learning_rate = self.learning_rate * (1.0 + 0.05 * self.iterations_without_improvement as f64);
                
                // Log the contrast learning application with more detailed information
                println!("\n🔄 Applying enhanced contrast learning:");
                println!("   - Current run is {:.1}% worse than best", deterioration * 100.0);
                println!("   - Iterations without improvement: {}", self.iterations_without_improvement);
                println!("   - Raw deterioration: {:.4}, Scaled: {:.4}", deterioration, scaled_deterioration);
                println!("   - Stagnation factor: {:.2}x", stagnation_factor);
                println!("   - Combined penalty multiplier: {:.4}", combined_penalty);
                println!("   - Adaptive learning rate: {:.4} (base: {:.4})", adaptive_learning_rate, self.learning_rate);
                
                // Calculate the penalty factor - more severe for worse runs and after more stagnation
                let penalty_factor = 1.0 / (1.0 + adaptive_learning_rate * 2.0 * combined_penalty);
                
                // Calculate the boost factor for best actions - increases with stagnation
                let best_boost_factor = 1.0 + (adaptive_learning_rate * 3.0 * stagnation_factor);
                
                println!("   - Penalty factor: {:.8}", penalty_factor);
                println!("   - Best action boost factor: {:.8}", best_boost_factor);
                
                // Debug - show an example of penalty effect on a typical weight
                let example_weight = 0.1;
                let penalized_weight = (example_weight * penalty_factor).max(MIN_WEIGHT);
                println!("   - Example: Weight of 0.1 becomes {:.8} after penalty", penalized_weight);
                
                // Track how many weights are at minimum value
                let mut min_weight_count = 0;
                let mut total_weights = 0;
                
                // For each year, compare actions in the current run with the best run
                for (year, best_year_actions) in best_actions {
                    // Get regular and deficit actions for the current run
                    let current_regular_actions = self.current_run_actions.get(year).cloned().unwrap_or_default();
                    let current_deficit_actions = self.current_deficit_actions.get(year).cloned().unwrap_or_default();
                    
                    // Combine current regular and deficit actions for comparison
                    let mut current_year_actions = current_regular_actions.clone();
                    current_year_actions.extend(current_deficit_actions.clone());
                    
                    // Get best deficit actions for this year if they exist
                    let best_deficit_actions = self.best_deficit_actions.as_ref()
                        .and_then(|bda| bda.get(year))
                        .cloned()
                        .unwrap_or_default();
                    
                    // Combine best regular and deficit actions
                    let mut complete_best_actions = best_year_actions.clone();
                    complete_best_actions.extend(best_deficit_actions);
                    
                    // Identify actions in the current run that differ from the best run
                    if let Some(year_weights) = self.weights.get_mut(year) {
                        // Store which actions got boosted vs penalized for normalization
                        let mut penalized_actions = Vec::new();
                        let mut boosted_actions = Vec::new();
                        let mut reward_actions: Vec<GridAction> = Vec::new();
                        
                        // First, STRONGLY boost all best actions, regardless of whether they appeared in current run
                        for best_action in &complete_best_actions {
                            if let Some(weight) = year_weights.get_mut(best_action) {
                                // Very strong boost to best actions, especially with high stagnation
                                *weight = (*weight * best_boost_factor).min(MAX_WEIGHT);
                                boosted_actions.push(best_action.clone());
                            }
                        }
                        
                        // Now process all current actions to possibly penalize
                        for (i, current_action) in current_year_actions.iter().enumerate() {
                            // Only penalize if this action doesn't appear in the best strategy at all
                            if !complete_best_actions.contains(current_action) {
                                if let Some(weight) = year_weights.get_mut(current_action) {
                                    *weight = (*weight * penalty_factor).max(MIN_WEIGHT);
                                    if *weight <= MIN_WEIGHT + 0.000001 {
                                        min_weight_count += 1;
                                    }
                                    total_weights += 1;
                                    penalized_actions.push(current_action.clone());
                                }
                            } else {
                                // This action appears in best strategy but might be at wrong time
                                // Check if it's at the same position
                                if i < complete_best_actions.len() && current_action != &complete_best_actions[i] {
                                    // It's in the best strategy but in the wrong order - mild penalty
                                    if let Some(weight) = year_weights.get_mut(current_action) {
                                        let mild_penalty = 1.0 / (1.0 + adaptive_learning_rate * combined_penalty * 0.5);
                                        *weight = (*weight * mild_penalty).max(MIN_WEIGHT);
                                        total_weights += 1;
                                    }
                                }
                            }
                        }
                        
                        // Log a summary of changes for this year
                        if !penalized_actions.is_empty() || !boosted_actions.is_empty() {
                            println!("   Year {}: Penalized {} actions, boosted {} actions, rewarded {} actions", 
                                    year, penalized_actions.len(), boosted_actions.len(), reward_actions.len());
                        }
                    }
                }
                
                // Log summary information about the weight changes
                println!("   - Applied enhanced contrast learning to deficit handling actions");

                // Show stats on how many weights were affected
                if total_weights > 0 {
                    println!("   - {}/{} weights ({:.1}%) reduced to minimum value", 
                            min_weight_count, total_weights, (min_weight_count as f64 / total_weights as f64) * 100.0);
                }

                // If we've been stagnating for a very long time, also apply some randomization
                // to break out of local optima
                if self.iterations_without_improvement > ITERATIONS_FOR_RANDOMIZATION {
                    println!("   - Applying weight randomization to break stagnation after {} iterations", 
                            self.iterations_without_improvement);
                    
                    let randomization_factor = 0.1; // 10% random variation
                    let mut rng = rand::thread_rng();
                    
                    for year_weights in self.weights.values_mut() {
                        for weight in year_weights.values_mut() {
                            let random_factor = 1.0 + randomization_factor * (rng.gen::<f64>() * 2.0 - 1.0);
                            *weight = (*weight * random_factor).clamp(MIN_WEIGHT, MAX_WEIGHT);
                        }
                    }
                }
            }
        }
    }

    // Update best deficit actions when we find a better overall strategy
    pub fn update_best_deficit_actions(&mut self) {
        if self.iterations_without_improvement == 0 {
            // We just found a better strategy, so update best deficit actions
            // Make sure we have entries for each year even if they're empty
            let mut complete_deficit_actions = HashMap::new();
            
            // Initialize empty action lists for all years
            for year in 2025..=2050 {
                complete_deficit_actions.insert(year, Vec::new());
            }
            
            // Then copy over any deficit actions we actually have
            for (year, actions) in &self.current_deficit_actions {
                if !actions.is_empty() {
                    println!("DEBUG: Copying {} deficit actions for year {} to best_deficit_actions", 
                             actions.len(), year);
                    complete_deficit_actions.insert(*year, actions.clone());
                }
            }
            
            // Debug: Check if we're actually capturing any deficit actions
            let total_complete_actions = complete_deficit_actions.values().map(|v| v.len()).sum::<usize>();
            let years_with_actions = complete_deficit_actions.values().filter(|v| !v.is_empty()).count();
            println!("DEBUG: Created best_deficit_actions map with {} actions across {} years", 
                     total_complete_actions, years_with_actions);
            
            // Store the complete map
            self.best_deficit_actions = Some(complete_deficit_actions);
        }
    }

    // Check if there are deficit actions for a given year
    pub fn has_deficit_actions_for_year(&self, year: u32) -> bool {
        self.current_deficit_actions.get(&year)
            .map_or(false, |actions| !actions.is_empty())
    }

    // Get deficit actions for a specific year
    pub fn get_deficit_actions_for_year(&self, year: u32) -> Option<Vec<GridAction>> {
        self.current_deficit_actions.get(&year)
            .map(|actions| actions.clone())
    }

    // Get best deficit actions for a specific year
    pub fn get_best_deficit_actions_for_year(&self, year: u32) -> Option<&Vec<GridAction>> {
        if let Some(ref best_deficit_actions) = self.best_deficit_actions {
            // If we have best_deficit_actions but not for this specific year, return empty vec instead of None
            return best_deficit_actions.get(&year).or_else(|| {
                println!("⚠️ WARNING: Best deficit actions exist but none for year {}", year);
                None
            });
        }
        None
    }

    // New method to get current run actions for a specific year
    pub fn get_current_run_actions_for_year(&self, year: u32) -> Option<&Vec<GridAction>> {
        self.current_run_actions.get(&year)
    }
    
    // We should also add a similar method for deficit actions
    pub fn get_current_deficit_actions_for_year(&self, year: u32) -> Option<&Vec<GridAction>> {
        self.current_deficit_actions.get(&year)
    }

    // Add a new helper function to diagnose action recording issues
    pub fn diagnose_best_actions(&self) {
        println!("✅ Best actions recorded: {} across {} years", 
            self.best_actions.as_ref().map_or(0, |actions| actions.values().map(|v| v.len()).sum::<usize>()),
            self.best_actions.as_ref().map_or(0, |actions| actions.values().filter(|v| !v.is_empty()).count()));
            
        println!("✅ Best deficit actions recorded: {} across {} years",
            self.best_deficit_actions.as_ref().map_or(0, |actions| actions.values().map(|v| v.len()).sum::<usize>()),
            self.best_deficit_actions.as_ref().map_or(0, |actions| actions.values().filter(|v| !v.is_empty()).count()));
        
        // Print the distribution of actions per year
        println!("Action distribution per year:");
        
        // If we have best metrics, also show those
        if let Some(ref metrics) = self.best_metrics {
            println!("✅ Best metrics recorded:");
            println!("  Net emissions: {:.2} tonnes", metrics.final_net_emissions);
            println!("  Is net zero: {}", if metrics.final_net_emissions <= 0.0 { "true" } else { "false" });
            println!("  Total cost: €{:.2}B", metrics.total_cost / 1_000_000_000.0);
            println!("  Public opinion: {:.1}%", metrics.average_public_opinion * 100.0);
            println!("  Power reliability: {:.1}%", metrics.power_reliability * 100.0);
        } else {
            println!("❌ No best metrics recorded yet");
        }
        
        println!("Total iterations: {}", self.iteration_count);
        println!("Iterations without improvement: {}", self.iterations_without_improvement);
    }

    // Add a new method to generate smart fallback actions
    pub fn generate_smart_fallback_action(&self, year: u32, fallback_reason: &str) -> GridAction {
        println!("🔧 SMART FALLBACK: Generating strategic action for year {} (reason: {})", year, fallback_reason);
        
        // The year will influence what kind of actions are taken
        // Early years: Focus on establishing renewable infrastructure
        // Middle years: Balance cost and emissions reduction
        // Late years: Focus heavily on carbon offsets and storage for net zero

        // Create weighted action pools for different years
        let mut action_pool = Vec::new();
        
        // Basic renewables always have some representation
        action_pool.push((GridAction::AddGenerator(GeneratorType::OnshoreWind), 15));
        action_pool.push((GridAction::AddGenerator(GeneratorType::OffshoreWind), 10));
        action_pool.push((GridAction::AddGenerator(GeneratorType::UtilitySolar), 15));
        
        // Storage becomes more important in middle and late years
        let storage_weight = if year < 2035 { 10 } else { 20 };
        action_pool.push((GridAction::AddGenerator(GeneratorType::BatteryStorage), storage_weight));
        
        // Carbon offsets become crucial in later years
        let offset_weight = if year < 2035 { 5 } else if year < 2045 { 15 } else { 25 };
        action_pool.push((GridAction::AddCarbonOffset("Forest".to_string()), offset_weight));
        action_pool.push((GridAction::AddCarbonOffset("ActiveCapture".to_string()), offset_weight));
        
        // Gas for reliable power - more important in early years, less in later
        let gas_weight = if year < 2035 { 15 } else if year < 2045 { 10 } else { 5 };
        action_pool.push((GridAction::AddGenerator(GeneratorType::GasCombinedCycle), gas_weight));
        
        // Calculate total weight
        let total_weight: u32 = action_pool.iter().map(|(_, w)| w).sum();
        
        // Select an action based on weighted random choice
        let mut rng = rand::thread_rng();
        let mut choice = rng.gen_range(0..total_weight);
        
        for (action, weight) in action_pool {
            if choice < weight {
                return action;
            }
            choice -= weight;
        }
        
        // Fallback to a safe default if something went wrong
        GridAction::AddGenerator(GeneratorType::BatteryStorage)
    }

    // Add a new method to generate smart deficit fallback actions
    pub fn generate_smart_deficit_fallback_action(&self, year: u32) -> GridAction {
        println!("🔧 SMART DEFICIT FALLBACK: Generating strategic deficit action for year {}", year);
        
        // For deficit handling, we need to prioritize reliable power generation
        // that can be deployed quickly and provide consistent output
        
        let mut action_pool = Vec::new();
        
        // Immediate response options get highest priority
        action_pool.push((GridAction::AddGenerator(GeneratorType::GasPeaker), 30));
        action_pool.push((GridAction::AddGenerator(GeneratorType::BatteryStorage), 30));
        
        // Medium-term reliable options
        action_pool.push((GridAction::AddGenerator(GeneratorType::GasCombinedCycle), 20));
        
        // Renewables - lower priority for deficit but still included
        action_pool.push((GridAction::AddGenerator(GeneratorType::OnshoreWind), 10));
        action_pool.push((GridAction::AddGenerator(GeneratorType::OffshoreWind), 5));
        action_pool.push((GridAction::AddGenerator(GeneratorType::UtilitySolar), 5));
        
        // Calculate total weight
        let total_weight: u32 = action_pool.iter().map(|(_, w)| w).sum();
        
        // Select an action based on weighted random choice
        let mut rng = rand::thread_rng();
        let mut choice = rng.gen_range(0..total_weight);
        
        for (action, weight) in action_pool {
            if choice < weight {
                return action;
            }
            choice -= weight;
        }
        
        // Fallback to a reliable default if something went wrong
        GridAction::AddGenerator(GeneratorType::BatteryStorage)
    }

    // First, add the restore_best_weights method
    // Add a new method to restore best weights with a mixing factor
    pub fn restore_best_weights(&mut self, best_weight_factor: f64) {
        if let Some(best_weights) = &self.best_weights {
            // Mix best weights with current weights using the specified factor
            for (year, best_year_weights) in best_weights {
                if let Some(current_year_weights) = self.weights.get_mut(year) {
                    for (action, &best_weight) in best_year_weights {
                        if let Some(current_weight) = current_year_weights.get_mut(action) {
                            // Mix weights
                            *current_weight = best_weight * best_weight_factor + 
                                            *current_weight * (1.0 - best_weight_factor);
                        } else {
                            // Action exists in best but not in current, add it
                            current_year_weights.insert(action.clone(), best_weight);
                        }
                    }
                }
            }
            
            println!("   - Restored weights with {:.0}% best weights / {:.0}% current weights", 
                    best_weight_factor * 100.0, (1.0 - best_weight_factor) * 100.0);
        }
    }

    // Then add the apply_deficit_contrast_learning method
    pub fn apply_deficit_contrast_learning(&mut self) {
        // Only apply contrast learning if we have a best run to compare against
        if let (Some(best_metrics), Some(best_deficit_actions)) = (&self.best_metrics, &self.best_deficit_actions) {
            let best_score = score_metrics(best_metrics);
            // We don't have a current metrics specific to deficit actions, but we can use the deterioration
            // from the regular contrast learning as an approximation
            let deterioration = self.iterations_without_improvement as f64 / 10.0; // Use iterations as a proxy for deterioration
            
            // Only apply contrast learning if there's some deterioration
            if deterioration > 0.0 {
                // Calculate stagnation penalty with exponential scaling
                let stagnation_iterations = self.iterations_without_improvement as f64 / 10.0;
                let stagnation_factor = 1.0 + (STAGNATION_PENALTY_FACTOR * stagnation_iterations.powf(STAGNATION_EXPONENT));
                
                // Scale the deterioration like in regular contrast learning
                let scaled_deterioration = deterioration.powf(DIVERGENCE_EXPONENT);
                
                // Calculate the combined penalty multiplier
                let combined_penalty = scaled_deterioration * stagnation_factor;
                
                // Calculate adaptive learning rate as in regular contrast learning
                let adaptive_learning_rate = self.learning_rate * (1.0 + 0.05 * self.iterations_without_improvement as f64);
                
                // Log the contrast learning application with more detailed information
                println!("\n🔄 Applying enhanced contrast learning to deficit handling actions:");
                println!("   - Iterations without improvement: {}", self.iterations_without_improvement);
                println!("   - Proxy deterioration: {:.4}, Scaled: {:.4}", deterioration, scaled_deterioration);
                println!("   - Stagnation factor: {:.2}x", stagnation_factor);
                println!("   - Combined penalty multiplier: {:.4}", combined_penalty);
                
                // Calculate the penalty factor - more severe for worse runs and after more stagnation
                let penalty_factor = 1.0 / (1.0 + adaptive_learning_rate * 2.0 * combined_penalty);
                let best_boost_factor = 1.0 + (adaptive_learning_rate * 3.0 * stagnation_factor);
                
                println!("   - Penalty factor: {:.8}", penalty_factor);
                println!("   - Best boost factor: {:.8}", best_boost_factor);
                
                // For each year, compare deficit actions with the best deficit actions
                for (year, best_year_actions) in best_deficit_actions {
                    let current_year_actions = self.current_deficit_actions.get(year).cloned().unwrap_or_default();
                    
                    if let Some(year_weights) = self.deficit_weights.get_mut(year) {
                        // First, boost all best deficit actions
                        for best_action in best_year_actions {
                            if let Some(weight) = year_weights.get_mut(best_action) {
                                *weight = (*weight * best_boost_factor).min(MAX_WEIGHT);
                            }
                        }
                        
                        // Then penalize current actions that don't appear in best actions
                        for current_action in &current_year_actions {
                            if !best_year_actions.contains(current_action) {
                                if let Some(weight) = year_weights.get_mut(current_action) {
                                    *weight = (*weight * penalty_factor).max(MIN_WEIGHT);
                                }
                            }
                        }
                    }
                }
                
                // Log summary information about the weight changes
                println!("   - Applied enhanced contrast learning to deficit handling actions");
            }
        }
    }

    // Add the missing methods back

    // Sample deficit action method
    pub fn sample_deficit_action(&mut self, year: u32) -> GridAction {
        // If we're forcing replay of best actions and we have best deficit actions, use those
        if self.force_best_actions {
            if let Some(best_deficit_actions) = &self.best_deficit_actions {
                if let Some(year_deficit_actions) = best_deficit_actions.get(&year) {
                    let current_index = self.current_deficit_actions.get(&year).map_or(0, |v| v.len());
                    if current_index < year_deficit_actions.len() {
                        let action = year_deficit_actions[current_index].clone();
                        println!("🔄 DEFICIT REPLAY: Using best deficit action #{} for year {}: {:?}", 
                                current_index + 1, year, action);
                        
                        // Make sure to record this replayed deficit action
                        self.current_deficit_actions.entry(year)
                            .or_insert_with(Vec::new)
                            .push(action.clone());
                        
                        return action;
                    } else {
                        println!("⚠️ DEFICIT REPLAY FALLBACK: Ran out of best deficit actions for year {} (needed action #{}, have {})",
                                year, current_index + 1, year_deficit_actions.len());
                        
                        // Smart fallback for deficit
                        let fallback_action = self.generate_smart_deficit_fallback_action(year);
                        
                        // Record this fallback action
                        self.current_deficit_actions.entry(year)
                            .or_insert_with(Vec::new)
                            .push(fallback_action.clone());
                        
                        return fallback_action;
                    }
                } else {
                    println!("⚠️ DEFICIT REPLAY FALLBACK: No best deficit actions recorded for year {}", year);
                    
                    // Smart fallback for deficit
                    let fallback_action = self.generate_smart_deficit_fallback_action(year);
                    
                    // Record this fallback action
                    self.current_deficit_actions.entry(year)
                        .or_insert_with(Vec::new)
                        .push(fallback_action.clone());
                    
                    return fallback_action;
                }
            } else {
                println!("⚠️ DEFICIT REPLAY FALLBACK: No best deficit actions recorded for any year");
                
                // Smart fallback for deficit
                let fallback_action = self.generate_smart_deficit_fallback_action(year);
                
                // Record this fallback action
                self.current_deficit_actions.entry(year)
                    .or_insert_with(Vec::new)
                    .push(fallback_action.clone());
                    
                return fallback_action;
            }
        }
        
        // Continue with normal deficit action selection
        // Default to normal deficit weights
        let year_weights = match self.deficit_weights.get(&year) {
            Some(weights) => weights,
            None => {
                // Fallback to initialize weights for this year if missing
                return GridAction::AddGenerator(GeneratorType::GasPeaker);
            }
        };
        
        // Determine if we should explore based on the deterministic RNG or thread_rng
        let should_explore = match &mut self.deterministic_rng {
            Some(rng) => rng.gen::<f64>() < self.exploration_rate,
            None => rand::thread_rng().gen::<f64>() < self.exploration_rate,
        };
        
        // Apply epsilon-greedy strategy similar to main action sampling
        if should_explore {
            // Random exploration
            let actions: Vec<_> = year_weights.keys()
                .filter(|action| matches!(action, GridAction::AddGenerator(_)))
                .collect();
            
            if actions.is_empty() {
                // Fallback to a reliable generator if no AddGenerator actions
                return GridAction::AddGenerator(GeneratorType::GasPeaker);
            }
            
            let random_idx = match &mut self.deterministic_rng {
                Some(rng) => rng.gen_range(0..actions.len()),
                None => rand::thread_rng().gen_range(0..actions.len()),
            };
            
            return actions[random_idx].clone();
        }
        
        // Exploitation - weighted selection of generator actions
        let total_weight: f64 = year_weights.iter()
            .filter(|(action, _)| matches!(action, GridAction::AddGenerator(_)))
            .map(|(_, &weight)| weight)
            .sum();
        
        if total_weight <= 0.0 {
            // If all weights are zero or negative, fall back to a reliable generator
            return GridAction::AddGenerator(GeneratorType::GasPeaker);
        }
        
        let mut random_val = match &mut self.deterministic_rng {
            Some(rng) => rng.gen::<f64>() * total_weight,
            None => rand::thread_rng().gen::<f64>() * total_weight,
        };
        
        for (action, weight) in year_weights {
            if matches!(action, GridAction::AddGenerator(_)) {
                random_val -= weight;
                if random_val <= 0.0 {
                    return action.clone();
                }
            }
        }
        
        // Fallback to a reliable generator if selection fails
        GridAction::AddGenerator(GeneratorType::GasPeaker)
    }

    // Record a deficit handling action
    pub fn record_deficit_action(&mut self, year: u32, action: GridAction) {
        self.current_deficit_actions.entry(year)
            .or_insert_with(Vec::new)
            .push(action);
    }

    // Update deficit handling weights based on the success or failure of an action
    pub fn update_deficit_weights(&mut self, action: &GridAction, year: u32, improvement: f64) {
        // Ensure we have weights for this year
        if !self.deficit_weights.contains_key(&year) {
            // Initialize with defaults biased toward fast-responding generators
            let mut deficit_year_weights = HashMap::new();
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::GasPeaker), 0.15);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::GasCombinedCycle), 0.15);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::BatteryStorage), 0.15);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::PumpedStorage), 0.10);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::Biomass), 0.10);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::OnshoreWind), 0.07);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::OffshoreWind), 0.07);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::UtilitySolar), 0.06);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::HydroDam), 0.06);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::Nuclear), 0.05);
            deficit_year_weights.insert(GridAction::DoNothing, 0.001);
            self.deficit_weights.insert(year, deficit_year_weights);
        }
        
        let year_weights = self.deficit_weights.get_mut(&year).expect("Year weights not found");
        
        // If the action doesn't exist in weights, initialize it
        if !year_weights.contains_key(action) {
            year_weights.insert(action.clone(), DEFAULT_WEIGHT);
        }
        
        let current_weight = year_weights.get(action).expect("Weight should exist");
        
        // Calculate adjustment factor similar to normal action weights
        let adjustment_factor = if improvement > 0.0 {
            // For improvements, increase weight proportionally to the improvement
            1.0 + (self.learning_rate * improvement * 1.5) // Apply stronger reinforcement for deficit handling
        } else {
            // For deteriorations, decrease weight proportionally to how bad it was
            1.0 / (1.0 + (self.learning_rate * improvement.abs() * 1.5)) // Apply stronger penalties for deficit handling
        };
        
        // Apply the adjustment with bounds
        let new_weight = (current_weight * adjustment_factor)
            .max(MIN_WEIGHT)
            .min(MAX_WEIGHT);
        
        year_weights.insert(action.clone(), new_weight);
        
        // If this was a bad outcome, slightly increase weights of other generator types
        if improvement < 0.0 {
            let boost_factor = 1.0 + (self.learning_rate * 0.1); // Small boost to alternatives
            for (other_action, weight) in year_weights.iter_mut() {
                if other_action != action && matches!(other_action, GridAction::AddGenerator(_)) {
                    *weight = (*weight * boost_factor).min(MAX_WEIGHT);
                }
            }
        }
    }

    // Get the best actions for a particular year
    pub fn get_best_actions_for_year(&self, year: u32) -> Option<&Vec<GridAction>> {
        if let Some(ref best_actions) = self.best_actions {
            // If we have best_actions but not for this specific year, return empty vec instead of None
            return best_actions.get(&year).or_else(|| {
                println!("⚠️ WARNING: Best actions exist but none for year {}", year);
                None
            });
        }
        None
    }

    // Debug method to print actions recorded in the current run
    pub fn debug_print_recorded_actions(&self) {
        let total_actions = self.current_run_actions.values().map(|v| v.len()).sum::<usize>();
        let years_with_actions = self.current_run_actions.values().filter(|v| !v.is_empty()).count();
        
        println!("📊 DEBUG: Actions recorded in current run:");
        println!("  Total: {} actions across {} years", total_actions, years_with_actions);
        
        // Add per-year breakdown for easier diagnostics
        let min_year = 2025;
        let max_year = 2050;
        
        println!("  Per-year action counts:");
        for year in min_year..=max_year {
            if let Some(actions) = self.current_run_actions.get(&year) {
                if !actions.is_empty() {
                    println!("    Year {}: {} actions", year, actions.len());
                }
            }
        }
    }
    
    // New method to debug print deficit actions
    pub fn debug_print_deficit_actions(&self) {
        let total_actions = self.current_deficit_actions.values().map(|v| v.len()).sum::<usize>();
        let years_with_actions = self.current_deficit_actions.values().filter(|v| !v.is_empty()).count();
        
        println!("📊 DEBUG: Deficit actions recorded in current run:");
        println!("  Total: {} deficit actions across {} years", total_actions, years_with_actions);
        
        // Add per-year breakdown for easier diagnostics
        let min_year = 2025;
        let max_year = 2050;
        
        println!("  Per-year deficit action counts:");
        for year in min_year..=max_year {
            if let Some(actions) = self.current_deficit_actions.get(&year) {
                if !actions.is_empty() {
                    println!("    Year {}: {} deficit actions", year, actions.len());
                }
            }
        }
    }

    // Check if we have best actions
    pub fn has_best_actions(&self) -> bool {
        self.best_actions.is_some()
    }

    // Set whether to force best actions
    pub fn set_force_best_actions(&mut self, force: bool) {
        self.force_best_actions = force;
    }

    // New method to set guaranteed force replay (100% probability)
    pub fn set_guaranteed_best_actions(&mut self, force: bool) {
        self.force_best_actions = force;
        // Setting this flag means we bypass the probability check in start_new_iteration
        // and always use best actions if available
        self.guaranteed_best_actions = force;
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
