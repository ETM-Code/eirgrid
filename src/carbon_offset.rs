use serde::{Deserialize, Serialize};
use crate::poi::{POI, Coordinate};
use crate::const_funcs::calc_inflation_factor;
use std::str::FromStr;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            CarbonOffsetType::ActiveCapture => size * 0.5, // 0.5 MW per ton of capture capacity
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
        }
    }

    pub fn get_current_cost(&self, year: u32) -> f64 {
        let inflation = calc_inflation_factor(year);
        let technology_factor = match self.offset_type {
            CarbonOffsetType::ActiveCapture => 0.95f64, // 5% cost reduction per year
            CarbonOffsetType::Forest => 1.0f64,        // Stable costs
            CarbonOffsetType::CarbonCredit => 1.03f64, // 3% increase as credits become scarcer
            CarbonOffsetType::Wetland => 1.01f64,      // 1% increase due to land costs
        };
        
        let years_from_base = (year - 2025) as f64;
        self.base_cost * inflation * technology_factor.powf(years_from_base)
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
        let base_offset = match self.offset_type {
            CarbonOffsetType::Forest => self.size * 5.0,      // 5 tons per hectare per year
            CarbonOffsetType::ActiveCapture => self.size,     // Direct capture capacity in tons
            CarbonOffsetType::CarbonCredit => self.size,      // Direct offset in tons
            CarbonOffsetType::Wetland => self.size * 8.0,     // 8 tons per hectare per year
        };

        let maturity_factor = match self.offset_type {
            CarbonOffsetType::Forest | CarbonOffsetType::Wetland => {
                // Natural solutions take time to mature
                let years_from_start = (year - 2025) as f64;
                (1.0 - (-0.1 * years_from_start).exp()).clamp(0.0, 1.0)
            },
            _ => 1.0, // Other solutions work at full capacity immediately
        };

        base_offset * self.capture_efficiency * maturity_factor
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
}

impl POI for CarbonOffset {
    fn get_coordinate(&self) -> &Coordinate {
        &self.coordinate
    }

    fn get_id(&self) -> &str {
        &self.id
    }
} 