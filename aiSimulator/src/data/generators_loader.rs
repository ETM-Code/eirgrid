use std::fs::File;
use std::io::Read;
use csv::ReaderBuilder;
use crate::models::generator::{Generator, GeneratorType};
use super::poi::Coordinate;
use crate::config::constants::*;
use crate::config::constants::{IRELAND_MIN_LAT, IRELAND_MAX_LAT, IRELAND_MIN_LON, IRELAND_MAX_LON};
use crate::config::const_funcs::{calc_generator_cost, calc_operating_cost, calc_initial_co2_output, calc_decommission_cost, transform_lat_lon_to_grid, is_location_on_land, is_coastal_location};

#[derive(Debug)]
pub enum GeneratorLoadError {
    IoError(std::io::Error),
    CsvError(csv::Error),
    InvalidFuelType(String),
    InvalidCapacity(String),
    InvalidCoordinate(String),
    InvalidLocation(String),
}

impl From<std::io::Error> for GeneratorLoadError {
    fn from(err: std::io::Error) -> Self {
        GeneratorLoadError::IoError(err)
    }
}

impl From<csv::Error> for GeneratorLoadError {
    fn from(err: csv::Error) -> Self {
        GeneratorLoadError::CsvError(err)
    }
}

impl std::fmt::Display for GeneratorLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeneratorLoadError::IoError(e) => write!(f, "IO error: {}", e),
            GeneratorLoadError::CsvError(e) => write!(f, "CSV error: {}", e),
            GeneratorLoadError::InvalidFuelType(s) => write!(f, "Invalid fuel type: {}", s),
            GeneratorLoadError::InvalidCapacity(s) => write!(f, "Invalid capacity: {}", s),
            GeneratorLoadError::InvalidCoordinate(s) => write!(f, "Invalid coordinate: {}", s),
            GeneratorLoadError::InvalidLocation(s) => write!(f, "Invalid location for generator type: {}", s),
        }
    }
}

impl std::error::Error for GeneratorLoadError {}

fn map_fuel_type_to_generator_type(fuel: &str) -> Result<GeneratorType, GeneratorLoadError> {
    match fuel.to_lowercase().as_str() {
        "gas" => Ok(GeneratorType::GasCombinedCycle), // Assuming most gas plants are combined cycle
        "coal" => Ok(GeneratorType::CoalPlant),
        "wind" => Ok(GeneratorType::OnshoreWind),
        "hydro" => Ok(GeneratorType::HydroDam),
        "oil" => Ok(GeneratorType::GasPeaker), // Oil plants typically serve as peakers
        "biomass" => Ok(GeneratorType::Biomass),
        _ => Err(GeneratorLoadError::InvalidFuelType(fuel.to_string())),
    }
}

fn transform_coordinates(lat: f64, lon: f64) -> Result<Coordinate, GeneratorLoadError> {
    // First check if the coordinates are within Ireland's bounds
    if lat < IRELAND_MIN_LAT || lat > IRELAND_MAX_LAT || 
       lon < IRELAND_MIN_LON || lon > IRELAND_MAX_LON {
        println!("WARNING: Coordinates outside Ireland's bounds: {}, {} - will clamp to valid range", lat, lon);
        
        // Clamp the coordinates to valid range instead of failing
        let lat_valid = lat.max(IRELAND_MIN_LAT).min(IRELAND_MAX_LAT);
        let lon_valid = lon.max(IRELAND_MIN_LON).min(IRELAND_MAX_LON);
        
        println!("Clamped coordinates: {}, {} -> {}, {}", lat, lon, lat_valid, lon_valid);
        
        // Continue with the clamped coordinates
        let coordinate = transform_lat_lon_to_grid(lat_valid, lon_valid)
            .ok_or_else(|| GeneratorLoadError::InvalidCoordinate(
                format!("Failed to transform coordinates: {}, {}", lat_valid, lon_valid)
            ))?;
        
        println!("Transformed clamped coordinates: ({}, {}) -> ({:.2}, {:.2})", 
                 lat_valid, lon_valid, coordinate.x, coordinate.y);
                 
        return Ok(coordinate);
    }

    // Transform using the proper full transformation
    let coordinate = transform_lat_lon_to_grid(lat, lon)
        .ok_or_else(|| GeneratorLoadError::InvalidCoordinate(
            format!("Failed to transform coordinates: {}, {}", lat, lon)
        ))?;
    
    // Verify the coordinate is valid and within expected range
    if coordinate.x < 0.0 || coordinate.x > MAP_MAX_X || 
       coordinate.y < 0.0 || coordinate.y > MAP_MAX_Y {
        println!("WARNING: Transformed grid coordinates outside valid range: ({:.2}, {:.2})", 
                 coordinate.x, coordinate.y);
                 
        // Clamp to valid range
        let x_valid = coordinate.x.max(0.0).min(MAP_MAX_X);
        let y_valid = coordinate.y.max(0.0).min(MAP_MAX_Y);
        
        println!("Clamped grid coordinates: ({:.2}, {:.2}) -> ({:.2}, {:.2})",
                 coordinate.x, coordinate.y, x_valid, y_valid);
                 
        return Ok(Coordinate::new(x_valid, y_valid));
    }
    
    // Additional check for values near zero which might indicate transformation issues
    if coordinate.x < 1000.0 && coordinate.y < 1000.0 {
        println!("WARNING: Generator coordinates transformed to near-origin values: ({:.2}, {:.2}) from ({}, {})",
                 coordinate.x, coordinate.y, lat, lon);
    }
    
    // Provide debug info about the transformation
    println!("Transformed coordinates: ({}, {}) -> ({:.2}, {:.2})", 
             lat, lon, coordinate.x, coordinate.y);
    
    Ok(coordinate)
}

