#[macro_use]
extern crate lazy_static;

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
pub mod logging;

use std::fs::File;
use std::io::Write;
use chrono::Local;
use rand::Rng;
use crate::action_weights::{SimulationMetrics, score_metrics};
use std::error::Error;
use crate::const_funcs::calc_inflation_factor;
use std::str::FromStr;
use rayon::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::path::Path;
use clap::Parser;
use parking_lot::RwLock;
use std::collections::HashMap;
use crate::logging::{
    OperationCategory, PowerCalcType, WeightsUpdateType, FileIOType
};

pub use poi::{POI, Coordinate};
pub use generator::{Generator, GeneratorType};
pub use settlement::Settlement;
pub use map_handler::Map;
pub use carbon_offset::{CarbonOffset, CarbonOffsetType};
pub use simulation_config::SimulationConfig;
pub use action_weights::{ActionWeights, GridAction, ActionResult, evaluate_action_impact};
pub use constants::*;

const SIMULATION_START_YEAR: u32 = 2025;
const SIMULATION_END_YEAR: u32 = 2050;

#[derive(Debug, Clone)]
struct YearlyMetrics {
    year: u32,
    total_population: u32,
    total_power_usage: f64,
    total_power_generation: f64,
    power_balance: f64,
    average_public_opinion: f64,
    total_operating_cost: f64,
    total_capital_cost: f64,
    inflation_factor: f64,
    total_co2_emissions: f64,
    total_carbon_offset: f64,
    net_co2_emissions: f64,
    generator_efficiencies: Vec<(String, f64)>,
    generator_operations: Vec<(String, f64)>,
    active_generators: usize,
    upgrade_costs: f64,
    closure_costs: f64,
    total_cost: f64,
}

struct SimulationResult {
    metrics: SimulationMetrics,
    output: String,
    actions: Vec<(u32, GridAction)>,
}

