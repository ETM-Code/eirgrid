//! Serialization module for AI learning components

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::ai::metrics::simulation_metrics::SimulationMetrics;
use crate::ai::actions::serializable_action::SerializableAction;
use crate::utils::csv_export::ImprovementRecord;

/// A serializable version of the ImprovementRecord
#[derive(Serialize, Deserialize)]
pub struct SerializableImprovementRecord {
    pub iteration: u32,
    pub score: f64,
    pub net_emissions: f64,
    pub total_cost: f64, 
    pub public_opinion: f64,
    pub power_reliability: f64,
    pub timestamp: String,
}

impl From<&ImprovementRecord> for SerializableImprovementRecord {
    fn from(record: &ImprovementRecord) -> Self {
        Self {
            iteration: record.iteration,
            score: record.score,
            net_emissions: record.net_emissions,
            total_cost: record.total_cost,
            public_opinion: record.public_opinion,
            power_reliability: record.power_reliability,
            timestamp: record.timestamp.clone(),
        }
    }
}

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
    pub improvement_history: Option<Vec<SerializableImprovementRecord>>,
}
