use crate::generator::GeneratorType;
use crate::constants::*;
use crate::settlement::Settlement;
use crate::poi::Coordinate;
use std::collections::HashMap;

// Helper function to calculate size-based efficiency bonus
fn calc_size_efficiency_bonus(size: f64) -> f64 {
    (size - MIN_GENERATOR_SIZE) / (MAX_GENERATOR_SIZE - MIN_GENERATOR_SIZE) * SIZE_EFFICIENCY_FACTOR
}

// Helper function to calculate cost-based efficiency bonus
fn calc_cost_efficiency_bonus(cost: f64) -> f64 {
    ((cost - MIN_GENERATOR_COST) / (REFERENCE_LARGE_GENERATOR_COST - MIN_GENERATOR_COST))
        .clamp(0.0, 1.0) * COST_EFFICIENCY_FACTOR
}

// Helper function to get max power output for generator type
fn get_max_power_output(gen_type: &GeneratorType) -> f64 {
    match gen_type {
        GeneratorType::Wind => MAX_WIND_POWER,
        GeneratorType::Solar => MAX_SOLAR_POWER,
        GeneratorType::Nuclear => MAX_NUCLEAR_POWER,
        GeneratorType::Coal => MAX_COAL_POWER,
        GeneratorType::Gas => MAX_GAS_POWER,
        GeneratorType::Hydro => MAX_HYDRO_POWER,
    }
}

pub fn calc_inflation_factor(year: u32) -> f64 {
    let annual_inflation = match year {
        year if year <= MEDIUM_TERM_YEAR => SHORT_TERM_INFLATION,
        year if year <= LONG_TERM_YEAR => MEDIUM_TERM_INFLATION,
        _ => LONG_TERM_INFLATION,
    };
    (1.0 + annual_inflation).powi((year - BASE_YEAR) as i32)
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
        1.0 + (URBAN_GROWTH_RATE * (years_passed as f64).min(MAX_URBAN_BOOST_YEARS))
    } else if is_medium {
        1.0
    } else {
        1.0 - (RURAL_DECLINE_RATE * (years_passed as f64).min(MAX_RURAL_DECLINE_YEARS))
    };

    let age_structure_factor = if is_urban {
        1.0 + (initial_pop as f64 / LARGE_CITY_REFERENCE).min(MAX_URBAN_BOOST)
    } else {
        1.0 - (URBAN_POPULATION_THRESHOLD as f64 / initial_pop as f64).min(MAX_RURAL_DECLINE)
    };

    let growth_rate = base_national_growth * urbanization_factor * age_structure_factor;
    (initial_pop as f64 * (1.0 + growth_rate).powi(years_passed as i32)) as u32
}

pub fn calc_power_usage_per_capita(base_usage: f64, year: u32) -> f64 {
    let years_passed = year - BASE_YEAR;
    
    let economic_growth = match year {
        year if year <= MEDIUM_TERM_YEAR => SHORT_TERM_ECONOMIC_GROWTH,
        year if year <= LONG_TERM_YEAR => MEDIUM_TERM_ECONOMIC_GROWTH,
        _ => LONG_TERM_ECONOMIC_GROWTH,
    };

    let efficiency_improvement = match year {
        year if year <= MEDIUM_TERM_YEAR => SHORT_TERM_EFFICIENCY_GAIN,
        year if year <= LONG_TERM_YEAR => MEDIUM_TERM_EFFICIENCY_GAIN,
        _ => LONG_TERM_EFFICIENCY_GAIN,
    };

    let data_center_factor = 1.0 + (DATA_CENTER_GROWTH * (years_passed as f64).min(MAX_DATA_CENTER_YEARS));
    let electrification_factor = 1.0 + (ELECTRIFICATION_GROWTH * (years_passed as f64).min(MAX_ELECTRIFICATION_YEARS));

    base_usage * 
        (1.0 + economic_growth).powi(years_passed as i32) * 
        (1.0 - efficiency_improvement).powi(years_passed as i32) *
        data_center_factor *
        electrification_factor
}

