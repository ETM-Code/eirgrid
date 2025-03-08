use crate::map_handler::Map;
use crate::metrics::YearlyMetrics;
use crate::logging::{self, OperationCategory, PowerCalcType};
use crate::const_funcs;
use crate::poi::POI;

pub fn calculate_average_opinion(map: &Map, year: u32) -> f64 {
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

pub fn calculate_yearly_metrics(
    map: &Map, 
    year: u32, 
    total_upgrade_costs: f64, 
    total_closure_costs: f64, 
    enable_energy_sales: bool,
    previous_metrics: Option<&YearlyMetrics>
) -> YearlyMetrics {
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
     
    // Properly accumulate total_cost across years by adding yearly costs to previous total
    let total_cost = match previous_metrics {
        Some(prev) => prev.total_cost + yearly_total_cost,
        None => yearly_total_cost // First year, just use current year's cost
    };
     
    // Track yearly and accumulated carbon credit revenue
    let yearly_carbon_credit_revenue = carbon_credit_revenue;
    
    // Properly accumulate total_carbon_credit_revenue across years
    let total_carbon_credit_revenue = match previous_metrics {
        Some(prev) => prev.total_carbon_credit_revenue + carbon_credit_revenue,
        None => carbon_credit_revenue // First year, just use current year's revenue
    };

    // Calculate total energy sales revenue with proper accumulation across years 
    let total_energy_sales_revenue = match previous_metrics {
        Some(prev) => prev.total_energy_sales_revenue + yearly_energy_sales_revenue,
        None => yearly_energy_sales_revenue // First year, just use current year's revenue
    };

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