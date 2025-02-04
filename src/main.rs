mod poi;
mod generator;
mod settlement;
mod map_handler;
mod constants;
mod const_funcs;
mod carbon_offset;
mod simulation_config;
mod action_weights;

use std::fs::File;
use std::io::Write;
use chrono::Local;
use rand::Rng;

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

#[derive(Debug)]
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
}

fn run_simulation(
    map: &mut Map,
    action_weights: Option<&mut ActionWeights>,
) -> Result<String, std::io::Error> {
    let mut output = String::new();
    output.push_str("Year,Total Population,Total Power Usage (MW),Total Power Generation (MW),\
                    Power Balance (MW),Average Public Opinion,Total Operating Cost (€),\
                    Total Capital Cost (€),Inflation Factor,Total CO2 Emissions (tons),\
                    Total Carbon Offset (tons),Net CO2 Emissions (tons),Active Generators,\
                    Upgrade Costs (€),Closure Costs (€)\n");

    let mut total_upgrade_costs = 0.0;
    let mut total_closure_costs = 0.0;
    
    // Use provided weights or create new ones
    let mut local_weights = action_weights.cloned().unwrap_or_else(ActionWeights::new);

    for year in SIMULATION_START_YEAR..=SIMULATION_END_YEAR {
        if action_weights.is_none() {
            println!("\nStarting year {}", year);
            
            // Print top actions from previous year's learning
            if year > SIMULATION_START_YEAR {
                local_weights.print_top_actions(year - 1, 5);
            }
        }
        
        // Calculate current state
        let current_state = ActionResult {
            net_emissions: map.calc_net_co2_emissions(year),
            public_opinion: calculate_average_opinion(map, year),
            power_balance: map.calc_total_power_generation() - map.calc_total_power_usage(year),
        };

        // Handle power deficit first
        if current_state.power_balance < 0.0 {
            handle_power_deficit(map, -current_state.power_balance, year, &mut local_weights)?;
        }

        // Perform random additional actions
        let mut rng = rand::thread_rng();
        let num_additional_actions = rng.gen_range(0..=10); //NOTE ensure the action num is subject to learning (allow growth past 10?)
        
        for action_num in 0..num_additional_actions {
            let best_action = find_best_action(map, year, &local_weights)?;
            apply_action(map, &best_action, year)?;
            
            let new_state = ActionResult {
                net_emissions: map.calc_net_co2_emissions(year),
                public_opinion: calculate_average_opinion(map, year),
                power_balance: map.calc_total_power_generation() - map.calc_total_power_usage(year),
            };
            
            let improvement = evaluate_action_impact(&current_state, &new_state, year); //NOTE modify so evaluation happens at the end
            local_weights.update_weights(&best_action, year, improvement);
            
            if action_weights.is_none() && action_num % 5 == 0 {
                println!("  Action {}/{}: {:?}", action_num + 1, num_additional_actions, best_action);
            }
        }

        // Calculate metrics for the year
        let metrics = calculate_yearly_metrics(map, year, total_upgrade_costs, total_closure_costs);
        
        // Add row to CSV
        output.push_str(&format!(
            "{},{},{:.2},{:.2},{:.2},{:.3},{:.2},{:.2},{:.3},{:.2},{:.2},{:.2},{},{:.2},{:.2}\n",
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
            metrics.closure_costs //NOTE add total costs
        ));

        if action_weights.is_none() {
            // Print detailed yearly summary only in single simulation mode
            print_yearly_summary(&metrics);
            print_generator_details(&metrics);
        }
        
        // Update provided weights if they exist
        if let Some(weights) = action_weights {
            weights.update_weights_from(&local_weights);
        }
    }

    Ok(output)
}