fn normalize_capacity(capacity: f64, gen_type: &GeneratorType) -> f64 {
    let max_power = match gen_type {
        GeneratorType::OnshoreWind => MAX_ONSHORE_WIND_POWER,
        GeneratorType::OffshoreWind => MAX_OFFSHORE_WIND_POWER,
        GeneratorType::CoalPlant => MAX_COAL_POWER,
        GeneratorType::GasCombinedCycle => MAX_GAS_CC_POWER,
        GeneratorType::GasPeaker => MAX_GAS_PEAKER_POWER,
        GeneratorType::HydroDam => MAX_HYDRO_DAM_POWER,
        GeneratorType::Biomass => MAX_BIOMASS_POWER,
        _ => MAX_GAS_CC_POWER, // Default case
    };

    (capacity / max_power).clamp(MIN_GENERATOR_SIZE, MAX_GENERATOR_SIZE)
}

pub fn load_generators(csv_path: &str, year: u32) -> Result<Vec<Generator>, GeneratorLoadError> {
    let mut file = File::open(csv_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(contents.as_bytes());

    let mut generators = Vec::new();
    let mut id_counter = 0;

    for result in reader.records() {
        let record = result?;
        
        // Parse required fields
        let capacity: f64 = record.get(0)
            .ok_or_else(|| GeneratorLoadError::InvalidCapacity("Missing capacity".to_string()))?
            .parse()
            .map_err(|_| GeneratorLoadError::InvalidCapacity("Invalid capacity format".to_string()))?;
            
        let latitude: f64 = record.get(1)
            .ok_or_else(|| GeneratorLoadError::InvalidCoordinate("Missing latitude".to_string()))?
            .parse()
            .map_err(|_| GeneratorLoadError::InvalidCoordinate("Invalid latitude format".to_string()))?;
            
        let longitude: f64 = record.get(2)
            .ok_or_else(|| GeneratorLoadError::InvalidCoordinate("Missing longitude".to_string()))?
            .parse()
            .map_err(|_| GeneratorLoadError::InvalidCoordinate("Invalid longitude format".to_string()))?;
            
        let fuel_type = record.get(3)
            .ok_or_else(|| GeneratorLoadError::InvalidFuelType("Missing fuel type".to_string()))?;

        // Transform and validate the data
        let gen_type = map_fuel_type_to_generator_type(fuel_type)?;
        let location = transform_coordinates(latitude, longitude)?;
        
        let size = normalize_capacity(capacity, &gen_type);
        let is_coastal = is_location_on_land(&location) && is_coastal_location(&location);

        // Calculate derived values using const_funcs
        let base_cost = calc_generator_cost(
            &gen_type,
            gen_type.get_base_cost(year),
            year,
            false, // Would need settlement data to determine if urban
            is_coastal,
            false, // Would need terrain data for river check
        );

        let operating_cost = calc_operating_cost(&gen_type, gen_type.get_operating_cost(year), year);
        let initial_co2_output = calc_initial_co2_output(&gen_type, size);
        let decommission_cost = calc_decommission_cost(base_cost);

        // Create the generator
        let generator = Generator::new(
            format!("Existing_{}_{}", gen_type.to_string(), id_counter),
            location,
            gen_type.clone(),
            base_cost,
            capacity,
            operating_cost,
            gen_type.get_lifespan(),
            size,
            initial_co2_output,
            decommission_cost,
        );

        generators.push(generator);
        id_counter += 1;
    }

    Ok(generators)
} 