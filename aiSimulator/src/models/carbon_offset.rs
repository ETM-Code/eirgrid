use serde::{Deserialize, Serialize};
use crate::data::poi::{POI, Coordinate};
use crate::config::const_funcs::calc_inflation_factor;
use std::str::FromStr;
use std::fmt;
use crate::config::const_funcs::{calc_carbon_offset_planning_time, calc_carbon_offset_construction_time};
use crate::config::constants::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CarbonOffsetType {
    Forest,              // Trees and natural carbon sinks
    ActiveCapture,       // Mechanical carbon capture
    CarbonCredit,       // Carbon credit purchases
    Wetland,            // Wetland restoration
}

impl FromStr for CarbonOffsetType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Forest" => Ok(CarbonOffsetType::Forest),
            "ActiveCapture" => Ok(CarbonOffsetType::ActiveCapture),
            "CarbonCredit" => Ok(CarbonOffsetType::CarbonCredit),
            "Wetland" => Ok(CarbonOffsetType::Wetland),
            _ => Err(format!("Unknown carbon offset type: {}", s)),
        }
    }
}

impl fmt::Display for CarbonOffsetType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CarbonOffsetType::Forest => write!(f, "Forest"),
            CarbonOffsetType::ActiveCapture => write!(f, "ActiveCapture"),
            CarbonOffsetType::CarbonCredit => write!(f, "CarbonCredit"),
            CarbonOffsetType::Wetland => write!(f, "Wetland"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstructionStatus {
    Planned,                // Initial state, waiting for planning permission
    PlanningPermissionGranted, // Planning permission granted, waiting for construction
    UnderConstruction,      // Currently being constructed
    Operational,            // Construction complete, offset operational
    Decommissioned,         // Offset has been decommissioned
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonOffset {
    id: String,
    coordinate: Coordinate,
    offset_type: CarbonOffsetType,
    base_cost: f64,
    base_operating_cost: f64,
    size: f64,                // Size in hectares for natural solutions, capacity in tons for active capture
    capture_efficiency: f64,   // Efficiency factor (0.0 to 1.0)
    power_consumption: f64,    // MW of power consumed (only for active capture)
    
    // Construction status fields
    construction_status: ConstructionStatus,
    planning_permission_time: f64,  // Time in years for planning permission
    construction_time: f64,         // Time in years for construction
    planning_permission_year: u32,  // Year planning permission was granted
    construction_start_year: u32,   // Year construction started
    construction_complete_year: u32, // Year construction completed
    commissioning_year: u32,        // Year the offset was commissioned (planned)
    
    // Cost multiplier for construction speedup
    construction_cost_multiplier: f64,
}

impl CarbonOffset {
    pub fn new(
        id: String,
        coordinate: Coordinate,
        offset_type: CarbonOffsetType,
        base_cost: f64,
        base_operating_cost: f64,
        size: f64,
        capture_efficiency: f64,
    ) -> Self {
        let power_consumption = match offset_type {
            CarbonOffsetType::ActiveCapture => size * ACTIVE_CAPTURE_POWER_PER_UNIT,
            _ => 0.0,
        };

        Self {
            id,
            coordinate,
            offset_type,
            base_cost,
            base_operating_cost,
            size,
            capture_efficiency: capture_efficiency.clamp(0.0, 1.0),
            power_consumption,
            
            // Construction status fields
            construction_status: ConstructionStatus::Planned,
            planning_permission_time: 0.0,
            construction_time: 0.0,
            planning_permission_year: 0,
            construction_start_year: 0,
            construction_complete_year: 0,
            commissioning_year: 0,
            
            // Cost multiplier for construction speedup
            construction_cost_multiplier: 1.0,
        }
    }
    
    // New method to initialize construction times
    pub fn initialize_construction(&mut self, year: u32, public_opinion: f64, enable_delays: bool) {
        self.commissioning_year = year;
        
        if !enable_delays {
            // If delays are disabled, set the offset to operational immediately
            self.construction_status = ConstructionStatus::Operational;
            self.planning_permission_year = year;
            self.construction_start_year = year;
            self.construction_complete_year = year;
            return;
        }
        
        // Calculate planning permission time with cost multiplier
        self.planning_permission_time = calc_carbon_offset_planning_time(
            &self.offset_type, 
            year, 
            public_opinion,
            self.construction_cost_multiplier
        );
        
        // Calculate construction time with cost multiplier
        self.construction_time = calc_carbon_offset_construction_time(
            &self.offset_type, 
            year,
            self.construction_cost_multiplier
        );
        
        // Set initial status to Planned
        self.construction_status = ConstructionStatus::Planned;
    }
    
    // New method to update construction status based on current year
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
                    return true;
                }
            },
            _ => {}
        }
        
        false
    }
    
    // Check if the offset is operational
    pub fn is_operational(&self) -> bool {
        self.construction_status == ConstructionStatus::Operational
    }

    pub fn get_current_cost(&self, year: u32) -> f64 {
        // Calculate base cost with inflation
        let inflation_factor = (1.0 + INFLATION_RATE).powi((year - BASE_YEAR) as i32);
        let base_cost = self.base_cost * inflation_factor;
        
        // Apply the construction cost multiplier
        base_cost * self.construction_cost_multiplier
    }

    pub fn get_current_operating_cost(&self, year: u32) -> f64 {
        let inflation = calc_inflation_factor(year);
        let efficiency_factor = match self.offset_type {
            CarbonOffsetType::ActiveCapture => 0.97f64, // 3% efficiency improvement
            CarbonOffsetType::Forest => 1.0f64,        // Stable maintenance costs
            CarbonOffsetType::CarbonCredit => 1.02f64, // 2% increase in verification costs
            CarbonOffsetType::Wetland => 1.01f64,      // 1% increase in maintenance
        };
        
        let years_from_base = (year - 2025) as f64;
        self.base_operating_cost * inflation * efficiency_factor.powf(years_from_base)
    }

    pub fn calc_carbon_offset(&self, year: u32) -> f64 {
        // If not operational, no carbon offset
        if !self.is_operational() {
            return 0.0;
        }
        
        let base_offset = match self.offset_type {
            CarbonOffsetType::Forest => self.size * FOREST_SEQUESTRATION_RATE,
            CarbonOffsetType::ActiveCapture => self.size * ACTIVE_CAPTURE_MULTIPLIER,
            CarbonOffsetType::CarbonCredit => self.size * CARBON_CREDIT_MULTIPLIER,
            CarbonOffsetType::Wetland => self.size * WETLAND_SEQUESTRATION_RATE,
        };

        let maturity_factor = match self.offset_type {
            CarbonOffsetType::Forest | CarbonOffsetType::Wetland => {
                // Natural solutions take time to mature
                let years_from_start = (year - self.construction_complete_year) as f64;
                (1.0 - (CARBON_OFFSET_MATURITY_FACTOR * years_from_start).exp()).clamp(0.0, 1.0)
            },
            _ => 1.0, // Other solutions work at full capacity immediately
        };

        // Calculate construction progress factor
        let construction_progress = match self.construction_status {
            ConstructionStatus::UnderConstruction => {
                let years_since_construction_start = (year - self.construction_start_year) as f64;
                (years_since_construction_start / self.construction_time).clamp(0.0, 1.0)
            },
            ConstructionStatus::PlanningPermissionGranted => 0.0,
            ConstructionStatus::Planned => 0.0,
            ConstructionStatus::Operational => 1.0,
            ConstructionStatus::Decommissioned => 0.0,
        };

        // For natural solutions, scale effectiveness during construction
        let effectiveness_factor = match self.offset_type {
            CarbonOffsetType::Forest | CarbonOffsetType::Wetland => {
                // Start at 20% effectiveness and increase linearly during construction
                0.2 + (0.8 * construction_progress)
            },
            _ => {
                // For other types, only start working at full capacity when operational
                if self.construction_status == ConstructionStatus::Operational {
                    1.0
                } else {
                    0.0
                }
            }
        };

        base_offset * self.capture_efficiency * maturity_factor * effectiveness_factor
    }

    pub fn get_power_consumption(&self) -> f64 {
        self.power_consumption
    }

    pub fn get_start_year(&self) -> u32 {
        if self.id.contains("_") {
            let parts: Vec<&str> = self.id.split('_').collect();
            if parts.len() >= 2 {
                if let Ok(year) = parts[1].parse::<u32>() {
                    return year;
                }
            }
        }
        2025
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
            self.planning_permission_time = calc_carbon_offset_planning_time(
                &self.offset_type, 
                self.commissioning_year, 
                default_opinion,
                self.construction_cost_multiplier
            );
            
            // Recalculate construction time
            self.construction_time = calc_carbon_offset_construction_time(
                &self.offset_type, 
                self.commissioning_year,
                self.construction_cost_multiplier
            );
        }
    }

    // Get the construction cost multiplier
    pub fn get_construction_cost_multiplier(&self) -> f64 {
        self.construction_cost_multiplier
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }
}

impl POI for CarbonOffset {
    fn get_coordinate(&self) -> &Coordinate {
        &self.coordinate
    }

    fn get_id(&self) -> &str {
        &self.id
    }
} 