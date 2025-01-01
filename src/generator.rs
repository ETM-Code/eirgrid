use serde::{Deserialize, Serialize};
use crate::poi::{POI, Coordinate};
use crate::constants::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeneratorType {
    // Wind variations
    OnshoreWind,
    OffshoreWind,
    
    // Solar variations
    DomesticSolar,
    CommercialSolar,
    UtilitySolar,
    
    // Nuclear (keeping as is due to standardization)
    Nuclear,
    
    // Fossil fuel variations
    CoalPlant,
    GasCombinedCycle,
    GasPeaker,
    
    // Hydro variations
    HydroDam,
    PumpedStorage,
    TidalGenerator,
    WaveEnergy,
}

impl GeneratorType {
    pub fn get_size_constraints(&self) -> (f64, f64) {
        match self {
            // Wind constraints
            GeneratorType::OnshoreWind => (0.2, 1.0),     // 20-100% size range
            GeneratorType::OffshoreWind => (0.5, 1.0),    // 50-100% size range (larger turbines)
            
            // Solar constraints
            GeneratorType::DomesticSolar => (0.001, 0.01),  // Fixed small size
            GeneratorType::CommercialSolar => (0.01, 0.1),  // Medium installations
            GeneratorType::UtilitySolar => (0.2, 1.0),      // Large solar farms
            
            // Nuclear constraints
            GeneratorType::Nuclear => (0.8, 1.0),         // Only large installations
            
            // Fossil fuel constraints
            GeneratorType::CoalPlant => (0.6, 1.0),      // Large baseload plants
            GeneratorType::GasCombinedCycle => (0.4, 1.0), // Medium to large
            GeneratorType::GasPeaker => (0.2, 0.6),      // Smaller peaking plants
            
            // Hydro constraints
            GeneratorType::HydroDam => (0.5, 1.0),       // Large installations
            GeneratorType::PumpedStorage => (0.4, 0.8),   // Medium to large
            GeneratorType::TidalGenerator => (0.1, 0.5),  // Smaller, experimental
            GeneratorType::WaveEnergy => (0.05, 0.3),    // Very small to medium
        }
    }

    pub fn can_be_urban(&self) -> bool {
        match self {
            GeneratorType::DomesticSolar => true,
            GeneratorType::CommercialSolar => true,
            GeneratorType::GasPeaker => true,
            _ => false,
        }
    }

    pub fn requires_water(&self) -> bool {
        match self {
            GeneratorType::OffshoreWind |
            GeneratorType::TidalGenerator |
            GeneratorType::WaveEnergy => true,
            GeneratorType::HydroDam |
            GeneratorType::PumpedStorage |
            GeneratorType::Nuclear |
            GeneratorType::CoalPlant |
            GeneratorType::GasCombinedCycle => false, // They need water but not necessarily coastal/river location
            _ => false,
        }
    }

    pub fn get_base_efficiency(&self, year: u32) -> f64 {
        match self {
            GeneratorType::OnshoreWind => 0.35,
            GeneratorType::OffshoreWind => 0.42,
            GeneratorType::DomesticSolar => 0.20,
            GeneratorType::CommercialSolar => 0.22,
            GeneratorType::UtilitySolar => 0.25,
            GeneratorType::Nuclear => 0.33,
            GeneratorType::CoalPlant => 0.37,
            GeneratorType::GasCombinedCycle => 0.45,
            GeneratorType::GasPeaker => 0.35,
            GeneratorType::HydroDam => 0.90,
            GeneratorType::PumpedStorage => 0.80,
            GeneratorType::TidalGenerator => {
                // Efficiency improves significantly over time as technology matures
                let years_from_base = (year - BASE_YEAR) as f64;
                0.20 + (years_from_base * 0.01).min(0.40) // Starts at 20%, can reach 60%
            },
            GeneratorType::WaveEnergy => {
                // Similar to tidal but starts lower
                let years_from_base = (year - BASE_YEAR) as f64;
                0.15 + (years_from_base * 0.01).min(0.35) // Starts at 15%, can reach 50%
            },
        }
    }

    pub fn get_cost_evolution_rate(&self) -> f64 {
        match self {
            GeneratorType::OnshoreWind => 0.97,      // 3% reduction per year
            GeneratorType::OffshoreWind => 0.95,     // 5% reduction per year
            GeneratorType::DomesticSolar => 0.93,    // 7% reduction per year
            GeneratorType::CommercialSolar => 0.93,  // 7% reduction per year
            GeneratorType::UtilitySolar => 0.92,     // 8% reduction per year
            GeneratorType::Nuclear => 1.01,          // 1% increase per year
            GeneratorType::CoalPlant => 1.04,        // 4% increase per year
            GeneratorType::GasCombinedCycle => 1.02, // 2% increase per year
            GeneratorType::GasPeaker => 1.02,        // 2% increase per year
            GeneratorType::HydroDam => 1.005,        // 0.5% increase per year
            GeneratorType::PumpedStorage => 1.01,    // 1% increase per year
            GeneratorType::TidalGenerator => 0.90,   // 10% reduction per year (rapid improvement)
            GeneratorType::WaveEnergy => 0.88,       // 12% reduction per year (very rapid improvement)
        }
    }

    pub fn get_base_opinion(&self) -> f64 {
        match self {
            GeneratorType::OnshoreWind => 0.75,      // Lower than offshore due to visual impact
            GeneratorType::OffshoreWind => 0.85,     // Higher due to less visual impact
            GeneratorType::DomesticSolar => 0.95,    // Very high acceptance
            GeneratorType::CommercialSolar => 0.90,
            GeneratorType::UtilitySolar => 0.85,     // Lower due to land use
            GeneratorType::Nuclear => 0.30,
            GeneratorType::CoalPlant => 0.25,
            GeneratorType::GasCombinedCycle => 0.45,
            GeneratorType::GasPeaker => 0.40,
            GeneratorType::HydroDam => 0.70,         // Lower due to environmental impact
            GeneratorType::PumpedStorage => 0.75,
            GeneratorType::TidalGenerator => 0.80,
            GeneratorType::WaveEnergy => 0.85,
        }
    }

