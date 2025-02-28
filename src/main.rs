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
    yearly_energy_sales_revenue: f64,  // Revenue from energy sales for the current year
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
    energy_sales_enabled: bool,
    energy_sales_rate: f64,
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
    
    // Year 2025 
    println!("\nYear 2025");
    println!("--------------------------");
    
    // Calculate and print initial state metrics
    let yearly_metrics = calculate_yearly_metrics(map, 2025, total_upgrade_costs, total_closure_costs, energy_sales_enabled, energy_sales_rate);
    yearly_metrics_collection.push(yearly_metrics.clone()); // collect metrics
    print_yearly_summary(&yearly_metrics);
    print_generator_details(&yearly_metrics);
    
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
            handle_power_deficit(map, -current_state.power_balance, year, &mut local_weights)?;
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
        let yearly_metrics = calculate_yearly_metrics(map, year, total_upgrade_costs, total_closure_costs, energy_sales_enabled, energy_sales_rate);
        
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
        let yearly_energy_sales = metrics.yearly_energy_sales_revenue;
        let total_energy_sales = metrics.total_energy_sales_revenue;
        
        let mut output_str = format!(
            "Population: {}\nPower usage: {:.2} MW\nPower generation: {:.2} MW\nPower balance: {:.2} MW\nPublic opinion: {:.2}%\nYearly Capital Cost: €{:.2}\nTotal Capital Cost: €{:.2}\nNet emissions: {:.2} tonnes",
            population, power_usage, power_generation, power_balance, 
            public_opinion * 100.0, yearly_capital_cost, total_capital_cost, net_emissions
        );
        
        // Add energy sales revenue if enabled
        if metrics.yearly_energy_sales_revenue > 0.0 {
            output_str.push_str(&format!(
                "\nYearly Energy Sales Revenue: €{:.2}\nTotal Energy Sales Revenue: €{:.2}",
                yearly_energy_sales, total_energy_sales
            ));
        }
        
        output = output_str;
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
            let overall_improvement = evaluate_action_impact(&current_state, &new_state);
            
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
    let overall_success = evaluate_action_impact(&initial_state, &final_state);
    
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

fn calculate_yearly_metrics(map: &Map, year: u32, total_upgrade_costs: f64, total_closure_costs: f64, energy_sales_enabled: bool, energy_sales_rate: f64) -> YearlyMetrics {
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

    // Calculate revenue from energy sales if enabled
    let energy_sales_revenue = if energy_sales_enabled {
        const_funcs::calculate_energy_sales_revenue(power_balance, year, energy_sales_rate)
    } else {
        0.0
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
    
    // Calculate yearly and accumulated costs, offsetting with revenues
    let yearly_total_cost = yearly_capital_cost + total_upgrade_costs + total_closure_costs - carbon_credit_revenue - energy_sales_revenue;
    let total_cost = total_capital_cost + total_upgrade_costs + total_closure_costs - carbon_credit_revenue - energy_sales_revenue;
    
    // Track yearly and accumulated revenues
    let yearly_carbon_credit_revenue = carbon_credit_revenue;
    let total_carbon_credit_revenue = carbon_credit_revenue;
    let yearly_energy_sales_revenue = energy_sales_revenue;
    let total_energy_sales_revenue = energy_sales_revenue;

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
        yearly_carbon_credit_revenue,
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
    println!("  Yearly Total Cost: €{:.2}", metrics.yearly_total_cost);
    println!("  Accumulated Total Cost: €{:.2}", metrics.total_cost);
    if metrics.yearly_carbon_credit_revenue > 0.0 {
        println!("  Yearly Carbon Credit Revenue: €{:.2}", metrics.yearly_carbon_credit_revenue);
        println!("  Total Carbon Credit Revenue: €{:.2}", metrics.total_carbon_credit_revenue);
    }
    if metrics.yearly_energy_sales_revenue > 0.0 {
        println!("  Yearly Energy Sales Revenue: €{:.2}", metrics.yearly_energy_sales_revenue);
        println!("  Total Energy Sales Revenue: €{:.2}", metrics.total_energy_sales_revenue);
    }
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

    #[arg(long, default_value_t = false, help = "Enable selling of energy surplus")]
    enable_energy_sales: bool,

    #[arg(long, default_value_t = 50000.0, help = "Rate for energy sales in euros per GWh")]
    energy_sales_rate: f64,
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
    energy_sales_enabled: bool,
    energy_sales_rate: f64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Running multi-simulation with energy sales enabled: {}, rate: {}", 
             energy_sales_enabled, energy_sales_rate);
    
    // This is a placeholder implementation
    // The real implementation would use these parameters
    Ok(())
}

fn initialize_map(map: &mut Map, seed: Option<u64>) {
    println!("Initializing map with seed: {:?}", seed);
    // This is a placeholder implementation
}

fn run_iteration(
    iteration: usize,
    map: &mut Map,
    weights: &mut ActionWeights,
    replay_best_strategy: bool,
    seed: Option<u64>,
    verbose_logging: bool,
    energy_sales_enabled: bool,
    energy_sales_rate: f64,
) -> Result<SimulationResult, Box<dyn Error + Send + Sync>> {
    println!("Running iteration {} with energy sales enabled: {}, rate: {}", 
             iteration, energy_sales_enabled, energy_sales_rate);
    
    // This is a placeholder implementation
    // In a real implementation, this would calculate metrics and return them
    Err("Not implemented".into())
}

fn handle_power_deficit_fallback(
    map: &mut Map,
    deficit: f64,
    year: u32,
    weights: &mut ActionWeights,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Handling power deficit of {} in year {}", deficit, year);
    
    // Taking fallback actions to handle power deficit
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
        apply_action(map, &action, year)?;
        
        // Log the result
        println!("  Applied deficit fallback action: {:?}", action);
        
        // Record this as a deficit action
        weights.record_deficit_action(year, action);
    }
    
    Ok(())
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
        args.enable_energy_sales,
        args.energy_sales_rate,
    )?;

    Ok(())
}

