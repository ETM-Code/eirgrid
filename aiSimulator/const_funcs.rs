use crate::generator::GeneratorType;
use crate::constants::*;
use crate::poi::Coordinate;
use serde_json;
use lazy_static::lazy_static;





pub fn calc_inflation_factor(year: u32) -> f64 {
    (1.0 + INFLATION_RATE).powi((year - BASE_YEAR) as i32)
}

pub fn calc_power_usage_per_capita(year: u32) -> f64 {
    // Base power usage per capita in 2025 (in MW)
    const BASE_USAGE: f64 = 0.001;  // 1 kW per person
    
    // Annual increase in power usage (e.g., due to increased electrification)
    const ANNUAL_INCREASE: f64 = 0.02;  // 2% increase per year
    
    let years_from_base = (year - BASE_YEAR) as f64;
    BASE_USAGE * (1.0 + ANNUAL_INCREASE).powf(years_from_base)
}

pub fn calc_generator_cost(gen_type: &GeneratorType, base_cost: f64, year: u32, is_urban: bool, is_coastal: bool, is_river: bool) -> f64 {
    let inflation = calc_inflation_factor(year);
    let years_from_base = (year - BASE_YEAR) as f64;
    
    // Get technology-specific cost evolution rate
    let cost_evolution_rate = gen_type.get_cost_evolution_rate();
    let technology_factor = cost_evolution_rate.powf(years_from_base);
    
    // Apply location-specific modifiers
    let mut location_modifier = 1.0;
    
    if is_urban {
        location_modifier *= match gen_type {
            GeneratorType::DomesticSolar |
            GeneratorType::CommercialSolar => URBAN_SOLAR_BONUS,
            GeneratorType::GasPeaker => URBAN_PEAKER_PENALTY,
            _ => 1.0,
        };
    }
    
    if gen_type.requires_water() {
        if is_coastal {
            location_modifier *= COASTAL_BONUS;
        } else if is_river {
            location_modifier *= RIVER_BONUS;
        }
    }
    
    base_cost * inflation * technology_factor * location_modifier
}

pub fn calc_operating_cost(gen_type: &GeneratorType, base_operating_cost: f64, year: u32) -> f64 {
    let inflation = calc_inflation_factor(year);
    let years_from_base = (year - BASE_YEAR) as f64;
    
    let efficiency_factor = match gen_type {
        GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => WIND_EFFICIENCY_GAIN.powf(years_from_base),
        GeneratorType::DomesticSolar | GeneratorType::CommercialSolar | GeneratorType::UtilitySolar => SOLAR_EFFICIENCY_GAIN.powf(years_from_base),
        GeneratorType::Nuclear => NUCLEAR_EFFICIENCY_GAIN.powf(years_from_base),
        GeneratorType::CoalPlant => COAL_EFFICIENCY_LOSS.powf(years_from_base),
        GeneratorType::GasCombinedCycle | GeneratorType::GasPeaker => GAS_EFFICIENCY_LOSS.powf(years_from_base),
        GeneratorType::HydroDam | GeneratorType::PumpedStorage => HYDRO_EFFICIENCY_GAIN.powf(years_from_base),
        GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => MARINE_EFFICIENCY_GAIN.powf(years_from_base),
        GeneratorType::BatteryStorage => BATTERY_EFFICIENCY_GAIN.powf(years_from_base),
        GeneratorType::Biomass => BIOMASS_EFFICIENCY_GAIN.powf(years_from_base),
    };
    
    base_operating_cost * inflation * efficiency_factor
}

pub fn calc_type_opinion(gen_type: &GeneratorType, year: u32) -> f64 {
    let _years_passed = (year - BASE_YEAR) as f64;
    let (base_opinion, annual_change) = match gen_type {
        GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => (WIND_BASE_OPINION, WIND_OPINION_CHANGE),
        GeneratorType::DomesticSolar | GeneratorType::CommercialSolar | GeneratorType::UtilitySolar => (SOLAR_BASE_OPINION, SOLAR_OPINION_CHANGE),
        GeneratorType::Nuclear => (NUCLEAR_BASE_OPINION, NUCLEAR_OPINION_CHANGE),
        GeneratorType::CoalPlant => (COAL_BASE_OPINION, COAL_OPINION_CHANGE),
        GeneratorType::GasCombinedCycle | GeneratorType::GasPeaker => (GAS_BASE_OPINION, GAS_OPINION_CHANGE),
        GeneratorType::HydroDam | GeneratorType::PumpedStorage => (HYDRO_BASE_OPINION, HYDRO_OPINION_CHANGE),
        GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => (MARINE_BASE_OPINION, MARINE_OPINION_CHANGE),
        GeneratorType::BatteryStorage => (BATTERY_BASE_OPINION, BATTERY_OPINION_CHANGE),
        GeneratorType::Biomass => (0.60, 0.001),
    };
    
    (base_opinion + annual_change * _years_passed).clamp(0.0, 1.0)
}

pub fn calc_cost_opinion(cost: f64, year: u32) -> f64 {
    let inflation_adjusted_max = REFERENCE_ANNUAL_EXPENDITURE * calc_inflation_factor(year);
    let normalized_cost = cost / inflation_adjusted_max;
    
    if normalized_cost <= 1.0 {
        // For costs below or at reference, keep linear scaling
        1.0 - normalized_cost
    } else {
        // For costs above reference, use exponential decay
        0.5 * (-0.5 * (normalized_cost - 1.0)).exp()
    }
}