pub fn calc_generator_cost(gen_type: &GeneratorType, base_cost: f64, year: u32) -> f64 {
    let inflation = calc_inflation_factor(year);
    let years_from_base = (year - BASE_YEAR) as f64;
    
    let technology_factor = match gen_type {
        GeneratorType::Wind => WIND_COST_REDUCTION.powf(years_from_base),
        GeneratorType::Solar => SOLAR_COST_REDUCTION.powf(years_from_base),
        GeneratorType::Nuclear => NUCLEAR_COST_INCREASE.powf(years_from_base),
        GeneratorType::Coal => COAL_COST_INCREASE.powf(years_from_base),
        GeneratorType::Gas => GAS_COST_INCREASE.powf(years_from_base),
        GeneratorType::Hydro => HYDRO_COST_INCREASE.powf(years_from_base),
    };
    
    base_cost * inflation * technology_factor
}

pub fn calc_generator_efficiency(gen_type: &GeneratorType, size: f64, base_cost: f64, year: u32) -> f64 {
    let years_from_base = (year - BASE_YEAR) as f64;
    
    let base_efficiency = BASE_EFFICIENCY;
    let size_bonus = calc_size_efficiency_bonus(size);
    let cost_bonus = calc_cost_efficiency_bonus(base_cost);
    
    let technology_evolution = match gen_type {
        GeneratorType::Wind => WIND_EFFICIENCY_GAIN.powf(years_from_base),
        GeneratorType::Solar => SOLAR_EFFICIENCY_GAIN.powf(years_from_base),
        GeneratorType::Nuclear => NUCLEAR_EFFICIENCY_GAIN.powf(years_from_base),
        GeneratorType::Coal => COAL_EFFICIENCY_LOSS.powf(years_from_base),
        GeneratorType::Gas => GAS_EFFICIENCY_LOSS.powf(years_from_base),
        GeneratorType::Hydro => HYDRO_EFFICIENCY_GAIN.powf(years_from_base),
    };

    (base_efficiency + size_bonus + cost_bonus)
        .min(MAX_EFFICIENCY) * technology_evolution
}

pub fn calc_power_output(gen_type: &GeneratorType, size: f64) -> f64 {
    let max_power = get_max_power_output(gen_type);
    max_power * size
}

pub fn calc_operating_cost(gen_type: &GeneratorType, base_operating_cost: f64, year: u32) -> f64 {
    let inflation = calc_inflation_factor(year);
    let years_from_base = (year - BASE_YEAR) as f64;
    
    let efficiency_factor = match gen_type {
        GeneratorType::Wind => WIND_EFFICIENCY_GAIN.powf(years_from_base),
        GeneratorType::Solar => SOLAR_EFFICIENCY_GAIN.powf(years_from_base),
        GeneratorType::Nuclear => NUCLEAR_EFFICIENCY_GAIN.powf(years_from_base),
        GeneratorType::Coal => COAL_EFFICIENCY_LOSS.powf(years_from_base),
        GeneratorType::Gas => GAS_EFFICIENCY_LOSS.powf(years_from_base),
        GeneratorType::Hydro => HYDRO_EFFICIENCY_GAIN.powf(years_from_base),
    };
    
    base_operating_cost * inflation * efficiency_factor
}

pub fn calc_type_opinion(gen_type: &GeneratorType, year: u32) -> f64 {
    let years_passed = (year - BASE_YEAR) as f64;
    
    let (base_opinion, annual_change) = match gen_type {
        GeneratorType::Wind => (WIND_BASE_OPINION, WIND_OPINION_CHANGE),
        GeneratorType::Solar => (SOLAR_BASE_OPINION, SOLAR_OPINION_CHANGE),
        GeneratorType::Nuclear => (NUCLEAR_BASE_OPINION, NUCLEAR_OPINION_CHANGE),
        GeneratorType::Coal => (COAL_BASE_OPINION, COAL_OPINION_CHANGE),
        GeneratorType::Gas => (GAS_BASE_OPINION, GAS_OPINION_CHANGE),
        GeneratorType::Hydro => (HYDRO_BASE_OPINION, HYDRO_OPINION_CHANGE),
    };
    
    (base_opinion + annual_change * years_passed).clamp(0.0, 1.0)
}