fn run_simulation(
    map: &mut Map,
    mut action_weights: Option<&mut ActionWeights>,
) -> Result<(String, Vec<(u32, GridAction)>), Box<dyn Error + Send + Sync>> {
    let _timing = logging::start_timing("run_simulation", OperationCategory::Simulation);
    
    let mut output = String::new();
    let mut recorded_actions = Vec::new();
    
    let total_upgrade_costs = 0.0;
    let total_closure_costs = 0.0;
    
    let mut local_weights = match action_weights.as_deref() {
        Some(weights) => weights.clone(),
        None => ActionWeights::new(),
    };

    let mut final_year_metrics: Option<YearlyMetrics> = None;

    for year in SIMULATION_START_YEAR..=SIMULATION_END_YEAR {
        let _year_timing = logging::start_timing(&format!("simulate_year_{}", year), OperationCategory::Simulation);
        
        if action_weights.is_none() {
            println!("\nStarting year {}", year);
            
            if year > SIMULATION_START_YEAR {
                local_weights.print_top_actions(year - 1, 5);
            }
        }
        
        let current_state = {
            let _timing = logging::start_timing("calculate_current_state", 
                OperationCategory::PowerCalculation { subcategory: PowerCalcType::Balance });
            let net_emissions = map.calc_net_co2_emissions(year);
            let public_opinion = calculate_average_opinion(map, year);
            let power_balance = map.calc_total_power_generation(year, None) - map.calc_total_power_usage(year);
            ActionResult {
                net_emissions,
                public_opinion,
                power_balance,
            }
        };

        if current_state.power_balance < 0.0 {
            let _timing = logging::start_timing("handle_power_deficit", 
                OperationCategory::PowerCalculation { subcategory: PowerCalcType::Balance });
            handle_power_deficit(map, -current_state.power_balance, year, &mut local_weights)?;
        }

        let mut rng = rand::thread_rng();
        let num_additional_actions = if action_weights.is_some() {
            local_weights.sample_additional_actions(year)
        } else {
            rng.gen_range(0..=20)
        };

        for _ in 0..num_additional_actions {
            let _timing = logging::start_timing("apply_additional_action", OperationCategory::Simulation);
            let action = local_weights.sample_action(year);
            apply_action(map, &action, year)?;
            recorded_actions.push((year, action));
        }

        if map.calc_total_power_generation(year, None) > map.calc_total_power_usage(year) {
            let mut worst_ratio = -1.0;
            let mut worst_generator_id: Option<String> = None;
            for generator in map.get_generators() {
                if generator.is_active() {
                    let power = generator.get_current_power_output(None);
                    if power > 0.0 {
                        let ratio = generator.get_co2_output() / power;
                        if ratio > worst_ratio {
                            worst_ratio = ratio;
                            worst_generator_id = Some(generator.get_id().to_string());
                        }
                    }
                }
            }
            if let Some(id) = worst_generator_id {
                let adjust_action = GridAction::AdjustOperation(id, 80);
                apply_action(map, &adjust_action, year)?;
                recorded_actions.push((year, adjust_action));
            }
        }

        let metrics = calculate_yearly_metrics(map, year, total_upgrade_costs, total_closure_costs);
        if year == SIMULATION_END_YEAR {
            final_year_metrics = Some(metrics.clone());
        }
        
        // Calculate operating costs only for CSV export
        let operating_cost_for_csv = map.calc_total_operating_cost(year);
        
        output.push_str(&format!(
            "{},{},{:.2},{:.2},{:.2},{:.3},{:.2},{:.2},{:.3},{:.2},{:.2},{:.2},{},{:.2},{:.2},{:.2}\n",
            metrics.year,
            metrics.total_population,
            metrics.total_power_usage,
            metrics.total_power_generation,
            metrics.power_balance,
            metrics.average_public_opinion,
            operating_cost_for_csv,
            metrics.total_capital_cost,
            metrics.inflation_factor,
            metrics.total_co2_emissions,
            metrics.total_carbon_offset,
            metrics.net_co2_emissions,
            metrics.active_generators,
            metrics.upgrade_costs,
            metrics.closure_costs,
            metrics.total_cost
        ));

        if action_weights.is_none() {
            print_yearly_summary(&metrics);
            print_generator_details(&metrics);
        }
        
        if let Some(weights) = action_weights.as_deref_mut() {
            weights.update_weights_from(&local_weights);
        }
    }

    if let Some(final_metrics) = final_year_metrics.clone() {
        let sim_metrics = SimulationMetrics {
            final_net_emissions: final_metrics.net_co2_emissions,
            average_public_opinion: final_metrics.average_public_opinion,
            total_cost: final_metrics.total_cost,
            power_reliability: if final_metrics.power_balance >= 0.0 { 1.0 } else { 0.0 },
        };
        let overall_improvement = score_metrics(&sim_metrics);
        
        // Group actions by year and count them
        let mut year_action_counts: HashMap<u32, u32> = HashMap::new();
        for (year, _) in &recorded_actions {
            *year_action_counts.entry(*year).or_insert(0) += 1;
        }
        
        // Update weights for both actions and action counts
        for (year, action) in recorded_actions.clone() {
            local_weights.update_weights(&action, year, overall_improvement);
            if let Some(count) = year_action_counts.get(&year) {
                local_weights.update_action_count_weights(year, *count, overall_improvement);
            }
        }
    }

    Ok((output, recorded_actions))
}

