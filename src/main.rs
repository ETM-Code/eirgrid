mod poi;
mod generator;
mod settlement;
mod map_handler;
mod constants;
mod const_funcs;
mod carbon_offset;
mod simulation_config;

use std::fs::File;
use std::io::Write;
use chrono::Local;

pub use poi::{POI, Coordinate};
pub use generator::{Generator, GeneratorType};
pub use settlement::Settlement;
pub use map_handler::Map;
pub use carbon_offset::{CarbonOffset, CarbonOffsetType};
pub use simulation_config::SimulationConfig;
pub use constants::*;

const SIMULATION_START_YEAR: u32 = 2025;
const SIMULATION_END_YEAR: u32 = 2050;

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

fn run_simulation(map: &mut Map) -> Result<String, std::io::Error> {
    let mut output = String::new();
    output.push_str("Year,Total Population,Total Power Usage (MW),Total Power Generation (MW),\
                    Power Balance (MW),Average Public Opinion,Total Operating Cost (€),\
                    Total Capital Cost (€),Inflation Factor,Total CO2 Emissions (tons),\
                    Total Carbon Offset (tons),Net CO2 Emissions (tons),Active Generators,\
                    Upgrade Costs (€),Closure Costs (€)\n");

    let mut total_upgrade_costs = 0.0;
    let mut total_closure_costs = 0.0;

    for year in SIMULATION_START_YEAR..=SIMULATION_END_YEAR {
        // Balance power generation with demand
        let (required_power, actual_generation) = map.balance_power_generation(year)?;
        if actual_generation < required_power {
            println!("WARNING: Power generation deficit in year {}: {:.2} MW", 
                    year, required_power - actual_generation);
        }

        // Try to optimize for net zero if approaching 2050
        if year >= 2045 {
            map.optimize_for_net_zero(year)?;
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
            metrics.closure_costs
        ));

        // Print detailed yearly summary
        print_yearly_summary(&metrics);
        print_generator_details(&metrics);
    }

    Ok(output)
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

    let avg_opinion = if opinion_count > 0 {
        total_opinion / opinion_count as f64
    } else {
        1.0
    };

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("EirGrid Power System Simulator (2025-2050)");
    
    // Create simulation configuration
    let config = SimulationConfig::default();
    
    // Create example map with some initial data
    let mut map = Map::new(config);

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

    map.add_generator(Generator::new(
        "Ardnacrusha".to_string(),
        Coordinate::new(30000.0, 45000.0),
        GeneratorType::Hydro,
        200_000_000.0,
        86.0,
        5_000_000.0,
        50,
        0.7,
        0.0,
    ));

    // Add carbon offset projects
    map.add_carbon_offset(CarbonOffset::new(
        "Wicklow Forest".to_string(),
        Coordinate::new(75000.0, 65000.0),
        CarbonOffsetType::Forest,
        10_000_000.0,
        500_000.0,
        1000.0,  // 1000 hectares
        0.85,    // 85% efficiency
    ));

    map.add_carbon_offset(CarbonOffset::new(
        "Shannon Wetlands".to_string(),
        Coordinate::new(35000.0, 45000.0),
        CarbonOffsetType::Wetland,
        15_000_000.0,
        750_000.0,
        500.0,   // 500 hectares
        0.9,     // 90% efficiency
    ));

    map.add_carbon_offset(CarbonOffset::new(
        "Dublin CCS".to_string(),
        Coordinate::new(71000.0, 71000.0),
        CarbonOffsetType::ActiveCapture,
        100_000_000.0,
        5_000_000.0,
        100.0,   // 100 tons capture capacity
        0.95,    // 95% efficiency
    ));

    // Run simulation
    let simulation_output = run_simulation(&mut map)?;

    // Save results to CSV
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let csv_filename = format!("simulation_results_{}.csv", timestamp);
    let mut file = File::create(&csv_filename)?;
    file.write_all(simulation_output.as_bytes())?;

    println!("\nSimulation results saved to: {}", csv_filename);
    
    Ok(())
} 