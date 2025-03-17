// Core operations for ActionWeights

use std::collections::HashMap;
use rand::rngs::StdRng;
use rand::Rng;
use crate::models::generator::GeneratorType;
use crate::models::carbon_offset::CarbonOffsetType;
use crate::ai::actions::grid_action::GridAction;
use crate::ai::metrics::simulation_metrics::SimulationMetrics;
use crate::ai::learning::constants::*;
use crate::ai::score_metrics;
use crate::config::constants::{DEFAULT_COST_MULTIPLIER, FAST_COST_MULTIPLIER, VERY_FAST_COST_MULTIPLIER};
use super::ActionWeights;
use crate::utils::csv_export::ImprovementRecord;

// Add a dummy public item to ensure this file is recognized by rust-analyzer
#[allow(dead_code)]
pub const MODULE_MARKER: &str = "core_module";

impl ActionWeights {

// This file contains extracted code from the original weights.rs file
// Appropriate imports will need to be added based on the specific requirements

    pub fn new() -> Self {
        let mut weights = HashMap::new();
        let mut deficit_weights = HashMap::new();
        let mut action_count_weights = HashMap::new();
        
        // Initialize weights for each year from 2025 to 2050
        for year in START_YEAR..=END_YEAR {
            let mut year_weights = HashMap::new();
            
            // Initialize wind generator weights
            year_weights.insert(GridAction::AddGenerator(GeneratorType::OnshoreWind, DEFAULT_COST_MULTIPLIER), ONSHORE_WIND_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::OnshoreWind, FAST_COST_MULTIPLIER), ONSHORE_WIND_WEIGHT * 0.5);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::OnshoreWind, VERY_FAST_COST_MULTIPLIER), ONSHORE_WIND_WEIGHT * 0.25);
            
            year_weights.insert(GridAction::AddGenerator(GeneratorType::OffshoreWind, DEFAULT_COST_MULTIPLIER), OFFSHORE_WIND_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::OffshoreWind, FAST_COST_MULTIPLIER), OFFSHORE_WIND_WEIGHT * 0.5);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::OffshoreWind, VERY_FAST_COST_MULTIPLIER), OFFSHORE_WIND_WEIGHT * 0.25);
            
            // Initialize solar generator weights
            year_weights.insert(GridAction::AddGenerator(GeneratorType::DomesticSolar, DEFAULT_COST_MULTIPLIER), DOMESTIC_SOLAR_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::DomesticSolar, FAST_COST_MULTIPLIER), DOMESTIC_SOLAR_WEIGHT * 0.5);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::DomesticSolar, VERY_FAST_COST_MULTIPLIER), DOMESTIC_SOLAR_WEIGHT * 0.25);
            
            year_weights.insert(GridAction::AddGenerator(GeneratorType::CommercialSolar, DEFAULT_COST_MULTIPLIER), COMMERCIAL_SOLAR_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::CommercialSolar, FAST_COST_MULTIPLIER), COMMERCIAL_SOLAR_WEIGHT * 0.5);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::CommercialSolar, VERY_FAST_COST_MULTIPLIER), COMMERCIAL_SOLAR_WEIGHT * 0.25);
            
            year_weights.insert(GridAction::AddGenerator(GeneratorType::UtilitySolar, DEFAULT_COST_MULTIPLIER), UTILITY_SOLAR_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::UtilitySolar, FAST_COST_MULTIPLIER), UTILITY_SOLAR_WEIGHT * 0.5);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::UtilitySolar, VERY_FAST_COST_MULTIPLIER), UTILITY_SOLAR_WEIGHT * 0.25);
            
            // Initialize nuclear and fossil fuel generator weights
            year_weights.insert(GridAction::AddGenerator(GeneratorType::Nuclear, DEFAULT_COST_MULTIPLIER), NUCLEAR_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::Nuclear, FAST_COST_MULTIPLIER), NUCLEAR_WEIGHT * 0.5);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::Nuclear, VERY_FAST_COST_MULTIPLIER), NUCLEAR_WEIGHT * 0.25);
            
            year_weights.insert(GridAction::AddGenerator(GeneratorType::CoalPlant, DEFAULT_COST_MULTIPLIER), COAL_PLANT_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::CoalPlant, FAST_COST_MULTIPLIER), COAL_PLANT_WEIGHT * 0.5);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::CoalPlant, VERY_FAST_COST_MULTIPLIER), COAL_PLANT_WEIGHT * 0.25);
            
            year_weights.insert(GridAction::AddGenerator(GeneratorType::GasCombinedCycle, DEFAULT_COST_MULTIPLIER), GAS_COMBINED_CYCLE_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::GasCombinedCycle, FAST_COST_MULTIPLIER), GAS_COMBINED_CYCLE_WEIGHT * 0.5);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::GasCombinedCycle, VERY_FAST_COST_MULTIPLIER), GAS_COMBINED_CYCLE_WEIGHT * 0.25);
            
            year_weights.insert(GridAction::AddGenerator(GeneratorType::GasPeaker, DEFAULT_COST_MULTIPLIER), GAS_PEAKER_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::GasPeaker, FAST_COST_MULTIPLIER), GAS_PEAKER_WEIGHT * 0.5);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::GasPeaker, VERY_FAST_COST_MULTIPLIER), GAS_PEAKER_WEIGHT * 0.25);
            
            year_weights.insert(GridAction::AddGenerator(GeneratorType::Biomass, DEFAULT_COST_MULTIPLIER), BIOMASS_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::Biomass, FAST_COST_MULTIPLIER), BIOMASS_WEIGHT * 0.5);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::Biomass, VERY_FAST_COST_MULTIPLIER), BIOMASS_WEIGHT * 0.25);
            
            // Initialize hydro and storage generator weights
            year_weights.insert(GridAction::AddGenerator(GeneratorType::HydroDam, DEFAULT_COST_MULTIPLIER), HYDRO_DAM_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::HydroDam, FAST_COST_MULTIPLIER), HYDRO_DAM_WEIGHT * 0.5);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::HydroDam, VERY_FAST_COST_MULTIPLIER), HYDRO_DAM_WEIGHT * 0.25);
            
            year_weights.insert(GridAction::AddGenerator(GeneratorType::PumpedStorage, DEFAULT_COST_MULTIPLIER), PUMPED_STORAGE_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::PumpedStorage, FAST_COST_MULTIPLIER), PUMPED_STORAGE_WEIGHT * 0.5);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::PumpedStorage, VERY_FAST_COST_MULTIPLIER), PUMPED_STORAGE_WEIGHT * 0.25);
            
            year_weights.insert(GridAction::AddGenerator(GeneratorType::BatteryStorage, DEFAULT_COST_MULTIPLIER), BATTERY_STORAGE_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::BatteryStorage, FAST_COST_MULTIPLIER), BATTERY_STORAGE_WEIGHT * 0.5);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::BatteryStorage, VERY_FAST_COST_MULTIPLIER), BATTERY_STORAGE_WEIGHT * 0.25);
            
            // Initialize marine generator weights
            year_weights.insert(GridAction::AddGenerator(GeneratorType::TidalGenerator, DEFAULT_COST_MULTIPLIER), TIDAL_GENERATOR_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::TidalGenerator, FAST_COST_MULTIPLIER), TIDAL_GENERATOR_WEIGHT * 0.5);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::TidalGenerator, VERY_FAST_COST_MULTIPLIER), TIDAL_GENERATOR_WEIGHT * 0.25);
            
            year_weights.insert(GridAction::AddGenerator(GeneratorType::WaveEnergy, DEFAULT_COST_MULTIPLIER), WAVE_ENERGY_WEIGHT);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::WaveEnergy, FAST_COST_MULTIPLIER), WAVE_ENERGY_WEIGHT * 0.5);
            year_weights.insert(GridAction::AddGenerator(GeneratorType::WaveEnergy, VERY_FAST_COST_MULTIPLIER), WAVE_ENERGY_WEIGHT * 0.25);
            
            // Initialize carbon offset weights
            year_weights.insert(GridAction::AddCarbonOffset(CarbonOffsetType::Forest, DEFAULT_COST_MULTIPLIER), CARBON_OFFSET_WEIGHT);
            year_weights.insert(GridAction::AddCarbonOffset(CarbonOffsetType::Forest, FAST_COST_MULTIPLIER), CARBON_OFFSET_WEIGHT * 0.5);
            year_weights.insert(GridAction::AddCarbonOffset(CarbonOffsetType::Forest, VERY_FAST_COST_MULTIPLIER), CARBON_OFFSET_WEIGHT * 0.25);
            
            year_weights.insert(GridAction::AddCarbonOffset(CarbonOffsetType::Wetland, DEFAULT_COST_MULTIPLIER), CARBON_OFFSET_WEIGHT);
            year_weights.insert(GridAction::AddCarbonOffset(CarbonOffsetType::Wetland, FAST_COST_MULTIPLIER), CARBON_OFFSET_WEIGHT * 0.5);
            year_weights.insert(GridAction::AddCarbonOffset(CarbonOffsetType::Wetland, VERY_FAST_COST_MULTIPLIER), CARBON_OFFSET_WEIGHT * 0.25);
            
            year_weights.insert(GridAction::AddCarbonOffset(CarbonOffsetType::ActiveCapture, DEFAULT_COST_MULTIPLIER), CARBON_OFFSET_WEIGHT);
            year_weights.insert(GridAction::AddCarbonOffset(CarbonOffsetType::ActiveCapture, FAST_COST_MULTIPLIER), CARBON_OFFSET_WEIGHT * 0.5);
            year_weights.insert(GridAction::AddCarbonOffset(CarbonOffsetType::ActiveCapture, VERY_FAST_COST_MULTIPLIER), CARBON_OFFSET_WEIGHT * 0.25);
            
            year_weights.insert(GridAction::AddCarbonOffset(CarbonOffsetType::CarbonCredit, DEFAULT_COST_MULTIPLIER), CARBON_OFFSET_WEIGHT);
            year_weights.insert(GridAction::AddCarbonOffset(CarbonOffsetType::CarbonCredit, FAST_COST_MULTIPLIER), CARBON_OFFSET_WEIGHT * 0.5);
            year_weights.insert(GridAction::AddCarbonOffset(CarbonOffsetType::CarbonCredit, VERY_FAST_COST_MULTIPLIER), CARBON_OFFSET_WEIGHT * 0.25);
            
            // Initialize other action weights
            year_weights.insert(GridAction::UpgradeEfficiency(String::new()), UPGRADE_EFFICIENCY_WEIGHT);
            year_weights.insert(GridAction::AdjustOperation(String::new(), OPERATION_PERCENTAGE_MIN), ADJUST_OPERATION_WEIGHT);
            year_weights.insert(GridAction::CloseGenerator(String::new()), CLOSE_GENERATOR_WEIGHT);
            year_weights.insert(GridAction::DoNothing, DO_NOTHING_WEIGHT);
            
            // Add year's weights to the map
            weights.insert(year, year_weights);

            // Initialize deficit handling weights with a separate set of weights
            // focused on reliable power generation options
            let mut deficit_year_weights = HashMap::new();
            
            // For deficit handling, prioritize fast-responding and reliable generators
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::GasPeaker, DEFAULT_COST_MULTIPLIER), DEFICIT_GAS_PEAKER_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::GasCombinedCycle, DEFAULT_COST_MULTIPLIER), DEFICIT_GAS_COMBINED_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::BatteryStorage, DEFAULT_COST_MULTIPLIER), DEFICIT_BATTERY_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::PumpedStorage, DEFAULT_COST_MULTIPLIER), DEFICIT_PUMPED_STORAGE_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::Biomass, DEFAULT_COST_MULTIPLIER), DEFICIT_BIOMASS_WEIGHT);
            
            // Include renewables with lower initial weights for deficit handling
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::OnshoreWind, DEFAULT_COST_MULTIPLIER), DEFICIT_ONSHORE_WIND_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::OffshoreWind, DEFAULT_COST_MULTIPLIER), DEFICIT_OFFSHORE_WIND_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::UtilitySolar, DEFAULT_COST_MULTIPLIER), DEFICIT_UTILITY_SOLAR_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::HydroDam, DEFAULT_COST_MULTIPLIER), DEFICIT_HYDRO_DAM_WEIGHT);
            
            // Include nuclear with a lower weight due to long build time
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::Nuclear, DEFAULT_COST_MULTIPLIER), DEFICIT_NUCLEAR_WEIGHT);
            
            // Add other types with minimal weights
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::DomesticSolar, DEFAULT_COST_MULTIPLIER), DEFICIT_SMALL_GENERATOR_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::CommercialSolar, DEFAULT_COST_MULTIPLIER), DEFICIT_SMALL_GENERATOR_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::TidalGenerator, DEFAULT_COST_MULTIPLIER), DEFICIT_SMALL_GENERATOR_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::WaveEnergy, DEFAULT_COST_MULTIPLIER), DEFICIT_SMALL_GENERATOR_WEIGHT);
            
            // DoNothing should have very low weight for deficit handling
            deficit_year_weights.insert(GridAction::DoNothing, DEFICIT_DO_NOTHING_WEIGHT);
            
            // Add deficit weights for this year
            deficit_weights.insert(year, deficit_year_weights);

            // Initialize action count weights for this year with extreme bias towards fewer actions
            let mut count_weights = HashMap::new();
            let decay_rate = ACTION_COUNT_DECAY_RATE; // Controls how quickly the probability decreases
            let mut total_weight = ZERO_F64;
            
            // Calculate weights with exponential decay, plus additional bias for lower counts
            for count in 0..=MAX_ACTION_COUNT {
                // Basic exponential decay
                let base_weight = (-decay_rate * count as f64).exp();
                
                // Apply additional bias for low counts (0-5 actions)
                let multiplier = match count {
                    0 => 4.0,  // Very high weight for taking no actions
                    1 => 3.5,  // Very high weight for just 1 action
                    2 => 3.0,  // High weight for 2 actions
                    3 => 2.5,  // High weight for 3 actions
                    4 => 2.0,  // Medium-high weight for 4 actions
                    5 => 1.5,  // Slightly increased weight for 5 actions
                    _ => 1.0,  // Normal weight for 6+ actions
                };
                
                let weight = base_weight * multiplier;
                count_weights.insert(count, weight);
                total_weight += weight;
            }
            
            // Normalize weights to sum to ONE_F64
            for weight in count_weights.values_mut() {
                *weight /= total_weight;
            }
            
            // Print the initial weights for visibility only if debug weights is enabled
            if year == START_YEAR && crate::ai::learning::constants::is_debug_weights_enabled() {
                println!("\nInitial action count weights:");
                let instance = ActionWeights {
                    weights: HashMap::new(),
                    action_count_weights: HashMap::from([(year, count_weights.clone())]),
                    learning_rate: DEFAULT_LEARNING_RATE,
                    best_metrics: None,
                    best_weights: None,
                    best_actions: None,
                    iteration_count: 0,
                    iterations_without_improvement: 0,
                    exploration_rate: DEFAULT_EXPLORATION_RATE,
                    current_run_actions: HashMap::new(),
                    force_best_actions: false,
                    deficit_weights: HashMap::new(),
                    current_deficit_actions: HashMap::new(),
                    best_deficit_actions: None,
                    deterministic_rng: None,
                    guaranteed_best_actions: false,
                    optimization_mode: None,
                    replay_index: HashMap::new(),
                    improvement_history: Vec::new(),
                };
                instance.print_action_count_weights(year);
            }
            
            action_count_weights.insert(year, count_weights);
        }
        
        // DIAGNOSTIC: Add logging to check ActionWeights initialization
        println!("DIAGNOSTIC: Initializing new ActionWeights");
        println!("  - Starting with weights for {} years", weights.len());
        
        let instance = Self {
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
            replay_index: HashMap::new(),
            improvement_history: Vec::new(),
        };
        
        // DIAGNOSTIC: Log the created instance details
        println!("  - Instance created with iteration_count: {}", instance.iteration_count);
        println!("  - Instance has best_metrics: {}", instance.best_metrics.is_some());
        println!("  - Instance has best_actions: {}", instance.best_actions.is_some());
        
        instance
    }

    pub fn set_rng(&mut self, rng: StdRng) {
        self.deterministic_rng = Some(rng);
    }

    pub fn start_new_iteration(&mut self) {
        // DIAGNOSTIC: Log the beginning of a new iteration
        println!("DIAGNOSTIC: Starting iteration {}", self.iteration_count + 1);
        
        // Clear current actions
        self.current_run_actions.clear();
        self.current_deficit_actions.clear();
        self.replay_index.clear();
        
        // Adjust exploration rate based on iteration count and stagnation
        self.exploration_rate = DEFAULT_EXPLORATION_RATE * (ONE_F64 / (ONE_F64 + EXPLORATION_DECAY_RATE * self.iteration_count as f64));
        
        // Increase exploration if we have stagnated
        if self.iterations_without_improvement > ZERO_U32 {
            if self.iterations_without_improvement % SMALL_LOG_INTERVAL == ZERO_U32 {
                println!("⚠️ Currently at {} iterations without improvement", self.iterations_without_improvement);
            }
            
            // Removed weight restoration to allow contrast learning to have more effect
            // Previously restored best weights here when iterations_without_improvement > HIGH_ITERATION_THRESHOLD
            
            if self.iterations_without_improvement > FORCE_REPLAY_THRESHOLD {
                // Based on how long we've been stagnant, consider forcing replay of best actions
                let force_replay_probability = ((self.iterations_without_improvement - FORCE_REPLAY_THRESHOLD) as f64 / FORCE_REPLAY_DIVISOR).min(PERCENTAGE_THRESHOLD);
                
                // Get a random number for decision
                let random_value = if let Some(ref mut rng) = self.deterministic_rng {
                    rng.gen::<f64>()
                } else {
                    rand::thread_rng().gen::<f64>()
                };
                
                // Decide whether to force replay based on probability
                self.force_best_actions = random_value < force_replay_probability;
                
                if self.force_best_actions {
                    println!("🔁 After {} iterations without improvement, forcing replay of best actions (p={:.1}%)", 
                            self.iterations_without_improvement, force_replay_probability * PERCENT_CONVERSION);
                }
            }
            
            // If we've been stagnant for a very long time, try more aggressive randomization
            if self.iterations_without_improvement > ITERATIONS_FOR_RANDOMIZATION {
                println!("⚠️ EXTREME stagnation: Random reset after {} iterations without improvement", 
                        self.iterations_without_improvement);
                
                // Apply randomization to weights
                let mut rng = rand::thread_rng();
                for year_weights in self.weights.values_mut() {
                    for weight in year_weights.values_mut() {
                        let random_factor = ONE_F64 + RANDOMIZATION_FACTOR * (rng.gen::<f64>() * RANDOM_RANGE_MULTIPLIER - ONE_F64);
                        *weight = (*weight * random_factor).clamp(MIN_WEIGHT, MAX_WEIGHT);
                    }
                }
            }
        }
        
        // DIAGNOSTIC: Log the iteration preparation details
        println!("  - Cleared action records for current run");
        println!("  - Set exploration_rate to {:.6}", self.exploration_rate);
        println!("  - force_best_actions: {}", self.force_best_actions);
        println!("  - iterations_without_improvement: {}", self.iterations_without_improvement);
    }

    pub fn get_year_weights(&self, year: u32) -> Option<&HashMap<GridAction, f64>> {
        self.weights.get(&year)
    }

    pub fn get_best_metrics(&self) -> Option<(f64, bool)> {
        self.best_metrics.as_ref().map(|metrics| {
            (score_metrics(metrics, self.optimization_mode.as_deref()), metrics.final_net_emissions <= ZERO_F64)
        })
    }

    pub fn get_simulation_metrics(&self) -> Option<&SimulationMetrics> {
        self.best_metrics.as_ref()
    }

    pub fn has_best_actions(&self) -> bool {
        self.best_actions.is_some()
    }

    pub fn set_force_best_actions(&mut self, force: bool) {
        self.force_best_actions = force;
    }

    pub fn set_guaranteed_best_actions(&mut self, force: bool) {
        self.force_best_actions = force;
        // Setting this flag means we bypass the probability check in start_new_iteration
        // and always use best actions if available
        self.guaranteed_best_actions = force;
    }

    pub fn set_optimization_mode(&mut self, mode: Option<String>) {
        self.optimization_mode = mode;
    }

    pub fn get_optimization_mode(&self) -> Option<&str> {
        self.optimization_mode.as_deref()
    }
    
    /// Clears the current run actions and deficit actions
    pub fn clear_current_run_actions(&mut self) {
        self.current_run_actions.clear();
        self.current_deficit_actions.clear();
    }
    
    /// Clears the replay index for all years
    pub fn clear_replay_index(&mut self) {
        self.replay_index.clear();
    }
    
    /// Returns the improvement history tracking all improvements to the best strategy
    pub fn get_improvement_history(&self) -> &[ImprovementRecord] {
        &self.improvement_history
    }
    
    /// Returns the total number of improvements found during training
    pub fn get_improvement_count(&self) -> usize {
        self.improvement_history.len()
    }
}
