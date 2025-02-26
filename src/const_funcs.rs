use crate::generator::GeneratorType;
use crate::constants::*;
use crate::settlement::Settlement;
use crate::poi::Coordinate;
use std::collections::HashMap;
use serde_json;
#[macro_use]
use lazy_static::lazy_static;

// Helper function to calculate size-based efficiency bonus
fn calc_size_efficiency_bonus(size: f64) -> f64 {
    (size - MIN_GENERATOR_SIZE) / (MAX_GENERATOR_SIZE - MIN_GENERATOR_SIZE) * SIZE_EFFICIENCY_FACTOR
}

// Helper function to calculate cost-based efficiency bonus
fn calc_cost_efficiency_bonus(cost: f64) -> f64 {
    ((cost) / (REFERENCE_ANNUAL_EXPENDITURE))
        .clamp(0.0_f64, 1.0_f64) * COST_EFFICIENCY_FACTOR
}

// Helper function to get max power output for generator type
fn get_max_power_output(gen_type: &GeneratorType) -> f64 {
    match gen_type {
        GeneratorType::OnshoreWind => f64::INFINITY,
        GeneratorType::OffshoreWind => f64::INFINITY,
        GeneratorType::DomesticSolar => f64::INFINITY,
        GeneratorType::CommercialSolar => f64::INFINITY,
        GeneratorType::UtilitySolar => f64::INFINITY,
        GeneratorType::Nuclear => f64::INFINITY,
        GeneratorType::CoalPlant => f64::INFINITY,
        GeneratorType::GasCombinedCycle => f64::INFINITY,
        GeneratorType::GasPeaker => f64::INFINITY,
        GeneratorType::Biomass => f64::INFINITY,
        GeneratorType::HydroDam => f64::INFINITY,
        GeneratorType::PumpedStorage => f64::INFINITY,
        GeneratorType::TidalGenerator => f64::INFINITY,
        GeneratorType::WaveEnergy => f64::INFINITY,
        GeneratorType::BatteryStorage => f64::INFINITY,
    }
}

pub fn calc_inflation_factor(year: u32) -> f64 {
    (1.0 + INFLATION_RATE).powi((year - BASE_YEAR) as i32)
}

