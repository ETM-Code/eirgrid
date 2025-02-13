use std::fmt;
use serde::{Deserialize, Serialize};
use crate::poi::{POI, Coordinate};
use crate::constants::*;
use crate::const_funcs::{calc_generator_cost, calc_operating_cost, calc_cost_opinion, calc_type_opinion};
use crate::simulation_config::GeneratorConstraints;
use crate::power_storage::PowerStorageSystem;
use crate::map_handler::Map;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
    Biomass, // New Biomass generator type
    
    // Hydro variations
    HydroDam,
    PumpedStorage,
    BatteryStorage,  // New type for battery storage
    TidalGenerator,
    WaveEnergy,
}

impl fmt::Display for GeneratorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            GeneratorType::OnshoreWind => write!(f, "OnshoreWind"),
            GeneratorType::OffshoreWind => write!(f, "OffshoreWind"),
            GeneratorType::DomesticSolar => write!(f, "DomesticSolar"),
            GeneratorType::CommercialSolar => write!(f, "CommercialSolar"),
            GeneratorType::UtilitySolar => write!(f, "UtilitySolar"),
            GeneratorType::Nuclear => write!(f, "Nuclear"),
            GeneratorType::CoalPlant => write!(f, "CoalPlant"),
            GeneratorType::GasCombinedCycle => write!(f, "GasCombinedCycle"),
            GeneratorType::GasPeaker => write!(f, "GasPeaker"),
            GeneratorType::Biomass => write!(f, "Biomass"),
            GeneratorType::HydroDam => write!(f, "HydroDam"),
            GeneratorType::PumpedStorage => write!(f, "PumpedStorage"),
            GeneratorType::BatteryStorage => write!(f, "BatteryStorage"),
            GeneratorType::TidalGenerator => write!(f, "TidalGenerator"),
            GeneratorType::WaveEnergy => write!(f, "WaveEnergy"),
        }
    }
}

impl GeneratorType {
    pub fn is_intermittent(&self) -> bool {
        matches!(self,
            GeneratorType::OnshoreWind |
            GeneratorType::OffshoreWind |
            GeneratorType::DomesticSolar |
            GeneratorType::CommercialSolar |
            GeneratorType::UtilitySolar
        )
    }

    pub fn is_storage(&self) -> bool {
        matches!(self,
            GeneratorType::PumpedStorage |
            GeneratorType::BatteryStorage
        )
    }

    pub fn get_size_constraints(&self) -> (f64, f64) {
        match *self {
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
            GeneratorType::Biomass => (0.4, 1.0),
            
            // Hydro constraints
            GeneratorType::HydroDam => (0.5, 1.0),       // Large installations
            GeneratorType::PumpedStorage => (0.4, 0.8),   // Medium to large
            GeneratorType::TidalGenerator => (0.1, 0.5),  // Smaller, experimental
            GeneratorType::WaveEnergy => (0.05, 0.3),    // Very small to medium
            GeneratorType::BatteryStorage => (0.1, 0.5),  // Flexible sizing for battery installations
        }
    }

