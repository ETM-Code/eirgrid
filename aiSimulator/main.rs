#[macro_use]
extern crate lazy_static;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::path::Path;
use std::time::{Duration, Instant};
use std::fs::File;
use std::io::Write;
use std::str::FromStr;
use std::error::Error;

use clap::Parser;
use rayon::prelude::*;
use parking_lot::{self, RwLock};
use serde::Serialize;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use chrono::Local;

// Module declarations
mod poi;
mod generator;
mod settlement;
mod map_handler;
mod constants;
mod const_funcs;
mod carbon_offset;
mod simulation_config;
mod action_weights;
mod power_storage;
mod settlements_loader;
mod generators_loader;
mod spatial_index;
mod metal_location_search;
mod logging;
mod csv_export;

// Import specific items from modules
use crate::map_handler::Map;
use crate::generator::{Generator, GeneratorType};
use crate::action_weights::{ActionWeights, GridAction, SimulationMetrics, evaluate_action_impact, ActionResult, score_metrics};
use crate::constants::*;
use crate::logging::{OperationCategory, FileIOType, PowerCalcType, WeightsUpdateType};
use crate::csv_export::CsvExporter;
use crate::carbon_offset::{CarbonOffset, CarbonOffsetType};

// Public exports as needed
pub use poi::{POI, Coordinate};
pub use settlement::Settlement;
pub use simulation_config::SimulationConfig;

const SIMULATION_START_YEAR: u32 = 2025;
const SIMULATION_END_YEAR: u32 = 2050;
// Percentage of total runs that should be full runs (0-100)
const FULL_RUN_PERCENTAGE: usize = 10;
// Whether to replay the best strategy during full runs
const REPLAY_BEST_STRATEGY_IN_FULL_RUNS: bool = true;

#[derive(Debug, Clone, Serialize)]
struct YearlyMetrics {
    year: u32,
    total_population: u32,
    total_power_usage: f64,
    total_power_generation: f64,
    power_balance: f64,
    average_public_opinion: f64,
    yearly_capital_cost: f64,        // Capital cost for the current year only
    total_capital_cost: f64,         // Accumulated capital cost up to this year
    inflation_factor: f64,
    total_co2_emissions: f64,
    total_carbon_offset: f64,
    net_co2_emissions: f64,
    yearly_carbon_credit_revenue: f64, // Revenue for the current year only
    total_carbon_credit_revenue: f64,  // Accumulated revenue up to this year
    yearly_energy_sales_revenue: f64,  // Revenue from energy sales for current year
    total_energy_sales_revenue: f64,   // Accumulated energy sales revenue up to this year
    generator_efficiencies: Vec<(String, f64)>,
    generator_operations: Vec<(String, f64)>,
    active_generators: usize,
    yearly_upgrade_costs: f64,        // Upgrade costs for the current year
    yearly_closure_costs: f64,        // Closure costs for the current year
    yearly_total_cost: f64,           // Total cost for this year only
    total_cost: f64,                  // Accumulated total cost up to this year
}

#[derive(Clone)]
struct SimulationResult {
    metrics: SimulationMetrics,
    output: String,
    actions: Vec<(u32, GridAction)>,
    yearly_metrics: Vec<YearlyMetrics>, // Add yearly metrics to the struct
}

// Implement YearlyMetricsLike trait from csv_export for our YearlyMetrics
impl csv_export::YearlyMetricsLike for YearlyMetrics {
    fn get_year(&self) -> u32 { self.year }
    fn get_total_population(&self) -> u32 { self.total_population }
    fn get_total_power_usage(&self) -> f64 { self.total_power_usage }
    fn get_total_power_generation(&self) -> f64 { self.total_power_generation }
    fn get_power_balance(&self) -> f64 { self.power_balance }
    fn get_average_public_opinion(&self) -> f64 { self.average_public_opinion }
    fn get_yearly_capital_cost(&self) -> f64 { self.yearly_capital_cost }
    fn get_total_capital_cost(&self) -> f64 { self.total_capital_cost }
    fn get_inflation_factor(&self) -> f64 { self.inflation_factor }
    fn get_total_co2_emissions(&self) -> f64 { self.total_co2_emissions }
    fn get_total_carbon_offset(&self) -> f64 { self.total_carbon_offset }
    fn get_net_co2_emissions(&self) -> f64 { self.net_co2_emissions }
    fn get_yearly_carbon_credit_revenue(&self) -> f64 { self.yearly_carbon_credit_revenue }
    fn get_total_carbon_credit_revenue(&self) -> f64 { self.total_carbon_credit_revenue }
    fn get_yearly_energy_sales_revenue(&self) -> f64 { self.yearly_energy_sales_revenue }
    fn get_total_energy_sales_revenue(&self) -> f64 { self.total_energy_sales_revenue }
    fn get_generator_efficiencies(&self) -> Vec<(String, f64)> { self.generator_efficiencies.clone() }
    fn get_generator_operations(&self) -> Vec<(String, f64)> { self.generator_operations.clone() }
    fn get_active_generators(&self) -> usize { self.active_generators }
    fn get_yearly_upgrade_costs(&self) -> f64 { self.yearly_upgrade_costs }
    fn get_yearly_closure_costs(&self) -> f64 { self.yearly_closure_costs }
    fn get_yearly_total_cost(&self) -> f64 { self.yearly_total_cost }
    fn get_total_cost(&self) -> f64 { self.total_cost }
}