pub fn calc_cost_opinion(cost: f64, year: u32) -> f64 {
    let inflation_adjusted_max = REFERENCE_LARGE_GENERATOR_COST * calc_inflation_factor(year);
    let normalized_cost = (cost / inflation_adjusted_max).min(1.0);
    1.0 - normalized_cost
}

pub fn calc_transmission_loss(
    from: &Coordinate,
    to: &Coordinate,
    year: u32,
    is_urban: bool,
    is_underwater: bool,
    is_mountainous: bool,
) -> f64 {
    let distance = ((from.x - to.x).powi(2) + (from.y - to.y).powi(2)).sqrt();
    
    // Base loss calculation
    let mut loss_rate = BASE_TRANSMISSION_LOSS_RATE * distance;
    
    // Apply infrastructure factors
    loss_rate *= if is_urban {
        URBAN_INFRASTRUCTURE_FACTOR
    } else {
        RURAL_INFRASTRUCTURE_FACTOR
    };
    
    // Apply terrain factors
    if is_underwater {
        loss_rate *= UNDERWATER_LOSS_FACTOR;
    }
    if is_mountainous {
        loss_rate *= MOUNTAIN_LOSS_FACTOR;
    }
    
    // Apply high voltage reduction
    loss_rate *= HIGH_VOLTAGE_LOSS_REDUCTION;
    
    // Apply technological improvements
    let years_passed = year - BASE_YEAR;
    let improvement_factor = (1.0 - GRID_IMPROVEMENT_RATE).powi(years_passed as i32);
    loss_rate *= improvement_factor;
    
    // Apply smart grid benefits if after adoption year
    if year >= SMART_GRID_ADOPTION_YEAR {
        loss_rate *= SMART_GRID_FACTOR;
    }
    
    loss_rate.clamp(0.0, 1.0)
}

pub fn calc_settlement_power_distribution(settlement: &Settlement, year: u32) -> (f64, f64, f64) {
    let total_power = settlement.current_power_usage;
    let population = settlement.current_pop;
    
    let mut residential = total_power * RESIDENTIAL_POWER_RATIO;
    let mut commercial = total_power * COMMERCIAL_POWER_RATIO;
    let mut industrial = total_power * INDUSTRIAL_POWER_RATIO;
    
    // Adjust for settlement size
    if population >= INDUSTRY_THRESHOLD_POP {
        industrial *= INDUSTRY_POWER_FACTOR;
    }
    
    // Adjust for data centers in larger cities
    if population >= URBAN_POPULATION_THRESHOLD {
        commercial *= DATA_CENTER_POWER_FACTOR;
    }
    
    // Normalize to maintain total
    let total = residential + commercial + industrial;
    let scale = total_power / total;
    
    (
        residential * scale,
        commercial * scale,
        industrial * scale
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
            year,
            settlement.current_pop >= URBAN_POPULATION_THRESHOLD,
            false, // Would need terrain data for these
            false,
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
        GeneratorType::Wind | GeneratorType::Solar => 0.9,
        GeneratorType::Hydro => 0.7,
        GeneratorType::Nuclear => 0.5,
        GeneratorType::Gas => 0.3,
        GeneratorType::Coal => 0.1,
    };
    total_score += environmental_score * ENVIRONMENTAL_WEIGHT;
    total_weight += ENVIRONMENTAL_WEIGHT;
    
    // Normalize final score
    Some(total_score / total_weight)
} 