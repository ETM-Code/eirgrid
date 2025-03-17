use std::fmt;
use serde::{Deserialize, Serialize};
use crate::data::poi::{POI, Coordinate};
use crate::config::constants::*;
use crate::config::const_funcs::{calc_generator_cost, calc_operating_cost, calc_cost_opinion, calc_type_opinion, calc_planning_permission_time, calc_construction_time};
use crate::config::simulation_config::GeneratorConstraints;
use super::power_storage::PowerStorageSystem;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl FromStr for GeneratorType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "OnshoreWind" => Ok(GeneratorType::OnshoreWind),
            "OffshoreWind" => Ok(GeneratorType::OffshoreWind),
            "DomesticSolar" => Ok(GeneratorType::DomesticSolar),
            "CommercialSolar" => Ok(GeneratorType::CommercialSolar),
            "UtilitySolar" => Ok(GeneratorType::UtilitySolar),
            "Nuclear" => Ok(GeneratorType::Nuclear),
            "CoalPlant" => Ok(GeneratorType::CoalPlant),
            "GasCombinedCycle" => Ok(GeneratorType::GasCombinedCycle),
            "GasPeaker" => Ok(GeneratorType::GasPeaker),
            "Biomass" => Ok(GeneratorType::Biomass),
            "HydroDam" => Ok(GeneratorType::HydroDam),
            "PumpedStorage" => Ok(GeneratorType::PumpedStorage),
            "BatteryStorage" => Ok(GeneratorType::BatteryStorage),
            "TidalGenerator" => Ok(GeneratorType::TidalGenerator),
            "WaveEnergy" => Ok(GeneratorType::WaveEnergy),
            _ => Err(format!("Unknown generator type: {}", s)),
        }
    }
}