pub fn calc_population_growth(initial_pop: u32, year: u32) -> u32 {
    let years_passed = year - BASE_YEAR;
    
    let is_urban = initial_pop > URBAN_POPULATION_THRESHOLD;
    let is_medium = initial_pop > MEDIUM_SETTLEMENT_THRESHOLD && initial_pop <= URBAN_POPULATION_THRESHOLD;
    
    let base_national_growth = match year {
        year if year <= MEDIUM_TERM_YEAR => SHORT_TERM_GROWTH,
        year if year <= LONG_TERM_YEAR => MEDIUM_TERM_GROWTH,
        _ => LONG_TERM_GROWTH,
    };

    let urbanization_factor = if is_urban {
        1.0 + (URBAN_GROWTH_RATE * (years_passed as f64))
    } else if is_medium {
        1.0
    } else {
        1.0 + (RURAL_GROWTH_RATE * (years_passed as f64))
    };

    let age_structure_factor = if is_urban {
        1.0 + (initial_pop as f64 / LARGE_CITY_REFERENCE)
    } else {
        1.0 - (URBAN_POPULATION_THRESHOLD as f64 / initial_pop as f64)
    };

    let growth_rate = base_national_growth * urbanization_factor * age_structure_factor;
    (initial_pop as f64 * (1.0 + growth_rate).powi(years_passed as i32)) as u32
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

pub fn calc_generator_efficiency(
    gen_type: &GeneratorType,
    size: f64,
    base_cost: f64,
    year: u32,
    is_urban: bool,
    is_coastal: bool,
) -> f64 {
    let base_efficiency = gen_type.get_base_efficiency(year);
    
    // Size bonus only applies to non-fixed size generators
    let size_bonus = if matches!(gen_type, GeneratorType::DomesticSolar) {
        0.0_f64
    } else {
        calc_size_efficiency_bonus(size)
    };
    
    let cost_bonus = calc_cost_efficiency_bonus(base_cost);
    
    // Location bonuses
    let mut location_bonus = 0.0_f64;
    
    // Urban bonus for solar
    if is_urban && matches!(gen_type, 
        GeneratorType::DomesticSolar | 
        GeneratorType::CommercialSolar |
        GeneratorType::UtilitySolar
    ) {
        location_bonus += URBAN_SOLAR_BONUS;
    }
    
    // Coastal bonus for offshore wind and marine generators
    if is_coastal && matches!(gen_type,
        GeneratorType::OffshoreWind |
        GeneratorType::TidalGenerator |
        GeneratorType::WaveEnergy
    ) {
        location_bonus += COASTAL_BONUS;
    }
    
    // Technology improvement over time
    let years_from_base = (year - BASE_YEAR) as f64;
    let tech_improvement = match gen_type {
        GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => 
            WIND_EFFICIENCY_GAIN.powf(years_from_base),
        GeneratorType::DomesticSolar | GeneratorType::CommercialSolar | 
        GeneratorType::UtilitySolar => 
            SOLAR_EFFICIENCY_GAIN.powf(years_from_base),
        GeneratorType::Nuclear => 
            NUCLEAR_EFFICIENCY_GAIN.powf(years_from_base),
        GeneratorType::CoalPlant => 
            COAL_EFFICIENCY_LOSS.powf(years_from_base),
        GeneratorType::GasCombinedCycle | GeneratorType::GasPeaker => 
            GAS_EFFICIENCY_LOSS.powf(years_from_base),
        GeneratorType::HydroDam | GeneratorType::PumpedStorage => 
            HYDRO_EFFICIENCY_GAIN.powf(years_from_base),
        GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => 
            MARINE_EFFICIENCY_GAIN.powf(years_from_base),
        GeneratorType::BatteryStorage => 
            BATTERY_EFFICIENCY_GAIN.powf(years_from_base),
        GeneratorType::Biomass => 
            BIOMASS_EFFICIENCY_GAIN.powf(years_from_base),
    };
    
    ((base_efficiency + size_bonus + cost_bonus + location_bonus) * tech_improvement)
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

pub fn calc_transmission_loss(
    from: &Coordinate,
    to: &Coordinate,
    year: u32,
) -> f64 {
    // Distance given in m
    let distance = ((from.x - to.x).powi(2) + (from.y - to.y).powi(2)).sqrt();
    
    // Base loss calculation
    let mut loss_rate = BASE_TRANSMISSION_LOSS_RATE / 1000.0 * distance;
    
    // Apply technological improvements
    let years_passed = year - BASE_YEAR;
    let improvement_factor = (1.0_f64 - GRID_IMPROVEMENT_RATE).powi(years_passed as i32);
    loss_rate *= improvement_factor;
    
    // Apply smart grid benefits if after adoption year
    if year >= SMART_GRID_ADOPTION_YEAR {
        loss_rate *= SMART_GRID_FACTOR;
    }
    
    loss_rate.clamp(0.0_f64, 1.0_f64)
}

pub fn calc_settlement_power_distribution(settlement: &Settlement, year: u32) -> (f64, f64, f64) {
    let total_power = settlement.get_power_usage();
    let population = settlement.get_population();
    
    // Base distribution ratios
    let mut residential = total_power * RESIDENTIAL_POWER_RATIO;
    let mut commercial = total_power * COMMERCIAL_POWER_RATIO;
    let mut industrial = total_power * INDUSTRIAL_POWER_RATIO;
    
    // Adjust for settlement size and type
    if population >= INDUSTRY_THRESHOLD_POP {
        // Large settlements have more industry
        industrial *= INDUSTRY_POWER_FACTOR;
        
        // Reduce residential and commercial proportionally to maintain total
        let total_reduction = industrial * (INDUSTRY_POWER_FACTOR - 1.0_f64);
        let reduction_ratio = total_reduction / (residential + commercial);
        residential *= (1.0_f64 - reduction_ratio);
        commercial *= (1.0_f64 - reduction_ratio);
    }
    
    // Adjust for data centers and commercial districts in urban areas
    if population >= URBAN_POPULATION_THRESHOLD {
        commercial *= COMMERCIAL_POWER_FACTOR;
        
        // If it's a major urban center, add data center load
        if population >= URBAN_POPULATION_THRESHOLD * 2 {
            commercial *= DATA_CENTER_POWER_FACTOR;
        }
        
        // Reduce residential and industrial proportionally
        let total_commercial_increase = commercial * 
            ((COMMERCIAL_POWER_FACTOR * (if population >= URBAN_POPULATION_THRESHOLD * 2 { 
                DATA_CENTER_POWER_FACTOR 
            } else { 
                1.0_f64 
            })) - 1.0_f64);
        let reduction_ratio = total_commercial_increase / (residential + industrial);
        residential *= (1.0_f64 - reduction_ratio);
        industrial *= (1.0_f64 - reduction_ratio);
    }
    
    // Technology evolution impact over time
    let years_from_base = (year - BASE_YEAR) as f64;
    
    // Residential becomes more efficient over time (smart homes, better appliances)
    residential *= (1.0_f64 - RESIDENTIAL_EFFICIENCY_GAIN).powf(years_from_base);
    
    // Commercial increases due to digitalization
    commercial *= (1.0_f64 + COMMERCIAL_GROWTH_RATE).powf(years_from_base);
    
    // Industrial varies based on automation and efficiency
    industrial *= (1.0_f64 + INDUSTRIAL_EVOLUTION_RATE).powf(years_from_base);
    
    // Normalize to maintain total power usage
    let current_total = residential + commercial + industrial;
    let scale_factor = total_power / current_total;
    
    (
        residential * scale_factor,
        commercial * scale_factor,
        industrial * scale_factor
    )
}

pub fn evaluate_generator_location(
    coordinate: &Coordinate,
    gen_type: &GeneratorType,
    size: f64,
    settlements: &[Settlement],
    existing_generators: &HashMap<(i32, i32), f64>,
    year: u32,
) -> Option<f64> {
    // Check if location is already at capacity
    let grid_x = (coordinate.x / GRID_CELL_SIZE).floor() as i32;
    let grid_y = (coordinate.y / GRID_CELL_SIZE).floor() as i32;
    let current_capacity = existing_generators.get(&(grid_x, grid_y)).unwrap_or(&0.0);
    
    if current_capacity + size > MAX_CUMULATIVE_GENERATOR_SIZE {
        return None;
    }
    
    let mut total_score = 0.0;
    let mut total_weight = 0.0;
    
    // Calculate weighted power delivery to settlements
    let mut weighted_transmission_score = 0.0;
    let mut total_power_needed = 0.0;
    
    for settlement in settlements {
        let (residential, commercial, industrial) = calc_settlement_power_distribution(settlement, year);
        let total_settlement_power = residential + commercial + industrial;
        let loss = calc_transmission_loss(
            coordinate,
            settlement.get_coordinate(),
            year
        );
        
        weighted_transmission_score += total_settlement_power * (1.0 - loss);
        total_power_needed += total_settlement_power;
    }
    
    // Normalize transmission score
    let transmission_score = if total_power_needed > 0.0 {
        weighted_transmission_score / total_power_needed
    } else {
        0.0
    };
    
    // Add transmission component
    total_score += transmission_score * TRANSMISSION_LOSS_WEIGHT;
    total_weight += TRANSMISSION_LOSS_WEIGHT;
    
    // Add public opinion component
    let mut avg_opinion = 0.0;
    for settlement in settlements {
        let opinion = settlement.calc_range_opinion(coordinate);
        avg_opinion += opinion;
    }
    avg_opinion /= settlements.len() as f64;
    
    total_score += avg_opinion * PUBLIC_OPINION_WEIGHT;
    total_weight += PUBLIC_OPINION_WEIGHT;
    
    // Add construction cost component (based on terrain and accessibility)
    let construction_score = 1.0; // Would need terrain data for better calculation
    total_score += construction_score * CONSTRUCTION_COST_WEIGHT;
    total_weight += CONSTRUCTION_COST_WEIGHT;
    
    // Add environmental component
    let environmental_score = match gen_type {
        GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => 0.9,
        GeneratorType::DomesticSolar | GeneratorType::CommercialSolar | GeneratorType::UtilitySolar => 0.9,
        GeneratorType::HydroDam | GeneratorType::PumpedStorage => 0.7,
        GeneratorType::Nuclear => 0.5,
        GeneratorType::GasCombinedCycle | GeneratorType::GasPeaker => 0.3,
        GeneratorType::CoalPlant => 0.1,
        GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => 0.8,
        GeneratorType::BatteryStorage => 0.95,
        GeneratorType::Biomass => 0.6,
    };
    total_score += environmental_score * ENVIRONMENTAL_WEIGHT;
    total_weight += ENVIRONMENTAL_WEIGHT;
    
    // Normalize final score
    Some(total_score / total_weight)
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

    // Normalize to our coordinate system
    let x = ((lon - IRELAND_MIN_LON) / (IRELAND_MAX_LON - IRELAND_MIN_LON)) * MAP_MAX_X;
    let y = ((lat - IRELAND_MIN_LAT) / (IRELAND_MAX_LAT - IRELAND_MIN_LAT)) * MAP_MAX_Y;

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

pub fn is_valid_generator_location(gen_type: &GeneratorType, coordinate: &Coordinate) -> bool {
    let on_land = is_location_on_land(coordinate);
    
    match gen_type {
        // These types must be on land
        GeneratorType::CoalPlant |
        GeneratorType::GasCombinedCycle |
        GeneratorType::GasPeaker |
        GeneratorType::Biomass |
        GeneratorType::OnshoreWind |
        GeneratorType::DomesticSolar |
        GeneratorType::CommercialSolar |
        GeneratorType::UtilitySolar |
        GeneratorType::HydroDam |
        GeneratorType::PumpedStorage |
        GeneratorType::Nuclear |
        GeneratorType::BatteryStorage => on_land,
        
        // These types must be offshore
        GeneratorType::OffshoreWind |
        GeneratorType::TidalGenerator |
        GeneratorType::WaveEnergy => !on_land,
    }
} 