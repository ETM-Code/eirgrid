use serde::{Deserialize, Serialize};
use crate::models::generator::GeneratorType;
use crate::models::carbon_offset::CarbonOffsetType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorConstraints {
    pub allowed_types: Vec<GeneratorType>,
    pub max_efficiency_by_year: Vec<(u32, f64)>, // Year -> Maximum achievable efficiency
    pub upgrade_cost_multiplier: f64,            // Cost multiplier for efficiency upgrades
    pub min_operation_percentage: f64,           // Minimum operating capacity (0.0-1.0)
    pub closure_cost_multiplier: f64,            // Cost multiplier for early closure
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonOffsetConstraints {
    pub allowed_types: Vec<CarbonOffsetType>,
    pub max_forest_area: f64,      // Maximum forest area in hectares
    pub max_wetland_area: f64,     // Maximum wetland area in hectares
    pub max_active_capture: f64,   // Maximum active capture capacity in tonnes
    pub max_carbon_credits: f64,   // Maximum carbon credits in tonnes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    pub target_net_zero_2050: bool,
    pub allow_generator_upgrades: bool,
    pub allow_generator_closure: bool,
    pub allow_operation_adjustment: bool,
    pub generator_constraints: GeneratorConstraints,
    pub offset_constraints: CarbonOffsetConstraints,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            target_net_zero_2050: true,
            allow_generator_upgrades: true,
            allow_generator_closure: true,
            allow_operation_adjustment: true,
            generator_constraints: GeneratorConstraints {
                allowed_types: vec![
                    GeneratorType::OnshoreWind,
                    GeneratorType::OffshoreWind,
                    GeneratorType::DomesticSolar,
                    GeneratorType::CommercialSolar,
                    GeneratorType::UtilitySolar,
                    GeneratorType::Nuclear,
                    GeneratorType::CoalPlant,
                    GeneratorType::GasCombinedCycle,
                    GeneratorType::GasPeaker,
                    GeneratorType::HydroDam,
                    GeneratorType::PumpedStorage,
                ],
                max_efficiency_by_year: vec![ //NOTE: Review later
                    (2025, 0.40), // Base year
                    (2030, 0.45),
                    (2035, 0.48),
                    (2040, 0.50),
                    (2045, 0.52),
                    (2050, 0.55),
                ],
                upgrade_cost_multiplier: 2.0,
                min_operation_percentage: 0.2,
                closure_cost_multiplier: 0.5,
            },
            offset_constraints: CarbonOffsetConstraints {
                allowed_types: vec![
                    CarbonOffsetType::Forest,
                    CarbonOffsetType::ActiveCapture,
                    CarbonOffsetType::CarbonCredit,
                    CarbonOffsetType::Wetland,
                ],
                max_forest_area: 50000.0,      // 50,000 hectares
                max_wetland_area: 20000.0,     // 20,000 hectares
                max_active_capture: 1000.0,    // 1,000 tonnes
                max_carbon_credits: 5000.0,    // 5,000 tonnes
            },
        }
    }
} 