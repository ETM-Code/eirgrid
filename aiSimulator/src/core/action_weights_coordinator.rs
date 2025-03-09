// Action Weights Coordinator Module
//
// This file serves as a compatibility layer to ensure all existing code
// that imports from core::action_weights continues to work after refactoring.
// It re-exports all components from the AI module.

// Re-export all components from the ai module
pub use crate::ai::GridAction;
pub use crate::ai::SimulationMetrics;
pub use crate::ai::ActionResult;
pub use crate::ai::ActionWeights;
pub use crate::ai::score_metrics;
pub use crate::ai::evaluate_action_impact;

// Also re-export internal components that might be used directly
pub use crate::ai::learning::serialization::SerializableWeights;
pub use crate::ai::actions::serializable_action::SerializableAction;

// This module serves as a compatibility layer to avoid breaking existing code
// New code should import directly from the ai module instead 