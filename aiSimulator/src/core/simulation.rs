use std::error::Error;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use crate::utils::map_handler::Map;
use super::action_weights::ActionWeights;
use super::action_weights::{GridAction, ActionResult, evaluate_action_impact};
use crate::analysis::metrics::YearlyMetrics;
use crate::utils::logging::{self, OperationCategory, PowerCalcType};
use crate::utils::logging::WeightsUpdateType;
use crate::config::const_funcs;
use crate::analysis::metrics_calculation::{calculate_yearly_metrics, calculate_average_opinion};
use crate::analysis::reporting::{print_yearly_summary, print_generator_details};
use crate::config::constants::{MAX_ACCEPTABLE_COST, BASE_YEAR, END_YEAR, DEFAULT_COST_MULTIPLIER};
use super::actions::apply_action;
use crate::models::generator::GeneratorType;
use chrono::Local;
use std::fs::File;
use std::io::Write;



pub fn run_simulation(
    map: &mut Map,
    action_weights: Option<&mut ActionWeights>,
    seed: Option<u64>,
    verbose_logging: bool,
    optimization_mode: Option<&str>,
    enable_energy_sales: bool,
    enable_construction_delays: bool,
    iteration: usize,
) -> Result<(String, Vec<(u32, GridAction)>, Vec<YearlyMetrics>), Box<dyn Error + Send + Sync>> {
    let _timing = logging::start_timing("run_simulation", OperationCategory::Simulation);
     
    // Set construction delays flag
    map.set_enable_construction_delays(enable_construction_delays);
     
    let mut output = String::new();
    let mut recorded_actions = Vec::new();
    let mut yearly_metrics_collection = Vec::new();
     
    let total_upgrade_costs = 0.0;
    let total_closure_costs = 0.0;
     
    let mut local_weights = match action_weights.as_deref() {
        Some(weights) => weights.clone(),
        None => ActionWeights::new(),
    };
     
    // Set deterministic RNG if seed is provided
    if let Some(seed_value) = seed {
        let rng = StdRng::seed_from_u64(seed_value);
        local_weights.set_rng(rng);
         
        if verbose_logging {
            println!("🔢 VERBOSE: Using deterministic seed: {}", seed_value);
        }
    }
     
    let mut final_year_metrics: Option<YearlyMetrics> = None;
     
    // Create a state log file if verbose logging is enabled
    let mut state_log_file = if verbose_logging {
        // Create the simulation_states directory if it doesn't exist
        let dir_path = "simulation_states";
        if !std::path::Path::new(dir_path).exists() {
            std::fs::create_dir_all(dir_path)?;
        }
         
        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let log_path = format!("simulation_states/simulation_state_{}.log", timestamp);
        println!("📝 Creating verbose state log at: {}", log_path);
        Some(File::create(log_path)?)
    } else {
        None
    };
     
    // At the beginning of the simulation, diagnose best actions if we have them
    if let Some(weights) = &action_weights {
        // Only print simulation start messages every 100 iterations
        if iteration % 100 == 0 {
            println!("\n=== STARTING SIMULATION ===");
            if let Some(seed_value) = seed {
                println!("Seed: {}", seed_value);
            }
            weights.diagnose_best_actions();
        }
    }
     
    for year in BASE_YEAR..=END_YEAR {
        let _year_timing = logging::start_timing(&format!("simulate_year_{}", year), OperationCategory::Simulation);
         
        // Update the current year in the map
        map.current_year = year;
        
        // Update construction status for all generators and offsets
        map.update_construction_status();
         
        if action_weights.is_none() {
            println!("\nStarting year {}", year);
             
            if year > BASE_YEAR {
                local_weights.print_top_actions(year - 1, 5);
            }
        }
         
        // Update population for each settlement based on the current year
        if year > BASE_YEAR {
            let _timing = logging::start_timing("update_population", OperationCategory::Simulation);
            for settlement in map.get_settlements_mut() {
                let current_pop = settlement.get_population();
                // Apply Irish population growth rate (roughly 1% per year)
                let new_pop = (current_pop as f64 * 1.01).round() as u32;
                settlement.update_population(new_pop);
                 
                // Also update power usage based on new population and per capita usage
                let per_capita_usage = const_funcs::calc_power_usage_per_capita(year);
                let new_usage = (new_pop as f64) * per_capita_usage;
                settlement.update_power_usage(new_usage);
            }
        }
         
        let current_state = {
            let _timing = logging::start_timing("calculate_current_state",
                OperationCategory::PowerCalculation { subcategory: PowerCalcType::Balance });
            let net_emissions = map.calc_net_co2_emissions(year);
            let public_opinion = calculate_average_opinion(map, year);
            let power_balance = map.calc_total_power_generation(year, None) - map.calc_total_power_usage(year);
            let total_cost = map.calc_total_capital_cost(year);
            ActionResult {
                net_emissions,
                public_opinion,
                power_balance,
                total_cost,
            }
        };

        if current_state.power_balance < 0.0 {
            let _timing = logging::start_timing("handle_power_deficit",
                OperationCategory::PowerCalculation { subcategory: PowerCalcType::Balance });
            handle_power_deficit(map, -current_state.power_balance, year, &mut local_weights, optimization_mode)?;
        }

        let mut rng = rand::thread_rng();
        let num_additional_actions = if action_weights.is_some() {
            // Check if we're forcing replay of best actions
            if local_weights.force_best_actions {
                // When replaying best actions, use the exact number of actions from the best simulation
                if let Some(best_actions) = local_weights.best_actions.as_ref().and_then(|ba| ba.get(&year)) {
                    let count = best_actions.len();
                    // Only print debug info if debug weights is enabled
                    if verbose_logging && crate::ai::learning::constants::is_debug_weights_enabled() {
                        println!("🔄 REPLAY: Using exact count of {} best actions for year {}", count, year);
                    }
                    count
                } else {
                    // If no best actions for this year, use 0
                    // Only print debug info if debug weights is enabled
                    if verbose_logging && crate::ai::learning::constants::is_debug_weights_enabled() {
                        println!("🔄 REPLAY: No best actions for year {}, using 0", year);
                    }
                    0
                }
            } else {
                // Print the action count weights for this year only if debug weights is enabled
                if crate::ai::learning::constants::is_debug_weights_enabled() {
                    local_weights.print_action_count_weights(year);
                    
                    // Get deficit actions count for detailed debugging output
                    let deficit_actions_count = local_weights.current_deficit_actions.get(&year)
                        .map_or(0, |actions| actions.len());
                        
                    // Sample the action count
                    let count = local_weights.sample_additional_actions(year) as usize;
                    
                    // Print detailed diagnostic information
                    println!("Year {}: Planning {} additional actions (plus {} deficit actions = {} total)",
                            year, count, deficit_actions_count, count + deficit_actions_count);
                    
                    count
                } else {
                    // Just sample the action count without extra output
                    local_weights.sample_additional_actions(year) as usize
                }
            }
        } else {
            rng.gen_range(0..=20)
        };

        for _ in 0..num_additional_actions {
            let _timing = logging::start_timing("apply_additional_action", OperationCategory::Simulation);
            let action = local_weights.sample_action(year);
            apply_action(map, &action, year)?;
            recorded_actions.push((year, action.clone()));
             
            // Record action in the weights with debug output
            // println!("📝 DEBUG: Recording action for year {}: {:?}", year, action);
            local_weights.record_action(year, action);
        }

        // Calculate yearly metrics
        // Get the previous year's metrics if available
        let previous_metrics = if year > BASE_YEAR {
            yearly_metrics_collection.last()
        } else {
            None
        };
        
        let yearly_metrics = calculate_yearly_metrics(
            map, 
            year, 
            total_upgrade_costs, 
            total_closure_costs, 
            enable_energy_sales,
            previous_metrics
        );
         
        // Collect yearly metrics for CSV export
        yearly_metrics_collection.push(yearly_metrics.clone());
        
        // Log yearly metrics to state log file if verbose logging is enabled
        if let Some(ref mut file) = state_log_file {
            if let Err(e) = writeln!(file, "Year {} metrics:", year) {
                eprintln!("Error writing to state log file: {}", e);
            }
            if let Err(e) = writeln!(file, "  Population: {}", yearly_metrics.total_population) {
                eprintln!("Error writing to state log file: {}", e);
            }
            if let Err(e) = writeln!(file, "  Power Usage: {:.2} MW", yearly_metrics.total_power_usage) {
                eprintln!("Error writing to state log file: {}", e);
            }
            if let Err(e) = writeln!(file, "  Power Generation: {:.2} MW", yearly_metrics.total_power_generation) {
                eprintln!("Error writing to state log file: {}", e);
            }
            if let Err(e) = writeln!(file, "  Power Balance: {:.2} MW", yearly_metrics.power_balance) {
                eprintln!("Error writing to state log file: {}", e);
            }
            if let Err(e) = writeln!(file, "  Yearly Carbon Credit Revenue: €{:.2}", yearly_metrics.yearly_carbon_credit_revenue) {
                eprintln!("Error writing to state log file: {}", e);
            }
            if let Err(e) = writeln!(file, "  Total Carbon Credit Revenue: €{:.2}", yearly_metrics.total_carbon_credit_revenue) {
                eprintln!("Error writing to state log file: {}", e);
            }
            if let Err(e) = writeln!(file, "  Yearly Energy Sales Revenue: €{:.2}", yearly_metrics.yearly_energy_sales_revenue) {
                eprintln!("Error writing to state log file: {}", e);
            }
            if let Err(e) = writeln!(file, "  Total Energy Sales Revenue: €{:.2}", yearly_metrics.total_energy_sales_revenue) {
                eprintln!("Error writing to state log file: {}", e);
            }
            if let Err(e) = writeln!(file, "  Yearly Total Cost: €{:.2}", yearly_metrics.yearly_total_cost) {
                eprintln!("Error writing to state log file: {}", e);
            }
            if let Err(e) = writeln!(file, "  Accumulated Total Cost: €{:.2}", yearly_metrics.total_cost) {
                eprintln!("Error writing to state log file: {}", e);
            }
            if let Err(e) = writeln!(file, "  CO2 Emissions: {:.2} tonnes", yearly_metrics.total_co2_emissions) {
                eprintln!("Error writing to state log file: {}", e);
            }
            if let Err(e) = writeln!(file, "  Carbon Offset: {:.2} tonnes", yearly_metrics.total_carbon_offset) {
                eprintln!("Error writing to state log file: {}", e);
            }
            if let Err(e) = writeln!(file, "  Net Emissions: {:.2} tonnes", yearly_metrics.net_co2_emissions) {
                eprintln!("Error writing to state log file: {}", e);
            }
            if let Err(e) = writeln!(file, "  Public Opinion: {:.3}", yearly_metrics.average_public_opinion) {
                eprintln!("Error writing to state log file: {}", e);
            }
            if let Err(e) = writeln!(file, "----------------------------------------\n") {
                eprintln!("Error writing to state log file: {}", e);
            }
        }
         
        if action_weights.is_none() {
            print_yearly_summary(&yearly_metrics);
        }
         
        // For the last year, save metrics for final output
        if year == END_YEAR {
            final_year_metrics = Some(yearly_metrics);
        }
    }

    if let Some(metrics) = final_year_metrics {
        if action_weights.is_none() {
            print_generator_details(&metrics);
        }
         
        // Format the output string
        let population = metrics.total_population;
        let power_usage = metrics.total_power_usage;
        let power_generation = metrics.total_power_generation;
        let power_balance = metrics.power_balance;
        let public_opinion = metrics.average_public_opinion;
        let yearly_capital_cost = metrics.yearly_capital_cost;
        let total_capital_cost = metrics.total_capital_cost;
        let net_emissions = metrics.net_co2_emissions;
         
        output = format!(
            "Population: {}\nPower usage: {:.2} MW\nPower generation: {:.2} MW\nPower balance: {:.2} MW\nPublic opinion: {:.2}%\nYearly Capital Cost: €{:.2}\nTotal Capital Cost: €{:.2}\nNet emissions: {:.2} tonnes",
            population, power_usage, power_generation, power_balance,
            public_opinion * 100.0, yearly_capital_cost, total_capital_cost, net_emissions
        );
    }
     
    // Update weights with the final state if we have weights
    if let Some(weights) = action_weights {
        let _timing = logging::start_timing("update_weights",
            OperationCategory::WeightsUpdate { subcategory: WeightsUpdateType::Other });
         
        // Debug check of actions before transferring to weights
        local_weights.debug_print_recorded_actions();
         
        // Transfer local weights to shared weights
        *weights = local_weights;
    }
     
    Ok((output, recorded_actions, yearly_metrics_collection))
}