    pub fn get_opinion_change_rate(&self) -> f64 {
        match self {
            GeneratorType::OnshoreWind => 0.002,
            GeneratorType::OffshoreWind => 0.003,
            GeneratorType::DomesticSolar => 0.001,   // Already high, slow increase
            GeneratorType::CommercialSolar => 0.002,
            GeneratorType::UtilitySolar => 0.002,
            GeneratorType::Nuclear => 0.008,         // Faster increase due to climate awareness
            GeneratorType::CoalPlant => -0.015,
            GeneratorType::GasCombinedCycle => -0.008,
            GeneratorType::GasPeaker => -0.008,
            GeneratorType::HydroDam => 0.001,
            GeneratorType::PumpedStorage => 0.002,
            GeneratorType::TidalGenerator => 0.005,  // Increasing acceptance as technology proves itself
            GeneratorType::WaveEnergy => 0.005,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generator {
    id: String,
    coordinate: Coordinate,
    generator_type: GeneratorType,
    base_cost: f64,
    power_out: f64,
    base_operating_cost: f64,
    eol: u32,  // End of Life in years
    size: f64, // Between 0.1 and 1.0
    co2_out: f64,
    efficiency: f64,
    operation_percentage: f64,
    is_active: bool,
    upgrade_history: Vec<(u32, f64)>, // Year -> New efficiency pairs
}

impl Generator {
    pub fn new(
        id: String,
        coordinate: Coordinate,
        generator_type: GeneratorType,
        base_cost: f64,
        power_out: f64,
        base_operating_cost: f64,
        eol: u32,
        size: f64,
        co2_out: f64,
    ) -> Self {
        let size = size.clamp(MIN_GENERATOR_SIZE, MAX_GENERATOR_SIZE);
        Self {
            id,
            coordinate,
            generator_type,
            base_cost,
            power_out,
            base_operating_cost,
            eol,
            size,
            co2_out,
            efficiency: 0.35, // Starting efficiency
            operation_percentage: 1.0,
            is_active: true,
            upgrade_history: Vec::new(),
        }
    }

    pub fn get_current_cost(&self, year: u32) -> f64 {
        calc_generator_cost(&self.generator_type, self.base_cost, year)
    }

    pub fn get_current_operating_cost(&self, year: u32) -> f64 {
        if !self.is_active {
            return 0.0;
        }
        let base_cost = calc_operating_cost(&self.generator_type, self.base_operating_cost, year);
        base_cost * self.operation_percentage
    }

    pub fn calc_cost_opinion(&self, year: u32) -> f64 {
        calc_cost_opinion(self.get_current_cost(year), year)
    }

    pub fn calc_type_opinion(&self, year: u32) -> f64 {
        calc_type_opinion(&self.generator_type, year)
    }

    pub fn calc_cost_over_time(&self, years: u32) -> f64 {
        let current_year = 2025 + years;
        self.get_current_cost(current_year) + 
            (0..years).map(|y| self.get_current_operating_cost(2025 + y)).sum::<f64>()
    }

    pub fn get_current_power_output(&self) -> f64 {
        if !self.is_active {
            return 0.0;
        }
        self.power_out * self.efficiency * self.operation_percentage
    }

    pub fn get_current_co2_output(&self) -> f64 {
        if !self.is_active {
            return 0.0;
        }
        self.co2_out * self.operation_percentage * (1.0 - self.efficiency)
    }

    pub fn can_upgrade_efficiency(&self, year: u32, constraints: &GeneratorConstraints) -> bool {
        if !self.is_active {
            return false;
        }

        // Find maximum efficiency for the current year
        let max_efficiency = constraints.max_efficiency_by_year
            .iter()
            .filter(|(y, _)| *y <= year)
            .map(|(_, e)| *e)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.4);

        self.efficiency < max_efficiency
    }

    pub fn upgrade_efficiency(&mut self, year: u32, new_efficiency: f64) -> f64 {
        let efficiency_increase = new_efficiency - self.efficiency;
        let upgrade_cost = self.base_cost * efficiency_increase * 2.0; // Cost scales with improvement
        self.efficiency = new_efficiency;
        self.upgrade_history.push((year, new_efficiency));
        upgrade_cost
    }

    pub fn adjust_operation(&mut self, new_percentage: f64, constraints: &GeneratorConstraints) -> bool {
        let clamped_percentage = new_percentage.clamp(
            constraints.min_operation_percentage,
            1.0
        );
        
        if !self.is_active {
            return false;
        }

        self.operation_percentage = clamped_percentage;
        true
    }

    pub fn close_generator(&mut self, year: u32) -> f64 {
        if !self.is_active {
            return 0.0;
        }

        let years_remaining = (self.eol as i32 - (year - 2025) as i32).max(0) as f64;
        let closure_cost = self.base_cost * 0.5 * (years_remaining / self.eol as f64);
        
        self.is_active = false;
        self.operation_percentage = 0.0;
        
        closure_cost
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn get_efficiency(&self) -> f64 {
        self.efficiency
    }

    pub fn get_operation_percentage(&self) -> f64 {
        self.operation_percentage
    }
}

impl POI for Generator {
    fn get_coordinate(&self) -> &Coordinate {
        &self.coordinate
    }

    fn get_id(&self) -> &str {
        &self.id
    }
} 