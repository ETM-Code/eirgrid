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
mod metrics;
mod simulation;
mod actions;
mod metrics_calculation;
mod reporting;
mod cli;
mod multi_simulation;
mod iteration;

// Import specific items from modules
use crate::map_handler::Map;
use crate::generator::{Generator, GeneratorType};
use crate::action_weights::{ActionWeights, GridAction, SimulationMetrics, evaluate_action_impact, ActionResult, score_metrics};
use crate::constants::*;
use crate::logging::{OperationCategory, FileIOType, PowerCalcType, WeightsUpdateType};
use crate::csv_export::CsvExporter;
use crate::carbon_offset::{CarbonOffset, CarbonOffsetType};
use crate::metrics::{YearlyMetrics, SimulationResult};
use crate::cli::Args;
use crate::simulation::{run_simulation, handle_power_deficit};
use crate::metrics_calculation::{calculate_yearly_metrics, calculate_average_opinion};
use crate::reporting::{print_yearly_summary, print_generator_details};
use crate::multi_simulation::run_multi_simulation;
use crate::iteration::run_iteration;
use crate::actions::apply_action;

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

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Parse command line arguments
    let args = Args::parse();
     
    // Initialize logging with timing enabled/disabled based on command line arg
    logging::init_logging(args.enable_timing());
     
    println!("EirGrid Power System Simulator (2025-2050)");
    println!("Debug: no_continue = {}, continue_from_checkpoint = {}", args.no_continue(), !args.no_continue());
     
    let config = SimulationConfig::default();
    let mut map = Map::new(config);
     
    // Initialize the map, now with seed support
    initialize_map(&mut map, args.seed());
     
    run_multi_simulation(
        &map,
        args.iterations(),
        args.parallel(),
        !args.no_continue(),
        args.checkpoint_dir(),
        args.checkpoint_interval(),
        args.progress_interval(),
        args.cache_dir(),
        args.force_full_simulation(),
        args.seed(),
        args.verbose_state_logging(),
        if args.cost_only() { Some("cost_only") } else { None },
        args.enable_energy_sales(),
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
        // Get the previous year's metrics if available
        let previous_metrics = if year > SIMULATION_START_YEAR {
            yearly_metrics_collection.last()
        } else {
            None
        };
        
        let metrics = calculate_yearly_metrics(
            map, 
            year, 
            total_upgrade_costs, 
            total_closure_costs, 
            enable_energy_sales,
            previous_metrics
        );
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