pub fn handle_power_deficit(
    map: &mut Map,
    deficit: f64,
    year: u32,
    action_weights: &mut ActionWeights,
    __optimization_mode: Option<&str>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let _timing = logging::start_timing(
        "handle_power_deficit",
        OperationCategory::PowerCalculation { subcategory: PowerCalcType::Balance },
    );

    // First, use any available stored power to reduce the deficit.
    // (Uses the existing storage-based method.)
    let mut remaining_deficit = map.handle_power_deficit(deficit, None);

    // We'll add generation until the deficit is met.
    // If several attempts have produced no improvement (reliability issues), force a storage action.
    let mut attempts: u32 = 0;
    const MAX_TRIES_BEFORE_STORAGE_OVERRIDE: u32 = 5; // after 5 tries, switch to storage

    // Calculate the initial state to use for evaluating deficit handling actions
    let initial_state = {
        let _timing = logging::start_timing(
            "calculate_initial_deficit_state",
            OperationCategory::PowerCalculation { subcategory: PowerCalcType::Balance },
        );
        let net_emissions = map.calc_net_co2_emissions(year);
        let public_opinion = calculate_average_opinion(map, year);
        let power_balance = map.calc_total_power_generation(year, None) - map.calc_total_power_usage(year);
        let total_cost = map.calc_total_capital_cost(year);
        ActionResult {
            net_emissions,
            public_opinion,
            power_balance,
            total_cost,
        }
    };

    while remaining_deficit > 0.0 {
        attempts += 1;

        // Sample an AddGenerator action using the weighted method, but from deficit-specific weights
        let action = if attempts < MAX_TRIES_BEFORE_STORAGE_OVERRIDE {
            // Use deficit-specific sampling
            let _timing = logging::start_timing(
                "sample_deficit_action",
                OperationCategory::WeightsUpdate { subcategory: WeightsUpdateType::ActionUpdate },
            );
            action_weights.sample_deficit_action(year)
        } else {
            // After several tries, force a storage action
            let _timing = logging::start_timing(
                "sample_deficit_action_storage_override",
                OperationCategory::WeightsUpdate { subcategory: WeightsUpdateType::ActionUpdate },
            );
            // Always use battery storage as a reliable final option
            GridAction::AddGenerator(GeneratorType::BatteryStorage, DEFAULT_COST_MULTIPLIER)
        };

        // Compute the current simulation state before applying the action.
        let current_state = {
            let _timing = logging::start_timing(
                "calculate_current_state",
                OperationCategory::PowerCalculation { subcategory: PowerCalcType::Balance },
            );
            let net_emissions = map.calc_net_co2_emissions(year);
            let public_opinion = calculate_average_opinion(map, year);
            let power_balance = map.calc_total_power_generation(year, None) - map.calc_total_power_usage(year);
            let total_cost = map.calc_total_capital_cost(year);
            ActionResult {
                net_emissions,
                public_opinion,
                power_balance,
                total_cost,
            }
        };

        // Only add a generator if the sampled action is an AddGenerator.
        if let GridAction::AddGenerator(_, _) = action {
            let _timing = logging::start_timing(
                "apply_generator_action",
                OperationCategory::Simulation,
            );
            apply_action(map, &action, year)?;
             
            // Record the action in both deficit-specific weights system and regular action record
            action_weights.record_deficit_action(year, action.clone());
             
            // Also record in the regular action system to ensure it's included in replays and CSV exports
            action_weights.record_action(year, action.clone());

            // Recalculate state after the action.
            let new_state = {
                let _timing = logging::start_timing(
                    "calculate_new_state",
                    OperationCategory::PowerCalculation { subcategory: PowerCalcType::Balance },
                );
                let net_emissions = map.calc_net_co2_emissions(year);
                let public_opinion = calculate_average_opinion(map, year);
                let power_balance = map.calc_total_power_generation(year, None) - map.calc_total_power_usage(year);
                let total_cost = map.calc_total_capital_cost(year);
                ActionResult {
                    net_emissions,
                    public_opinion,
                    power_balance,
                    total_cost,
                }
            };

            // Calculate improvement based on all metrics using evaluate_action_impact
            // This uses the same logic as regular action assessment
            let overall_improvement = evaluate_action_impact(&current_state, &new_state, None);
             
            // Calculate specific improvements for different metrics
             
            // Emissions improvement: Priority when emissions are positive
            let emissions_improvement = if new_state.net_emissions < current_state.net_emissions {
                (current_state.net_emissions - new_state.net_emissions) / current_state.net_emissions.abs().max(1.0)
            } else {
                0.0
            };
             
            // Cost improvement: Important if we have achieved close to net zero emissions
            let cost_improvement = if new_state.net_emissions < 1000.0 {
                let cost_change = new_state.total_cost - current_state.total_cost;
                -cost_change / current_state.total_cost.abs().max(1.0)
            } else {
                0.0
            };
             
            // Public opinion improvement: Important if cost is reasonable
            let opinion_improvement = if new_state.total_cost < MAX_ACCEPTABLE_COST * 8.0 {
                (new_state.public_opinion - current_state.public_opinion) / (1.0 - current_state.public_opinion).max(0.1)
            } else {
                0.0
            };
             
            // Power balance improvement: While we need to handle the deficit, this is a means to an end, not the main goal
            let __power_improvement = if new_state.power_balance > current_state.power_balance {
                (new_state.power_balance - current_state.power_balance) / deficit.max(1.0)
            } else {
                0.0
            };
             
            // Combined improvement, with focus on overall result with a balance of metrics
            // Primary focus is on the overall evaluation (70%)
            // Secondary focus is on specific metrics that align with our goals (emissions, cost, opinion)
            let combined_improvement = overall_improvement * 0.7 +
                emissions_improvement * 0.15 +
                cost_improvement * 0.1 +
                opinion_improvement * 0.05;

            {
                let _timing = logging::start_timing(
                    "update_deficit_weights",
                    OperationCategory::WeightsUpdate { subcategory: WeightsUpdateType::ActionUpdate },
                );
                 
                // Update deficit-specific weights
                action_weights.update_deficit_weights(&action, year, combined_improvement);
                 
                // Also update standard weights but with less impact
                action_weights.update_weights(&action, year, overall_improvement * 0.5);
            }

            // Update the deficit based on the new state.
            remaining_deficit = -new_state.power_balance.min(0.0);
        }
    }
     
    // Calculate overall deficit handling success by comparing final state to initial state
    let final_state = {
        let net_emissions = map.calc_net_co2_emissions(year);
        let public_opinion = calculate_average_opinion(map, year);
        let power_balance = map.calc_total_power_generation(year, None) - map.calc_total_power_usage(year);
        let total_cost = map.calc_total_capital_cost(year);
        ActionResult {
            net_emissions,
            public_opinion,
            power_balance,
            total_cost,
        }
    };
     
    // Evaluate overall success using the standard action impact evaluation
    let overall_success = evaluate_action_impact(&initial_state, &final_state, None);
     
    // If we successfully handled the deficit and our metrics improved, provide a bonus
    if final_state.power_balance >= 0.0 && overall_success > 0.0 &&
        action_weights.has_deficit_actions_for_year(year) {
        // Add a success factor based on overall improvement, scaled by its magnitude
        let success_factor = 0.1 * overall_success;
         
        // Get deficit actions and apply rewards
        if let Some(deficit_actions) = action_weights.get_deficit_actions_for_year(year) {
            for action in deficit_actions {
                action_weights.update_deficit_weights(&action, year, success_factor);
            }
        }
    }
     
    Ok(())
}