fn handle_power_deficit(
    map: &mut Map,
    deficit: f64,
    year: u32,
    action_weights: &mut ActionWeights,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let _timing = logging::start_timing("handle_power_deficit", 
        OperationCategory::PowerCalculation { subcategory: PowerCalcType::Balance });
    
    let mut remaining_deficit = deficit;

    while remaining_deficit > 0.0 {
        let action = loop {
            let _timing = logging::start_timing("sample_generator_action", 
                OperationCategory::WeightsUpdate { subcategory: WeightsUpdateType::ActionUpdate });
            let candidate = action_weights.sample_action(year);
            if let GridAction::AddGenerator(_) = candidate {
                break candidate;
            }
        };

        let current_state = {
            let _timing = logging::start_timing("calculate_current_state", 
                OperationCategory::PowerCalculation { subcategory: PowerCalcType::Balance });
            let net_emissions = map.calc_net_co2_emissions(year);
            let public_opinion = calculate_average_opinion(map, year);
            let power_balance = map.calc_total_power_generation(year, None) - map.calc_total_power_usage(year);
            ActionResult {
                net_emissions,
                public_opinion,
                power_balance,
            }
        };

        if let GridAction::AddGenerator(_) = action {
            let _timing = logging::start_timing("apply_generator_action", 
                OperationCategory::Simulation);
            apply_action(map, &action, year)?;

            let new_state = {
                let _timing = logging::start_timing("calculate_new_state", 
                    OperationCategory::PowerCalculation { subcategory: PowerCalcType::Balance });
                let net_emissions = map.calc_net_co2_emissions(year);
                let public_opinion = calculate_average_opinion(map, year);
                let power_balance = map.calc_total_power_generation(year, None) - map.calc_total_power_usage(year);
                ActionResult {
                    net_emissions,
                    public_opinion,
                    power_balance,
                }
            };

            let improvement = evaluate_action_impact(&current_state, &new_state);
            
            {
                let _timing = logging::start_timing("update_weights", 
                    OperationCategory::WeightsUpdate { subcategory: WeightsUpdateType::ActionUpdate });
                action_weights.update_weights(&action, year, improvement);
            }

            remaining_deficit = -new_state.power_balance.min(0.0);
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
                CarbonOffsetType::from_str(offset_type)?,
                match CarbonOffsetType::from_str(offset_type)? {
                    CarbonOffsetType::Forest => FOREST_BASE_COST,
                    CarbonOffsetType::Wetland => WETLAND_BASE_COST,
                    CarbonOffsetType::ActiveCapture => ACTIVE_CAPTURE_BASE_COST,
                    CarbonOffsetType::CarbonCredit => CARBON_CREDIT_BASE_COST,
                },
                match CarbonOffsetType::from_str(offset_type)? {
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
                        GeneratorType::Nuclear => NUCLEAR_MIN_CLOSURE_AGE,
                        GeneratorType::HydroDam => HYDRO_DAM_MIN_CLOSURE_AGE,
                        GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => WIND_MIN_CLOSURE_AGE,
                        GeneratorType::UtilitySolar => SOLAR_MIN_CLOSURE_AGE,
                        _ => DEFAULT_MIN_CLOSURE_AGE,
                    };
                    
                    if age >= min_age {
                        generator.close_generator(year);
                    }
                }
            }
            map.after_generator_modification();
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

fn calculate_yearly_metrics(map: &Map, year: u32, total_upgrade_costs: f64, total_closure_costs: f64) -> YearlyMetrics {
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
                generator_operations.push((generator.get_id().to_string(), generator.get_operation_percentage() as f64));
            }
        }
    }

    let total_operating_cost = 0.0; // We'll only calculate this when needed for CSV export
    let total_capital_cost = map.calc_total_capital_cost(year);
    let inflation_factor = calc_inflation_factor(year);
    let total_cost = total_capital_cost;

    YearlyMetrics {
        year,
        total_population: total_pop,
        total_power_usage,
        total_power_generation: total_power_gen,
        power_balance,
        average_public_opinion: if opinion_count > 0 { total_opinion / opinion_count as f64 } else { 1.0 },
        total_operating_cost,
        total_capital_cost,
        inflation_factor,
        total_co2_emissions,
        total_carbon_offset,
        net_co2_emissions,
        generator_efficiencies,
        generator_operations,
        active_generators: active_count,
        upgrade_costs: total_upgrade_costs,
        closure_costs: total_closure_costs,
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
    println!("  Operating Cost: €{:.2}", metrics.total_operating_cost);
    println!("  Capital Cost: €{:.2}", metrics.total_capital_cost);
    println!("  Upgrade Costs: €{:.2}", metrics.upgrade_costs);
    println!("  Closure Costs: €{:.2}", metrics.closure_costs);
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
                id, efficiency, operation * 100.0);
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
                        println!("Loaded and merged weights with best score: {:.4}", best_score);
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
                        "✓ Net Zero Achieved".to_string()
                    } else {
                        format!("⚠ {:.1}% above target", 
                            (best.final_net_emissions / MAX_ACCEPTABLE_EMISSIONS) * 100.0)
                    };
                    
                    let cost_status = if best.total_cost <= MAX_ACCEPTABLE_COST {
                        "✓ Within Budget".to_string()
                    } else {
                        format!("⚠ {:.1}% over budget", 
                            ((best.total_cost - MAX_ACCEPTABLE_COST) / MAX_ACCEPTABLE_COST) * 100.0)
                    };
                    
                    format!("\nMetrics Status:\n\
                            - Emissions: {} ({:.1} tonnes)\n\
                            - Cost: {} (€{:.1}B/year)\n\
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
                    "\nProgress Update:\n\
                    ----------------------------------------\n\
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
                    - [NET ZERO]: Achieved net zero, score is now public opinion",
                    completed,
                    total_iterations,
                    (completed as f64 / total_iterations as f64) * 100.0,
                    iterations_per_second,
                    eta_seconds / 60.0,
                    best_score,
                    if is_net_zero { "[NET ZERO]" } else { "" },
                    metrics_info
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

                    // Set simulation mode based on iteration number
                    let final_full_sim_count = num_iterations.min(20);
                    let use_fast = !force_full_simulation && 
                                 cache_loaded && 
                                 i < num_iterations.saturating_sub(final_full_sim_count);
                    map_clone.set_simulation_mode(use_fast);
                    
                    if i == num_iterations.saturating_sub(final_full_sim_count) {
                        println!("\nSwitching to full simulation mode for final {} iterations", final_full_sim_count);
                    }
                    
                    // Create local weights and immediately drop the read lock
                    let mut local_weights = {
                        let weights = action_weights.read();
                        weights.clone()
                    }; // Read lock is dropped here
                    
                    let result = run_iteration(i, map_clone, &mut local_weights)?;
                    
                    // Update best metrics immediately
                    {
                        let mut weights = parking_lot::RwLock::write(&*action_weights);
                        weights.update_best_strategy(result.metrics.clone());
                    }
                    
                    // Only acquire write lock periodically to merge weights
                    if (i + 1) % 10 == 0 {
                        let mut weights = parking_lot::RwLock::write(&*action_weights);
                        weights.update_weights_from(&local_weights);
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
                    
                    Ok(result)
                })
                .collect::<Result<Vec<_>, _>>()?;
            
            for result in results {
                if best_result.as_ref().map_or(true, |best| {
                    score_metrics(&result.metrics) > score_metrics(&best.metrics)
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

                // Set simulation mode based on iteration number
                let final_full_sim_count = num_iterations.min(20);
                let use_fast = !force_full_simulation && 
                             cache_loaded && 
                             i < num_iterations.saturating_sub(final_full_sim_count);
                map_clone.set_simulation_mode(use_fast);
                
                if i == num_iterations.saturating_sub(final_full_sim_count) {
                    println!("\nSwitching to full simulation mode for final {} iterations", final_full_sim_count);
                }
                
                // Get a write lock to update the shared weights
                let mut weights = parking_lot::RwLock::write(&*action_weights);
                let result = run_iteration(i, map_clone, &mut weights)?;
                
                // Increment completed iterations counter
                completed_iterations.fetch_add(1, Ordering::Relaxed);
                
                if best_result.as_ref().map_or(true, |best| {
                    score_metrics(&result.metrics) > score_metrics(&best.metrics)
                }) {
                    best_result = Some(result);
                }
                
                // Save checkpoint at intervals
                if (i + 1) % checkpoint_interval == 0 {
                    let checkpoint_path = Path::new(&run_dir).join("latest_weights.json");
                    weights.save_to_file(checkpoint_path.to_str().unwrap())?;
                    
                    // Save iteration number
                    let iteration_path = Path::new(&run_dir).join("checkpoint_iteration.txt");
                    std::fs::write(iteration_path, (i + 1).to_string())?;
                    
                    println!("Saved checkpoint at iteration {} in {}", i + 1, run_dir);
                }
            }
        }
        
        if let Some(best) = best_result {
            println!("\nBest simulation results:");
            println!("Final net emissions: {:.2} tonnes", best.metrics.final_net_emissions);
            println!("Average public opinion: {:.3}", best.metrics.average_public_opinion);
            println!("Total cost: €{:.2}", best.metrics.total_cost);
            println!("Power reliability: {:.3}", best.metrics.power_reliability);
            
            // Save final results in the run directory
            let csv_filename = Path::new(&run_dir).join("best_simulation.csv");
            let mut file = File::create(&csv_filename)?;
            file.write_all(best.output.as_bytes())?;
            println!("\nBest simulation results saved to: {}", csv_filename.display());
            
            // Save actions to a separate CSV file with enhanced spatial information
            let actions_filename = Path::new(&run_dir).join("best_simulation_actions.csv");
            let mut actions_file = File::create(&actions_filename)?;
            actions_file.write_all(
                "Year,Action Type,Details,Capital Cost (EUR),Operating Cost (EUR),\
                Location X,Location Y,Generator Type,Power Output (MW),Efficiency,\
                CO2 Output (tonnes),Operation Percentage,Lifespan (years),\
                Previous State,Impact Description\n".as_bytes()
            )?;
            
            for (year, action) in best.actions {
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
                            gen_type.get_base_cost(year),
                            gen_type.get_operating_cost(year),
                            location.0,
                            location.1,
                            gen_type.to_string(),
                            gen_type.get_base_power(year),
                            gen_type.get_base_efficiency(year),
                            match gen_type {
                                GeneratorType::CoalPlant => 1000.0,
                                GeneratorType::GasCombinedCycle => 500.0,
                                GeneratorType::GasPeaker => 700.0,
                                GeneratorType::Biomass => 50.0,
                                _ => 0.0,
                            },
                            100i32, // New generators start at 100%
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
                            let upgrade_cost = gen.get_current_cost(year) * (max_efficiency - gen.get_efficiency()) * 2.0;
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
                                percentage as i32,
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
                            100i32, // Always fully operational
                            30, // Standard offset lifespan
                            "New Offset".to_string(),
                            format!("Added new {} carbon offset project", offset_type.to_string())
                        )
                    },
                    GridAction::CloseGenerator(id) => {
                        let generator = base_map.get_generators().iter().find(|g| g.get_id() == id);
                        if let Some(gen) = generator {
                            let years_remaining = (gen.eol as i32 - (year - 2025) as i32).max(0) as f64;
                            let closure_cost = gen.get_current_cost(year) * 0.5 * (years_remaining / gen.eol as f64);
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
            println!("Enhanced action history with spatial data saved to: {}", actions_filename.display());
            
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
    mut map: Map,
    mut weights: &mut ActionWeights,
) -> Result<SimulationResult, Box<dyn Error + Send + Sync>> {
    let _timing = logging::start_timing("run_iteration", 
        OperationCategory::Simulation);
    
    let result = (|| {
        println!("\nStarting iteration {}", iteration + 1);
        weights.start_new_iteration();

        
        let (simulation_output, recorded_actions) = run_simulation(&mut map, Some(&mut weights))?;
        
        // Calculate final metrics properly
        let final_emissions = map.calc_net_co2_emissions(2050);
        let final_opinion = calculate_average_opinion(&map, 2050);
        let final_power_gen = map.calc_total_power_generation(2050, None);
        let final_power_usage = map.calc_total_power_usage(2050);
        let power_deficit = if final_power_gen < final_power_usage {
            (final_power_usage - final_power_gen) / final_power_usage
        } else {
            0.0
        };
        
        let capital_cost = map.calc_total_capital_cost(2050);
        let total_cost = capital_cost;  // Only use capital costs for budget
        
        let final_metrics = SimulationMetrics {
            final_net_emissions: final_emissions,
            average_public_opinion: final_opinion,
            total_cost,
            power_reliability: 1.0 - power_deficit,
        };
        
        weights.update_best_strategy(final_metrics.clone());
        
        Ok(SimulationResult {
            metrics: final_metrics,
            output: simulation_output,
            actions: recorded_actions,
        })
    })();
    result
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = Args::parse();
    
    // Initialize logging with timing enabled/disabled based on command line arg
    logging::init_logging(args.enable_timing);
    
    println!("EirGrid Power System Simulator (2025-2050)");
    println!("Debug: no_continue = {}, continue_from_checkpoint = {}", args.no_continue, !args.no_continue);
    
    let config = SimulationConfig::default();
    let mut map = Map::new(config);
    
    initialize_map(&mut map);
    
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
    )?;
    
    Ok(())
}

fn initialize_map(map: &mut Map) {
    let _timing = logging::start_timing("initialize_map", 
        OperationCategory::FileIO { subcategory: FileIOType::DataLoad });
    
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

    // Load existing generators from CSV
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
            // Add fallback generators
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

