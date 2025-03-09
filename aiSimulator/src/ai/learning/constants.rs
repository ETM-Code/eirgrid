// Constants module for AI learning components
// All constants used by the ActionWeights implementation

// Re-export these to avoid breaking existing code
pub use crate::models::generator::GeneratorType;
pub use crate::ai::actions::grid_action::GridAction;
pub use crate::ai::metrics::simulation_metrics::{SimulationMetrics, ActionResult};
pub use crate::config::constants::{MAX_ACCEPTABLE_EMISSIONS, MAX_ACCEPTABLE_COST};

//---------------------------------------------------------------------
// Default Values
//---------------------------------------------------------------------
pub const DEFAULT_WEIGHT: f64 = 0.5;
pub const MIN_WEIGHT: f64 = 0.0001;  // Ensure weight doesn't go too close to zero
pub const MAX_WEIGHT: f64 = 0.999;  // Ensure weight doesn't dominate completely
pub const MIN_ACTION_WEIGHT: f64 = 0.01;
pub const DEFAULT_LEARNING_RATE: f64 = 0.2;
pub const DEFAULT_EXPLORATION_RATE: f64 = 0.2;

//---------------------------------------------------------------------
// Common Numeric Constants
//---------------------------------------------------------------------
pub const ZERO_F64: f64 = 0.0;
pub const ONE_F64: f64 = 1.0;
pub const ZERO_USIZE: usize = 0;
pub const ONE_USIZE: usize = 1;
pub const ZERO_U32: u32 = 0;
pub const ONE_U32: u32 = 1;
pub const ZERO_U8: u8 = 0;

//---------------------------------------------------------------------
// Simulation Year Constants
//---------------------------------------------------------------------
pub const START_YEAR: u32 = 2025;
pub const END_YEAR: u32 = 2050;
pub const MID_YEAR_THRESHOLD: u32 = 2035;
pub const LATE_YEAR_THRESHOLD: u32 = 2045;

//---------------------------------------------------------------------
// Generator Weight Constants
//---------------------------------------------------------------------
pub const DIVERGENCE_FOR_NEGATIVE_WEIGHT: f64 = 0.03; // The difference of improvement necessary for a negative weight
pub const ONSHORE_WIND_WEIGHT: f64 = 0.08;
pub const OFFSHORE_WIND_WEIGHT: f64 = 0.08;
pub const DOMESTIC_SOLAR_WEIGHT: f64 = 0.05;
pub const COMMERCIAL_SOLAR_WEIGHT: f64 = 0.05;
pub const UTILITY_SOLAR_WEIGHT: f64 = 0.08;
pub const NUCLEAR_WEIGHT: f64 = 0.03;
pub const COAL_PLANT_WEIGHT: f64 = 0.04;
pub const GAS_COMBINED_CYCLE_WEIGHT: f64 = 0.06;
pub const GAS_PEAKER_WEIGHT: f64 = 0.02;
pub const BIOMASS_WEIGHT: f64 = 0.04;
pub const HYDRO_DAM_WEIGHT: f64 = 0.06;
pub const PUMPED_STORAGE_WEIGHT: f64 = 0.06;
pub const BATTERY_STORAGE_WEIGHT: f64 = 0.07;
pub const TIDAL_GENERATOR_WEIGHT: f64 = 0.05;
pub const WAVE_ENERGY_WEIGHT: f64 = 0.05;
pub const UPGRADE_EFFICIENCY_WEIGHT: f64 = 0.04;
pub const ADJUST_OPERATION_WEIGHT: f64 = 0.04;
pub const CARBON_OFFSET_WEIGHT: f64 = 0.02;
pub const CLOSE_GENERATOR_WEIGHT: f64 = 0.02;
pub const DO_NOTHING_WEIGHT: f64 = 0.1;
pub const DEFICIT_GAS_PEAKER_WEIGHT: f64 = 0.15;
pub const DEFICIT_GAS_COMBINED_WEIGHT: f64 = 0.15;
pub const DEFICIT_BATTERY_WEIGHT: f64 = 0.15;
pub const DEFICIT_PUMPED_STORAGE_WEIGHT: f64 = 0.10;
pub const DEFICIT_BIOMASS_WEIGHT: f64 = 0.10;
pub const DEFICIT_ONSHORE_WIND_WEIGHT: f64 = 0.07;
pub const DEFICIT_OFFSHORE_WIND_WEIGHT: f64 = 0.07;
pub const DEFICIT_UTILITY_SOLAR_WEIGHT: f64 = 0.06;
pub const DEFICIT_HYDRO_DAM_WEIGHT: f64 = 0.06;
pub const DEFICIT_NUCLEAR_WEIGHT: f64 = 0.05;
pub const DEFICIT_SMALL_GENERATOR_WEIGHT: f64 = 0.01;
pub const DEFICIT_DO_NOTHING_WEIGHT: f64 = 0.001;
pub const ONSHORE_WIND_FALLBACK_WEIGHT: u32 = 15;
pub const OFFSHORE_WIND_FALLBACK_WEIGHT: u32 = 10;
pub const UTILITY_SOLAR_FALLBACK_WEIGHT: u32 = 15;
pub const DEFICIT_GAS_PEAKER_FALLBACK_WEIGHT: u32 = 30;
pub const DEFICIT_BATTERY_FALLBACK_WEIGHT: u32 = 30;
pub const DEFICIT_GAS_COMBINED_FALLBACK_WEIGHT: u32 = 20;
pub const DEFICIT_ONSHORE_WIND_FALLBACK_WEIGHT: u32 = 10;
pub const HIGH_COST_WEIGHT: f64 = 0.8;
pub const NORMAL_COST_WEIGHT: f64 = 0.5;