fn find_best_action(map: &Map, year: u32, action_weights: &ActionWeights) -> Result<GridAction, std::io::Error> { //NOTE oho we dont want this, we're learning not assuming
    let mut best_action = None;
    let mut best_improvement = f64::NEG_INFINITY;
    let mut best_state = None;
    
    let current_state = ActionResult {
        net_emissions: map.calc_net_co2_emissions(year),
        public_opinion: calculate_average_opinion(map, year),
        power_balance: map.calc_total_power_generation() - map.calc_total_power_usage(year),
    };
    
    // Try multiple random actions and pick the best one
    for _ in 0..NUM_ACTION_SAMPLES {
        let action = action_weights.sample_action(year);
        let mut map_clone = map.clone();
        
        if apply_action(&mut map_clone, &action, year).is_ok() {
            let new_state = ActionResult {
                net_emissions: map_clone.calc_net_co2_emissions(year),
                public_opinion: calculate_average_opinion(&map_clone, year),
                power_balance: map_clone.calc_total_power_generation() - map_clone.calc_total_power_usage(year),
            };
            
            let improvement = evaluate_action_impact(&current_state, &new_state, year);
            
            // Update best action if this one is better
            if improvement > best_improvement {
                best_improvement = improvement;
                best_action = Some(action);
                best_state = Some(new_state);
            } else if improvement == best_improvement && best_state.is_some() {
                // If improvements are equal, choose based on public opinion
                if new_state.public_opinion > best_state.as_ref().unwrap().public_opinion {
                    best_action = Some(action);
                    best_state = Some(new_state);
                }
            }
        }
    }
    
    Ok(best_action.unwrap_or_else(|| action_weights.sample_action(year)))
}

fn handle_power_deficit(
    map: &mut Map,
    deficit: f64,
    year: u32,
    action_weights: &mut ActionWeights,
) -> Result<(), std::io::Error> {
    let mut remaining_deficit = deficit;
    
    while remaining_deficit > 0.0 {
        let action = action_weights.sample_action(year);
        let current_state = ActionResult {
            net_emissions: map.calc_net_co2_emissions(year),
            public_opinion: calculate_average_opinion(map, year),
            power_balance: map.calc_total_power_generation() - map.calc_total_power_usage(year),
        };
        
        if let GridAction::AddGenerator(_) = action { //add upgrade and conditionslly allow power storage
            apply_action(map, &action, year)?;
            
            let new_state = ActionResult {
                net_emissions: map.calc_net_co2_emissions(year),
                public_opinion: calculate_average_opinion(map, year),
                power_balance: map.calc_total_power_generation() - map.calc_total_power_usage(year),
            };
            
            let improvement = evaluate_action_impact(&current_state, &new_state, year);
            action_weights.update_weights(&action, year, improvement);
            
            remaining_deficit = -new_state.power_balance.min(0.0);
        }
    }
    
    Ok(())
}

