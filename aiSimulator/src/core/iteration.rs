use std::error::Error;
use crate::utils::map_handler::Map;
use crate::ai::metrics::simulation_metrics::SimulationMetrics;
use crate::analysis::metrics::YearlyMetrics;
use crate::models::generator::GeneratorType;
use crate::utils::logging;
use crate::utils::logging::OperationCategory;
use crate::core::action_weights::ActionWeights;
use crate::analysis::metrics::SimulationResult;
use crate::core::simulation::run_simulation;
use crate::config::constants::{OPERATION_PERCENTAGE_SCALE, DEFAULT_POWER};

pub fn run_iteration(
    __iteration: usize,
    map: &mut Map,
    weights: &mut ActionWeights,
    replay_best_strategy: bool,
    seed: Option<u64>,
    verbose_logging: bool,
    optimization_mode: Option<&str>,
    enable_energy_sales: bool,
    enable_construction_delays: bool,
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
        enable_energy_sales,
        enable_construction_delays
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
            
            // Calculate power reliability using Map's implementation for the final year
            let power_reliability = map.calc_power_reliability(final_year_metrics.year);
            println!("  - power_reliability: {:.2}%", power_reliability * OPERATION_PERCENTAGE_SCALE);
            
            // Also calculate the best power reliability across all years
            let mut worst_reliability = power_reliability;
            for year_metrics in yearly_metrics.iter().rev().skip(1) {
                let year_reliability = map.calc_power_reliability(year_metrics.year);
                if year_reliability < worst_reliability {
                    worst_reliability = year_reliability;
                }
            }
            
            // Use the best reliability value among all years
            let final_reliability = if worst_reliability < power_reliability {
                println!("  - found better power_reliability in earlier year: {:.2}%", 
                          worst_reliability * OPERATION_PERCENTAGE_SCALE);
                worst_reliability
            } else {
                power_reliability
            };
            
            SimulationMetrics {
                final_net_emissions: final_year_metrics.net_co2_emissions,
                average_public_opinion: final_year_metrics.average_public_opinion,
                total_cost: final_year_metrics.total_capital_cost,
                power_reliability: final_reliability,
                worst_power_reliability: worst_reliability,
            }
        } else {
            // Calculate power reliability using Map's implementation for the final year
            let power_reliability = map.calc_power_reliability(final_year_metrics.year);
            
            // Also calculate the best power reliability across all years
            let mut best_reliability = power_reliability;
            for year_metrics in yearly_metrics.iter().rev().skip(1) {
                let year_reliability = map.calc_power_reliability(year_metrics.year);
                if year_reliability > best_reliability {
                    best_reliability = year_reliability;
                }
            }
            
            // Also calculate the worst power reliability across all years
            let mut worst_reliability = power_reliability;
            for year_metrics in yearly_metrics.iter().rev().skip(1) {
                let year_reliability = map.calc_power_reliability(year_metrics.year);
                if year_reliability < worst_reliability {
                    worst_reliability = year_reliability;
                }
            }
            
            SimulationMetrics {
                final_net_emissions: final_year_metrics.net_co2_emissions,
                average_public_opinion: final_year_metrics.average_public_opinion,
                total_cost: final_year_metrics.total_capital_cost,
                power_reliability: best_reliability,
                worst_power_reliability: worst_reliability,
            }
        }
    } else {
        // If no yearly metrics, use default values (should never happen)
        println!("WARNING: No yearly metrics available to calculate final metrics");
        SimulationMetrics {
            final_net_emissions: DEFAULT_POWER,
            average_public_opinion: DEFAULT_POWER,
            total_cost: DEFAULT_POWER,
            power_reliability: DEFAULT_POWER,
            worst_power_reliability: DEFAULT_POWER,
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