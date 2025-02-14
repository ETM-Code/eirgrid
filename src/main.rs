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

use std::fs::File;
use std::io::Write;
use chrono::Local;
use rand::Rng;
use crate::action_weights::{SimulationMetrics, score_metrics};
use std::error::Error;
use crate::const_funcs::{calc_inflation_factor, is_valid_generator_location, evaluate_generator_location};
use std::str::FromStr;
use rayon::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use crate::map_handler::MapStaticData;
use crate::spatial_index::GeneratorSuitabilityType;
use std::path::Path;
use clap::{Parser, ArgAction};
use parking_lot::RwLock;

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
const NUM_ACTION_SAMPLES: usize = 100;

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

fn run_simulation(
    map: &mut Map,
    mut action_weights: Option<&mut ActionWeights>,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let mut output = String::new();
    output.push_str("Year,Total Population,Total Power Usage (MW),Total Power Generation (MW),\
                    Power Balance (MW),Average Public Opinion,Total Operating Cost (€),\
                    Total Capital Cost (€),Inflation Factor,Total CO2 Emissions (tons),\
                    Total Carbon Offset (tons),Net CO2 Emissions (tons),Active Generators,\
                    Upgrade Costs (€),Closure Costs (€),Total Cost (€)\n");

    let mut total_upgrade_costs = 0.0;
    let mut total_closure_costs = 0.0;
    
    let mut local_weights = match action_weights.as_deref() {
        Some(weights) => weights.clone(),
        None => ActionWeights::new(),
    };

    let mut recorded_actions: Vec<(u32, GridAction)> = Vec::new();
    let mut final_year_metrics: Option<YearlyMetrics> = None;

    for year in SIMULATION_START_YEAR..=SIMULATION_END_YEAR {
        if action_weights.is_none() {
            println!("\nStarting year {}", year);
            
            if year > SIMULATION_START_YEAR {
                local_weights.print_top_actions(year - 1, 5);
            }
        }
        
        let current_state = {
            let net_emissions = map.calc_net_co2_emissions(year);
            let public_opinion = calculate_average_opinion(map, year);
            let power_balance = map.calc_total_power_generation(year, None) - map.calc_total_power_usage(year);
            ActionResult {
                net_emissions,
                public_opinion,
                power_balance,
            }
        }; //NOTE: ANALYSE LATER

        if current_state.power_balance < 0.0 {
            handle_power_deficit(map, -current_state.power_balance, year, &mut local_weights)?;
        }

        let mut rng = rand::thread_rng();
        let num_additional_actions = if action_weights.is_some() {
            local_weights.sample_additional_actions(year)
        } else {
            rng.gen_range(0..=20)
        };

        for _ in 0..num_additional_actions {
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
        
        output.push_str(&format!(
            "{},{},{:.2},{:.2},{:.2},{:.3},{:.2},{:.2},{:.3},{:.2},{:.2},{:.2},{},{:.2},{:.2},{:.2}\n",
            metrics.year,
            metrics.total_population,
            metrics.total_power_usage,
            metrics.total_power_generation,
            metrics.power_balance,
            metrics.average_public_opinion,
            metrics.total_operating_cost,
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
        for (year, action) in recorded_actions {
            local_weights.update_weights(&action, year, overall_improvement);
        }
    }

    Ok(output)
}

fn handle_power_deficit(
    map: &mut Map,
    deficit: f64,
    year: u32,
    action_weights: &mut ActionWeights,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut remaining_deficit = deficit;

    // Continue trying until the deficit is remedied
    while remaining_deficit > 0.0 {
        // Force sampling until we get an AddGenerator action.
        let action = loop {
            let candidate = action_weights.sample_action(year);
            if let GridAction::AddGenerator(_) = candidate {
                break candidate;
            } else {
                // Optionally log that a non-AddGenerator action was skipped.
                // e.g., println!("Skipping non-generator action in deficit handling: {:?}", candidate);
                continue;
            }
        };

        let current_state = {
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
            // Try to apply the AddGenerator action
            apply_action(map, &action, year)?;

            let new_state = {
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
            action_weights.update_weights(&action, year, improvement);

            // Update the deficit: if new_state.power_balance is negative,
            // its minimum with zero (which is negative) is negated to a positive deficit amount.
            remaining_deficit = -new_state.power_balance.min(0.0);
        }
        // (Since the inner loop guarantees only AddGenerator actions are processed,
        // this branch is not needed and the loop will simply repeat until the deficit is fixed.)
    }
    Ok(())
}

fn find_best_generator_location(map: &Map, gen_type: &GeneratorType, gen_size: u8, year: u32) -> Option<Coordinate> {
    let suitability_type = match gen_type {
        GeneratorType::OnshoreWind => GeneratorSuitabilityType::Onshore,
        GeneratorType::OffshoreWind => GeneratorSuitabilityType::Offshore,
        GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => GeneratorSuitabilityType::Coastal,
        GeneratorType::Nuclear => {
            // Try multiple times with gradually reduced requirements
            for min_score in [0.5, 0.4, 0.3].iter() {
                for min_distance in [12000.0, 10000.0, 8000.0].iter() {
                    if let Some(location) = map.spatial_index.find_best_location(GeneratorSuitabilityType::Coastal, *min_score) {
                        if map.get_settlements().iter()
                            .filter(|s| s.get_population() > 100_000)
                            .all(|s| s.get_coordinate().distance_to(&location) >= *min_distance) {
                            println!("Found suitable location for Nuclear plant with score {} and min distance {}", min_score, min_distance);
                            return Some(location);
                        }
                    }
                }
            }
            println!("Could not find suitable location for Nuclear plant even with reduced requirements");
            return None;
        },
        GeneratorType::DomesticSolar | GeneratorType::CommercialSolar => GeneratorSuitabilityType::Urban,
        GeneratorType::HydroDam | GeneratorType::PumpedStorage => GeneratorSuitabilityType::Rural,
        _ => GeneratorSuitabilityType::Rural,
    };

    // Initial minimum scores
    let initial_min_score = match gen_type {
        GeneratorType::OnshoreWind => 0.15,  // Further reduced from 0.3
        GeneratorType::OffshoreWind => 0.3,  // Reduced from 0.35
        GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => 0.3,
        GeneratorType::Nuclear => 0.4,
        GeneratorType::DomesticSolar | GeneratorType::CommercialSolar => 0.2,
        GeneratorType::UtilitySolar => 0.25,
        GeneratorType::HydroDam | GeneratorType::PumpedStorage => 0.3,
        _ => 0.15,
    };

    // Try multiple times with gradually reduced requirements
    let size_factor = gen_size as f64 / 100.0;
    let reduction_steps = [1.0, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3]; // Added more steps
    
    for reduction in reduction_steps.iter() {
        let adjusted_min_score = (initial_min_score * reduction) * (1.0 + size_factor * 0.02); // Further reduced size penalty
        
        if let Some(location) = map.spatial_index.find_best_location(suitability_type, adjusted_min_score) {
            if reduction < &1.0 {
                println!("Found location for {:?} generator with {:.1}% of original requirements (score: {:.2}, size factor: {:.2})", 
                    gen_type, reduction * 100.0, adjusted_min_score, size_factor);
            }
            return Some(location);
        }
        
        println!("Failed to find location for {:?} at {:.1}% requirements (min score: {:.3})", 
            gen_type, reduction * 100.0, adjusted_min_score);
    }

    println!("Could not find suitable location for {:?} generator after trying all reduction steps", gen_type);
    None
}

fn apply_action(map: &mut Map, action: &GridAction, year: u32) -> Result<(), Box<dyn Error + Send + Sync>> {
    match action {
        GridAction::AddGenerator(gen_type) => {
            let gen_size = 100;
            match map.find_best_generator_location(gen_type, gen_size as f64 / 100.0) {
                Some(location) => {
                    let base_efficiency = gen_type.get_base_efficiency(year);
                    let initial_co2_output = match gen_type {
                        GeneratorType::CoalPlant => 1000.0,
                        GeneratorType::GasCombinedCycle => 500.0,
                        GeneratorType::GasPeaker => 700.0,
                        GeneratorType::Biomass => 50.0,
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
                        GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => 0.45,
                        GeneratorType::UtilitySolar => 0.40,
                        GeneratorType::Nuclear => 0.50,
                        GeneratorType::GasCombinedCycle => 0.60,
                        GeneratorType::HydroDam | GeneratorType::PumpedStorage => 0.85,
                        GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => 0.35,
                        _ => 0.40,
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
            let offset_size = rand::thread_rng().gen_range(100.0..1000.0);
            let base_efficiency = rand::thread_rng().gen_range(0.7..0.95);
            
            let location = Coordinate::new(
                rand::thread_rng().gen_range(0.0..MAP_MAX_X),
                rand::thread_rng().gen_range(0.0..MAP_MAX_Y),
            );
            
            let offset = CarbonOffset::new(
                format!("Offset_{}_{}_{}", offset_type, year, map.get_carbon_offset_count()),
                location,
                CarbonOffsetType::from_str(offset_type)?,
                match CarbonOffsetType::from_str(offset_type)? {
                    CarbonOffsetType::Forest => 10_000_000.0,
                    CarbonOffsetType::Wetland => 15_000_000.0,
                    CarbonOffsetType::ActiveCapture => 100_000_000.0,
                    CarbonOffsetType::CarbonCredit => 5_000_000.0,
                },
                match CarbonOffsetType::from_str(offset_type)? {
                    CarbonOffsetType::Forest => 500_000.0,
                    CarbonOffsetType::Wetland => 750_000.0,
                    CarbonOffsetType::ActiveCapture => 5_000_000.0,
                    CarbonOffsetType::CarbonCredit => 250_000.0,
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
                        GeneratorType::Nuclear => 30,
                        GeneratorType::HydroDam => 40,
                        GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => 15,
                        GeneratorType::UtilitySolar => 20,
                        _ => 25,
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
    let total_pop = map.calc_total_population(year);
    let total_power_usage = map.calc_total_power_usage(year);
    let total_power_gen = map.calc_total_power_generation(year, None);
    let power_balance = total_power_gen - total_power_usage;
    let total_co2_emissions = map.calc_total_co2_emissions();
    let total_carbon_offset = map.calc_total_carbon_offset(year);
    let net_co2_emissions = map.calc_net_co2_emissions(year);

    let mut total_opinion = 0.0;
    let mut opinion_count = 0;
    let mut generator_efficiencies = Vec::new();
    let mut generator_operations = Vec::new();
    let mut active_count = 0;

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

    let total_operating_cost = map.calc_total_operating_cost(year);
    let total_capital_cost = map.calc_total_capital_cost(year);
    let inflation_factor = calc_inflation_factor(year);
    let total_cost = total_operating_cost + total_capital_cost;

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
    println!("  CO2 Emissions: {:.2} tons", metrics.total_co2_emissions);
    println!("  Carbon Offset: {:.2} tons", metrics.total_carbon_offset);
    println!("  Net Emissions: {:.2} tons", metrics.net_co2_emissions);
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

struct SimulationResult {
    metrics: SimulationMetrics,
    output: String,
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
}

fn run_multi_simulation(
    base_map: &Map,
    num_iterations: usize,
    parallel: bool,
    continue_from_checkpoint: bool,
    checkpoint_dir: &str,
    checkpoint_interval: usize,
    progress_interval: usize,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Create checkpoint directory if it doesn't exist
    std::fs::create_dir_all(checkpoint_dir)?;
    
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
            let checkpoint_path = Path::new(checkpoint_dir)
                .join(&latest)
                .join("latest_weights.json");
            
            println!("Checking for weights at: {:?}", checkpoint_path);
            if checkpoint_path.exists() {
                println!("Loading weights from checkpoint: {:?}", checkpoint_path);
                match ActionWeights::load_from_file(checkpoint_path.to_str().unwrap()) {
                    Ok(loaded_weights) => {
                        if let Some((best_score, _)) = loaded_weights.get_best_metrics() {
                            println!("Loaded previous best score: {:.4}", best_score);
                        }
                        loaded_weights
                    },
                    Err(e) => {
                        println!("Error loading weights: {}. Starting fresh.", e);
                        ActionWeights::new()
                    }
                }
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
                        - Emissions: {} ({:.1} tons)\n\
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
                
                // Create local weights and immediately drop the read lock
                let mut local_weights = {
                    let weights = action_weights.read();
                    weights.clone()
                }; // Read lock is dropped here
                
                let result = run_iteration(i, map_clone, &mut local_weights)?;
                
                // Only acquire write lock periodically to merge weights
                if (i + 1) % 10 == 0 {
                    let mut weights = action_weights.write();
                    weights.update_weights_from(&local_weights);
                }
                
                // Increment completed iterations counter
                completed_iterations.fetch_add(1, Ordering::Relaxed);
                
                // Save checkpoint at intervals
                if (i + 1) % checkpoint_interval == 0 {
                    let mut weights = action_weights.write();
                    let checkpoint_path = Path::new(&run_dir).join("latest_weights.json");
                    weights.save_to_file(checkpoint_path.to_str().unwrap())?;
                    
                    // Save iteration number
                    let iteration_path = Path::new(&run_dir).join("checkpoint_iteration.txt");
                    std::fs::write(iteration_path, (i + 1).to_string())?;
                    
                    println!("Saved checkpoint at iteration {} in {}", i + 1, run_dir);
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
            
            // Get a write lock to update the shared weights
            let mut weights = action_weights.write();
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
        println!("Final net emissions: {:.2} tons", best.metrics.final_net_emissions);
        println!("Average public opinion: {:.3}", best.metrics.average_public_opinion);
        println!("Total cost: €{:.2}", best.metrics.total_cost);
        println!("Power reliability: {:.3}", best.metrics.power_reliability);
        
        // Save final results in the run directory
        let csv_filename = Path::new(&run_dir).join("best_simulation.csv");
        let mut file = File::create(&csv_filename)?;
        file.write_all(best.output.as_bytes())?;
        println!("\nBest simulation results saved to: {}", csv_filename.display());
        
        // Save final weights in the run directory
        let final_weights_path = Path::new(&run_dir).join("best_weights.json");
        let weights = action_weights.write();
        weights.save_to_file(final_weights_path.to_str().unwrap())?;
        println!("Final weights saved to: {}", final_weights_path.display());
    }
    
    Ok(())
}

fn run_iteration(
    iteration: usize,
    mut map: Map,
    mut weights: &mut ActionWeights,
) -> Result<SimulationResult, Box<dyn Error + Send + Sync>> {
    println!("\nStarting iteration {}", iteration + 1);
    weights.start_new_iteration();
    
    let mut total_opinion = 0.0;
    let mut total_cost = 0.0;
    let mut power_deficits = 0.0;
    let mut measurements = 0;
    
    let simulation_output = run_simulation(&mut map, Some(&mut weights))?;
    
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
    
    let operating_cost = map.calc_total_operating_cost(2050);
    let capital_cost = map.calc_total_capital_cost(2050);
    let total_cost = operating_cost + capital_cost;
    
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
    })
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = Args::parse();
    
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
    )?;
    
    Ok(())
}

fn initialize_map(map: &mut Map) { 
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