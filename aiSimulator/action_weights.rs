use std::collections::HashMap;
use rand::Rng;
use rand::rngs::StdRng;
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
const FORCE_REPLAY_THRESHOLD: u32 = 1000; // After this many iterations without improvement, start forcing replay
const ITERATIONS_FOR_RANDOMIZATION: u32 = 1000; // After this many iterations without improvement, apply randomization

// Simulation year constants
const START_YEAR: u32 = 2025;
const END_YEAR: u32 = 2050;

// Weight initialization constants
const ONSHORE_WIND_WEIGHT: f64 = 0.08;
const OFFSHORE_WIND_WEIGHT: f64 = 0.08;
const DOMESTIC_SOLAR_WEIGHT: f64 = 0.05;
const COMMERCIAL_SOLAR_WEIGHT: f64 = 0.05;
const UTILITY_SOLAR_WEIGHT: f64 = 0.08;
const NUCLEAR_WEIGHT: f64 = 0.03;
const COAL_PLANT_WEIGHT: f64 = 0.04;
const GAS_COMBINED_CYCLE_WEIGHT: f64 = 0.06;
const GAS_PEAKER_WEIGHT: f64 = 0.02;
const BIOMASS_WEIGHT: f64 = 0.04;
const HYDRO_DAM_WEIGHT: f64 = 0.06;
const PUMPED_STORAGE_WEIGHT: f64 = 0.06;
const BATTERY_STORAGE_WEIGHT: f64 = 0.07;
const TIDAL_GENERATOR_WEIGHT: f64 = 0.05;
const WAVE_ENERGY_WEIGHT: f64 = 0.05;
const UPGRADE_EFFICIENCY_WEIGHT: f64 = 0.04;
const ADJUST_OPERATION_WEIGHT: f64 = 0.04;
const CARBON_OFFSET_WEIGHT: f64 = 0.02;
const CLOSE_GENERATOR_WEIGHT: f64 = 0.02;
const DO_NOTHING_WEIGHT: f64 = 0.1;

// Deficit weights
const DEFICIT_GAS_PEAKER_WEIGHT: f64 = 0.15;
const DEFICIT_GAS_COMBINED_WEIGHT: f64 = 0.15;
const DEFICIT_BATTERY_WEIGHT: f64 = 0.15;
const DEFICIT_PUMPED_STORAGE_WEIGHT: f64 = 0.10;
const DEFICIT_BIOMASS_WEIGHT: f64 = 0.10;
const DEFICIT_ONSHORE_WIND_WEIGHT: f64 = 0.07;
const DEFICIT_OFFSHORE_WIND_WEIGHT: f64 = 0.07;
const DEFICIT_UTILITY_SOLAR_WEIGHT: f64 = 0.06;
const DEFICIT_HYDRO_DAM_WEIGHT: f64 = 0.06;
const DEFICIT_NUCLEAR_WEIGHT: f64 = 0.05;
const DEFICIT_SMALL_GENERATOR_WEIGHT: f64 = 0.01;
const DEFICIT_DO_NOTHING_WEIGHT: f64 = 0.001;

// Learning rate constants
const DEFAULT_LEARNING_RATE: f64 = 0.2;
const DEFAULT_EXPLORATION_RATE: f64 = 0.2;
const EXPLORATION_DECAY_RATE: f64 = 0.1;

// Action count constants
const MAX_ACTION_COUNT: u32 = 20;
const ACTION_COUNT_DECAY_RATE: f64 = 0.4;

// Probability and weight constants
const PERCENT_CONVERSION: f64 = 100.0;
const PERCENTAGE_THRESHOLD: f64 = 0.9;
const FORCE_REPLAY_DIVISOR: f64 = 500.0;
const BEST_WEIGHT_FACTOR: f64 = 0.75;
const EXPLORATION_DECAY_FACTOR: f64 = 0.01;
const STAGNATION_DIVISOR: f64 = 1000.0;
const STAGNATION_SCALE_MIN: f64 = 1.0;
const STAGNATION_SCALE_FACTOR: f64 = 2.0;
const STAGNATION_SCALE_MAX: f64 = 3.0;
const STAGNATION_ITERATIONS_DIVISOR: f64 = 10.0;
const IMMEDIATE_WEIGHT_FACTOR_POSITIVE: f64 = 0.7;
const IMMEDIATE_WEIGHT_FACTOR_NEGATIVE: f64 = 0.3;
const RANDOMIZATION_FACTOR: f64 = 0.1;

// Smart fallback action weights
const ONSHORE_WIND_FALLBACK_WEIGHT: u32 = 15;
const OFFSHORE_WIND_FALLBACK_WEIGHT: u32 = 10;
const UTILITY_SOLAR_FALLBACK_WEIGHT: u32 = 15;
const STORAGE_WEIGHT_EARLY: u32 = 10;
const STORAGE_WEIGHT_LATE: u32 = 20;
const OFFSET_WEIGHT_EARLY: u32 = 5;
const OFFSET_WEIGHT_MID: u32 = 15;
const OFFSET_WEIGHT_LATE: u32 = 25;
const GAS_WEIGHT_EARLY: u32 = 15;
const GAS_WEIGHT_MID: u32 = 10;
const GAS_WEIGHT_LATE: u32 = 5;
const DEFICIT_GAS_PEAKER_FALLBACK_WEIGHT: u32 = 30;
const DEFICIT_BATTERY_FALLBACK_WEIGHT: u32 = 30;
const DEFICIT_GAS_COMBINED_FALLBACK_WEIGHT: u32 = 20;
const DEFICIT_ONSHORE_WIND_FALLBACK_WEIGHT: u32 = 10;

// Year thresholds for weight transitions
const MID_YEAR_THRESHOLD: u32 = 2035;
const LATE_YEAR_THRESHOLD: u32 = 2045;

// Debug print formatting
const DEBUG_STAR_COUNT: usize = 40;
const DEBUG_EQUALS_COUNT: usize = 80;

// Other thresholds
const HIGH_ITERATION_THRESHOLD: u32 = 800;
const MID_ITERATION_THRESHOLD: u32 = 500;
const LOW_ITERATION_THRESHOLD: u32 = 100;
const MEDIUM_LOG_INTERVAL: u32 = 100;
const SMALL_LOG_INTERVAL: u32 = 10;
const MAX_ACTIONS_DIVISOR: f64 = 12.0;

// Numeric defaults
const ZERO_F64: f64 = 0.0;
const ONE_F64: f64 = 1.0;
const SMALL_BOOST_FACTOR: f64 = 0.1;
const NOOP_BOOST_FACTOR: f64 = 0.2;
const COST_MULTIPLICATION_FACTOR: f64 = 8.0;
const BILLION_DIVISOR: f64 = 1_000_000_000.0;
const RENEWABLE_FALLBACK_WEIGHT_FACTOR: f64 = 0.5;