//---------------------------------------------------------------------
// Factor Constants
//---------------------------------------------------------------------
pub const STAGNATION_PENALTY_FACTOR: f64 = 0.2; // Base factor for stagnation penalty
pub const BEST_WEIGHT_FACTOR: f64 = 0.75;
pub const EXPLORATION_DECAY_FACTOR: f64 = 0.01;
pub const STAGNATION_SCALE_FACTOR: f64 = 2.0;
pub const RANDOMIZATION_FACTOR: f64 = 0.1;
pub const SMALL_BOOST_FACTOR: f64 = 0.1;
pub const NOOP_BOOST_FACTOR: f64 = 0.2;
pub const COST_MULTIPLICATION_FACTOR: f64 = 8.0;
pub const RENEWABLE_FALLBACK_WEIGHT_FACTOR: f64 = 0.5;
pub const ADAPTIVE_LEARNING_RATE_FACTOR: f64 = 0.05;
pub const MILD_PENALTY_FACTOR: f64 = 0.5;

//---------------------------------------------------------------------
// Multiplier Constants
//---------------------------------------------------------------------
pub const PENALTY_MULTIPLIER: f64 = 2.0;
pub const BOOST_MULTIPLIER: f64 = 3.0;
pub const DEFICIT_REINFORCEMENT_MULTIPLIER: f64 = 1.5;
pub const MAX_BUDGET_MULTIPLIER: f64 = 100.0;
pub const HIGH_COST_THRESHOLD_MULTIPLIER: f64 = 8.0;
pub const RANDOM_RANGE_MULTIPLIER: f64 = 2.0;
pub const MAX_ACTIONS_MULTIPLIER: f64 = 12.0;

//---------------------------------------------------------------------
// Threshold Constants
//---------------------------------------------------------------------
pub const FORCE_REPLAY_THRESHOLD: u32 = 1000; // After this many iterations without improvement, start forcing replay
pub const PERCENTAGE_THRESHOLD: f64 = 0.9;
pub const HIGH_ITERATION_THRESHOLD: u32 = 800;
pub const MID_ITERATION_THRESHOLD: u32 = 500;
pub const LOW_ITERATION_THRESHOLD: u32 = 100;
pub const WEIGHT_PRECISION_THRESHOLD: f64 = 0.000001;