impl fmt::Display for GeneratorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
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
            GeneratorType::OnshoreWind => (ONSHORE_WIND_MIN_SIZE, MAX_GENERATOR_SIZE),
            GeneratorType::OffshoreWind => (OFFSHORE_WIND_MIN_SIZE, MAX_GENERATOR_SIZE),
            
            // Solar constraints
            GeneratorType::DomesticSolar => (DOMESTIC_SOLAR_MIN_SIZE, DOMESTIC_SOLAR_MAX_SIZE),
            GeneratorType::CommercialSolar => (COMMERCIAL_SOLAR_MIN_SIZE, COMMERCIAL_SOLAR_MAX_SIZE),
            GeneratorType::UtilitySolar => (UTILITY_SOLAR_MIN_SIZE, MAX_GENERATOR_SIZE),
            
            // Nuclear constraints
            GeneratorType::Nuclear => (NUCLEAR_MIN_SIZE, MAX_GENERATOR_SIZE),
            
            // Fossil fuel constraints
            GeneratorType::CoalPlant => (COAL_MIN_SIZE, MAX_GENERATOR_SIZE),
            GeneratorType::GasCombinedCycle => (GAS_CC_MIN_SIZE, MAX_GENERATOR_SIZE),
            GeneratorType::GasPeaker => (GAS_PEAKER_MIN_SIZE, GAS_PEAKER_MAX_SIZE),
            GeneratorType::Biomass => (BIOMASS_MIN_SIZE, MAX_GENERATOR_SIZE),
            
            // Hydro constraints
            GeneratorType::HydroDam => (HYDRO_MIN_SIZE, MAX_GENERATOR_SIZE),
            GeneratorType::PumpedStorage => (PUMPED_STORAGE_MIN_SIZE, PUMPED_STORAGE_MAX_SIZE),
            GeneratorType::TidalGenerator => (TIDAL_MIN_SIZE, TIDAL_MAX_SIZE),
            GeneratorType::WaveEnergy => (WAVE_MIN_SIZE, WAVE_MAX_SIZE),
            GeneratorType::BatteryStorage => (BATTERY_MIN_SIZE, BATTERY_MAX_SIZE),
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
            GeneratorType::OnshoreWind => 1.0,
            GeneratorType::OffshoreWind => 1.0,
            GeneratorType::DomesticSolar => 1.0,
            GeneratorType::CommercialSolar => 1.0,
            GeneratorType::UtilitySolar => 1.0,
            GeneratorType::Nuclear => 1.0,
            GeneratorType::CoalPlant => 1.0,
            GeneratorType::GasCombinedCycle => 1.0,
            GeneratorType::GasPeaker => 1.0,
            GeneratorType::Biomass => 1.0,
            GeneratorType::HydroDam => 1.0,
            GeneratorType::PumpedStorage => 1.0,
            GeneratorType::BatteryStorage => 1.0,
            GeneratorType::TidalGenerator => {
                // Efficiency improves significantly over time as technology matures
                let years_from_base = (year - BASE_YEAR) as f64;
                1.0 + (years_from_base * 0.01).min(0.40) // Starts at 20%, can reach 60%
            },
            GeneratorType::WaveEnergy => {
                // Similar to tidal but starts lower
                let years_from_base = (year - BASE_YEAR) as f64;
                1.0 + (years_from_base * 0.01).min(0.35) // Starts at 15%, can reach 50%
            },
        }
    }

    pub fn get_cost_evolution_rate(&self) -> f64 {
        match *self {
            GeneratorType::OnshoreWind => WIND_COST_REDUCTION,
            GeneratorType::OffshoreWind => WIND_COST_REDUCTION,
            GeneratorType::DomesticSolar => SOLAR_COST_REDUCTION,
            GeneratorType::CommercialSolar => SOLAR_COST_REDUCTION,
            GeneratorType::UtilitySolar => SOLAR_COST_REDUCTION,
            GeneratorType::Nuclear => NUCLEAR_COST_REDUCTION,
            GeneratorType::CoalPlant => COAL_COST_INCREASE,
            GeneratorType::GasCombinedCycle => GAS_COST_INCREASE,
            GeneratorType::GasPeaker => GAS_COST_INCREASE,
            GeneratorType::Biomass => 0.99,
            GeneratorType::HydroDam => HYDRO_COST_INCREASE,
            GeneratorType::PumpedStorage => HYDRO_COST_INCREASE,
            GeneratorType::BatteryStorage => 0.97,
            GeneratorType::TidalGenerator => 0.95,
            GeneratorType::WaveEnergy => 0.95,
        }
    }

    pub fn get_base_opinion(&self) -> f64 {
        match *self {
            GeneratorType::OnshoreWind => WIND_BASE_OPINION,
            GeneratorType::OffshoreWind => WIND_BASE_OPINION,
            GeneratorType::DomesticSolar => SOLAR_BASE_OPINION,
            GeneratorType::CommercialSolar => SOLAR_BASE_OPINION,
            GeneratorType::UtilitySolar => SOLAR_BASE_OPINION,
            GeneratorType::Nuclear => NUCLEAR_BASE_OPINION,
            GeneratorType::CoalPlant => COAL_BASE_OPINION,
            GeneratorType::GasCombinedCycle => GAS_BASE_OPINION,
            GeneratorType::GasPeaker => GAS_BASE_OPINION,
            GeneratorType::Biomass => BIOMASS_BASE_OPINION,
            GeneratorType::HydroDam => HYDRO_BASE_OPINION,
            GeneratorType::PumpedStorage => PUMPED_STORAGE_BASE_OPINION,
            GeneratorType::BatteryStorage => BATTERY_BASE_OPINION,
            GeneratorType::TidalGenerator => TIDAL_BASE_OPINION,
            GeneratorType::WaveEnergy => WAVE_BASE_OPINION,
        }
    }

    pub fn get_opinion_change_rate(&self) -> f64 {
        match *self {
            GeneratorType::OnshoreWind => WIND_OPINION_CHANGE,
            GeneratorType::OffshoreWind => WIND_OPINION_CHANGE,
            GeneratorType::DomesticSolar => SOLAR_OPINION_CHANGE,
            GeneratorType::CommercialSolar => SOLAR_OPINION_CHANGE,
            GeneratorType::UtilitySolar => SOLAR_OPINION_CHANGE,
            GeneratorType::Nuclear => NUCLEAR_OPINION_CHANGE,
            GeneratorType::CoalPlant => COAL_OPINION_CHANGE,
            GeneratorType::GasCombinedCycle => GAS_OPINION_CHANGE,
            GeneratorType::GasPeaker => GAS_OPINION_CHANGE,
            GeneratorType::Biomass => BIOMASS_OPINION_CHANGE,
            GeneratorType::HydroDam => HYDRO_OPINION_CHANGE,
            GeneratorType::PumpedStorage => PUMPED_STORAGE_OPINION_CHANGE,
            GeneratorType::BatteryStorage => MARINE_OPINION_CHANGE,
            GeneratorType::TidalGenerator => TIDAL_OPINION_CHANGE,
            GeneratorType::WaveEnergy => WAVE_OPINION_CHANGE,
        }
    }

    pub fn get_base_cost(&self, year: u32) -> f64 {
        let base_cost = match *self {
            // Onshore wind: assume ~€1.5M per MW; for a 500 MW plant, that's roughly €750M total.
            GeneratorType::OnshoreWind => 1_500_000.0,        // per MW basis

            // Offshore wind: assume ~€4M per MW; for an 800 MW plant, roughly €3.2B total.
            GeneratorType::OffshoreWind => 4_000_000.0,       

            // Domestic solar: for a small (10 MW) rooftop-type installation, about €1M per MW.
            GeneratorType::DomesticSolar => 10_000_000.0,       // 10 MW * €1M/MW

            // Commercial solar: assume ~€800k per MW; for 50 MW, ~€40M.
            GeneratorType::CommercialSolar => 40_000_000.0,     

            // Utility-scale solar: similarly, ~€800k per MW; for 300 MW, ~€240M.
            GeneratorType::UtilitySolar => 240_000_000.0,       

            // Nuclear: while no new builds exist in Ireland, recent European projects can be in the range of
            // ~€8–10M per MW; here we assume €10M per MW, so for 1500 MW, roughly €15B.
            GeneratorType::Nuclear => 15_000_000_000.0,         

            // Coal plants: new-build coal plants (without CCS) might run about €1.5M per MW.
            GeneratorType::CoalPlant => 1_500_000_000.0,        

            // Gas combined cycle (CCGT): competitive new builds have come in around €700k per MW.
            GeneratorType::GasCombinedCycle => 560_000_000.0,     

            // Gas peakers are usually more expensive per MW due to lower capacity factors;
            // here roughly €1.25M per MW is assumed.
            GeneratorType::GasPeaker => 500_000_000.0,            

            // Biomass: typical estimates are on the order of €3M per MW; for 50 MW, about €150M.
            GeneratorType::Biomass => 150_000_000.0,              

            // Hydro dams: large civil works mean costs around €2–2.5M per MW; for 1200 MW, roughly €2.5B.
            GeneratorType::HydroDam => 2_500_000_000.0,           

            // Pumped storage: often around €2M per MW; for 600 MW, about €1.2B.
            GeneratorType::PumpedStorage => 1_200_000_000.0,      

            // Battery storage: recent utility‐scale battery installations can be around €300k per MW,
            // so for 500 MW, roughly €150M.
            GeneratorType::BatteryStorage => 150_000_000.0,       

            // Tidal generators: as a less mature technology, costs might be around €5M per MW; for 200 MW, ~€1B.
            GeneratorType::TidalGenerator => 1_000_000_000.0,     

            // Wave energy: typically higher on a per MW basis – around €8M per MW; for 100 MW, ~€800M.
            GeneratorType::WaveEnergy => 800_000_000.0,
        };

        let years_from_base = (year - BASE_YEAR) as f64;
        let evolution_rate = self.get_cost_evolution_rate();
        base_cost * evolution_rate.powf(years_from_base)
    }

    pub fn get_base_power(&self, __year: u32) -> f64 {
        match *self {
            GeneratorType::OnshoreWind => MAX_ONSHORE_WIND_POWER,
            GeneratorType::OffshoreWind => MAX_OFFSHORE_WIND_POWER,
            GeneratorType::DomesticSolar => MAX_DOMESTIC_SOLAR_POWER,
            GeneratorType::CommercialSolar => MAX_COMMERCIAL_SOLAR_POWER,
            GeneratorType::UtilitySolar => MAX_UTILITY_SOLAR_POWER,
            GeneratorType::Nuclear => MAX_NUCLEAR_POWER,
            GeneratorType::CoalPlant => MAX_COAL_POWER,
            GeneratorType::GasCombinedCycle => MAX_GAS_CC_POWER,
            GeneratorType::GasPeaker => MAX_GAS_PEAKER_POWER,
            GeneratorType::Biomass => MAX_BIOMASS_POWER,
            GeneratorType::HydroDam => MAX_HYDRO_DAM_POWER,
            GeneratorType::PumpedStorage => MAX_PUMPED_STORAGE_POWER,
            GeneratorType::BatteryStorage => MAX_BATTERY_STORAGE_POWER,
            GeneratorType::TidalGenerator => MAX_TIDAL_POWER,
            GeneratorType::WaveEnergy => MAX_WAVE_POWER,
        }
    }

    pub fn get_operating_cost(&self, year: u32) -> f64 {
        let base_cost = match *self {
            GeneratorType::OnshoreWind => ONSHORE_WIND_OPERATING_COST,
            GeneratorType::OffshoreWind => OFFSHORE_WIND_OPERATING_COST,
            GeneratorType::DomesticSolar => DOMESTIC_SOLAR_OPERATING_COST,
            GeneratorType::CommercialSolar => UTILITY_SOLAR_OPERATING_COST,
            GeneratorType::UtilitySolar => UTILITY_SOLAR_OPERATING_COST,
            GeneratorType::Nuclear => NUCLEAR_OPERATING_COST,
            GeneratorType::CoalPlant => COAL_OPERATING_COST,
            GeneratorType::GasCombinedCycle => GAS_CC_OPERATING_COST,
            GeneratorType::GasPeaker => GAS_PEAKER_OPERATING_COST,
            GeneratorType::Biomass => BIOMASS_OPERATING_COST,
            GeneratorType::HydroDam => HYDRO_DAM_OPERATING_COST,
            GeneratorType::PumpedStorage => PUMPED_STORAGE_OPERATING_COST,
            GeneratorType::BatteryStorage => BATTERY_STORAGE_OPERATING_COST,
            GeneratorType::TidalGenerator => TIDAL_OPERATING_COST,
            GeneratorType::WaveEnergy => WAVE_OPERATING_COST,
        };

        let years_from_base = (year - BASE_YEAR) as f64;
        let evolution_rate = self.get_cost_evolution_rate();
        base_cost * evolution_rate.powf(years_from_base)
    }

    pub fn get_lifespan(&self) -> u32 {
        match *self {
            GeneratorType::OnshoreWind => 25,
            GeneratorType::OffshoreWind => 25,
            GeneratorType::DomesticSolar => 25,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstructionStatus {
    Planned,                // Initial state, waiting for planning permission
    PlanningPermissionGranted, // Planning permission granted, waiting for construction
    UnderConstruction,      // Currently being constructed
    Operational,            // Construction complete, generator operational
    Decommissioned,         // Generator has been decommissioned
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
    
    // Construction status fields
    pub construction_status: ConstructionStatus,
    pub planning_permission_time: f64,  // Time in years for planning permission
    pub construction_time: f64,         // Time in years for construction
    pub planning_permission_year: u32,  // Year planning permission was granted
    pub construction_start_year: u32,   // Year construction started
    pub construction_complete_year: u32, // Year construction completed
    
    // Cost multiplier for construction speedup
    pub construction_cost_multiplier: f64,
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
            efficiency: BASE_EFFICIENCY,
            decommission_cost,
            commissioning_year: 0,
            operation_percentage: 1.0,
            storage,
            is_active: true,
            upgrade_history: Vec::new(),
            construction_status: ConstructionStatus::Planned,
            planning_permission_time: 0.0,
            construction_time: 0.0,
            planning_permission_year: 0,
            construction_start_year: 0,
            construction_complete_year: 0,
            construction_cost_multiplier: 1.0,
        }
    }

    pub fn initialize_construction(&mut self, year: u32, public_opinion: f64, enable_delays: bool) {
        self.commissioning_year = year;
        
        if !enable_delays {
            // If delays are disabled, set the generator to operational immediately
            self.construction_status = ConstructionStatus::Operational;
            self.planning_permission_year = year;
            self.construction_start_year = year;
            self.construction_complete_year = year;
            return;
        }
        
        // Calculate planning permission time with cost multiplier
        self.planning_permission_time = calc_planning_permission_time(
            &self.generator_type, 
            year, 
            public_opinion,
            self.construction_cost_multiplier
        );
        
        // Calculate construction time with cost multiplier
        self.construction_time = calc_construction_time(
            &self.generator_type, 
            year,
            self.construction_cost_multiplier
        );
        
        // Set initial status to Planned
        self.construction_status = ConstructionStatus::Planned;
    }

    pub fn update_construction_status(&mut self, current_year: u32) -> bool {
        // If already operational or decommissioned, no change needed
        if self.construction_status == ConstructionStatus::Operational || 
           self.construction_status == ConstructionStatus::Decommissioned {
            return false;
        }
        
        let years_since_commissioning = (current_year - self.commissioning_year) as f64;
        
        match self.construction_status {
            ConstructionStatus::Planned => {
                if years_since_commissioning >= self.planning_permission_time {
                    self.construction_status = ConstructionStatus::PlanningPermissionGranted;
                    self.planning_permission_year = current_year;
                    return true;
                }
            },
            ConstructionStatus::PlanningPermissionGranted => {
                self.construction_status = ConstructionStatus::UnderConstruction;
                self.construction_start_year = current_year;
                return true;
            },
            ConstructionStatus::UnderConstruction => {
                let years_since_construction_start = (current_year - self.construction_start_year) as f64;
                if years_since_construction_start >= self.construction_time {
                    self.construction_status = ConstructionStatus::Operational;
                    self.construction_complete_year = current_year;
                    self.is_active = true;
                    return true;
                }
            },
            _ => {}
        }
        
        false
    }

    pub fn is_active(&self) -> bool {
        self.is_active && self.construction_status == ConstructionStatus::Operational
    }

    pub fn get_current_power_output(&self, hour: Option<u8>) -> f64 {
        if !self.is_active() {
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
                        base_output * WIND_CAPACITY_FACTOR
                    },
                    GeneratorType::UtilitySolar |
                    GeneratorType::CommercialSolar |
                    GeneratorType::DomesticSolar => {
                        base_output * SOLAR_CAPACITY_FACTOR
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
                base_output
            },
            GeneratorType::DomesticSolar |
            GeneratorType::CommercialSolar |
            GeneratorType::UtilitySolar => {
                // Solar output peaks at noon
                let hour_f = hour as f64;
                let solar_factor = if hour >= NIGHT_START_HOUR && hour <= DAY_END_HOUR {
                    (1.0 - ((hour_f - SOLAR_PEAK_HOUR) / SOLAR_WINDOW).powi(2)).max(0.0)
                } else {
                    0.0
                };
                base_output * solar_factor
            },
            _ => base_output,
        }
    }

    pub fn get_storage_capacity(&self) -> f64 {
        self.storage.as_ref().map_or(0.0, |s| s.capacity)
    }

    pub fn get_current_cost(&self, year: u32) -> f64 {
        let base_cost = calc_generator_cost(
            &self.generator_type,
            self.base_cost,
            year,
            self.generator_type.can_be_urban(),
            self.generator_type.requires_water(),
            self.generator_type.requires_water()
        );
        
        // Apply the construction cost multiplier
        base_cost * self.construction_cost_multiplier
    }

    pub fn get_current_operating_cost(&self, year: u32) -> f64 {
        if !self.is_active() {
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
        
        // Using stored co2_out value (initialized from constants) and adjusting by
        // operation percentage and efficiency improvements
        self.co2_out * self.operation_percentage * (1.0 - (self.efficiency - BASE_EFFICIENCY))
    }

    pub fn can_upgrade_efficiency(&self, year: u32, constraints: &GeneratorConstraints) -> bool {
        if !self.is_active() {
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
        let upgrade_cost = self.base_cost * efficiency_increase * EFFICIENCY_UPGRADE_COST_FACTOR;
        self.efficiency = new_efficiency;
        self.upgrade_history.push((year, new_efficiency));
        upgrade_cost
    }

    pub fn adjust_operation(&mut self, new_percentage: u8, __constraints: &GeneratorConstraints) -> bool {
        let min_percentage = match self.generator_type {
            GeneratorType::Nuclear => NUCLEAR_MIN_OPERATION,
            GeneratorType::HydroDam | GeneratorType::PumpedStorage => HYDRO_MIN_OPERATION,
            GeneratorType::OnshoreWind | GeneratorType::OffshoreWind |
            GeneratorType::UtilitySolar => 0,
            _ => DEFAULT_MIN_OPERATION,
        };
        
        let clamped_percentage = new_percentage.clamp(min_percentage, MAX_OPERATION_PERCENTAGE);
        
        if !self.is_active() {
            return false;
        }

        self.operation_percentage = clamped_percentage as f64 / 100.0;
        true
    }

    pub fn close_generator(&mut self, year: u32) -> f64 {
        if !self.is_active() {
            return 0.0;
        }

        let years_remaining = (self.eol as i32 - (year - BASE_YEAR) as i32).max(0) as f64;
        let closure_cost = self.base_cost * CLOSURE_COST_FACTOR * (years_remaining / self.eol as f64);
        
        self.is_active = false;
        self.operation_percentage = 0.0;
        
        closure_cost
    }

    pub fn get_efficiency(&self) -> f64 {
        self.efficiency
    }

    pub fn get_build_year(&self) -> u32 {
        // Extract year from the ID for generators built during simulation
        if self.id.starts_with("Gen_") {
            let parts: Vec<&str> = self.id.split('_').collect();
            if parts.len() >= 3 {
                if let Ok(year) = parts[2].parse::<u32>() {
                    return year;
                }
            }
        }
        // Default to 2025 for existing generators
        2025
    }

    pub fn get_operation_percentage(&self) -> u8 {
        (self.operation_percentage * 100.0) as u8
    }

    pub fn get_min_operation_percentage(&self) -> u8 {
        match self.generator_type {
            GeneratorType::Nuclear => NUCLEAR_MIN_OPERATION,
            GeneratorType::HydroDam | GeneratorType::PumpedStorage => HYDRO_MIN_OPERATION,
            GeneratorType::OnshoreWind | GeneratorType::OffshoreWind |
            GeneratorType::UtilitySolar => 0,
            _ => DEFAULT_MIN_OPERATION,
        }
    }

    pub fn get_generator_type(&self) -> &GeneratorType {
        &self.generator_type
    }

    pub fn get_size(&self) -> f64 {
        self.size
    }

    pub fn set_construction_cost_multiplier(&mut self, multiplier: f64) {
        // Ensure the multiplier is within bounds
        self.construction_cost_multiplier = multiplier.clamp(
            MIN_CONSTRUCTION_COST_MULTIPLIER, 
            MAX_CONSTRUCTION_COST_MULTIPLIER
        );
        
        // Recalculate planning and construction times if already in planning phase
        if self.construction_status == ConstructionStatus::Planned && self.commissioning_year > 0 {
            // Use a default public opinion if we don't have access to the map
            let default_opinion = 0.65;
            
            // Recalculate planning permission time
            self.planning_permission_time = calc_planning_permission_time(
                &self.generator_type, 
                self.commissioning_year, 
                default_opinion,
                self.construction_cost_multiplier
            );
            
            // Recalculate construction time
            self.construction_time = calc_construction_time(
                &self.generator_type, 
                self.commissioning_year,
                self.construction_cost_multiplier
            );
        }
    }

    // Get the construction cost multiplier
    pub fn get_construction_cost_multiplier(&self) -> f64 {
        self.construction_cost_multiplier
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