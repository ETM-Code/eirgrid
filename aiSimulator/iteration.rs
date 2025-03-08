use std::error::Error;
use crate::map_handler::Map;
use crate::action_weights::ActionWeights;
use crate::action_weights::GridAction;
use crate::metrics::{YearlyMetrics, SimulationResult};
use crate::action_weights::SimulationMetrics;
use crate::simulation::run_simulation;
use crate::logging;
use crate::logging::OperationCategory;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use crate::run_simulation_with_best_actions;

pub fn run_iteration(
    iteration: usize,
    map: &mut Map,
    weights: &mut ActionWeights,
    replay_best_strategy: bool,
    seed: Option<u64>,
    verbose_logging: bool,
    optimization_mode: Option<&str>,
    enable_energy_sales: bool,
) -> Result<SimulationResult, Box<dyn Error + Send + Sync>> {
    let _timing = logging::start_timing("run_iteration", OperationCategory::Simulation);
    
    // Clone the map to avoid modifying the original
    let mut map_clone = map.clone();
    
    // Run the simulation
    let (simulation_output, recorded_actions, yearly_metrics) = run_simulation(
        &mut map_clone, 
        Some(weights), 
        seed, 
        verbose_logging, 
        optimization_mode, 
        enable_energy_sales
    )?;
    
    // Get the metrics from the weights
    let metrics = if let Some(metrics) = weights.get_simulation_metrics() {
        metrics.clone()
    } else {
        // Default metrics if none available
        SimulationMetrics {
            final_net_emissions: 0.0,
            average_public_opinion: 0.0,
            total_cost: 0.0,
            power_reliability: 0.0,
        }
    };
    
    // Create the simulation result
    let result = SimulationResult {
        metrics,
        output: simulation_output,
        actions: recorded_actions,
        yearly_metrics,
    };
    
    Ok(result)
}