// Function to run a simulation that replays the best actions from a previous run
pub fn run_simulation_with_best_actions(
    map: &mut Map,
    weights: &mut ActionWeights,
    seed: Option<u64>,
    __verbose_logging: bool,
    optimization_mode: Option<&str>,
    enable_energy_sales: bool,
    enable_construction_delays: bool,
) -> Result<(String, Vec<(u32, GridAction)>, Vec<YearlyMetrics>), Box<dyn Error + Send + Sync>> {
    let _timing = logging::start_timing("run_simulation_with_best_actions", OperationCategory::Simulation);

    // Set construction delays flag
    map.set_enable_construction_delays(enable_construction_delays);

    let mut output = String::new();
    let mut recorded_actions = Vec::new();
    let mut yearly_metrics_collection = Vec::new();
     
    let total_upgrade_costs = 0.0;
    let total_closure_costs = 0.0;
     
    let mut final_year_metrics: Option<YearlyMetrics> = None;
     
    // Set the flag to guarantee best action replays (100% probability)
    weights.set_guaranteed_best_actions(true);
     
    // Set deterministic RNG if seed is provided
    if let Some(seed_value) = seed {
        let rng = StdRng::seed_from_u64(seed_value);
        weights.set_rng(rng);
    }
     
    // println!("\nReplaying best strategy from previous runs with 100% probability");
     
    // Hard-code the year range to 2025..=2050 rather than using constants
    for year in 2025..=2050 {
        let _year_timing = crate::utils::logging::start_timing(&format!("simulate_year_{}", year), OperationCategory::Simulation);
         
        // Update the current year in the map
        map.current_year = year;
        
        // Update construction status for all generators and offsets
        map.update_construction_status();
         
        // Update population for each settlement based on the current year
        if year > 2025 {
            let _timing = crate::utils::logging::start_timing("update_population", OperationCategory::Simulation);
            for settlement in map.get_settlements_mut() {
                let current_pop = settlement.get_population();
                // Apply Irish population growth rate (roughly 1% per year)
                let new_pop = (current_pop as f64 * 1.01).round() as u32;
                settlement.update_population(new_pop);
                 
                // Also update power usage based on new population and per capita usage
                let per_capita_usage = crate::config::const_funcs::calc_power_usage_per_capita(year);
                let new_usage = (new_pop as f64) * per_capita_usage;
                settlement.update_power_usage(new_usage);
            }
        }
         
        // Calculate current state before actions
        let current_state = {
            let net_emissions = map.calc_net_co2_emissions(year);
            let public_opinion = crate::analysis::metrics_calculation::calculate_average_opinion(map, year);
            let power_balance = map.calc_total_power_generation(year, None) - map.calc_total_power_usage(year);
            let total_cost = map.calc_total_capital_cost(year);
            ActionResult {
                net_emissions,
                public_opinion,
                power_balance,
                total_cost,
            }
        };

        // Handle any power deficit before applying best actions, but don't record actions separately
        // (they will be included in the best_deficit_actions)
        if current_state.power_balance < 0.0 {
            let deficit_before_actions = -current_state.power_balance;
            println!("Year {}: Handling initial power deficit of {} MW", year, deficit_before_actions);
        }
         
        // Debug: Print information about best actions availability
        println!("DEBUG: Has best actions overall: {}", weights.has_best_actions());
         
        // Get and apply the best actions for this year
        if let Some(best_actions) = weights.get_best_actions_for_year(year) {
            println!("Year {}: Applying {} best actions", year, best_actions.len());
             
            // Apply each of the best actions
            for action in best_actions {
                crate::core::actions::apply_action(map, action, year)?;
                recorded_actions.push((year, action.clone()));
            }
        } else {
            println!("Year {}: No best actions found", year);
        }

        // Get and apply the best deficit actions for this year
        if let Some(best_deficit_actions) = weights.get_best_deficit_actions_for_year(year) {
            println!("Year {}: Applying {} best deficit actions", year, best_deficit_actions.len());
             
            // Apply each of the best deficit actions
            for action in best_deficit_actions {
                crate::core::actions::apply_action(map, &action, year)?;
                recorded_actions.push((year, action.clone()));
            }
        }

        // Verify power balance after actions and handle any remaining deficit
        let post_action_state = {
            let net_emissions = map.calc_net_co2_emissions(year);
            let public_opinion = crate::analysis::metrics_calculation::calculate_average_opinion(map, year);
            let power_balance = map.calc_total_power_generation(year, None) - map.calc_total_power_usage(year);
            let total_cost = map.calc_total_capital_cost(year);
            ActionResult {
                net_emissions,
                public_opinion,
                power_balance,
                total_cost,
            }
        };

        if post_action_state.power_balance < 0.0 {
            let remaining_deficit = -post_action_state.power_balance;
            println!("Year {}: Handling remaining power deficit of {} MW", year, remaining_deficit);
            handle_power_deficit(map, remaining_deficit, year, weights, optimization_mode)?;
             
            // Add any new deficit actions to the recorded actions list
            if let Some(current_deficit_actions) = weights.get_deficit_actions_for_year(year) {
                for action in current_deficit_actions {
                    recorded_actions.push((year, action.clone()));
                }
            }
        }

        // Calculate and save yearly metrics
        // Get the previous year's metrics if available
        let previous_metrics = if year > 2025 {
            yearly_metrics_collection.last()
        } else {
            None
        };
        
        let metrics = crate::analysis::metrics_calculation::calculate_yearly_metrics(
            map, 
            year, 
            total_upgrade_costs, 
            total_closure_costs, 
            enable_energy_sales,
            previous_metrics
        );
        yearly_metrics_collection.push(metrics.clone());
         
        crate::analysis::reporting::print_yearly_summary(&metrics);
         
        // Save the final year metrics
        if year == 2050 {
            final_year_metrics = Some(metrics);
        }
    }

    if let Some(metrics) = final_year_metrics {
        crate::analysis::reporting::print_generator_details(&metrics);
         
         
        // Format the output string
        let population = metrics.total_population;
        let power_usage = metrics.total_power_usage;
        let power_generation = metrics.total_power_generation;
        let power_balance = metrics.power_balance;
        let public_opinion = metrics.average_public_opinion;
        let yearly_capital_cost = metrics.yearly_capital_cost;
        let total_capital_cost = metrics.total_capital_cost;
        let net_emissions = metrics.net_co2_emissions;
         
        output = format!(
            "Population: {}\nPower usage: {:.2} MW\nPower generation: {:.2} MW\nPower balance: {:.2} MW\nPublic opinion: {:.2}%\nYearly Capital Cost: €{:.2}\nTotal Capital Cost: €{:.2}\nNet emissions: {:.2} tonnes",
            population, power_usage, power_generation, power_balance,
            public_opinion * 100.0, yearly_capital_cost, total_capital_cost, net_emissions
        );
    }
     
    Ok((output, recorded_actions, yearly_metrics_collection))
}