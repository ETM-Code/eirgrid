use super::metrics::YearlyMetrics;

pub fn print_yearly_summary(metrics: &YearlyMetrics) {
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
    
    // Add special debug logging for accumulated values
    println!("DEBUG ACCUMULATION:");
    println!("  Year: {}", metrics.year);
    println!("  Yearly Carbon Credit Revenue: €{:.2}", metrics.yearly_carbon_credit_revenue);
    println!("  Total Carbon Credit Revenue: €{:.2}", metrics.total_carbon_credit_revenue);
    println!("  Yearly Energy Sales Revenue: €{:.2}", metrics.yearly_energy_sales_revenue);
    println!("  Total Energy Sales Revenue: €{:.2}", metrics.total_energy_sales_revenue);
    println!("  Yearly Total Cost: €{:.2}", metrics.yearly_total_cost);
    println!("  Accumulated Total Cost: €{:.2}", metrics.total_cost);
}

pub fn print_generator_details(metrics: &YearlyMetrics) {
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