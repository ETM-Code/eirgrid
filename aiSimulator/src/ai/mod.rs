// AI module structure for the EirGrid simulator
// Organized in sub-modules for better maintainability

// Actions module - contains grid action definitions and serialization
pub mod actions {
    pub mod grid_action;
    pub mod serializable_action;
}

// Metrics module - contains simulation metrics and scoring
pub mod metrics {
    pub mod simulation_metrics;
    pub mod scoring;
}

// Learning module - contains weight-based machine learning components
pub mod learning {
    pub mod weights;
    pub mod constants;
    pub mod serialization;
}

// Re-export common types for convenience
pub use actions::grid_action::GridAction;
pub use metrics::simulation_metrics::{SimulationMetrics, ActionResult};
pub use metrics::scoring::{score_metrics, evaluate_action_impact};
pub use learning::weights::ActionWeights;