    pub fn can_be_urban(&self) -> bool {
        match *self {
            GeneratorType::DomesticSolar => true,
            GeneratorType::CommercialSolar => true,
            GeneratorType::GasPeaker => true,
            GeneratorType::Biomass => false,
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
        match *self {
            GeneratorType::OnshoreWind => 0.35,
            GeneratorType::OffshoreWind => 0.42,
            GeneratorType::DomesticSolar => 0.20,
            GeneratorType::CommercialSolar => 0.22,
            GeneratorType::UtilitySolar => 0.25,
            GeneratorType::Nuclear => 0.33,
            GeneratorType::CoalPlant => 0.37,
            GeneratorType::GasCombinedCycle => 0.45,
            GeneratorType::GasPeaker => 0.35,
            GeneratorType::Biomass => 0.30,
            GeneratorType::HydroDam => 0.90,
            GeneratorType::PumpedStorage => 0.80,
            GeneratorType::BatteryStorage => 0.85,  // High efficiency for battery storage
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
        match *self {
            GeneratorType::OnshoreWind => 0.97,      // 3% reduction per year
            GeneratorType::OffshoreWind => 0.95,     // 5% reduction per year
            GeneratorType::DomesticSolar => 0.93,    // 7% reduction per year
            GeneratorType::CommercialSolar => 0.93,  // 7% reduction per year
            GeneratorType::UtilitySolar => 0.92,     // 8% reduction per year
            GeneratorType::Nuclear => 1.01,          // 1% increase per year
            GeneratorType::CoalPlant => 1.04,        // 4% increase per year
            GeneratorType::GasCombinedCycle => 1.02, // 2% increase per year
            GeneratorType::GasPeaker => 1.02,        // 2% increase per year
            GeneratorType::Biomass => 0.98,
            GeneratorType::HydroDam => 1.005,        // 0.5% increase per year
            GeneratorType::PumpedStorage => 1.01,    // 1% increase per year
            GeneratorType::BatteryStorage => 0.90,   // 10% reduction per year (rapid improvement)
            GeneratorType::TidalGenerator => 0.90,   // 10% reduction per year (rapid improvement)
            GeneratorType::WaveEnergy => 0.88,       // 12% reduction per year (very rapid improvement)
        }
    }

    pub fn get_base_opinion(&self) -> f64 {
        match *self {
            GeneratorType::OnshoreWind => 0.75,
            GeneratorType::OffshoreWind => 0.85,
            GeneratorType::DomesticSolar => 0.95,
            GeneratorType::CommercialSolar => 0.90,
            GeneratorType::UtilitySolar => 0.85,
            GeneratorType::Nuclear => 0.30,
            GeneratorType::CoalPlant => 0.25,
            GeneratorType::GasCombinedCycle => 0.45,
            GeneratorType::GasPeaker => 0.40,
            GeneratorType::Biomass => 0.55,
            GeneratorType::HydroDam => 0.70,
            GeneratorType::PumpedStorage => 0.75,
            GeneratorType::BatteryStorage => 0.85,
            GeneratorType::TidalGenerator => 0.80,
            GeneratorType::WaveEnergy => 0.85,
        }
    }

    pub fn get_opinion_change_rate(&self) -> f64 {
        match *self {
            GeneratorType::OnshoreWind => 0.002,
            GeneratorType::OffshoreWind => 0.003,
            GeneratorType::DomesticSolar => 0.001,
            GeneratorType::CommercialSolar => 0.002,
            GeneratorType::UtilitySolar => 0.002,
            GeneratorType::Nuclear => 0.008,
            GeneratorType::CoalPlant => -0.015,
            GeneratorType::GasCombinedCycle => -0.008,
            GeneratorType::GasPeaker => -0.008,
            GeneratorType::Biomass => 0.001,
            GeneratorType::HydroDam => 0.001,
            GeneratorType::PumpedStorage => 0.002,
            GeneratorType::BatteryStorage => 0.004,
            GeneratorType::TidalGenerator => 0.005,
            GeneratorType::WaveEnergy => 0.005,
        }
    }

    pub fn get_base_cost(&self, year: u32) -> f64 {
        let base_cost = match *self {
            GeneratorType::OnshoreWind => 1_000_000.0,
            GeneratorType::OffshoreWind => 2_000_000.0,
            GeneratorType::DomesticSolar => 10_000.0,
            GeneratorType::CommercialSolar => 100_000.0,
            GeneratorType::UtilitySolar => 1_000_000.0,
            GeneratorType::Nuclear => 5_000_000_000.0,
            GeneratorType::CoalPlant => 2_000_000_000.0,
            GeneratorType::GasCombinedCycle => 1_000_000_000.0,
            GeneratorType::GasPeaker => 500_000_000.0,
            GeneratorType::Biomass => 800_000_000.0,
            GeneratorType::HydroDam => 3_000_000_000.0,
            GeneratorType::PumpedStorage => 2_000_000_000.0,
            GeneratorType::BatteryStorage => 500_000_000.0,
            GeneratorType::TidalGenerator => 1_500_000_000.0,
            GeneratorType::WaveEnergy => 1_000_000_000.0,
        };

        let years_from_base = (year - BASE_YEAR) as f64;
        let evolution_rate = self.get_cost_evolution_rate();
        base_cost * evolution_rate.powf(years_from_base)
    }

    pub fn get_base_power(&self, year: u32) -> f64 {
        match *self {
            GeneratorType::OnshoreWind => 3.0,
            GeneratorType::OffshoreWind => 5.0,
            GeneratorType::DomesticSolar => 0.005,
            GeneratorType::CommercialSolar => 0.05,
            GeneratorType::UtilitySolar => 2.0,
            GeneratorType::Nuclear => 1000.0,
            GeneratorType::CoalPlant => 500.0,
            GeneratorType::GasCombinedCycle => 400.0,
            GeneratorType::GasPeaker => 100.0,
            GeneratorType::Biomass => 50.0,
            GeneratorType::HydroDam => 200.0,
            GeneratorType::PumpedStorage => 300.0,
            GeneratorType::BatteryStorage => 100.0,
            GeneratorType::TidalGenerator => 20.0,
            GeneratorType::WaveEnergy => 10.0,
        }
    }

    pub fn get_operating_cost(&self, year: u32) -> f64 {
        let base_cost = match *self {
            GeneratorType::OnshoreWind => 50_000.0,
            GeneratorType::OffshoreWind => 100_000.0,
            GeneratorType::DomesticSolar => 1_000.0,
            GeneratorType::CommercialSolar => 5_000.0,
            GeneratorType::UtilitySolar => 50_000.0,
            GeneratorType::Nuclear => 200_000_000.0,
            GeneratorType::CoalPlant => 100_000_000.0,
            GeneratorType::GasCombinedCycle => 50_000_000.0,
            GeneratorType::GasPeaker => 20_000_000.0,
            GeneratorType::Biomass => 120_000.0,
            GeneratorType::HydroDam => 30_000_000.0,
            GeneratorType::PumpedStorage => 40_000_000.0,
            GeneratorType::BatteryStorage => 10_000_000.0,
            GeneratorType::TidalGenerator => 30_000_000.0,
            GeneratorType::WaveEnergy => 20_000_000.0,
        };

        let years_from_base = (year - BASE_YEAR) as f64;
        let evolution_rate = self.get_cost_evolution_rate();
        base_cost * evolution_rate.powf(years_from_base)
    }

    pub fn get_lifespan(&self) -> u32 {
        match *self {
            GeneratorType::OnshoreWind => 25,
            GeneratorType::OffshoreWind => 25,
            GeneratorType::DomesticSolar => 20,
            GeneratorType::CommercialSolar => 25,
            GeneratorType::UtilitySolar => 30,
            GeneratorType::Nuclear => 60,
            GeneratorType::CoalPlant => 40,
            GeneratorType::GasCombinedCycle => 30,
            GeneratorType::GasPeaker => 25,
            GeneratorType::Biomass => 25,
            GeneratorType::HydroDam => 100,
            GeneratorType::PumpedStorage => 80,
            GeneratorType::BatteryStorage => 15,
            GeneratorType::TidalGenerator => 25,
            GeneratorType::WaveEnergy => 20,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generator {
    pub id: String,
    pub coordinate: Coordinate,
    pub generator_type: GeneratorType,
    pub base_cost: f64,
    pub power_out: f64,
    pub base_operating_cost: f64,
    pub eol: u32,  // End of Life in years
    pub size: f64, // Between 0.1 and 1.0
    pub co2_out: f64,
    pub efficiency: f64,
    pub decommission_cost: f64,
    pub commissioning_year: u32,
    pub is_active: bool,
    pub operation_percentage: f64,
    pub upgrade_history: Vec<(u32, f64)>, // Year -> New efficiency pairs
    pub storage: Option<PowerStorageSystem>,  // New field for storage capabilities
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
        decommission_cost: f64,
    ) -> Self {
        let size = size.clamp(MIN_GENERATOR_SIZE, MAX_GENERATOR_SIZE);
        let storage = if generator_type.is_storage() {
            Some(PowerStorageSystem::new(power_out * size))
        } else {
            None
        };
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
            decommission_cost,
            commissioning_year: 0,
            operation_percentage: 1.0,
            storage,
            is_active: true,
            upgrade_history: Vec::new(),
        }
    }

    pub fn get_current_power_output(&self, hour: Option<u8>) -> f64 {
        if !self.is_active {
            return 0.0;
        }

        let base_output = self.power_out * self.efficiency * self.operation_percentage;

        if let Some(hour) = hour {
            if self.generator_type.is_intermittent() {
                self.calculate_intermittent_output(hour)
            } else {
                base_output
            }
        } else {
            // When no hour is provided, use average output
            if self.generator_type.is_intermittent() {
                match self.generator_type {
                    GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => {
                        base_output * 0.35  // Average wind capacity factor
                    },
                    GeneratorType::UtilitySolar |
                    GeneratorType::CommercialSolar |
                    GeneratorType::DomesticSolar => {
                        base_output * 0.20  // Average solar capacity factor
                    },
                    _ => base_output,
                }
            } else {
                base_output
            }
        }
    }

    fn calculate_intermittent_output(&self, hour: u8) -> f64 {
        let base_output = self.power_out * self.efficiency * self.operation_percentage;
        match self.generator_type {
            GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => {
                // Simple wind pattern with higher output at night
                let hour_factor = if hour < 6 || hour > 18 { 1.2 } else { 0.8 };
                base_output * hour_factor
            },
            GeneratorType::DomesticSolar |
            GeneratorType::CommercialSolar |
            GeneratorType::UtilitySolar => {
                // Solar output peaks at noon
                let hour_f = hour as f64;
                let solar_factor = if hour >= 6 && hour <= 18 {
                    (1.0 - ((hour_f - 12.0) / 6.0).powi(2)).max(0.0)
                } else {
                    0.0
                };
                base_output * solar_factor
            },
            _ => base_output,
        }
    }

    pub fn get_storage_system(&mut self) -> Option<&mut PowerStorageSystem> {
        self.storage.as_mut()
    }

    pub fn get_storage_capacity(&self) -> f64 {
        self.storage.as_ref().map_or(0.0, |s| s.capacity)
    }

    pub fn get_current_cost(&self, year: u32) -> f64 {
        calc_generator_cost(
            &self.generator_type,
            self.base_cost,
            year,
            self.generator_type.is_intermittent(),
            self.generator_type.requires_water(),
            self.generator_type.can_be_urban()
        )
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

    pub fn get_co2_output(&self) -> f64 {
        if !self.is_active() {
            return 0.0;
        }
        self.power_out * self.efficiency * self.operation_percentage * (1.0 - self.efficiency)
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

    pub fn adjust_operation(&mut self, new_percentage: u8, constraints: &GeneratorConstraints) -> bool {
        let min_percentage = match self.generator_type {
            GeneratorType::Nuclear => 60, // Nuclear needs high base load
            GeneratorType::HydroDam | GeneratorType::PumpedStorage => 20, // Flexible operation
            GeneratorType::OnshoreWind | GeneratorType::OffshoreWind |
            GeneratorType::UtilitySolar => 0, // Can be fully curtailed
            _ => 30, // Default minimum for other types
        };
        
        let clamped_percentage = new_percentage.clamp(min_percentage, 100);
        
        if !self.is_active {
            return false;
        }

        self.operation_percentage = clamped_percentage as f64 / 100.0;
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

    pub fn get_operation_percentage(&self) -> u8 {
        (self.operation_percentage * 100.0) as u8
    }

    pub fn get_min_operation_percentage(&self) -> u8 {
        match self.generator_type {
            GeneratorType::Nuclear => 60, // Nuclear needs high base load
            GeneratorType::HydroDam | GeneratorType::PumpedStorage => 20, // Flexible operation
            GeneratorType::OnshoreWind | GeneratorType::OffshoreWind |
            GeneratorType::UtilitySolar => 0, // Can be fully curtailed
            _ => 30, // Default minimum for other types
        }
    }

    pub fn get_generator_type(&self) -> &GeneratorType {
        &self.generator_type
    }

    pub fn get_size(&self) -> f64 {
        self.size
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