//---------------------------------------------------------------------
// Divisor Constants
//---------------------------------------------------------------------
pub const FORCE_REPLAY_DIVISOR: f64 = 500.0;
pub const STAGNATION_DIVISOR: f64 = 1000.0;
pub const STAGNATION_ITERATIONS_DIVISOR: f64 = 10.0;
pub const MAX_ACTIONS_DIVISOR: f64 = 12.0;
pub const BILLION_DIVISOR: f64 = 1_000_000_000.0;
pub const EXPLORATION_DIVISOR: f64 = 2.0;

//---------------------------------------------------------------------
// Rate Constants
//---------------------------------------------------------------------
pub const EXPLORATION_DECAY_RATE: f64 = 0.1;
pub const ACTION_COUNT_DECAY_RATE: f64 = 0.8;

//---------------------------------------------------------------------
// Exponent Constants
//---------------------------------------------------------------------
pub const DIVERGENCE_EXPONENT: f64 = 0.3; // How rapidly to increase penalty with worse divergence (lower = more severe for values < 1)
pub const STAGNATION_EXPONENT: f64 = 1.8; // How rapidly to increase penalty with more iterations without improvement

//---------------------------------------------------------------------
// Count Constants
//---------------------------------------------------------------------
pub const MAX_ACTION_COUNT: u32 = 20;
pub const DEBUG_STAR_COUNT: usize = 40;
pub const DEBUG_EQUALS_COUNT: usize = 80;

//---------------------------------------------------------------------
// Interval Constants
//---------------------------------------------------------------------
pub const MEDIUM_LOG_INTERVAL: u32 = 100;
pub const SMALL_LOG_INTERVAL: u32 = 10;

//---------------------------------------------------------------------
// Other Constants
//---------------------------------------------------------------------
pub const ITERATIONS_FOR_RANDOMIZATION: u32 = 1000; // After this many iterations without improvement, apply randomization
pub const PERCENT_CONVERSION: f64 = 100.0;
pub const STAGNATION_SCALE_MIN: f64 = 1.0;
pub const STAGNATION_SCALE_MAX: f64 = 3.0;
pub const IMMEDIATE_WEIGHT_FACTOR_POSITIVE: f64 = 0.7;
pub const IMMEDIATE_WEIGHT_FACTOR_NEGATIVE: f64 = 0.3;
pub const STORAGE_WEIGHT_EARLY: u32 = 10;
pub const STORAGE_WEIGHT_LATE: u32 = 20;
pub const OFFSET_WEIGHT_EARLY: u32 = 5;
pub const OFFSET_WEIGHT_MID: u32 = 15;
pub const OFFSET_WEIGHT_LATE: u32 = 25;
pub const GAS_WEIGHT_EARLY: u32 = 15;
pub const GAS_WEIGHT_MID: u32 = 10;
pub const GAS_WEIGHT_LATE: u32 = 5;
pub const BASE_NET_ZERO_SCORE: f64 = 1.0;
pub const MAX_SCORE_RANGE: f64 = 2.0;
pub const OPERATION_PERCENTAGE_MIN: u8 = 0;
pub const STAGNATION_DIVISOR_INT: u32 = 100;

// Use a static AtomicBool for debug weights output that can be set at runtime
use std::sync::atomic::{AtomicBool, Ordering};

// Use existing lazy_static macro since it's already imported at the crate root
static DEBUG_WEIGHTS: AtomicBool = AtomicBool::new(false);

pub fn set_debug_weights(enabled: bool) {
    DEBUG_WEIGHTS.store(enabled, Ordering::SeqCst);
}

pub fn is_debug_weights_enabled() -> bool {
    DEBUG_WEIGHTS.load(Ordering::SeqCst)
}
