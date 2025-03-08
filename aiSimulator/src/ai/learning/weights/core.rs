// Core operations for ActionWeights

use std::collections::HashMap;
use rand::rngs::StdRng;
use rand::Rng;
use crate::models::generator::GeneratorType;
use crate::ai::actions::grid_action::GridAction;
use crate::ai::metrics::simulation_metrics::SimulationMetrics;
use crate::ai::learning::constants::*;
use crate::ai::score_metrics;
use super::ActionWeights;

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
            year_weights.insert(GridAction::AddCarbonOffset("Wetland".to_string()), CARBON_OFFSET_WEIGHT * 0.8);
            year_weights.insert(GridAction::AddCarbonOffset("ActiveCapture".to_string()), CARBON_OFFSET_WEIGHT * 1.2);
            year_weights.insert(GridAction::AddCarbonOffset("CarbonCredit".to_string()), CARBON_OFFSET_WEIGHT * 0.6);
            
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
        
        // Adjust exploration rate based on iteration count and stagnation
        self.exploration_rate = DEFAULT_EXPLORATION_RATE * (ONE_F64 / (ONE_F64 + EXPLORATION_DECAY_RATE * self.iteration_count as f64));
        
        // Increase exploration if we have stagnated
        if self.iterations_without_improvement > ZERO_U32 {
            if self.iterations_without_improvement % SMALL_LOG_INTERVAL == ZERO_U32 {
                println!("⚠️ Currently at {} iterations without improvement", self.iterations_without_improvement);
            }
            
            if self.iterations_without_improvement > HIGH_ITERATION_THRESHOLD && self.iterations_without_improvement % STAGNATION_DIVISOR_INT == ZERO_U32 {
                // If we're stagnating, periodically restore the best weights with some randomization
                println!("⚠️ Stagnation detected: Restoring best weights with randomization after {} iterations without improvement",
                            self.iterations_without_improvement);
                
                self.restore_best_weights(BEST_WEIGHT_FACTOR);
            }
            
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

}