fn apply_action(map: &mut Map, action: &GridAction, year: u32) -> Result<(), std::io::Error> {
    match action {
        GridAction::AddGenerator(gen_type) => {
            // Find best location based on public opinion
            let best_location = find_best_generator_location(map, gen_type, year);
            if let Some(location) = best_location {
                let generator = Generator::new(
                    format!("Gen_{}_{}_{}", gen_type.to_string(), year, map.generators.len()),
                    location,
                    *gen_type,
                    gen_type.get_base_cost(),
                    gen_type.get_base_power(),
                    gen_type.get_operating_cost(),
                    gen_type.get_lifespan(),
                    gen_type.get_base_efficiency(),
                    gen_type.get_decommission_cost(),
                );
                map.add_generator(generator);
            }
        },
        GridAction::UpgradeEfficiency(id) => {
            // Find the generator and upgrade its efficiency
            if let Some(generator) = map.generators.iter_mut().find(|g| g.get_id() == id) {
                if generator.is_active() {
                    // Calculate max efficiency based on year and generator type
                    let base_max = match generator.get_type() {
                        GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => 0.45,
                        GeneratorType::UtilitySolar => 0.40,
                        GeneratorType::Nuclear => 0.50,
                        GeneratorType::GasCombinedCycle => 0.60,
                        GeneratorType::HydroDam | GeneratorType::PumpedStorage => 0.85,
                        GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => 0.35,
                        _ => 0.40,
                    };
                    
                    // Apply year-based improvement
                    let years_passed = year - BASE_YEAR;
                    let tech_improvement = match generator.get_type() {
                        GeneratorType::OnshoreWind | GeneratorType::OffshoreWind | 
                        GeneratorType::UtilitySolar => DEVELOPING_TECH_IMPROVEMENT_RATE,
                        GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => EMERGING_TECH_IMPROVEMENT_RATE,
                        _ => MATURE_TECH_IMPROVEMENT_RATE,
                    }.powi(years_passed as i32);
                    
                    let max_efficiency = base_max * (1.0 + (1.0 - tech_improvement));
                    generator.upgrade_efficiency(max_efficiency);
                }
            }
        },
        GridAction::AdjustOperation(id, percentage) => { //NOTE we should do this for the highest CO2 generstor when surpassing usage
            // Find the generator and adjust its operation percentage
            if let Some(generator) = map.generators.iter_mut().find(|g| g.get_id() == id) {
                if generator.is_active() {
                    generator.adjust_operation(percentage, &map.config.generator_constraints);
                }
            }
        },
        GridAction::AddCarbonOffset(offset_type) => { //NOTE Add carbon credit purchase
            // Create and add a new carbon offset project
            let offset_size = rand::thread_rng().gen_range(100.0..1000.0); // Size in hectares or capture capacity
            let base_efficiency = rand::thread_rng().gen_range(0.7..0.95);
            
            // Find a suitable location (for now, random within map bounds)
            let location = Coordinate::new(
                rand::thread_rng().gen_range(0.0..MAP_MAX_X),
                rand::thread_rng().gen_range(0.0..MAP_MAX_Y),
            );
            
            let offset = CarbonOffset::new(
                format!("Offset_{}_{}_{}", offset_type, year, map.carbon_offsets.len()),
                location,
                offset_type.parse().unwrap_or(CarbonOffsetType::Forest),
                match offset_type.parse().unwrap_or(CarbonOffsetType::Forest) {
                    CarbonOffsetType::Forest => 10_000_000.0,
                    CarbonOffsetType::Wetland => 15_000_000.0,
                    CarbonOffsetType::ActiveCapture => 100_000_000.0,
                },
                match offset_type.parse().unwrap_or(CarbonOffsetType::Forest) {
                    CarbonOffsetType::Forest => 500_000.0,
                    CarbonOffsetType::Wetland => 750_000.0,
                    CarbonOffsetType::ActiveCapture => 5_000_000.0,
                },
                offset_size,
                base_efficiency,
            );
            
            map.add_carbon_offset(offset);
        },
        GridAction::CloseGenerator(id) => { //NOTE allow closing fossil fuel plants
            // Find the generator and close it
            if let Some(generator) = map.generators.iter_mut().find(|g| g.get_id() == id) {
                if generator.is_active() {
                    // Check if it's appropriate to close based on age and type
                    let age = year - generator.get_commissioning_year();
                    let min_age = match generator.get_type() {
                        GeneratorType::Nuclear => 30, // Nuclear plants have long lifespans
                        GeneratorType::HydroDam => 40, // Hydro dams are very long-term
                        GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => 15, // Wind farms have shorter lifespans
                        GeneratorType::UtilitySolar => 20, // Solar farms medium lifespan
                        _ => 25, // Default minimum age
                    };
                    
                    if age >= min_age {
                        generator.close_generator(year);
                    }
                }
            }
        },
    }
    Ok(())
}

fn find_best_generator_location(map: &Map, gen_type: &GeneratorType, gen_size: u8, year: u32) -> Option<Coordinate> {
    // Implementation to find best location based on public opinion
    // This would need to sample various locations and evaluate public opinion
    // For now, return a simple location
    
    //NOTE for this sample 100 locations and take the best (cant really add this to the learning idk)
    
    Some(Coordinate::new(50000.0, 50000.0))
}

