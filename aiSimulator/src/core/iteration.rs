use std::error::Error;
use crate::utils::map_handler::Map;
use super::action_weights::ActionWeights;
use crate::analysis::metrics::SimulationResult;
use super::action_weights::SimulationMetrics;
use super::simulation::run_simulation;
use crate::utils::logging;
use crate::utils::logging::OperationCategory;

pub fn run_iteration(
    __iteration: usize,
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
    
    // Clear current run actions to prevent accumulation across simulations
    weights.clear_current_run_actions();
    weights.clear_replay_index();
    
    if verbose_logging {
        println!("🧹 VERBOSE: Cleared current run actions and replay index at start of iteration");
    }
    
    // Set force_best_actions if replay_best_strategy is true
    if replay_best_strategy {
        weights.set_force_best_actions(true);
        if verbose_logging {
            println!("🔄 VERBOSE: Forcing use of best actions for this iteration");
        }
    } else {
        weights.set_force_best_actions(false);
    }
    
    // Run the simulation
    let (simulation_output, recorded_actions, yearly_metrics) = run_simulation(
        &mut map_clone, 
        Some(weights), 
        seed, 
        verbose_logging, 
        optimization_mode, 
        enable_energy_sales
    )?;
    
    // Calculate metrics from the last yearly metrics instead of relying on weights
    let metrics = if let Some(final_year_metrics) = yearly_metrics.last() {
        // Only print diagnostic info if debug weights is enabled
        if crate::ai::learning::constants::is_debug_weights_enabled() {
            // Convert from yearly metrics to simulation metrics
            println!("DIAGNOSTIC: Creating SimulationMetrics from final year metrics:");
            println!("  - final_net_emissions: {}", final_year_metrics.net_co2_emissions);
            println!("  - total_cost: {}", final_year_metrics.total_capital_cost);
            println!("  - average_public_opinion: {}", final_year_metrics.average_public_opinion);
            println!("  - power_reliability: {}", 
                if final_year_metrics.power_balance >= 0.0 { 1.0 } else { 0.0 });
        }
        
        SimulationMetrics {
            final_net_emissions: final_year_metrics.net_co2_emissions,
            average_public_opinion: final_year_metrics.average_public_opinion,
            total_cost: final_year_metrics.total_capital_cost,
            power_reliability: if final_year_metrics.power_balance >= 0.0 { 1.0 } else { 0.0 },
        }
    } else {
        // If no yearly metrics, use default values (should never happen)
        println!("WARNING: No yearly metrics available to calculate final metrics");
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