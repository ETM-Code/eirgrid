// This file is maintained for backward compatibility.
// It re-exports all the components that used to be defined here but have now
// been moved to the AI module for better organization.

// Re-export the main components
pub use crate::ai::GridAction;
pub use crate::ai::SimulationMetrics;
pub use crate::ai::ActionResult;
pub use crate::ai::ActionWeights;
pub use crate::ai::score_metrics;
pub use crate::ai::evaluate_action_impact;

// These were internal in the original file but may be used by tests
pub use crate::ai::actions::serializable_action::SerializableAction;
pub use crate::ai::learning::serialization::SerializableWeights;