// Adaptive learning constants
const ADAPTIVE_LEARNING_RATE_FACTOR: f64 = 0.05;
const PENALTY_MULTIPLIER: f64 = 2.0;
const BOOST_MULTIPLIER: f64 = 3.0;
const MILD_PENALTY_FACTOR: f64 = 0.5;
const WEIGHT_PRECISION_THRESHOLD: f64 = 0.000001;
const DEFICIT_REINFORCEMENT_MULTIPLIER: f64 = 1.5;

// Scoring constants
const MAX_BUDGET_MULTIPLIER: f64 = 100.0;
const BASE_NET_ZERO_SCORE: f64 = 1.0;
const MAX_SCORE_RANGE: f64 = 2.0;
const HIGH_COST_THRESHOLD_MULTIPLIER: f64 = 8.0;
const HIGH_COST_WEIGHT: f64 = 0.8;
const NORMAL_COST_WEIGHT: f64 = 0.5;

// Additional constants
const ZERO_USIZE: usize = 0;
const ONE_USIZE: usize = 1;
const ZERO_U32: u32 = 0;
const ONE_U32: u32 = 1;
const ZERO_U8: u8 = 0;
const OPERATION_PERCENTAGE_MIN: u8 = 0;
const MIN_ACTION_WEIGHT: f64 = 0.01;

// Stagnation divisor as integer
const STAGNATION_DIVISOR_INT: u32 = 100;

// Define a new constant for the random range
const RANDOM_RANGE_MULTIPLIER: f64 = 2.0;

// Define a new constant for the exploration divisor
const EXPLORATION_DIVISOR: f64 = 2.0;

// Define a new constant for the max actions multiplier
const MAX_ACTIONS_MULTIPLIER: f64 = 12.0;

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
    optimization_mode: Option<String>,
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
    optimization_mode: Option<String>, // Store the optimization mode
}

