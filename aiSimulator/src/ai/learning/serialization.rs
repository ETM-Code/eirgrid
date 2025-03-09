//! Serialization module for AI learning components

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::ai::metrics::simulation_metrics::SimulationMetrics;
use crate::ai::actions::serializable_action::SerializableAction;

/// A serializable version of the weights data structure
/// Used for saving and loading weights to/from JSON files
#[derive(Serialize, Deserialize)]
pub struct SerializableWeights {
    pub weights: HashMap<u32, Vec<(SerializableAction, f64)>>,
    pub learning_rate: f64,
    pub best_metrics: Option<SimulationMetrics>,
    pub best_weights: Option<HashMap<u32, Vec<(SerializableAction, f64)>>>,
    pub best_actions: Option<HashMap<u32, Vec<SerializableAction>>>,
    pub iteration_count: u32,
    pub iterations_without_improvement: u32,
    pub exploration_rate: f64,
    pub deficit_weights: HashMap<u32, Vec<(SerializableAction, f64)>>,
    pub best_deficit_actions: Option<HashMap<u32, Vec<SerializableAction>>>,
    pub optimization_mode: Option<String>,
}
