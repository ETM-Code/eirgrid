use std::error::Error;
use serde::Serialize;
use crate::generator::Generator;
use crate::map_handler::Map;
use crate::action_weights::GridAction;
use crate::action_weights::SimulationMetrics;
use crate::csv_export;
use crate::logging::{self, OperationCategory, PowerCalcType};
use crate::const_funcs;

#[derive(Debug, Clone, Serialize)]
pub struct YearlyMetrics {
    pub year: u32,
    pub total_population: u32,
    pub total_power_usage: f64,
    pub total_power_generation: f64,
    pub power_balance: f64,
    pub average_public_opinion: f64,
    pub yearly_capital_cost: f64,            // Capital cost for the current year only
    pub total_capital_cost: f64,             // Accumulated capital cost up to this year
    pub inflation_factor: f64,
    pub total_co2_emissions: f64,
    pub total_carbon_offset: f64,
    pub net_co2_emissions: f64,
    pub yearly_carbon_credit_revenue: f64, // Revenue for the current year only
    pub total_carbon_credit_revenue: f64,  // Accumulated revenue up to this year
    pub yearly_energy_sales_revenue: f64,  // Revenue from energy sales for current year
    pub total_energy_sales_revenue: f64,   // Accumulated energy sales revenue up to this year
    pub generator_efficiencies: Vec<(String, f64)>,
    pub generator_operations: Vec<(String, f64)>,
    pub active_generators: usize,
    pub yearly_upgrade_costs: f64,            // Upgrade costs for the current year
    pub yearly_closure_costs: f64,            // Closure costs for the current year
    pub yearly_total_cost: f64,               // Total cost for this year only
    pub total_cost: f64,                      // Accumulated total cost up to this year
}

#[derive(Clone)]
pub struct SimulationResult {
    pub metrics: SimulationMetrics,
    pub output: String,
    pub actions: Vec<(u32, GridAction)>,
    pub yearly_metrics: Vec<YearlyMetrics>, // Add yearly metrics to the struct
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