impl ActionWeights {
    pub fn new() -> Self {
        let mut weights = HashMap::new();
        let mut deficit_weights = HashMap::new();
        let mut action_count_weights = HashMap::new();
        
        // Initialize weights for each year from 2025 to 2050
        for year in START_YEAR..=END_YEAR {
            let mut year_weights = HashMap::new();
            
            // Initialize wind generator weights
            year_weights.insert(GridAction::AddGenerator(GeneratorType::OnshoreWind), ONSHORE_WIND_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::OffshoreWind), OFFSHORE_WIND_WEIGHT);
            
            // Initialize solar generator weights
            year_weights.insert(GridAction::AddGenerator(GeneratorType::DomesticSolar), DOMESTIC_SOLAR_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::CommercialSolar), COMMERCIAL_SOLAR_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::UtilitySolar), UTILITY_SOLAR_WEIGHT);
            
            // Initialize nuclear and fossil fuel generator weights
            year_weights.insert(GridAction::AddGenerator(GeneratorType::Nuclear), NUCLEAR_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::CoalPlant), COAL_PLANT_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::GasCombinedCycle), GAS_COMBINED_CYCLE_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::GasPeaker), GAS_PEAKER_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::Biomass), BIOMASS_WEIGHT);
            
            // Initialize hydro and storage generator weights
            year_weights.insert(GridAction::AddGenerator(GeneratorType::HydroDam), HYDRO_DAM_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::PumpedStorage), PUMPED_STORAGE_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::BatteryStorage), BATTERY_STORAGE_WEIGHT);
            
            // Initialize marine generator weights
            year_weights.insert(GridAction::AddGenerator(GeneratorType::TidalGenerator), TIDAL_GENERATOR_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::WaveEnergy), WAVE_ENERGY_WEIGHT);
            
            // Initialize other action weights
            year_weights.insert(GridAction::UpgradeEfficiency(String::new()), UPGRADE_EFFICIENCY_WEIGHT);
            year_weights.insert(GridAction::AdjustOperation(String::new(), OPERATION_PERCENTAGE_MIN), ADJUST_OPERATION_WEIGHT);
            
            // Initialize carbon offset weights
            year_weights.insert(GridAction::AddCarbonOffset("Forest".to_string()), CARBON_OFFSET_WEIGHT);
            year_weights.insert(GridAction::AddCarbonOffset("Wetland".to_string()), CARBON_OFFSET_WEIGHT);
            year_weights.insert(GridAction::AddCarbonOffset("ActiveCapture".to_string()), CARBON_OFFSET_WEIGHT);
            year_weights.insert(GridAction::AddCarbonOffset("CarbonCredit".to_string()), CARBON_OFFSET_WEIGHT);
            
            year_weights.insert(GridAction::CloseGenerator(String::new()), CLOSE_GENERATOR_WEIGHT);
            
            // Initialize DoNothing action weight (base value can be tuned)
            year_weights.insert(GridAction::DoNothing, DO_NOTHING_WEIGHT);
            
            // Add year's weights to the map
            weights.insert(year, year_weights);

            // Initialize deficit handling weights with a separate set of weights
            // focused on reliable power generation options
            let mut deficit_year_weights = HashMap::new();
            
            // For deficit handling, prioritize fast-responding and reliable generators
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::GasPeaker), DEFICIT_GAS_PEAKER_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::GasCombinedCycle), DEFICIT_GAS_COMBINED_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::BatteryStorage), DEFICIT_BATTERY_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::PumpedStorage), DEFICIT_PUMPED_STORAGE_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::Biomass), DEFICIT_BIOMASS_WEIGHT);
            
            // Include renewables with lower initial weights for deficit handling
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::OnshoreWind), DEFICIT_ONSHORE_WIND_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::OffshoreWind), DEFICIT_OFFSHORE_WIND_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::UtilitySolar), DEFICIT_UTILITY_SOLAR_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::HydroDam), DEFICIT_HYDRO_DAM_WEIGHT);
            
            // Include nuclear with a lower weight due to long build time
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::Nuclear), DEFICIT_NUCLEAR_WEIGHT);
            
            // Add other types with minimal weights
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::DomesticSolar), DEFICIT_SMALL_GENERATOR_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::CommercialSolar), DEFICIT_SMALL_GENERATOR_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::TidalGenerator), DEFICIT_SMALL_GENERATOR_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::WaveEnergy), DEFICIT_SMALL_GENERATOR_WEIGHT);
            
            // DoNothing should have very low weight for deficit handling
            deficit_year_weights.insert(GridAction::DoNothing, DEFICIT_DO_NOTHING_WEIGHT);
            
            // Add deficit weights for this year
            deficit_weights.insert(year, deficit_year_weights);

            // Initialize action count weights for this year with bias towards fewer actions
            let mut count_weights = HashMap::new();
            let decay_rate = ACTION_COUNT_DECAY_RATE; // Controls how quickly the probability decreases
            let mut total_weight = ZERO_F64;
            
            // Calculate weights with exponential decay
            for count in 0..=MAX_ACTION_COUNT {
                let weight = (-decay_rate * count as f64).exp();
                count_weights.insert(count, weight);
                total_weight += weight;
            }
            
            // Normalize weights to sum to ONE_F64
            for weight in count_weights.values_mut() {
                *weight /= total_weight;
            }
            
            action_count_weights.insert(year, count_weights);
        }
        
        Self {
            weights,
            action_count_weights,
            learning_rate: DEFAULT_LEARNING_RATE,
            best_metrics: None,
            best_weights: None,
            best_actions: None,
            iteration_count: 0,
            iterations_without_improvement: 0,
            exploration_rate: DEFAULT_EXPLORATION_RATE,
            current_run_actions: HashMap::new(),
            force_best_actions: false,
            deficit_weights,
            current_deficit_actions: HashMap::new(),
            best_deficit_actions: None,
            deterministic_rng: None,
            guaranteed_best_actions: false,
            optimization_mode: None,
        }
    }

    pub fn set_rng(&mut self, rng: StdRng) {
        self.deterministic_rng = Some(rng);
    }
    
    pub fn start_new_iteration(&mut self) {
        self.iteration_count += 1;
        // Decay exploration rate over time
        self.exploration_rate = DEFAULT_EXPLORATION_RATE * (ONE_F64 / (ONE_F64 + EXPLORATION_DECAY_RATE * self.iteration_count as f64));
        // Clear actions from the previous run
        self.current_run_actions.clear();
        self.current_deficit_actions.clear();
        
        // Don't override force_best_actions if guaranteed_best_actions is set
        if self.guaranteed_best_actions {
            self.force_best_actions = true;
            return;
        }
        
        // Add logging for debugging if we have many iterations without improvement
        if self.iterations_without_improvement > ZERO_U32 {
            if self.iterations_without_improvement % SMALL_LOG_INTERVAL == ZERO_U32 {
                println!("⚠️ Currently at {} iterations without improvement", self.iterations_without_improvement);
            }
            
            // Occasionally restore best weights when stuck for a long time
            if self.iterations_without_improvement > HIGH_ITERATION_THRESHOLD && self.iterations_without_improvement % STAGNATION_DIVISOR_INT == ZERO_U32 {
                if let Some(best_weights) = &self.best_weights {
                    println!("🔄 RESTORING BEST WEIGHTS after {} iterations without improvement", 
                            self.iterations_without_improvement);
                    // Create a partial copy of the best weights (75%) mixed with current weights (25%)
                    self.restore_best_weights(BEST_WEIGHT_FACTOR);
                }
            }
            
            // Calculate probability of forcing replay based on stagnation
            if self.iterations_without_improvement > FORCE_REPLAY_THRESHOLD {
                let force_replay_probability = ((self.iterations_without_improvement - FORCE_REPLAY_THRESHOLD) as f64 / FORCE_REPLAY_DIVISOR).min(PERCENTAGE_THRESHOLD);
                
                let random_val = match &mut self.deterministic_rng {
                    Some(rng) => rng.gen::<f64>(),
                    None => rand::thread_rng().gen::<f64>(),
                };
                
                if random_val < force_replay_probability {
                    println!("🔄 FORCE REPLAY: Directly using best known actions (probability: {:.1}%)", 
                            force_replay_probability * PERCENT_CONVERSION);
                    self.force_best_actions = true;
                    return;
                }
            }
            
            // If we've been stagnating for a very long time, also apply some randomization
            // to break out of local optima
            if self.iterations_without_improvement > ITERATIONS_FOR_RANDOMIZATION {
                println!("   - Applying weight randomization to break stagnation after {} iterations", 
                        self.iterations_without_improvement);
                
                let mut rng = rand::thread_rng();
                
                for year_weights in self.weights.values_mut() {
                    for weight in year_weights.values_mut() {
                        let random_factor = ONE_F64 + RANDOMIZATION_FACTOR * (rng.gen::<f64>() * RANDOM_RANGE_MULTIPLIER - ONE_F64);
                        *weight = (*weight * random_factor).clamp(MIN_WEIGHT, MAX_WEIGHT);
                    }
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
                    let current_index = self.current_run_actions.get(&year).map_or(ZERO_USIZE, |v| v.len());
                    if current_index < year_actions.len() {
                        let action = year_actions[current_index].clone();
                        println!("🔄 REPLAY: Using best action #{} for year {}: {:?}", 
                                current_index + ONE_USIZE, year, action);
                        
                        // Make sure to record the replayed action in the current run
                        self.current_run_actions.entry(year)
                            .or_insert_with(Vec::new)
                            .push(action.clone());
                        
                        return action;
                    } else {
                        println!("⚠️ REPLAY FALLBACK: Ran out of best actions for year {} (needed action #{}, have {})", 
                                year, current_index + ONE_USIZE, year_actions.len());
                        
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
        let current_exploration = if self.iterations_without_improvement > LOW_ITERATION_THRESHOLD {
            // Reduce exploration drastically after being stuck for a while to focus on best known actions
            self.exploration_rate * (ONE_F64 / (ONE_F64 + EXPLORATION_DECAY_FACTOR * self.iterations_without_improvement as f64))
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
                Some(rng) => rng.gen_range(ZERO_USIZE..actions.len()),
                None => rand::thread_rng().gen_range(ZERO_USIZE..actions.len()),
            };
            
            return actions[random_idx].clone();
        }

        // Exploitation - weighted selection
        let total_weight: f64 = year_weights.values().sum();
        if total_weight <= ZERO_F64 {
            // If all weights are zero or negative, fall back to a safe default
            return GridAction::AddGenerator(GeneratorType::GasPeaker);
        }

        // When stuck for many iterations, use a more aggressive selection strategy
        // by applying a power scaling to the weights, making higher weights even more likely
        if self.iterations_without_improvement > MID_ITERATION_THRESHOLD {
            // Extract actions and weights
            let mut actions_with_weights: Vec<_> = year_weights.iter().collect();
            // Sort by weight in descending order
            actions_with_weights.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
            
            // Calculate how aggressive the selection should be based on stagnation
            let stagnation_factor = (self.iterations_without_improvement as f64 / STAGNATION_DIVISOR).min(STAGNATION_SCALE_MAX);
            let power_scaling = STAGNATION_SCALE_MIN + (STAGNATION_SCALE_FACTOR * stagnation_factor); // Ranges from 1.0 to 3.0
            
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
                if random_val <= ZERO_F64 {
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
                if random_val <= ZERO_F64 {
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
        year_weights.insert(GridAction::AddGenerator(GeneratorType::OnshoreWind), ONSHORE_WIND_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::OffshoreWind), OFFSHORE_WIND_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::DomesticSolar), DOMESTIC_SOLAR_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::CommercialSolar), COMMERCIAL_SOLAR_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::UtilitySolar), UTILITY_SOLAR_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::Nuclear), NUCLEAR_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::CoalPlant), COAL_PLANT_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::GasCombinedCycle), GAS_COMBINED_CYCLE_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::GasPeaker), GAS_PEAKER_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::Biomass), BIOMASS_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::HydroDam), HYDRO_DAM_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::PumpedStorage), PUMPED_STORAGE_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::BatteryStorage), BATTERY_STORAGE_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::TidalGenerator), TIDAL_GENERATOR_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::WaveEnergy), WAVE_ENERGY_WEIGHT);
        year_weights.insert(GridAction::UpgradeEfficiency(String::new()), UPGRADE_EFFICIENCY_WEIGHT);
        year_weights.insert(GridAction::AdjustOperation(String::new(), OPERATION_PERCENTAGE_MIN), ADJUST_OPERATION_WEIGHT);
        year_weights.insert(GridAction::AddCarbonOffset("Forest".to_string()), CARBON_OFFSET_WEIGHT);
        year_weights.insert(GridAction::AddCarbonOffset("Wetland".to_string()), CARBON_OFFSET_WEIGHT);
        year_weights.insert(GridAction::AddCarbonOffset("ActiveCapture".to_string()), CARBON_OFFSET_WEIGHT);
        year_weights.insert(GridAction::AddCarbonOffset("CarbonCredit".to_string()), CARBON_OFFSET_WEIGHT);
        year_weights.insert(GridAction::CloseGenerator(String::new()), CLOSE_GENERATOR_WEIGHT);
        // Initialize DoNothing with a base weight
        year_weights.insert(GridAction::DoNothing, DO_NOTHING_WEIGHT);
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
        let final_impact_score = self.best_metrics.as_ref().map_or(ZERO_F64, |metrics| score_metrics(metrics, self.optimization_mode.as_deref()));
        
        // Calculate the relative improvement compared to the best score
        let relative_improvement = if let Some(best) = &self.best_metrics {
            let best_score = score_metrics(best, self.optimization_mode.as_deref());
            if best_score > ZERO_F64 {
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
        let immediate_weight = if relative_improvement > ZERO_F64 { IMMEDIATE_WEIGHT_FACTOR_POSITIVE } else { IMMEDIATE_WEIGHT_FACTOR_NEGATIVE };
        let combined_improvement = immediate_weight * improvement + (ONE_F64 - immediate_weight) * relative_improvement;
        
        // Calculate weight adjustment
        let adjustment_factor = if combined_improvement > ZERO_F64 {
            // For improvements, increase weight proportionally to the improvement
            ONE_F64 + (self.learning_rate * combined_improvement)
        } else {
            // For deteriorations, decrease weight proportionally to how bad it was
            ONE_F64 / (ONE_F64 + (self.learning_rate * combined_improvement.abs()))
        };
        
        // Apply the adjustment with bounds
        let new_weight = (current_weight * adjustment_factor)
            .max(MIN_WEIGHT)
            .min(MAX_WEIGHT);
        
        year_weights.insert(action.clone(), new_weight);
        
        // If this was a bad outcome, slightly increase weights of other actions.
        if combined_improvement < ZERO_F64 {
            let boost_factor = ONE_F64 + (self.learning_rate * SMALL_BOOST_FACTOR); // Small boost to alternatives
            for (other_action, weight) in year_weights.iter_mut() {
                if other_action != action && matches!(other_action, GridAction::AddGenerator(_)) {
                    *weight = (*weight * boost_factor).min(MAX_WEIGHT);
                }
            }
            // If we've achieved net zero but are suffering from high costs, further boost DoNothing.
            if self.best_metrics.as_ref().map(|m| m.final_net_emissions <= ZERO_F64 && m.total_cost > MAX_ACCEPTABLE_COST * HIGH_COST_THRESHOLD_MULTIPLIER).unwrap_or(false) {
                if let Some(noop_weight) = year_weights.get_mut(&GridAction::DoNothing) {
                    *noop_weight = (*noop_weight * (ONE_F64 + self.learning_rate * NOOP_BOOST_FACTOR)).min(MAX_WEIGHT);
                }
            }
        }
    }

    pub fn update_best_strategy(&mut self, metrics: SimulationMetrics) {
        let current_score = score_metrics(&metrics, self.optimization_mode.as_deref());
        
        // Debug: Print current_run_actions info with more detailed breakdown
        let total_curr_actions = self.current_run_actions.values().map(|v| v.len()).sum::<usize>();
        let years_with_curr_actions = self.current_run_actions.values().filter(|v| !v.is_empty()).count();
        println!("DEBUG: Before update - Current run has {} actions across {} years", 
                total_curr_actions, years_with_curr_actions);
        
        // More detailed per-year breakdown for the current run
        println!("Current run actions per year:");
        // for year in 2025..=2050 {
        //     if let Some(actions) = self.current_run_actions.get(&year) {
        //         if !actions.is_empty() {
        //             println!("  Year {}: {} actions", year, actions.len());
        //         }
        //     }
        // }
        
        // If we have empty current_run_actions but non-empty best actions, something's wrong
        if total_curr_actions == ZERO_USIZE && self.best_actions.is_some() {
            println!("⚠️ WARNING: Attempting to update best strategy with 0 actions in current run!");
            println!("This suggests actions aren't being recorded properly during simulation");
        }
        
        let should_update = match &self.best_metrics {
            None => true,
            Some(best) => {
                let best_score = score_metrics(best, self.optimization_mode.as_deref());
                current_score > best_score
            }
        };

        if should_update {
            // Only print improvement message if we actually had a previous best
            if let Some(best) = &self.best_metrics {
                let best_score = score_metrics(best, self.optimization_mode.as_deref());
                let improvement = ((current_score - best_score) / best_score * PERCENT_CONVERSION).abs();
                
                // Create a VERY visible message with details about the improvement
                println!("\n\n");
                println!("{}", "*".repeat(DEBUG_STAR_COUNT));
                println!("{}", "=".repeat(DEBUG_EQUALS_COUNT));
                println!("🎉🎉🎉  MAJOR STRATEGY IMPROVEMENT FOUND!  🎉🎉🎉");
                println!("{}", "=".repeat(DEBUG_EQUALS_COUNT));
                println!("{}", "*".repeat(DEBUG_STAR_COUNT));
                println!("\nScore improved by {:.2}%", improvement);
                println!("Previous best score: {:.4} → New best score: {:.4}", best_score, current_score);
                println!("Found after {} iterations without improvement", self.iterations_without_improvement);

                // Add more detailed metrics information with better formatting
                println!("\n📊 DETAILED METRICS COMPARISON:");
                
                // Net emissions comparison with appropriate emoji
                let emissions_change = metrics.final_net_emissions - best.final_net_emissions;
                let emissions_emoji = if emissions_change <= ZERO_F64 { "✅" } else { "⚠️" };
                println!("  {} Net emissions: {:.2} → {:.2} ({:+.2})", 
                        emissions_emoji, best.final_net_emissions, metrics.final_net_emissions, emissions_change);
                
                // Net zero status comparison
                let old_net_zero = best.final_net_emissions <= ZERO_F64;
                let new_net_zero = metrics.final_net_emissions <= ZERO_F64;
                let net_zero_emoji = if new_net_zero { "✅" } else { "⚠️" };
                println!("  {} Net zero: {} → {}", 
                        net_zero_emoji, 
                        if old_net_zero { "YES" } else { "NO" }, 
                        if new_net_zero { "YES" } else { "NO" });
                
                // Total cost comparison
                let cost_change = metrics.total_cost - best.total_cost;
                let cost_emoji = if cost_change <= ZERO_F64 { "✅" } else { "⚠️" };
                println!("  {cost_emoji} Cost: €{:.2}B/year → €{:.2}B/year ({:+.2}B)",
                    best.total_cost / BILLION_DIVISOR,
                    metrics.total_cost / BILLION_DIVISOR,
                    cost_change / BILLION_DIVISOR);
                
                // Public opinion comparison
                let opinion_change = metrics.average_public_opinion - best.average_public_opinion;
                let opinion_emoji = if opinion_change >= ZERO_F64 { "✅" } else { "⚠️" };
                println!("  {opinion_emoji} Public opinion: {:.1}% → {:.1}% ({:+.1}%)",
                    best.average_public_opinion * PERCENT_CONVERSION,
                    metrics.average_public_opinion * PERCENT_CONVERSION,
                    opinion_change * PERCENT_CONVERSION);
                
                // Power reliability comparison
                let reliability_change = metrics.power_reliability - best.power_reliability;
                let reliability_emoji = if reliability_change >= ZERO_F64 { "✅" } else { "⚠️" };
                println!("  {reliability_emoji} Power reliability: {:.1}% → {:.1}% ({:+.1}%)",
                    best.power_reliability * PERCENT_CONVERSION,
                    metrics.power_reliability * PERCENT_CONVERSION,
                    reliability_change * PERCENT_CONVERSION);
            } else {
                // First successful strategy found - make this VERY visible too
                println!("\n\n");
                println!("{}", "*".repeat(DEBUG_STAR_COUNT));
                println!("{}", "=".repeat(DEBUG_EQUALS_COUNT));
                println!("🎉🎉🎉  FIRST SUCCESSFUL STRATEGY FOUND!  🎉🎉🎉");
                println!("{}", "=".repeat(DEBUG_EQUALS_COUNT));
                println!("{}", "*".repeat(DEBUG_STAR_COUNT));
                println!("\nInitial score: {:.4}", current_score);
                
                // Add detailed metrics for the first strategy
                println!("\n📊 INITIAL METRICS:");
                println!("  Net emissions: {:.2} tonnes", metrics.final_net_emissions);
                println!("  Total cost: €{:.2}B/year", metrics.total_cost / BILLION_DIVISOR);
                println!("  Public opinion: {:.1}%", metrics.average_public_opinion * PERCENT_CONVERSION);
                println!("  Power reliability: {:.1}%", metrics.power_reliability * PERCENT_CONVERSION);
                
                println!("{}", "=".repeat(DEBUG_EQUALS_COUNT));
                println!("{}", "*".repeat(DEBUG_STAR_COUNT));
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
            for year in START_YEAR..=END_YEAR {
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
            for year in START_YEAR..=END_YEAR {
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
                for year in START_YEAR..=END_YEAR {
                    if let Some(actions) = best_actions.get(&year) {
                        if !actions.is_empty() {
                            println!("  Year {}: {} best actions", year, actions.len());
                        }
                    }
                }
            }
            
            // Reset iterations without improvement counter when we find a better strategy
            self.iterations_without_improvement = ZERO_U32;
        } else {
            // Track iterations without improvement
            self.iterations_without_improvement += 1;
            
            // Occasionally log if we have many iterations without improvement
            if self.iterations_without_improvement % SMALL_LOG_INTERVAL == ZERO_U32 {
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
            optimization_mode: self.optimization_mode.clone(),
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
            for year in START_YEAR..=END_YEAR {
                let mut deficit_year_weights = HashMap::new();
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::GasPeaker), DEFICIT_GAS_PEAKER_WEIGHT);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::GasCombinedCycle), DEFICIT_GAS_COMBINED_WEIGHT);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::BatteryStorage), DEFICIT_BATTERY_WEIGHT);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::PumpedStorage), DEFICIT_PUMPED_STORAGE_WEIGHT);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::Biomass), DEFICIT_BIOMASS_WEIGHT);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::OnshoreWind), DEFICIT_ONSHORE_WIND_WEIGHT);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::OffshoreWind), DEFICIT_OFFSHORE_WIND_WEIGHT);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::UtilitySolar), DEFICIT_UTILITY_SOLAR_WEIGHT);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::HydroDam), DEFICIT_HYDRO_DAM_WEIGHT);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::Nuclear), DEFICIT_NUCLEAR_WEIGHT);
                deficit_year_weights.insert(GridAction::DoNothing, DEFICIT_DO_NOTHING_WEIGHT);
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
            optimization_mode: serializable.optimization_mode.clone(),
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
                
                *weight = (*weight * adjustment_factor).max(MIN_ACTION_WEIGHT).min(ONE_F64);
                
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
            if total_weight <= ZERO_F64 {
                return 0;
            }
            
            let mut random_choice = random_val * total_weight;
            
            for (count, weight) in year_counts {
                random_choice -= weight;
                if random_choice <= ZERO_F64 {
                    return *count;
                }
            }
            
            // Fallback to a reasonable default if sampling fails
            return 5;
        } else {
            // Fallback to simple heuristic if no historical data
            let scaled_exploration = self.exploration_rate.powf(0.5); // Square root to increase base value
            let min_actions = (EXPLORATION_DIVISOR / scaled_exploration).round() as u32;
            let max_actions = (MAX_ACTIONS_MULTIPLIER / scaled_exploration).round() as u32;
            
            match &mut self.deterministic_rng {
                Some(rng) => rng.gen_range(min_actions..=max_actions),
                None => rand::thread_rng().gen_range(min_actions..=max_actions),
            }
        }
    }

    pub fn get_best_metrics(&self) -> Option<(f64, bool)> {
        self.best_metrics.as_ref().map(|metrics| {
            (score_metrics(metrics, self.optimization_mode.as_deref()), metrics.final_net_emissions <= ZERO_F64)
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
            let best_score = score_metrics(best_metrics, self.optimization_mode.as_deref());
            let current_score = score_metrics(current_metrics, self.optimization_mode.as_deref());
            
            // Calculate how much worse the current run is compared to the best
            let deterioration = if best_score > ZERO_F64 {
                (best_score - current_score) / best_score
            } else {
                ZERO_F64
            };
            
            // Only apply contrast learning if the current run is significantly worse (>3%)
            if deterioration > DIVERGENCE_FOR_NEGATIVE_WEIGHT {
                // Calculate stagnation penalty with exponential scaling
                // For stagnation, we want more iterations to have a stronger effect, so we use a power > 1
                let stagnation_iterations = self.iterations_without_improvement as f64 / STAGNATION_ITERATIONS_DIVISOR;
                let stagnation_factor = ONE_F64 + (STAGNATION_PENALTY_FACTOR * stagnation_iterations.powf(STAGNATION_EXPONENT));
                
                // Fix the divergence scaling - for values between 0 and 1, using a power < 1 makes them larger
                // This ensures that worse divergence (higher values) results in stronger penalties
                let scaled_deterioration = deterioration.powf(DIVERGENCE_EXPONENT);
                
                // Calculate the combined penalty multiplier
                let combined_penalty = scaled_deterioration * stagnation_factor;
                
                // Enhanced adaptive learning rate based on stagnation and performance degradation
                let adaptive_learning_rate = self.learning_rate * (ONE_F64 + ADAPTIVE_LEARNING_RATE_FACTOR * self.iterations_without_improvement as f64);
                
                // Log the contrast learning application with more detailed information
                println!("\n🔄 Applying enhanced contrast learning:");
                println!("   - Current run is {:.1}% worse than best", deterioration * PERCENT_CONVERSION);
                println!("   - Iterations without improvement: {}", self.iterations_without_improvement);
                println!("   - Raw deterioration: {:.4}, Scaled: {:.4}", deterioration, scaled_deterioration);
                println!("   - Stagnation factor: {:.2}x", stagnation_factor);
                println!("   - Combined penalty multiplier: {:.4}", combined_penalty);
                println!("   - Adaptive learning rate: {:.4} (base: {:.4})", adaptive_learning_rate, self.learning_rate);
                
                // Calculate the penalty factor - more severe for worse runs and after more stagnation
                let penalty_factor = ONE_F64 / (ONE_F64 + adaptive_learning_rate * PENALTY_MULTIPLIER * combined_penalty);
                
                // Calculate the boost factor for best actions - increases with stagnation
                let best_boost_factor = ONE_F64 + (adaptive_learning_rate * BOOST_MULTIPLIER * stagnation_factor);
                
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
                                    if *weight <= MIN_WEIGHT + WEIGHT_PRECISION_THRESHOLD {
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
                                        let mild_penalty = ONE_F64 / (ONE_F64 + adaptive_learning_rate * combined_penalty * MILD_PENALTY_FACTOR);
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
                    
                    let randomization_factor = RANDOMIZATION_FACTOR; // 10% random variation
                    let mut rng = rand::thread_rng();
                    
                    for year_weights in self.weights.values_mut() {
                        for weight in year_weights.values_mut() {
                            let random_factor = ONE_F64 + randomization_factor * (rng.gen::<f64>() * RANDOM_RANGE_MULTIPLIER - ONE_F64);
                            *weight = (*weight * random_factor).clamp(MIN_WEIGHT, MAX_WEIGHT);
                        }
                    }
                }
            }
        }
    }

    // Update best deficit actions when we find a better overall strategy
    pub fn update_best_deficit_actions(&mut self) {
        if self.iterations_without_improvement == ZERO_U32 {
            // We just found a better strategy, so update best deficit actions
            // Make sure we have entries for each year even if they're empty
            let mut complete_deficit_actions = HashMap::new();
            
            // Initialize empty action lists for all years
            for year in START_YEAR..=END_YEAR {
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
            println!("  Is net zero: {}", if metrics.final_net_emissions <= ZERO_F64 { "true" } else { "false" });
            println!("  Total cost: €{:.2}B", metrics.total_cost / BILLION_DIVISOR);
            println!("  Public opinion: {:.1}%", metrics.average_public_opinion * PERCENT_CONVERSION);
            println!("  Power reliability: {:.1}%", metrics.power_reliability * PERCENT_CONVERSION);
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
        action_pool.push((GridAction::AddGenerator(GeneratorType::OnshoreWind), ONSHORE_WIND_FALLBACK_WEIGHT as u32));
        action_pool.push((GridAction::AddGenerator(GeneratorType::OffshoreWind), OFFSHORE_WIND_FALLBACK_WEIGHT as u32));
        action_pool.push((GridAction::AddGenerator(GeneratorType::UtilitySolar), UTILITY_SOLAR_FALLBACK_WEIGHT as u32));
        
        // Storage becomes more important in middle and late years
        let storage_weight = if year < MID_YEAR_THRESHOLD { STORAGE_WEIGHT_EARLY } else { STORAGE_WEIGHT_LATE };
        action_pool.push((GridAction::AddGenerator(GeneratorType::BatteryStorage), storage_weight as u32));
        
        // Carbon offsets become crucial in later years
        let offset_weight = if year < MID_YEAR_THRESHOLD { OFFSET_WEIGHT_EARLY } else if year < LATE_YEAR_THRESHOLD { OFFSET_WEIGHT_MID } else { OFFSET_WEIGHT_LATE };
        action_pool.push((GridAction::AddCarbonOffset("Forest".to_string()), offset_weight as u32));
        action_pool.push((GridAction::AddCarbonOffset("ActiveCapture".to_string()), offset_weight as u32));
        
        // Gas for reliable power - more important in early years, less in later
        let gas_weight = if year < MID_YEAR_THRESHOLD { GAS_WEIGHT_EARLY } else if year < LATE_YEAR_THRESHOLD { GAS_WEIGHT_MID } else { GAS_WEIGHT_LATE };
        action_pool.push((GridAction::AddGenerator(GeneratorType::GasCombinedCycle), gas_weight as u32));
        
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
        action_pool.push((GridAction::AddGenerator(GeneratorType::GasPeaker), DEFICIT_GAS_PEAKER_FALLBACK_WEIGHT as u32));
        action_pool.push((GridAction::AddGenerator(GeneratorType::BatteryStorage), DEFICIT_BATTERY_FALLBACK_WEIGHT as u32));
        
        // Medium-term reliable options
        action_pool.push((GridAction::AddGenerator(GeneratorType::GasCombinedCycle), DEFICIT_GAS_COMBINED_FALLBACK_WEIGHT as u32));
        
        // Renewables - lower priority for deficit but still included
        action_pool.push((GridAction::AddGenerator(GeneratorType::OnshoreWind), DEFICIT_ONSHORE_WIND_FALLBACK_WEIGHT as u32));
        action_pool.push((GridAction::AddGenerator(GeneratorType::OffshoreWind), (DEFICIT_OFFSHORE_WIND_WEIGHT * RENEWABLE_FALLBACK_WEIGHT_FACTOR) as u32));
        action_pool.push((GridAction::AddGenerator(GeneratorType::UtilitySolar), (DEFICIT_UTILITY_SOLAR_WEIGHT * RENEWABLE_FALLBACK_WEIGHT_FACTOR * PERCENT_CONVERSION) as u32));
        
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
                                            *current_weight * (ONE_F64 - best_weight_factor);
                        } else {
                            // Action exists in best but not in current, add it
                            current_year_weights.insert(action.clone(), best_weight);
                        }
                    }
                }
            }
            
            println!("   - Restored weights with {:.0}% best weights / {:.0}% current weights", 
                    best_weight_factor * PERCENT_CONVERSION, (ONE_F64 - best_weight_factor) * PERCENT_CONVERSION);
        }
    }

    // Then add the apply_deficit_contrast_learning method
    pub fn apply_deficit_contrast_learning(&mut self) {
        // Only apply contrast learning if we have a best run to compare against
        if let (Some(best_metrics), Some(best_deficit_actions)) = (&self.best_metrics, &self.best_deficit_actions) {
            let best_score = score_metrics(best_metrics, self.optimization_mode.as_deref());
            // We don't have a current metrics specific to deficit actions, but we can use the deterioration
            // from the regular contrast learning as an approximation
            let deterioration = self.iterations_without_improvement as f64 / STAGNATION_ITERATIONS_DIVISOR; // Use iterations as a proxy for deterioration
            
            // Only apply contrast learning if there's some deterioration
            if deterioration > ZERO_F64 {
                // Calculate stagnation penalty with exponential scaling
                let stagnation_iterations = self.iterations_without_improvement as f64 / STAGNATION_ITERATIONS_DIVISOR;
                let stagnation_factor = ONE_F64 + (STAGNATION_PENALTY_FACTOR * stagnation_iterations.powf(STAGNATION_EXPONENT));
                
                // Scale the deterioration like in regular contrast learning
                let scaled_deterioration = deterioration.powf(DIVERGENCE_EXPONENT);
                
                // Calculate the combined penalty multiplier
                let combined_penalty = scaled_deterioration * stagnation_factor;
                
                // Calculate adaptive learning rate as in regular contrast learning
                let adaptive_learning_rate = self.learning_rate * (ONE_F64 + ADAPTIVE_LEARNING_RATE_FACTOR * self.iterations_without_improvement as f64);
                
                // Log the contrast learning application with more detailed information
                println!("\n🔄 Applying enhanced contrast learning to deficit handling actions:");
                println!("   - Iterations without improvement: {}", self.iterations_without_improvement);
                println!("   - Proxy deterioration: {:.4}, Scaled: {:.4}", deterioration, scaled_deterioration);
                println!("   - Stagnation factor: {:.2}x", stagnation_factor);
                println!("   - Combined penalty multiplier: {:.4}", combined_penalty);
                
                // Calculate the penalty factor - more severe for worse runs and after more stagnation
                let penalty_factor = ONE_F64 / (ONE_F64 + adaptive_learning_rate * PENALTY_MULTIPLIER * combined_penalty);
                let best_boost_factor = ONE_F64 + (adaptive_learning_rate * BOOST_MULTIPLIER * stagnation_factor);
                
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

                // If we've been stagnating for a very long time, also apply some randomization
                // to break out of local optima
                if self.iterations_without_improvement > ITERATIONS_FOR_RANDOMIZATION {
                    println!("   - Applying weight randomization to deficit weights after {} iterations", 
                            self.iterations_without_improvement);
                    
                    let mut rng = rand::thread_rng();
                    
                    for year_weights in self.deficit_weights.values_mut() {
                        for weight in year_weights.values_mut() {
                            let random_factor = ONE_F64 + RANDOMIZATION_FACTOR * (rng.gen::<f64>() * 2.0 - 1.0);
                            *weight = (*weight * random_factor).clamp(MIN_WEIGHT, MAX_WEIGHT);
                        }
                    }
                }
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
                    let current_index = self.current_deficit_actions.get(&year).map_or(ZERO_USIZE, |v| v.len());
                    if current_index < year_deficit_actions.len() {
                        let action = year_deficit_actions[current_index].clone();
                        println!("🔄 DEFICIT REPLAY: Using best deficit action #{} for year {}: {:?}", 
                                current_index + ONE_USIZE, year, action);
                        
                        // Make sure to record this replayed deficit action
                        self.current_deficit_actions.entry(year)
                            .or_insert_with(Vec::new)
                            .push(action.clone());
                        
                        return action;
                    } else {
                        println!("⚠️ DEFICIT REPLAY FALLBACK: Ran out of best deficit actions for year {} (needed action #{}, have {})",
                                year, current_index + ONE_USIZE, year_deficit_actions.len());
                        
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
                Some(rng) => rng.gen_range(ZERO_USIZE..actions.len()),
                None => rand::thread_rng().gen_range(ZERO_USIZE..actions.len()),
            };
            
            return actions[random_idx].clone();
        }
        
        // Exploitation - weighted selection of generator actions
        let total_weight: f64 = year_weights.iter()
            .filter(|(action, _)| matches!(action, GridAction::AddGenerator(_)))
            .map(|(_, &weight)| weight)
            .sum();
        
        if total_weight <= ZERO_F64 {
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
                if random_val <= ZERO_F64 {
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
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::GasPeaker), DEFICIT_GAS_PEAKER_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::GasCombinedCycle), DEFICIT_GAS_COMBINED_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::BatteryStorage), DEFICIT_BATTERY_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::PumpedStorage), DEFICIT_PUMPED_STORAGE_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::Biomass), DEFICIT_BIOMASS_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::OnshoreWind), DEFICIT_ONSHORE_WIND_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::OffshoreWind), DEFICIT_OFFSHORE_WIND_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::UtilitySolar), DEFICIT_UTILITY_SOLAR_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::HydroDam), DEFICIT_HYDRO_DAM_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::Nuclear), DEFICIT_NUCLEAR_WEIGHT);
            deficit_year_weights.insert(GridAction::DoNothing, DEFICIT_DO_NOTHING_WEIGHT);
            self.deficit_weights.insert(year, deficit_year_weights);
        }
        
        let year_weights = self.deficit_weights.get_mut(&year).expect("Year weights not found");
        
        // If the action doesn't exist in weights, initialize it
        if !year_weights.contains_key(action) {
            year_weights.insert(action.clone(), DEFAULT_WEIGHT);
        }
        
        let current_weight = year_weights.get(action).expect("Weight should exist");
        
        // Calculate adjustment factor similar to normal action weights
        let adjustment_factor = if improvement > ZERO_F64 {
            // For improvements, increase weight proportionally to the improvement
            ONE_F64 + (self.learning_rate * improvement * DEFICIT_REINFORCEMENT_MULTIPLIER)
        } else {
            // For deteriorations, decrease weight proportionally to how bad it was
            ONE_F64 / (ONE_F64 + (self.learning_rate * improvement.abs() * DEFICIT_REINFORCEMENT_MULTIPLIER))
        };
        
        // Apply the adjustment with bounds
        let new_weight = (current_weight * adjustment_factor)
            .max(MIN_WEIGHT)
            .min(MAX_WEIGHT);
        
        year_weights.insert(action.clone(), new_weight);
        
        // If this was a bad outcome, slightly increase weights of other generator types
        if improvement < ZERO_F64 {
            let boost_factor = ONE_F64 + (self.learning_rate * SMALL_BOOST_FACTOR); // Small boost to alternatives
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
        let min_year = START_YEAR;
        let max_year = END_YEAR;
        
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
        let min_year = START_YEAR;
        let max_year = END_YEAR;
        
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

    // Add around line 1993 before the score_metrics function
    pub fn set_optimization_mode(&mut self, mode: Option<String>) {
        self.optimization_mode = mode;
    }

    pub fn get_optimization_mode(&self) -> Option<&str> {
        self.optimization_mode.as_deref()
    }
}