fn calculate_average_opinion(map: &Map, year: u32) -> f64 {
    let mut total_opinion = 0.0;
    let mut count = 0;
    
    for generator in &map.generators {
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

fn calculate_yearly_metrics(map: &mut Map, year: u32, total_upgrade_costs: f64, total_closure_costs: f64) -> YearlyMetrics {
    let total_pop = map.calc_total_population(year);
    let total_power_usage = map.calc_total_power_usage(year);
    let total_power_gen = map.calc_total_power_generation();
    let power_balance = total_power_gen - total_power_usage;
    let total_co2_emissions = map.calc_total_co2_emissions();
    let total_carbon_offset = map.calc_total_carbon_offset(year);
    let net_co2_emissions = map.calc_net_co2_emissions(year);

    // Calculate average public opinion for all existing generators
    let mut total_opinion = 0.0;
    let mut opinion_count = 0;
    let mut generator_efficiencies = Vec::new();
    let mut generator_operations = Vec::new();
    let mut active_count = 0;
    
    for generator in &map.generators {
        if generator.is_active() {
            total_opinion += map.calc_new_generator_opinion(
                generator.get_coordinate(),
                generator,
                year
            );
            opinion_count += 1;
            active_count += 1;

            generator_efficiencies.push((generator.get_id().to_string(), generator.get_efficiency()));
            generator_operations.push((generator.get_id().to_string(), generator.get_operation_percentage()));
        }
    }

    let total_operating_cost = map.calc_total_operating_cost(year);
    let total_capital_cost = map.calc_total_capital_cost(year);
    let inflation_factor = const_funcs::calc_inflation_factor(year);

    YearlyMetrics {
        year,
        total_population: total_pop,
        total_power_usage,
        total_power_generation: total_power_gen,
        power_balance,
        average_public_opinion: avg_opinion,
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

fn run_multi_simulation( //NOTE can we do this on GPU?
    base_map: &Map,
    num_iterations: usize,
    parallel: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut action_weights = ActionWeights::new();
    let mut best_result: Option<SimulationResult> = None;
    
    println!("Starting multi-simulation optimization with {} iterations", num_iterations);
    
    let run_iteration = |iteration: usize, mut map: Map, mut weights: ActionWeights| -> Result<SimulationResult, Box<dyn std::error::Error>> {
        println!("\nStarting iteration {}/{}", iteration + 1, num_iterations);
        weights.start_new_iteration();
        
        let mut total_opinion = 0.0;
        let mut total_cost = 0.0;
        let mut power_deficits = 0.0;
        let mut measurements = 0;
        
        let simulation_output = run_simulation(&mut map, Some(&mut weights))?;
        
        // Calculate final metrics
        let final_metrics = SimulationMetrics {
            final_net_emissions: map.calc_net_co2_emissions(2050),
            average_public_opinion: total_opinion / measurements as f64,
            total_cost,
            power_reliability: 1.0 - (power_deficits / measurements as f64),
        };
        
        weights.update_best_strategy(final_metrics.clone());
        
        Ok(SimulationResult {
            metrics: final_metrics,
            output: simulation_output,
        })
    };
    
    if parallel {
        use rayon::prelude::*;
        let results: Vec<_> = (0..num_iterations)
            .into_par_iter()
            .map(|i| {
                let map_clone = base_map.clone();
                let weights_clone = action_weights.clone();
                run_iteration(i, map_clone, weights_clone)
            })
            .collect::<Result<Vec<_>, _>>()?;
        
        // Find best result
        for result in results {
            if best_result.as_ref().map_or(true, |best| {
                score_metrics(&result.metrics) > score_metrics(&best.metrics)
            }) {
                best_result = Some(result);
            }
        }
    } else {
        for i in 0..num_iterations {
            let mut map_clone = base_map.clone();
            let result = run_iteration(i, map_clone, action_weights.clone())?;
            
            if best_result.as_ref().map_or(true, |best| {
                score_metrics(&result.metrics) > score_metrics(&best.metrics)
            }) {
                best_result = Some(result);
            }
        }
    }
    
    // Save best result
    if let Some(best) = best_result {
        println!("\nBest simulation results:");
        println!("Final net emissions: {:.2} tons", best.metrics.final_net_emissions);
        println!("Average public opinion: {:.3}", best.metrics.average_public_opinion);
        println!("Total cost: €{:.2}", best.metrics.total_cost);
        println!("Power reliability: {:.3}", best.metrics.power_reliability);
        
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let csv_filename = format!("best_simulation_{}.csv", timestamp);
        let mut file = File::create(&csv_filename)?;
        file.write_all(best.output.as_bytes())?;
        println!("\nBest simulation results saved to: {}", csv_filename);
        
        // Save best weights
        action_weights.save_to_file("best_weights.json")?;
    }
    
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("EirGrid Power System Simulator (2025-2050)");
    
    // Create simulation configuration
    let config = SimulationConfig::default();
    
    // Create example map with some initial data
    let mut map = Map::new(config); //NOTE use json data
    
    // Add settlements and initial generators
    initialize_map(&mut map);
    
    // Run multi-simulation optimization
    let num_iterations = 1000; // Adjust based on available compute resources
    let use_parallel = true;   // Set to true to use parallel execution
    
    run_multi_simulation(&map, num_iterations, use_parallel)?;
    
    Ok(())
}

fn initialize_map(map: &mut Map) { // NOTE use json data
    // Add major settlements
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

    // Add existing power plants
    map.add_generator(Generator::new(
        "Moneypoint".to_string(),
        Coordinate::new(30000.0, 50000.0),
        GeneratorType::Coal,
        800_000_000.0,
        915.0,
        50_000_000.0,
        40,
        1.0,
        2_000_000.0,
    ));

    map.add_generator(Generator::new(
        "Dublin Bay".to_string(),
        Coordinate::new(72000.0, 72000.0),
        GeneratorType::Gas,
        400_000_000.0,
        415.0,
        20_000_000.0,
        30,
        0.8,
        800_000.0,
    ));
} 