fn run_simulation(
    map: &mut Map,
    action_weights: Option<&mut ActionWeights>,
    seed: Option<u64>,
    verbose_logging: bool,
    optimization_mode: Option<&str>,
    enable_energy_sales: bool,
) -> Result<(String, Vec<(u32, GridAction)>, Vec<YearlyMetrics>), Box<dyn Error + Send + Sync>> {
    let _timing = logging::start_timing("run_simulation", OperationCategory::Simulation);
    
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
    let state_log_file = if verbose_logging {
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
        println!("\n=== STARTING SIMULATION ===");
        if let Some(seed_value) = seed {
            println!("Seed: {}", seed_value);
        }
        weights.diagnose_best_actions();
    }
    
    for year in SIMULATION_START_YEAR..=SIMULATION_END_YEAR {
        let _year_timing = logging::start_timing(&format!("simulate_year_{}", year), OperationCategory::Simulation);
        
        if action_weights.is_none() {
            println!("\nStarting year {}", year);
            
            if year > SIMULATION_START_YEAR {
                local_weights.print_top_actions(year - 1, 5);
            }
        }
        
        // Update population for each settlement based on the current year
        if year > SIMULATION_START_YEAR {
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
            local_weights.sample_additional_actions(year) as usize
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
        let yearly_metrics = calculate_yearly_metrics(map, year, total_upgrade_costs, total_closure_costs, enable_energy_sales);
        
        // Collect yearly metrics for CSV export
        yearly_metrics_collection.push(yearly_metrics.clone());
        
        if action_weights.is_none() {
            print_yearly_summary(&yearly_metrics);
        }
        
        // For the last year, save metrics for final output
        if year == SIMULATION_END_YEAR {
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

fn handle_power_deficit(
    map: &mut Map,
    deficit: f64,
    year: u32,
    action_weights: &mut ActionWeights,
    optimization_mode: Option<&str>,
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
            GridAction::AddGenerator(GeneratorType::BatteryStorage)
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
        if let GridAction::AddGenerator(_) = action {
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
            let power_improvement = if new_state.power_balance > current_state.power_balance {
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

fn apply_action(map: &mut Map, action: &GridAction, year: u32) -> Result<(), Box<dyn Error + Send + Sync>> {
    match action {
        GridAction::AddGenerator(gen_type) => {
            let gen_size = DEFAULT_GENERATOR_SIZE;
            match map.find_best_generator_location(gen_type, gen_size as f64 / 100.0) {
                Some(location) => {
                    let base_efficiency = gen_type.get_base_efficiency(year);
                    let initial_co2_output = match gen_type {
                        GeneratorType::CoalPlant => COAL_CO2_RATE,
                        GeneratorType::GasCombinedCycle => GAS_CC_CO2_RATE,
                        GeneratorType::GasPeaker => GAS_PEAKER_CO2_RATE,
                        GeneratorType::Biomass => BIOMASS_CO2_RATE,
                        _ => 0.0,  // All other types have zero direct CO2 emissions
                    } * (gen_size as f64 / 100.0);  // Scale by size
                    
                    let generator = Generator::new(
                        format!("Gen_{}_{}_{}", gen_type.to_string(), year, map.get_generator_count()),
                        location,
                        gen_type.clone(),
                        gen_type.get_base_cost(year),
                        gen_type.get_base_power(year),
                        gen_type.get_operating_cost(year),
                        gen_type.get_lifespan(),
                        gen_size as f64 / 100.0,
                        initial_co2_output,
                        base_efficiency,
                    );
                    map.add_generator(generator);
                    Ok(())
                },
                None => {
                    // Fallback: Try a different generator type
                    let fallback_type = match gen_type {
                        GeneratorType::Nuclear => GeneratorType::GasCombinedCycle,
                        GeneratorType::HydroDam | GeneratorType::PumpedStorage => GeneratorType::GasPeaker,
                        GeneratorType::OffshoreWind => GeneratorType::OnshoreWind,
                        GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => GeneratorType::OffshoreWind,
                        _ => GeneratorType::GasPeaker, // Default fallback
                    };
                    
                    println!("Falling back to {:?} generator instead of {:?}", fallback_type, gen_type);
                    apply_action(map, &GridAction::AddGenerator(fallback_type), year)
                }
            }
        },
        GridAction::UpgradeEfficiency(id) => {
            if let Some(generator) = map.get_generator_mut(id) {
                if generator.is_active() {
                    let base_max = match generator.get_generator_type() {
                        GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => WIND_BASE_MAX_EFFICIENCY,
                        GeneratorType::UtilitySolar => UTILITY_SOLAR_BASE_MAX_EFFICIENCY,
                        GeneratorType::Nuclear => NUCLEAR_BASE_MAX_EFFICIENCY,
                        GeneratorType::GasCombinedCycle => GAS_CC_BASE_MAX_EFFICIENCY,
                        GeneratorType::HydroDam | GeneratorType::PumpedStorage => HYDRO_BASE_MAX_EFFICIENCY,
                        GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => MARINE_BASE_MAX_EFFICIENCY,
                        _ => DEFAULT_BASE_MAX_EFFICIENCY,
                    };
                    
                    let tech_improvement = match generator.get_generator_type() {
                        GeneratorType::OnshoreWind | GeneratorType::OffshoreWind | 
                        GeneratorType::UtilitySolar => DEVELOPING_TECH_IMPROVEMENT_RATE,
                        GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => EMERGING_TECH_IMPROVEMENT_RATE,
                        _ => MATURE_TECH_IMPROVEMENT_RATE,
                    }.powi((year - BASE_YEAR) as i32);
                    
                    let max_efficiency = base_max * (1.0 + (1.0 - tech_improvement));
                    generator.upgrade_efficiency(year, max_efficiency);
                }
            }
            map.after_generator_modification();
            Ok(())
        },
        GridAction::AdjustOperation(id, percentage) => {
            let constraints = map.get_generator_constraints().clone();
            if let Some(generator) = map.get_generator_mut(id) {
                if generator.is_active() {
                    generator.adjust_operation(*percentage, &constraints);
                }
            }
            map.after_generator_modification();
            Ok(())
        },
        GridAction::AddCarbonOffset(offset_type) => {
            let offset_size = rand::thread_rng().gen_range(MIN_CARBON_OFFSET_SIZE..MAX_CARBON_OFFSET_SIZE);
            let base_efficiency = rand::thread_rng().gen_range(MIN_CARBON_OFFSET_EFFICIENCY..MAX_CARBON_OFFSET_EFFICIENCY);
            
            let location = Coordinate::new(
                rand::thread_rng().gen_range(0.0..MAP_MAX_X),
                rand::thread_rng().gen_range(0.0..MAP_MAX_Y),
            );
            
            let offset = CarbonOffset::new(
                format!("Offset_{}_{}_{}", offset_type, year, map.get_carbon_offset_count()),
                location,
                CarbonOffsetType::from_str(offset_type).unwrap_or(CarbonOffsetType::Forest),
                match CarbonOffsetType::from_str(offset_type).unwrap_or(CarbonOffsetType::Forest) {
                    CarbonOffsetType::Forest => FOREST_BASE_COST,
                    CarbonOffsetType::Wetland => WETLAND_BASE_COST,
                    CarbonOffsetType::ActiveCapture => ACTIVE_CAPTURE_BASE_COST,
                    CarbonOffsetType::CarbonCredit => CARBON_CREDIT_BASE_COST,
                },
                match CarbonOffsetType::from_str(offset_type).unwrap_or(CarbonOffsetType::Forest) {
                    CarbonOffsetType::Forest => FOREST_OPERATING_COST,
                    CarbonOffsetType::Wetland => WETLAND_OPERATING_COST,
                    CarbonOffsetType::ActiveCapture => ACTIVE_CAPTURE_OPERATING_COST,
                    CarbonOffsetType::CarbonCredit => CARBON_CREDIT_OPERATING_COST,
                },
                offset_size,
                base_efficiency,
            );
            
            map.add_carbon_offset(offset);
            Ok(())
        },
        GridAction::CloseGenerator(id) => {
            if let Some(generator) = map.get_generator_mut(id) {
                if generator.is_active() {
                    let age = year - generator.commissioning_year;
                    let min_age = match generator.get_generator_type() {
                        GeneratorType::Nuclear => 1,
                        GeneratorType::HydroDam => 1,
                        GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => 1,
                        GeneratorType::UtilitySolar => 1,
                        _ => 1,
                    };
                    
                    if age >= min_age {
                        generator.close_generator(year);
                    }
                }
            }
            map.after_generator_modification();
            Ok(())
        },
        GridAction::DoNothing => {
            Ok(())
        },
    }
}

fn calculate_average_opinion(map: &Map, year: u32) -> f64 {
    let _timing = logging::start_timing("calculate_average_opinion", 
        OperationCategory::PowerCalculation { subcategory: PowerCalcType::Other });
    
    let mut total_opinion = 0.0;
    let mut count = 0;
    
    for generator in map.get_generators() {
        if generator.is_active() {
            total_opinion += map.calc_new_generator_opinion(
                generator.get_coordinate(),
                generator,
                year
            );
            count += 1;
        }
    }
    
    if count > 0 {
        total_opinion / count as f64
    } else {
        1.0
    }
}

fn calculate_yearly_metrics(map: &Map, year: u32, total_upgrade_costs: f64, total_closure_costs: f64, enable_energy_sales: bool) -> YearlyMetrics {
    let _timing = logging::start_timing("calculate_yearly_metrics", 
        OperationCategory::PowerCalculation { subcategory: PowerCalcType::Other });
    
    let total_pop = {
        let _timing = logging::start_timing("calc_total_population", 
            OperationCategory::PowerCalculation { subcategory: PowerCalcType::Usage });
        map.calc_total_population(year)
    };
    
    let total_power_usage = {
        let _timing = logging::start_timing("calc_total_power_usage", 
            OperationCategory::PowerCalculation { subcategory: PowerCalcType::Usage });
        map.calc_total_power_usage(year)
    };
    
    let total_power_gen = {
        let _timing = logging::start_timing("calc_total_power_generation", 
            OperationCategory::PowerCalculation { subcategory: PowerCalcType::Generation });
        map.calc_total_power_generation(year, None)
    };
    
    let power_balance = total_power_gen - total_power_usage;
    
    let (total_co2_emissions, total_carbon_offset, net_co2_emissions) = {
        let _timing = logging::start_timing("calc_emissions", 
            OperationCategory::PowerCalculation { subcategory: PowerCalcType::Other });
        (
            map.calc_total_co2_emissions(),
            map.calc_total_carbon_offset(year),
            map.calc_net_co2_emissions(year)
        )
    };
    
    // Calculate revenue from carbon credits for negative emissions
    let carbon_credit_revenue = {
        let _timing = logging::start_timing("calc_carbon_credit_revenue", 
            OperationCategory::PowerCalculation { subcategory: PowerCalcType::Other });
        const_funcs::calculate_carbon_credit_revenue(net_co2_emissions, year)
    };

    let mut total_opinion = 0.0;
    let mut opinion_count = 0;
    let mut generator_efficiencies = Vec::new();
    let mut generator_operations = Vec::new();
    let mut active_count = 0;

    {
        let _timing = logging::start_timing("calculate_generator_metrics", 
            OperationCategory::PowerCalculation { subcategory: PowerCalcType::Efficiency });
        
        for generator in map.get_generators() {
            if generator.is_active() {
                total_opinion += map.calc_new_generator_opinion(
                    generator.get_coordinate(),
                    generator,
                    year
                );
                opinion_count += 1;
                active_count += 1;

                generator_efficiencies.push((generator.get_id().to_string(), generator.get_efficiency()));
                // Store the operation percentage as a percentage (0-100)
                generator_operations.push((generator.get_id().to_string(), generator.get_operation_percentage() as f64));
            }
        }
    }

    // Calculate yearly and total costs
    // For 2025 (base year), subtract existing generators' costs if needed
    let yearly_capital_cost = if year == 2025 {
        // For the first year, we only count newly added generators
        map.calc_yearly_capital_cost(year)
    } else if year > 2025 {
        // For subsequent years, calculate the difference from previous year
        map.calc_total_capital_cost(year) - map.calc_total_capital_cost(year - 1)
    } else {
        0.0
    };
    
    let total_capital_cost = map.calc_total_capital_cost(year);
    let inflation_factor = const_funcs::calc_inflation_factor(year);
    
    // Calculate energy sales revenue based on power surplus
    let yearly_energy_sales_revenue = if enable_energy_sales && power_balance > 0.0 {
        // Use power surplus (positive power balance) to calculate energy sales revenue
        const_funcs::calculate_energy_sales_revenue(power_balance, year, crate::constants::DEFAULT_ENERGY_SALES_RATE)
    } else {
        0.0
    };
    
    // Calculate yearly and accumulated costs, subtracting energy sales revenue if enabled
    let yearly_total_cost = yearly_capital_cost + total_upgrade_costs + total_closure_costs - carbon_credit_revenue - 
        (if enable_energy_sales { yearly_energy_sales_revenue } else { 0.0 });
    
    let total_cost = total_capital_cost + total_upgrade_costs + total_closure_costs - carbon_credit_revenue - 
        (if enable_energy_sales { yearly_energy_sales_revenue } else { 0.0 });
    
    // Track yearly and accumulated carbon credit revenue
    let yearly_carbon_credit_revenue = carbon_credit_revenue;
    let total_carbon_credit_revenue = carbon_credit_revenue; // Would need to accumulate this across years

    // Calculate total energy sales revenue 
    let total_energy_sales_revenue = yearly_energy_sales_revenue; // Would need accumulation across years

    YearlyMetrics {
        year,
        total_population: total_pop,
        total_power_usage,
        total_power_generation: total_power_gen,
        power_balance,
        average_public_opinion: if opinion_count > 0 { total_opinion / opinion_count as f64 } else { 1.0 },
        yearly_capital_cost,
        total_capital_cost,
        inflation_factor,
        total_co2_emissions,
        total_carbon_offset,
        net_co2_emissions,
        yearly_carbon_credit_revenue: carbon_credit_revenue,
        total_carbon_credit_revenue,
        yearly_energy_sales_revenue,
        total_energy_sales_revenue,
        generator_efficiencies,
        generator_operations,
        active_generators: active_count,
        yearly_upgrade_costs: total_upgrade_costs,
        yearly_closure_costs: total_closure_costs,
        yearly_total_cost,
        total_cost,
    }
}

fn print_yearly_summary(metrics: &YearlyMetrics) {
    println!("\nYear {} Summary", metrics.year);
    println!("----------------------------------------");
    println!("Population: {}", metrics.total_population);
    println!("Power Metrics:");
    println!("  Usage: {:.2} MW", metrics.total_power_usage);
    println!("  Generation: {:.2} MW", metrics.total_power_generation);
    println!("  Balance: {:.2} MW", metrics.power_balance);
    println!("Financial Metrics:");
    println!("  Yearly Capital Cost: €{:.2}", metrics.yearly_capital_cost);
    println!("  Total Capital Cost: €{:.2}", metrics.total_capital_cost);
    println!("  Yearly Upgrade Costs: €{:.2}", metrics.yearly_upgrade_costs);
    println!("  Yearly Closure Costs: €{:.2}", metrics.yearly_closure_costs);
    if metrics.yearly_carbon_credit_revenue > 0.0 {
        println!("  Yearly Carbon Credit Revenue: €{:.2}", metrics.yearly_carbon_credit_revenue);
        println!("  Total Carbon Credit Revenue: €{:.2}", metrics.total_carbon_credit_revenue);
    }
    if metrics.yearly_energy_sales_revenue > 0.0 {
        println!("  Yearly Energy Sales Revenue: €{:.2}", metrics.yearly_energy_sales_revenue);
        println!("  Total Energy Sales Revenue: €{:.2}", metrics.total_energy_sales_revenue);
    }
    println!("  Yearly Total Cost: €{:.2}", metrics.yearly_total_cost);
    println!("  Accumulated Total Cost: €{:.2}", metrics.total_cost);
    println!("Environmental Metrics:");
    println!("  CO2 Emissions: {:.2} tonnes", metrics.total_co2_emissions);
    println!("  Carbon Offset: {:.2} tonnes", metrics.total_carbon_offset);
    println!("  Net Emissions: {:.2} tonnes", metrics.net_co2_emissions);
    println!("Public Opinion: {:.3}", metrics.average_public_opinion);
    println!("Active Generators: {}", metrics.active_generators);
}

fn print_generator_details(metrics: &YearlyMetrics) {
    println!("\nGenerator Details:");
    println!("----------------------------------------");
    for (id, efficiency) in &metrics.generator_efficiencies {
        let operation = metrics.generator_operations.iter()
            .find(|(gen_id, _)| gen_id == id)
            .map(|(_, op)| op)
            .unwrap_or(&0.0);
        
        println!("{}: Efficiency: {:.2}, Operation: {:.1}%", 
                id, efficiency, operation);
    }
    println!("----------------------------------------");
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short = 'n', long, default_value_t = 1000)]
    iterations: usize,

    #[arg(short, long, default_value_t = true)]
    parallel: bool,

    #[arg(long, default_value_t = false)]
    no_continue: bool,

    #[arg(short, long, default_value = "checkpoints")]
    checkpoint_dir: String,

    #[arg(short = 'i', long, default_value_t = 5)]
    checkpoint_interval: usize,

    #[arg(short = 'r', long, default_value_t = 10)]
    progress_interval: usize,

    #[arg(short = 'C', long, default_value = "cache")]
    cache_dir: String,

    #[arg(long, default_value_t = false)]
    force_full_simulation: bool,

    #[arg(long, default_value_t = false)]
    enable_timing: bool,

    #[arg(long, help = "Random seed for deterministic simulation")]
    seed: Option<u64>,

    #[arg(short, long, default_value_t = true)]
    verbose_state_logging: bool,
    
    #[arg(long, help = "Optimize for cost only, ignoring emissions and public opinion", default_value_t = false)]
    cost_only: bool,
    
    #[arg(long, help = "Enable revenue from energy sales to offset costs", default_value_t = false)]
    enable_energy_sales: bool,
}

fn run_multi_simulation(
    base_map: &Map,
    num_iterations: usize,
    parallel: bool,
    continue_from_checkpoint: bool,
    checkpoint_dir: &str,
    checkpoint_interval: usize,
    progress_interval: usize,
    cache_dir: &str,
    force_full_simulation: bool,
    seed: Option<u64>,
    verbose_logging: bool,
    optimization_mode: Option<&str>,
    enable_energy_sales: bool,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let _timing = logging::start_timing("run_multi_simulation", OperationCategory::Simulation);
    
    let result = (|| {
        // Create checkpoint directory if it doesn't exist
        std::fs::create_dir_all(checkpoint_dir)?;
        
        // Load location analysis cache
        let mut base_map = base_map.clone();
        let cache_loaded = base_map.load_location_analysis(cache_dir)?;
        if !cache_loaded {
            println!("Warning: Location analysis cache not found in {}. All simulations will use full mode.", cache_dir);
        }

        // Initialize progress tracking
        let completed_iterations = Arc::new(AtomicUsize::new(0));
        let start_time = Instant::now();
        
        // Load or create initial weights
        let initial_weights = if continue_from_checkpoint {
            // Try to find the most recent checkpoint
            let entries: Vec<_> = std::fs::read_dir(checkpoint_dir)?
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_dir())
                .collect();
            
            let latest_dir = entries.iter()
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| {
                    // Filter out directories with invalid format
                    if name.len() != 15 || !name.chars().all(|c| c.is_ascii_digit() || c == '_') {
                        return false;
                    }
                    // Parse the date from the directory name (format: YYYYMMDD_HHMMSS)
                    let year = name[0..4].parse::<i32>().unwrap_or(9999);
                    let month = name[4..6].parse::<u32>().unwrap_or(99);
                    let day = name[6..8].parse::<u32>().unwrap_or(99);
                    
                    // Accept 2025 and earlier, but ensure month and day are valid
                    if year > 2025 || month > 12 || day > 31 {
                        return false;
                    }
                    true
                })
                .max();
            
            if let Some(latest) = latest_dir {
                let checkpoint_dir = Path::new(checkpoint_dir).join(&latest);
                println!("Checking for weights in: {:?}", checkpoint_dir);
                
                // Load and merge all thread weights
                let mut merged_weights = ActionWeights::new();
                let mut found_weights = false;
                
                // First load the shared weights if they exist
                let shared_weights_path = checkpoint_dir.join("latest_weights.json");
                if shared_weights_path.exists() {
                    println!("Loading shared weights from: {:?}", shared_weights_path);
                    if let Ok(weights) = ActionWeights::load_from_file(shared_weights_path.to_str().unwrap()) {
                        merged_weights = weights;
                        found_weights = true;
                    }
                }
                
                // Then load and merge all thread-specific weights
                for entry in std::fs::read_dir(&checkpoint_dir)? {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                            if filename.starts_with("thread_") && filename.ends_with("_weights.json") {
                                println!("Loading thread weights from: {:?}", path);
                                if let Ok(thread_weights) = ActionWeights::load_from_file(path.to_str().unwrap()) {
                                    merged_weights.update_weights_from(&thread_weights);
                                    found_weights = true;
                                }
                            }
                        }
                    }
                }
                
                if found_weights {
                    if let Some((best_score, _)) = merged_weights.get_best_metrics() {
                        println!("\n{}", "=".repeat(80));
                        println!("📊 LOADED AND MERGED WEIGHTS FROM PREVIOUS RUNS 📊");
                        println!("Best score from loaded weights: {:.4}", best_score);
                        println!("{}", "=".repeat(80));
                    }
                    merged_weights
                } else {
                    println!("No weights found in latest directory, starting fresh");
                    ActionWeights::new()
                }
            } else {
                println!("No checkpoint directories found, starting fresh");
                ActionWeights::new()
            }
        } else {
            println!("Starting fresh simulation (--no-continue specified)");
            ActionWeights::new()
        };

        // Create timestamp directory after loading weights
        let now = Local::now();
        let timestamp = format!("2024{}", now.format("%m%d_%H%M%S"));
        let run_dir = format!("{}/{}", checkpoint_dir, timestamp);
        std::fs::create_dir_all(&run_dir)?;
        
        // Create shared weights
        let action_weights = Arc::new(RwLock::new(initial_weights));
        
        // Spawn progress monitoring thread
        let progress_counter = completed_iterations.clone();
        let total_iterations = num_iterations;
        let action_weights_for_progress = Arc::clone(&action_weights);
        
        std::thread::spawn(move || {
            while progress_counter.load(Ordering::Relaxed) < total_iterations {
                std::thread::sleep(Duration::from_secs(progress_interval as u64));
                let completed = progress_counter.load(Ordering::Relaxed);
                let elapsed = start_time.elapsed();
                let iterations_per_second = completed as f64 / elapsed.as_secs_f64();
                let remaining = total_iterations - completed;
                let eta_seconds = if iterations_per_second > 0.0 {
                    remaining as f64 / iterations_per_second
                } else {
                    0.0
                };

                // Get best score and metrics from the shared weights
                let weights = action_weights_for_progress.read();
                let (best_score, is_net_zero) = weights.get_best_metrics()
                    .unwrap_or((0.0, false));
                
                // Get emissions metrics from the best metrics
                let metrics_info = if let Some(best) = weights.get_simulation_metrics() {
                    let emissions_status = if best.final_net_emissions <= 0.0 {
                        "✅ NET ZERO ACHIEVED".to_string()
                    } else {
                        format!("⚠ {:.1}% above target", 
                            (best.final_net_emissions / MAX_ACCEPTABLE_EMISSIONS) * 100.0)
                    };
                    
                    let cost_status = if best.total_cost <= MAX_ACCEPTABLE_COST {
                        "✅ WITHIN BUDGET".to_string()
                    } else {
                        format!("❌ {:.1}% OVER BUDGET", 
                            ((best.total_cost - MAX_ACCEPTABLE_COST) / MAX_ACCEPTABLE_COST) * 100.0)
                    };
                    
                    format!("\nMetrics Status:\n\
                            - Emissions: {} ({:.1} tonnes)\n\
                            - Cost: {} (€{:.1}B accumulated)\n\
                            - Public Opinion: {:.1}%\n\
                            - Power Reliability: {:.1}%", 
                            emissions_status,
                            best.final_net_emissions,
                            cost_status,
                            best.total_cost / 1_000_000_000.0,
                            best.average_public_opinion * 100.0,
                            best.power_reliability * 100.0)
                } else {
                    "\nMetrics Status: No data yet".to_string()
                };
                
                println!(
                    "\n{}\n\
                    📈 PROGRESS UPDATE 📈\n\
                    {}\n\
                    Iterations: {}/{} ({:.1}%)\n\
                    Speed: {:.1} iterations/sec\n\
                    ETA: {:.1} minutes\n\
                    \n\
                    Best Score: {:.9} {}\n\
                    Target: 1.0000 (Net Zero + Max Public Opinion){}\n\
                    \n\
                    Score Explanation:\n\
                    - Score < 1.0000: Working on reducing emissions\n\
                    - Score = 0.0000: Emissions at or above maximum\n\
                    - Score > 0.0000: Making progress on emissions\n\
                    - [NET ZERO]: Achieved net zero, score is now public opinion\n\
                    {}",
                    "=".repeat(80),
                    "=".repeat(80),
                    completed,
                    total_iterations,
                    (completed as f64 / total_iterations as f64) * 100.0,
                    iterations_per_second,
                    eta_seconds / 60.0,
                    best_score,
                    if is_net_zero { "✅ [NET ZERO]" } else { "" },
                    metrics_info,
                    "=".repeat(80)
                );
            }
        });

        let mut best_result: Option<SimulationResult> = None;
        let start_iteration = if continue_from_checkpoint {
            let entries: Vec<_> = std::fs::read_dir(checkpoint_dir)?
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_dir())
                .collect();
            
            if let Some(latest_dir) = entries.iter()
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| name.chars().all(|c| c.is_ascii_digit() || c == '_'))
                .max()
            {
                let iteration_path = Path::new(checkpoint_dir)
                    .join(&latest_dir)
                    .join("checkpoint_iteration.txt");
                
                if iteration_path.exists() {
                    std::fs::read_to_string(iteration_path)?
                        .trim()
                        .parse::<usize>()
                        .unwrap_or(0)
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        };
        
        println!("Starting multi-simulation optimization with {} iterations ({} completed, {} remaining) in directory {}", 
                 num_iterations,
                 start_iteration,
                 num_iterations - start_iteration,
                 run_dir);
        
        // Create a clone of the base map's static data once
        let static_data = base_map.get_static_data();
        
        if parallel {
            let results: Vec<_> = (start_iteration..num_iterations)
                .into_par_iter()
                .map(|i| -> Result<SimulationResult, Box<dyn Error + Send + Sync>> {
                    // Create a new map instance with shared static data
                    let mut map_clone = Map::new_with_static_data(static_data.clone());
                    
                    // Clone only the dynamic data
                    map_clone.set_generators(base_map.get_generators().to_vec());
                    map_clone.set_settlements(base_map.get_settlements().to_vec());
                    map_clone.set_carbon_offsets(base_map.get_carbon_offsets().to_vec());

                    // Set simulation mode based on global progress
                    let final_full_sim_count = (num_iterations * FULL_RUN_PERCENTAGE) / 100;
                    let total_completed = completed_iterations.load(Ordering::Relaxed);
                    
                    // A run should be a full run if:
                    // 1. Full simulation is forced by command line
                    // 2. We're in the final X% of iterations (based on FULL_RUN_PERCENTAGE)
                    // 3. Cache isn't loaded (forcing full sim)
                    let is_full_run = force_full_simulation || 
                                      !cache_loaded || 
                                      total_completed >= num_iterations.saturating_sub(final_full_sim_count);
                    
                    // Set simulation mode
                    map_clone.set_simulation_mode(!is_full_run);
                    
                    if total_completed == num_iterations.saturating_sub(final_full_sim_count) {
                        println!("\nSwitching to full simulation mode for final {} iterations ({:.1}% of total)", 
                                final_full_sim_count, FULL_RUN_PERCENTAGE as f64);
                    }
                    
                    // Create local weights and immediately drop the read lock
                    let mut local_weights = {
                        let weights = action_weights.read();
                        weights.clone()
                    }; // Read lock is dropped here
                    
                    // Determine if we should replay the best strategy
                    let replay_best_strategy = is_full_run && 
                                              REPLAY_BEST_STRATEGY_IN_FULL_RUNS && 
                                              local_weights.has_best_actions();
                    
                    // Log if we're replaying the best strategy
                    if replay_best_strategy {
                        println!("🔁 Iteration {} is replaying the best strategy for thorough analysis", i + 1);
                    }
                    
                    let result = run_iteration(i, &mut map_clone, &mut local_weights, replay_best_strategy, seed, verbose_logging, optimization_mode, enable_energy_sales)?;
                    
                    // Update best metrics immediately - changed order to transfer actions first
                    {
                        let mut weights = parking_lot::RwLock::write(&*action_weights);
                        // First transfer weights and recorded actions from the local instance
                        weights.update_weights_from(&local_weights);
                        // Then update best strategy after we have the actions
                        weights.update_best_strategy(result.metrics.clone());
                    }
                    
                    // Increment completed iterations counter
                    completed_iterations.fetch_add(1, Ordering::Relaxed);
                    
                    // Save checkpoint at intervals
                    if (i + 1) % checkpoint_interval == 0 {
                        let thread_id = rayon::current_thread_index().unwrap_or(0);
                        let weights = parking_lot::RwLock::write(&*action_weights);
                        
                        // Save thread-specific weights
                        let thread_weights_path = Path::new(&run_dir)
                            .join(format!("thread_{}_weights.json", thread_id));
                        local_weights.save_to_file(thread_weights_path.to_str().unwrap())?;
                        
                        // Save shared weights
                        let checkpoint_path = Path::new(&run_dir).join("latest_weights.json");
                        weights.save_to_file(checkpoint_path.to_str().unwrap())?;
                        
                        // Save iteration number
                        let iteration_path = Path::new(&run_dir).join("checkpoint_iteration.txt");
                        std::fs::write(iteration_path, (i + 1).to_string())?;
                        
                        println!("Saved checkpoint at iteration {} in {} (thread {})", i + 1, run_dir, thread_id);
                    }
                    
                    // Return the result (clone not needed anymore since we're returning it)
                    Ok(result)
                })
                .collect::<Result<Vec<_>, _>>()?;
            
            // Find the best result from all results AFTER the parallel execution completes
            for result in results {
                if best_result.as_ref().map_or(true, |best| {
                    evaluate_action_impact(&metrics_to_action_result(&result.metrics), &metrics_to_action_result(&best.metrics), optimization_mode) > 0.0
                }) {
                    best_result = Some(result);
                }
            }
        } else {
            for i in start_iteration..num_iterations {
                // Create a new map instance with shared static data
                let mut map_clone = Map::new_with_static_data(static_data.clone());
                
                // Clone only the dynamic data
                map_clone.set_generators(base_map.get_generators().to_vec());
                map_clone.set_settlements(base_map.get_settlements().to_vec());
                map_clone.set_carbon_offsets(base_map.get_carbon_offsets().to_vec());

                // Set simulation mode based on global progress
                let final_full_sim_count = (num_iterations * FULL_RUN_PERCENTAGE) / 100;
                let total_completed = completed_iterations.load(Ordering::Relaxed);
                
                // A run should be a full run if:
                // 1. Full simulation is forced by command line
                // 2. We're in the final X% of iterations (based on FULL_RUN_PERCENTAGE)
                // 3. Cache isn't loaded (forcing full sim)
                let is_full_run = force_full_simulation || 
                                  !cache_loaded || 
                                  total_completed >= num_iterations.saturating_sub(final_full_sim_count);
                
                // Set simulation mode
                map_clone.set_simulation_mode(!is_full_run);
                
                if total_completed == num_iterations.saturating_sub(final_full_sim_count) {
                    println!("\nSwitching to full simulation mode for final {} iterations ({:.1}% of total)", 
                            final_full_sim_count, FULL_RUN_PERCENTAGE as f64);
                }
                
                // Create local weights and immediately drop the read lock
                let mut local_weights = {
                    let weights = action_weights.read();
                    weights.clone()
                }; // Read lock is dropped here
                
                // Determine if we should replay the best strategy
                let replay_best_strategy = is_full_run && 
                                          REPLAY_BEST_STRATEGY_IN_FULL_RUNS && 
                                          local_weights.has_best_actions();
                
                // Log if we're replaying the best strategy
                if replay_best_strategy {
                    println!("🔁 Iteration {} is replaying the best strategy for thorough analysis", i + 1);
                }
                
                let result = run_iteration(i, &mut map_clone, &mut local_weights, replay_best_strategy, seed, verbose_logging, optimization_mode, enable_energy_sales)?;
                
                // Update best metrics immediately - changed order to transfer actions first
                {
                    let mut weights = parking_lot::RwLock::write(&*action_weights);
                    // First transfer weights and recorded actions from the local instance
                    weights.update_weights_from(&local_weights);
                    // Then update best strategy after we have the actions
                    weights.update_best_strategy(result.metrics.clone());
                }
                
                // Store each result for later comparison
                let curr_result = result.clone();
                
                // Increment completed iterations counter
                completed_iterations.fetch_add(1, Ordering::Relaxed);
                
                // Save checkpoint at intervals
                if (i + 1) % checkpoint_interval == 0 {
                    let checkpoint_path = Path::new(&run_dir).join("latest_weights.json");
                    
                    // Get a write lock to save the weights
                    {
                        let weights = parking_lot::RwLock::write(&*action_weights);
                        weights.save_to_file(checkpoint_path.to_str().unwrap())?;
                    }
                    
                    // Save iteration number
                    let iteration_path = Path::new(&run_dir).join("checkpoint_iteration.txt");
                    std::fs::write(iteration_path, (i + 1).to_string())?;
                    
                    println!("Saved checkpoint at iteration {} in {}", i + 1, run_dir);
                }
                
                // Check if this result is better than our best result
                if best_result.as_ref().map_or(true, |best| {
                    evaluate_action_impact(&metrics_to_action_result(&curr_result.metrics), &metrics_to_action_result(&best.metrics), optimization_mode) > 0.0
                }) {
                    best_result = Some(curr_result);
                }
            }
        }
        
        if let Some(best) = best_result {
            println!("\n{}", "=".repeat(80));
            println!("🏆 BEST SIMULATION RESULTS SUMMARY 🏆");
            println!("{}", "=".repeat(80));
            println!("Final net emissions: {:.2} tonnes", best.metrics.final_net_emissions);
            
            // Add emoji indicators for success/failure
            let emissions_status = if best.metrics.final_net_emissions <= 0.0 {
                "✅ NET ZERO ACHIEVED".to_string()
            } else {
                "❌ NET ZERO NOT ACHIEVED".to_string()
            };
            
            let cost_status = if best.metrics.total_cost <= MAX_ACCEPTABLE_COST {
                "✅ WITHIN BUDGET".to_string()
            } else {
                format!("❌ {:.1}% OVER BUDGET", 
                    ((best.metrics.total_cost - MAX_ACCEPTABLE_COST) / (MAX_ACCEPTABLE_COST)) * 100.0)
            };
            
            println!("Emissions Status: {}", emissions_status);
            println!("Average public opinion: {:.1}%", best.metrics.average_public_opinion * 100.0);
            
            // Display total cost in billions
            let total_cost_billions = best.metrics.total_cost / 1_000_000_000.0;
            println!("Total cost: €{:.2} billion accumulated ({})", 
                total_cost_billions, cost_status);
            
            println!("Power reliability: {:.1}%", best.metrics.power_reliability * 100.0);
            println!("{}", "=".repeat(80));
            
            // Use our enhanced CSV exporter for more detailed data export
            let csv_export_dir = Path::new(&run_dir).join("enhanced_csv");
            std::fs::create_dir_all(&csv_export_dir)?;
            
            // Create a CSV exporter instance
            let csv_exporter = csv_export::CsvExporter::new(&csv_export_dir);
            
            // Create a new map and apply all the best actions to it
            let mut final_map = base_map.clone();
            println!("Applying all actions to map for CSV export...");
            for (year, action) in &best.actions {
                if let Err(e) = apply_action(&mut final_map, action, *year) {
                    println!("Warning: Failed to apply action {:?} for year {}: {}", action, year, e);
                }
            }
            
            // Export all simulation results with detailed data using the final map
            if let Ok(()) = csv_exporter.export_simulation_results(
                &final_map,  // Use the map with all actions applied instead of base_map
                &best.actions,
                &best.metrics,
                &csv_export::convert_yearly_metrics(&best.yearly_metrics),
            ) {
                println!("\nEnhanced simulation results exported to: {}", csv_export_dir.display());
                println!("Use these files for detailed analysis and visualization.");
            } else {
                // Fallback to basic export if the enhanced export fails
                let csv_filename = Path::new(&run_dir).join("best_simulation.csv");
                let mut file = File::create(&csv_filename)?;
                file.write_all(best.output.as_bytes())?;
                println!("\nBasic simulation results saved to: {}", csv_filename.display());
                
                // Still create basic actions file as fallback
                let actions_filename = Path::new(&run_dir).join("best_simulation_actions.csv");
                let mut actions_file = File::create(&actions_filename)?;
                actions_file.write_all(
                    "Year,Action Type,Details,Capital Cost (EUR),Operating Cost (EUR),\
                    Location X,Location Y,Generator Type,Power Output (MW),Efficiency,\
                    CO2 Output (tonnes),Operation Percentage,Lifespan (years),\
                    Previous State,Impact Description\n".as_bytes()
                )?;
                
                // Write actions in basic format as fallback
                for (year, action) in &best.actions {
                    let (
                        action_type, details, capital_cost, operating_cost,
                        location_x, location_y, gen_type, power_output, efficiency,
                        co2_output, operation_percentage, lifespan, prev_state, impact
                    ) = match action {
                        GridAction::AddGenerator(gen_type) => {
                            let location = if let Some(gen) = base_map.get_generators().iter()
                                .find(|g| g.get_id().contains(&format!("{}_{}", gen_type.to_string(), year))) {
                                (gen.coordinate.x, gen.coordinate.y)
                            } else {
                                (0.0, 0.0) // Default if not found
                            };
                            (
                                "Add Generator",
                                gen_type.to_string(),
                                gen_type.get_base_cost(*year),
                                gen_type.get_operating_cost(*year),
                                location.0,
                                location.1,
                                gen_type.to_string(),
                                gen_type.get_base_power(*year),
                                gen_type.get_base_efficiency(*year),
                                match gen_type {
                                    GeneratorType::CoalPlant => 1000.0,
                                    GeneratorType::GasCombinedCycle => 500.0,
                                    GeneratorType::GasPeaker => 700.0,
                                    GeneratorType::Biomass => 50.0,
                                    _ => 0.0,
                                },
                                100, // New generators start at 100%
                                gen_type.get_lifespan(),
                                "New Installation".to_string(),
                                format!("Added new {} power generation capacity", gen_type.to_string())
                            )
                        },
                        GridAction::UpgradeEfficiency(id) => {
                            let generator = base_map.get_generators().iter().find(|g| g.get_id() == id);
                            if let Some(gen) = generator {
                                let base_max = match gen.get_generator_type() {
                                    GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => 0.45,
                                    GeneratorType::UtilitySolar => 0.40,
                                    GeneratorType::Nuclear => 0.50,
                                    GeneratorType::GasCombinedCycle => 0.60,
                                    GeneratorType::HydroDam | GeneratorType::PumpedStorage => 0.85,
                                    GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => 0.35,
                                    _ => 0.40,
                                };
                                let tech_improvement = match gen.get_generator_type() {
                                    GeneratorType::OnshoreWind | GeneratorType::OffshoreWind | 
                                    GeneratorType::UtilitySolar => DEVELOPING_TECH_IMPROVEMENT_RATE,
                                    GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => EMERGING_TECH_IMPROVEMENT_RATE,
                                    _ => MATURE_TECH_IMPROVEMENT_RATE,
                                }.powi((year - BASE_YEAR) as i32);
                                let max_efficiency = base_max * (1.0 + (1.0 - tech_improvement));
                                let upgrade_cost = gen.get_current_cost(*year) * (max_efficiency - gen.get_efficiency()) * 2.0;
                                (
                                    "Upgrade Efficiency",
                                    id.clone(),
                                    upgrade_cost,
                                    0.0,
                                    gen.coordinate.x,
                                    gen.coordinate.y,
                                    gen.get_generator_type().to_string(),
                                    gen.power_out,
                                    max_efficiency,
                                    gen.co2_out as f64,
                                    gen.get_operation_percentage() as i32,
                                    gen.eol,
                                    "Previous Efficiency".to_string(),
                                    format!("Upgraded efficiency from {:.1}% to {:.1}%", 
                                        gen.get_efficiency() * 100.0, max_efficiency * 100.0)
                                )
                            } else {
                                continue; // Skip if generator not found
                            }
                        },
                        GridAction::AdjustOperation(id, percentage) => {
                            let generator = base_map.get_generators().iter().find(|g| g.get_id() == id);
                            if let Some(gen) = generator {
                                (
                                    "Adjust Operation",
                                    format!("{} to {}%", id, percentage),
                                    0.0,
                                    0.0,
                                    gen.coordinate.x,
                                    gen.coordinate.y,
                                    gen.get_generator_type().to_string(),
                                    gen.power_out,
                                    gen.get_efficiency(),
                                    gen.co2_out as f64,
                                    *percentage as i32,
                                    gen.eol,
                                    "Previous Operation".to_string(),
                                    format!("Adjusted operation from {}% to {}%", 
                                        gen.get_operation_percentage(), percentage)
                                )
                            } else {
                                continue; // Skip if generator not found
                            }
                        },
                        GridAction::AddCarbonOffset(offset_type) => {
                            let offset_type = CarbonOffsetType::from_str(&offset_type).unwrap_or(CarbonOffsetType::Forest);
                            let base_cost = match offset_type {
                                CarbonOffsetType::Forest => 10_000_000.0,
                                CarbonOffsetType::Wetland => 15_000_000.0,
                                CarbonOffsetType::ActiveCapture => 100_000_000.0,
                                CarbonOffsetType::CarbonCredit => 5_000_000.0,
                            };
                            let operating_cost = match offset_type {
                                CarbonOffsetType::Forest => 500_000.0,
                                CarbonOffsetType::Wetland => 750_000.0,
                                CarbonOffsetType::ActiveCapture => 5_000_000.0,
                                CarbonOffsetType::CarbonCredit => 250_000.0,
                            };
                            // Find the offset in the map to get its location
                            let offset = base_map.get_carbon_offsets().iter()
                                .find(|o| o.get_id().contains(&format!("{}_{}", offset_type.to_string(), year)));
                            let location = if let Some(o) = offset {
                                (o.get_coordinate().x, o.get_coordinate().y)
                            } else {
                                (0.0, 0.0)
                            };
                            (
                                "Add Carbon Offset",
                                offset_type.to_string(),
                                base_cost,
                                operating_cost,
                                location.0,
                                location.1,
                                "Carbon Offset".to_string(),
                                0.0, // No power output
                                0.0, // No efficiency
                                0.0, // No direct CO2 output
                                100, // Always fully operational
                                30, // Standard offset lifespan
                                "New Offset".to_string(),
                                format!("Added new {} carbon offset project", offset_type.to_string())
                            )
                        },
                        GridAction::CloseGenerator(id) => {
                            let generator = base_map.get_generators().iter().find(|g| g.get_id() == id);
                            if let Some(gen) = generator {
                                let years_remaining = (gen.eol as i32 - (year - 2025) as i32).max(0) as f64;
                                let closure_cost = gen.get_current_cost(*year) * 0.5 * (years_remaining / gen.eol as f64);
                                (
                                    "Close Generator",
                                    id.clone(),
                                    closure_cost,
                                    0.0,
                                    gen.coordinate.x,
                                    gen.coordinate.y,
                                    gen.get_generator_type().to_string(),
                                    gen.power_out,
                                    gen.get_efficiency(),
                                    gen.co2_out as f64,
                                    0i32, // Closed generators have 0% operation
                                    gen.eol,
                                    "Previously Active".to_string(),
                                    format!("Closed {} generator after {} years of operation", 
                                        gen.get_generator_type().to_string(), year - gen.commissioning_year)
                                )
                            } else {
                                continue; // Skip if generator not found
                            }
                        },
                        GridAction::DoNothing => {
                            (
                                "Do Nothing",
                                "".to_string(), // no details
                                0.0,            // capital cost
                                0.0,            // operating cost
                                0.0,            // location_x
                                0.0,            // location_y
                                "".to_string(), // generator type
                                0.0,            // power output
                                0.0,            // efficiency
                                0.0,            // co2 output
                                0,              // operation percentage
                                0,              // lifespan
                                "".to_string(), // previous state
                                "".to_string()  // impact description
                            )
                        },
                    };
                    
                    actions_file.write_all(format!(
                        "{},{},\"{}\",{:.2},{:.2},{:.1},{:.1},\"{}\",{:.2},{:.3},{:.2},{},{},\"{}\",\"{}\"\n",
                        year,
                        action_type,
                        details,
                        capital_cost,
                        operating_cost,
                        location_x,
                        location_y,
                        gen_type,
                        power_output,
                        efficiency,
                        co2_output,
                        operation_percentage,
                        lifespan,
                        prev_state,
                        impact
                    ).as_bytes())?;
                }
                
                println!("Basic action history saved to: {}", actions_filename.display());
            }
            
            // Save final weights in the run directory
            let final_weights_path = Path::new(&run_dir).join("best_weights.json");
            let weights = parking_lot::RwLock::write(&*action_weights);
            weights.save_to_file(final_weights_path.to_str().unwrap())?;
            println!("Final weights saved to: {}", final_weights_path.display());
        }
        
        Ok(())
    })();
    
    // Print final timing report
    if logging::is_timing_enabled() {
        logging::print_timing_report();
    }
    
    result
}

fn run_iteration(
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
    let start = std::time::Instant::now();

    // Set a fixed random seed if specified
    if let Some(seed_value) = seed {
        // Create a deterministic seed that's unique to this iteration but reproducible
        let iteration_seed = seed_value.wrapping_add(iteration as u64);
        let rng = StdRng::seed_from_u64(iteration_seed);
        
        // Pass the seeded RNG to the weights
        weights.set_rng(rng);
        
        println!("\n🔢 Using deterministic seed: {} (iteration adjusted: {})", 
                seed_value, iteration_seed);
    }

    // Set replay flag in weights if needed
    weights.set_force_best_actions(replay_best_strategy);
    if replay_best_strategy {
        println!("\n🔄 Replaying best strategy from previous runs");
    } else {
        println!("\n🔄 Running iteration {} with adaptive action selection", iteration + 1);
    }
    
    // Start a new iteration in the weights tracking - this clears current_run_actions
    weights.start_new_iteration();
    
    // Debug: Verify current_run_actions is empty after start_new_iteration
    println!("\n📊 DEBUG: After start_new_iteration - verifying current_run_actions is empty");
    weights.debug_print_recorded_actions();
    
    // Debug: Verify current_deficit_actions is empty as well
    println!("\n📊 DEBUG: Verifying current_deficit_actions is empty");
    weights.debug_print_deficit_actions();

    // Run the simulation inside a closure to manage errors
    let result: Result<SimulationResult, Box<dyn Error + Send + Sync>> = (|| {
        // Create a new clean clone of the map to run this iteration on
        let mut map_clone = map.clone();
        
        // Run the simulation
        let (simulation_output, recorded_actions, yearly_metrics) = if replay_best_strategy {
            run_simulation_with_best_actions(&mut map_clone, weights, seed, verbose_logging, optimization_mode, enable_energy_sales)?
        } else {
            run_simulation(&mut map_clone, Some(weights), seed, verbose_logging, optimization_mode, enable_energy_sales)?
        };
        
        // Debug: Verify actions were recorded during simulation
        println!("\n📊 DEBUG: After run_simulation - verifying actions were recorded");
        weights.debug_print_recorded_actions();
        
        // Debug: Verify deficit actions were recorded during simulation
        println!("\n📊 DEBUG: Verifying deficit actions were recorded");
        weights.debug_print_deficit_actions();
        
        // Calculate final metrics
        let simulation_duration = start.elapsed();
        
        // Extract performance metrics from the final yearly metrics
        let final_metrics = if let Some(metrics) = yearly_metrics.last() {
            let total_cost = metrics.total_cost;
            
            SimulationMetrics {
                final_net_emissions: metrics.net_co2_emissions,
                average_public_opinion: metrics.average_public_opinion,
                total_cost,
                power_reliability: if metrics.power_balance < 0.0 { 0.0 } else { 1.0 },
            }
        } else {
            // Fallback metrics if no yearly metrics (shouldn't happen)
            SimulationMetrics {
                final_net_emissions: f64::MAX,
                average_public_opinion: 0.0,
                total_cost: f64::MAX,
                power_reliability: 0.0,
            }
        };
        
        // Only update best strategy if we're not replaying (to avoid circular updates)
        if !replay_best_strategy {
            // Update best strategy with the final metrics from this iteration
            println!("\n📊 DEBUG: Updating best strategy with final metrics");
            weights.update_best_strategy(final_metrics.clone());
            
            // Also update best deficit actions when we update the best strategy
            println!("\n📊 DEBUG: Updating best deficit actions");
            weights.update_best_deficit_actions();
            
            // Debug: Print the best actions status after updating
            println!("\n📊 BEST ACTIONS DIAGNOSIS:");
            weights.diagnose_best_actions();
        } else {
            println!("\n📊 DEBUG: Skipping best strategy update for replay run");
        }

        
        
        // Apply contrast learning if this wasn't the best run and wasn't a replay
        if !replay_best_strategy {
            weights.apply_contrast_learning(&final_metrics);
        }
        
        println!("⏱️  Simulation completed in {:.2?}", simulation_duration);
        
        Ok(SimulationResult {
            metrics: final_metrics,
            output: simulation_output,
            actions: recorded_actions,
            yearly_metrics: yearly_metrics, // Add yearly metrics to the struct
        })
    })();
    
    // Return the result or propagate errors
    result
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Parse command line arguments
    let args = Args::parse();
    
    // Initialize logging with timing enabled/disabled based on command line arg
    logging::init_logging(args.enable_timing);
    
    println!("EirGrid Power System Simulator (2025-2050)");
    println!("Debug: no_continue = {}, continue_from_checkpoint = {}", args.no_continue, !args.no_continue);
    
    let config = SimulationConfig::default();
    let mut map = Map::new(config);
    
    // Initialize the map, now with seed support
    initialize_map(&mut map, args.seed);
    
    run_multi_simulation(
        &map, 
        args.iterations,
        args.parallel,
        !args.no_continue,
        &args.checkpoint_dir,
        args.checkpoint_interval,
        args.progress_interval,
        &args.cache_dir,
        args.force_full_simulation,
        args.seed,
        args.verbose_state_logging,
        if args.cost_only { Some("cost_only") } else { None },
        args.enable_energy_sales,
    )?;

    Ok(())
}

// Modified to accept a seed parameter
fn initialize_map(map: &mut Map, seed: Option<u64>) {
    let _timing = logging::start_timing("initialize_map", 
        OperationCategory::FileIO { subcategory: FileIOType::DataLoad });
    
    // Create a deterministic RNG if seed is provided
    let mut seeded_rng = seed.map(StdRng::seed_from_u64);
    
    // Load settlements
    match settlements_loader::load_settlements("mapData/sourceData/settlements.json", SIMULATION_START_YEAR) {
        Ok(settlements) => {
            for settlement in settlements {
                map.add_settlement(settlement);
            }
        },
        Err(e) => {
            eprintln!("Failed to load settlements from JSON: {}. Using fallback settlements.", e);
            map.add_settlement(Settlement::new(
                "Dublin".to_string(),
                Coordinate::new(70000.0, 70000.0),
                1_200_000,
                2000.0,
            ));
            map.add_settlement(Settlement::new(
                "Cork".to_string(),
                Coordinate::new(50000.0, 30000.0),
                190_000,
                350.0,
            ));
            map.add_settlement(Settlement::new(
                "Galway".to_string(),
                Coordinate::new(20000.0, 60000.0),
                80_000,
                150.0,
            ));
            map.add_settlement(Settlement::new(
                "Limerick".to_string(),
                Coordinate::new(30000.0, 40000.0),
                94_000,
                180.0,
            ));
        }
    }
    
    // Load existing generators from CSV, with deterministic fallbacks if needed
    match generators_loader::load_generators("src/ireland_generators.csv", SIMULATION_START_YEAR) {
        Ok(loaded_generators) => {
            let num_generators = loaded_generators.len();
            for generator in loaded_generators {
                map.add_generator(generator.clone());  // Clone each generator before adding
            }
            println!("Successfully loaded {} generators from CSV", num_generators);
        },
        Err(e) => {
            eprintln!("Failed to load generators from CSV: {}. Using fallback generators.", e);
            
            // When using a seed, we can generate deterministic locations instead of fixed ones
            if let Some(rng) = &mut seeded_rng {
                // Use seeded RNG for deterministic but varied placement
                let x1 = rng.gen_range(20000.0..40000.0);
                let y1 = rng.gen_range(40000.0..60000.0);
                
                map.add_generator(Generator::new(
                    "Moneypoint".to_string(),
                    Coordinate::new(x1, y1),
                    GeneratorType::CoalPlant,
                    800_000_000.0,
                    915.0,
                    50_000_000.0,
                    40,
                    1.0,
                    2_000_000.0,
                    0.37,
                ));
                
                let x2 = rng.gen_range(65000.0..75000.0);
                let y2 = rng.gen_range(65000.0..75000.0);
                
                map.add_generator(Generator::new(
                    "Dublin Bay".to_string(),
                    Coordinate::new(x2, y2),
                    GeneratorType::GasCombinedCycle,
                    400_000_000.0,
                    415.0,
                    20_000_000.0,
                    30,
                    0.8,
                    800_000.0,
                    0.45,
                ));
            } else {
                // No seed, use fixed positions
                map.add_generator(Generator::new(
                    "Moneypoint".to_string(),
                    Coordinate::new(30000.0, 50000.0),
                    GeneratorType::CoalPlant,
                    800_000_000.0,
                    915.0,
                    50_000_000.0,
                    40,
                    1.0,
                    2_000_000.0,
                    0.37,
                ));
                
                map.add_generator(Generator::new(
                    "Dublin Bay".to_string(),
                    Coordinate::new(72000.0, 72000.0),
                    GeneratorType::GasCombinedCycle,
                    400_000_000.0,
                    415.0,
                    20_000_000.0,
                    30,
                    0.8,
                    800_000.0,
                    0.45,
                ));
            }
        }
    }
}

// Function to run a simulation that replays the best actions from a previous run
fn run_simulation_with_best_actions(
    map: &mut Map, 
    weights: &mut ActionWeights,
    seed: Option<u64>,
    verbose_logging: bool,
    optimization_mode: Option<&str>,
    enable_energy_sales: bool,
) -> Result<(String, Vec<(u32, GridAction)>, Vec<YearlyMetrics>), Box<dyn Error + Send + Sync>> {
    let _timing = logging::start_timing("run_simulation_with_best_actions", OperationCategory::Simulation);

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
    
    println!("\nReplaying best strategy from previous runs with 100% probability");
    
    for year in SIMULATION_START_YEAR..=SIMULATION_END_YEAR {
        let _year_timing = logging::start_timing(&format!("simulate_year_{}", year), OperationCategory::Simulation);
        
        // Update population for each settlement based on the current year
        if year > SIMULATION_START_YEAR {
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
        
        // Calculate current state before actions
        let current_state = {
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
                apply_action(map, action, year)?;
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
                apply_action(map, &action, year)?;
                recorded_actions.push((year, action.clone()));
            }
        }

        // Verify power balance after actions and handle any remaining deficit
        let post_action_state = {
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
        let metrics = calculate_yearly_metrics(map, year, total_upgrade_costs, total_closure_costs, enable_energy_sales);
        yearly_metrics_collection.push(metrics.clone());
        
        print_yearly_summary(&metrics);
        
        // Save the final year metrics
        if year == SIMULATION_END_YEAR {
            final_year_metrics = Some(metrics);
        }
    }

    if let Some(metrics) = final_year_metrics {
        print_generator_details(&metrics);
        
        
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

// Fix the helper function for converting SimulationMetrics to ActionResult
fn metrics_to_action_result(metrics: &SimulationMetrics) -> ActionResult {
    ActionResult {
        net_emissions: metrics.final_net_emissions,
        public_opinion: metrics.average_public_opinion,
        power_balance: 0.0, // Not directly available in SimulationMetrics
        total_cost: metrics.total_cost,
    }
}

// Add a conversion function for YearlyMetrics
fn convert_yearly_metrics(metrics: &[YearlyMetrics]) -> Vec<csv_export::YearlyMetrics> {
    metrics.iter().map(|m| csv_export::YearlyMetrics {
        year: m.year,
        total_population: m.total_population,
        total_power_usage: m.total_power_usage,
        total_power_generation: m.total_power_generation,
        power_balance: m.power_balance,
        average_public_opinion: m.average_public_opinion,
        yearly_capital_cost: m.yearly_capital_cost,
        total_capital_cost: m.total_capital_cost,
        inflation_factor: m.inflation_factor,
        total_co2_emissions: m.total_co2_emissions,
        total_carbon_offset: m.total_carbon_offset,
        net_co2_emissions: m.net_co2_emissions,
        yearly_carbon_credit_revenue: m.yearly_carbon_credit_revenue,
        total_carbon_credit_revenue: m.total_carbon_credit_revenue,
        yearly_energy_sales_revenue: m.yearly_energy_sales_revenue,
        total_energy_sales_revenue: m.total_energy_sales_revenue,
        generator_efficiencies: m.generator_efficiencies.clone(),
        generator_operations: m.generator_operations.clone(),
        active_generators: m.active_generators,
        yearly_upgrade_costs: m.yearly_upgrade_costs,
        yearly_closure_costs: m.yearly_closure_costs,
        yearly_total_cost: m.yearly_total_cost,
        total_cost: m.total_cost,
    }).collect()
}

// Add a new function to handle deficit fallback
fn handle_power_deficit_fallback(
    map: &mut Map,
    deficit: f64,
    year: u32,
    weights: &mut ActionWeights,
    optimization_mode: Option<&str>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("  Taking fallback actions to handle power deficit of {:.2} MW", deficit);
    
    // Take a small number of actions focused on power generation
    let action_count = 2; // Fixed small number of actions for stability
    
    // Prioritize actions that generate power quickly
    let fallback_actions = vec![
        GridAction::AddGenerator(GeneratorType::GasPeaker),
        GridAction::AddGenerator(GeneratorType::BatteryStorage),
        GridAction::AddGenerator(GeneratorType::UtilitySolar),
        GridAction::AddGenerator(GeneratorType::OnshoreWind),
    ];
    
    for _ in 0..action_count {
        let action = fallback_actions[rand::thread_rng().gen_range(0..fallback_actions.len())].clone();
        
        // Apply the action and get the impact
        let result = apply_action(map, &action, year)?;
        
        // Log the result
        println!("  Applied deficit fallback action: {:?}", action);
        
        // Record this as a deficit action
        weights.record_deficit_action(year, action);
    }
    
    Ok(())
}