pub fn score_metrics(metrics: &SimulationMetrics, optimization_mode: Option<&str>) -> f64 {
    // Check for cost-only optimization mode
    if let Some(mode) = optimization_mode {
        if mode == "cost_only" {
            // In cost-only mode, only consider cost improvements regardless of emissions state
            // Normalize and invert cost so lower costs give higher scores
            let normalized_cost = (metrics.total_cost / MAX_ACCEPTABLE_COST).max(ONE_F64);
            let log_cost = normalized_cost.ln();
            let max_expected_log_cost = (MAX_ACCEPTABLE_COST * MAX_BUDGET_MULTIPLIER / MAX_ACCEPTABLE_COST).ln(); // Assume 100x budget is max
            return MAX_SCORE_RANGE - (log_cost / max_expected_log_cost).min(ONE_F64); // Return value between 1.0 and 2.0
        }
    }

    // Default scoring logic - First priority: Reach net zero emissions
    if metrics.final_net_emissions > ZERO_F64 {
        // If we haven't achieved net zero, only focus on reducing emissions
        ONE_F64 - (metrics.final_net_emissions / MAX_ACCEPTABLE_EMISSIONS).min(ONE_F64)
    }
    // Second priority: Optimize costs after achieving net zero
    else {
        // Base score of 1.0 for achieving net zero
        let base_score = BASE_NET_ZERO_SCORE;
        
        // Cost component - normalized and inverted so lower costs give higher scores
        // Use log scale to differentiate between very high costs
        let normalized_cost = (metrics.total_cost / MAX_ACCEPTABLE_COST).max(ONE_F64);
        let log_cost = normalized_cost.ln();
        let max_expected_log_cost = (MAX_ACCEPTABLE_COST * MAX_BUDGET_MULTIPLIER / MAX_ACCEPTABLE_COST).ln(); // Assume 100x budget is max
        let cost_score = ONE_F64 - (log_cost / max_expected_log_cost).min(ONE_F64);
        
        // Public opinion component
        let opinion_score = metrics.average_public_opinion;
        
        // Combine scores with appropriate weights
        // Cost is higher priority until it's reasonable
        let cost_weight = if normalized_cost > HIGH_COST_THRESHOLD_MULTIPLIER { HIGH_COST_WEIGHT } else { NORMAL_COST_WEIGHT };
        let opinion_weight = ONE_F64 - cost_weight;
        
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
    optimization_mode: Option<&str>,
) -> f64 {
    // Check for cost-only optimization mode
    if let Some(mode) = optimization_mode {
        if mode == "cost_only" {
            // In cost-only mode, only consider cost improvements regardless of emissions state
            let cost_change = new_state.total_cost - current_state.total_cost;
            return -cost_change / current_state.total_cost.abs().max(ONE_F64);
        }
    }

    // Default evaluation logic
    if current_state.net_emissions > ZERO_F64 {
        // First priority: If we haven't achieved net zero, only consider emissions
        let emissions_improvement = (current_state.net_emissions - new_state.net_emissions) / 
                                  current_state.net_emissions.abs().max(ONE_F64);
        emissions_improvement
    }
    else {
        // If we've achieved net zero, consider both cost and opinion improvements
        
        // Cost improvement (negative is better)
        let cost_change = new_state.total_cost - current_state.total_cost;
        let cost_improvement = -cost_change / current_state.total_cost.abs().max(ONE_F64);
        
        // Opinion improvement
        let opinion_improvement = (new_state.public_opinion - current_state.public_opinion) /
                                current_state.public_opinion.abs().max(ONE_F64);
        
        // Weight cost more heavily if it's very high
        let cost_weight = if current_state.total_cost > MAX_ACCEPTABLE_COST * HIGH_COST_THRESHOLD_MULTIPLIER { HIGH_COST_WEIGHT } else { NORMAL_COST_WEIGHT };
        let opinion_weight = ONE_F64 - cost_weight;
        
        // Combined improvement score
        cost_improvement * cost_weight + opinion_improvement * opinion_weight
    }
}