// Main module declarations for organized aiSimulator

// Core simulation modules
pub mod core {
    pub mod simulation;
    pub mod multi_simulation;
    pub mod iteration;
    pub mod actions;
    pub mod action_weights_coordinator;
    // Re-export with the old name for backward compatibility
    pub use self::action_weights_coordinator as action_weights;
}

// AI components
pub mod ai;

// Configuration modules
pub mod config {
    pub mod constants;
    pub mod const_funcs;
    pub mod simulation_config;
    pub mod tech_type;
}

// Model definitions
pub mod models {
    pub mod settlement;
    pub mod generator;
    pub mod power_storage;
    pub mod carbon_offset;
}

// Data loaders
pub mod data {
    pub mod settlements_loader;
    pub mod generators_loader;
    pub mod poi;
}

// Analysis and metrics
pub mod analysis {
    pub mod metrics;
    pub mod metrics_calculation;
    pub mod location_analysis;
    pub mod analysis;
    pub mod reporting;
}

// Utility functions
pub mod utils {
    pub mod map_handler;
    pub mod spatial_index;
    pub mod logging;
    pub mod csv_export;
    pub mod traits;
}

// GPU/Metal acceleration
pub mod gpu {
    pub mod metal_location_search;
}

// CLI interface
pub mod cli {
    pub mod cli;
}

// Re-export commonly used modules
pub use crate::core::simulation;
pub use crate::core::multi_simulation;
pub use crate::core::actions;
pub use crate::models::generator;
pub use crate::utils::map_handler::Map;
