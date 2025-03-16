//! Action Weights Module
//!
//! This module provides weight-based learning for the AI simulation system.
//! It contains the ActionWeights struct and its implementation, divided into
//! logical submodules based on functionality.

// Import internal modules - make them public to be recognized by rust-analyzer
pub mod core;
pub mod sampling;
pub mod learning;
pub mod strategy;
pub mod deficit;
pub mod serialization;
pub mod diagnostics;

// Remove the re-exports that are causing issues
// pub use self::core::*;
// pub use self::sampling::*;
// pub use self::learning::*;
// pub use self::strategy::*;
// pub use self::deficit::*;
// pub use self::serialization::*;
// pub use self::diagnostics::*;

use std::collections::HashMap;
use std::sync::Mutex;

// External crate imports
use rand::rngs::StdRng;
use lazy_static::lazy_static;

// Internal module imports
use crate::ai::actions::grid_action::GridAction;
use crate::ai::metrics::simulation_metrics::SimulationMetrics;
use crate::utils::csv_export::ImprovementRecord;

/// Mutex for file operations to prevent race conditions when
/// multiple threads try to read/write weight files
lazy_static! {
    static ref FILE_MUTEX: Mutex<()> = Mutex::new(());
}

/// The ActionWeights struct is responsible for managing the weights used
/// to determine which actions to take during grid simulation.
///
/// It learns from simulation results and adapts its weights over time
/// to optimize for specified objectives like emissions reduction,
/// public opinion, power reliability, and cost.
#[derive(Debug, Clone)]
pub struct ActionWeights {
    /// Maps years to action weights (action -> weight)
    pub weights: HashMap<u32, HashMap<GridAction, f64>>,
    
    /// Maps years to weights for different action counts
    pub action_count_weights: HashMap<u32, HashMap<u32, f64>>, 
    
    /// Learning rate for weight adjustments
    pub learning_rate: f64,
    
    /// Best metrics achieved so far
    pub best_metrics: Option<SimulationMetrics>,
    
    /// Weights that produced the best results
    pub best_weights: Option<HashMap<u32, HashMap<GridAction, f64>>>,
    
    /// Actions that produced the best results
    pub best_actions: Option<HashMap<u32, Vec<GridAction>>>, 
    
    /// Number of learning iterations so far
    pub iteration_count: u32,
    
    /// Number of iterations without improvement
    pub iterations_without_improvement: u32,
    
    /// Exploration rate for epsilon-greedy algorithm
    pub exploration_rate: f64,
    
    /// Actions taken in the current simulation run
    pub current_run_actions: HashMap<u32, Vec<GridAction>>,
    
    /// Whether to force replay of best actions
    pub force_best_actions: bool,
    
    /// Weights for handling power deficits
    pub deficit_weights: HashMap<u32, HashMap<GridAction, f64>>,
    
    /// Deficit actions taken in the current run
    pub current_deficit_actions: HashMap<u32, Vec<GridAction>>,
    
    /// Best deficit actions found so far
    pub best_deficit_actions: Option<HashMap<u32, Vec<GridAction>>>,
    
    /// Optional deterministic RNG for reproducible runs
    pub deterministic_rng: Option<StdRng>,
    
    /// Flag to force replay of best actions with 100% probability
    pub guaranteed_best_actions: bool,
    
    /// Optimization mode (e.g., "emissions", "cost", "balanced")
    pub optimization_mode: Option<String>,
    
    /// Tracks the current index when replaying best actions for each year
    pub replay_index: HashMap<u32, usize>,
    
    /// Improvement history tracking - records each time the best strategy is improved
    pub improvement_history: Vec<ImprovementRecord>,
}