pub fn calc_decommission_cost(base_cost: f64) -> f64 {
    base_cost * DECOMMISSION_COST_RATIO
}

pub fn calc_initial_co2_output(gen_type: &GeneratorType, size: f64) -> f64 {
    let base_rate = match gen_type {
        GeneratorType::CoalPlant => COAL_CO2_RATE,
        GeneratorType::GasCombinedCycle => GAS_CC_CO2_RATE,
        GeneratorType::GasPeaker => GAS_PEAKER_CO2_RATE,
        GeneratorType::Biomass => BIOMASS_CO2_RATE,
        _ => 0.0,
    };
    base_rate * size
}

pub fn transform_lat_lon_to_grid(lat: f64, lon: f64) -> Option<Coordinate> {
    if lat < IRELAND_MIN_LAT || lat > IRELAND_MAX_LAT || 
       lon < IRELAND_MIN_LON || lon > IRELAND_MAX_LON {
        return None;
    }

    // Transform using the origin and scale from constants
    // This follows the transformation matrix: [x, y] = [origin_x, origin_y] + [lon, lat] * [scale_x, scale_y]
    let x = (lon - IRELAND_MIN_LON) * GRID_SCALE_X;
    let y = (lat - IRELAND_MIN_LAT) * GRID_SCALE_Y;

    Some(Coordinate::new(x, y))
}

pub fn is_coastal_location(coordinate: &Coordinate) -> bool {
    coordinate.x < MAP_MAX_X * COASTAL_THRESHOLD
}

// Point in polygon check using ray casting algorithm
pub fn is_point_inside_polygon(point: &Coordinate, polygon: &Vec<Coordinate>) -> bool {
    let mut inside = false;
    let mut j = polygon.len() - 1;
    
    for i in 0..polygon.len() {
        if ((polygon[i].y > point.y) != (polygon[j].y > point.y)) &&
            (point.x < (polygon[j].x - polygon[i].x) * (point.y - polygon[i].y) / 
                      (polygon[j].y - polygon[i].y) + polygon[i].x)
        {
            inside = !inside;
        }
        j = i;
    }
    
    inside
}

lazy_static! {
    static ref IRELAND_COASTLINE: Vec<Coordinate> = {
        let coastline_file = include_str!("coastline_points.json");
        let coastline_data: serde_json::Value = serde_json::from_str(coastline_file)
            .expect("Failed to parse coastline data");
        
        coastline_data["grid_coords"]
            .as_array()
            .expect("Invalid coastline format")
            .iter()
            .map(|point| {
                let coords = point.as_array().expect("Invalid point format");
                Coordinate::new(
                    coords[0].as_f64().expect("Invalid x coordinate"),
                    coords[1].as_f64().expect("Invalid y coordinate")
                )
            })
            .collect()
    };
}

pub fn is_location_on_land(coordinate: &Coordinate) -> bool {
    is_point_inside_polygon(coordinate, &IRELAND_COASTLINE)
}

/// Returns the carbon price (€/tCO₂) based on the simulation year.
pub fn carbon_price(year: u32) -> f64 {
    if year < PRICE_PHASE1_START_YEAR {
        PRICE_BEFORE_PHASE1
    } else if year < PRICE_PHASE1_END_YEAR {
        // Linear interpolation from PRICE_PHASE1_START to PRICE_PHASE1_END over the phase length.
        let phase_length = (PRICE_PHASE1_END_YEAR - PRICE_PHASE1_START_YEAR) as f64;
        let t = (year - PRICE_PHASE1_START_YEAR) as f64 / phase_length;
        PRICE_PHASE1_START + t * (PRICE_PHASE1_END - PRICE_PHASE1_START)
    } else if year <= PRICE_PHASE2_END_YEAR {
        // Linear interpolation from PRICE_PHASE2_START to PRICE_PHASE2_END over the phase length.
        let phase_length = (PRICE_PHASE2_END_YEAR - PRICE_PHASE2_START_YEAR) as f64;
        let t = (year - PRICE_PHASE2_START_YEAR) as f64 / phase_length;
        PRICE_PHASE2_START + t * (PRICE_PHASE2_END - PRICE_PHASE2_START)
    } else {
        // For years beyond PRICE_PHASE2_END_YEAR, assume a constant price.
        PRICE_PHASE2_END
    }
}

/// Calculates the revenue from selling carbon credits for negative emissions.
pub fn calculate_carbon_credit_revenue(net_emissions: f64, year: u32) -> f64 {
    if net_emissions >= 0.0 {
        // No negative emissions, no carbon credit revenue
        return 0.0;
    }

    // Convert negative emissions to positive value for calculation
    let negative_emissions = -net_emissions;
    
    // Calculate revenue based on current carbon price
    let price = carbon_price(year);
    negative_emissions * price
}

/// Calculates the revenue from selling excess energy.
/// 
/// * `power_surplus` - The power surplus in MW
/// * `year` - The simulation year
/// * `sales_rate` - The sales rate in € per GWh
pub fn calculate_energy_sales_revenue(power_surplus: f64, year: u32, sales_rate: f64) -> f64 {
    if power_surplus <= 0.0 {
        // No surplus, no energy sales revenue
        return 0.0;
    }

    // Convert power surplus (MW) to yearly energy (GWh)
    // A power of 1 MW for a full year is 8.76 GWh (8760 hours / 1000)
    let yearly_energy_gwh = power_surplus * crate::constants::MW_TO_GWH_CONVERSION;
    
    // Calculate revenue
    yearly_energy_gwh * sales_rate
}