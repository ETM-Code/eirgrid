use std::fs::File;
use std::io::BufReader;
use serde::Deserialize;

use crate::settlement::Settlement;
use crate::poi::Coordinate;
use crate::const_funcs;

#[derive(Debug, Deserialize)]
pub struct SettlementData {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub population: u32,
    pub grid_x: f64,
    pub grid_y: f64,
}

#[derive(Debug, Deserialize)]
pub struct SettlementsList {
    pub settlements: Vec<SettlementData>,
}

pub fn load_settlements(path: &str, base_year: u32) -> Result<Vec<Settlement>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let settlements_list: SettlementsList = serde_json::from_reader(reader)?;

    let mut settlements_vec = Vec::new();
    for s in settlements_list.settlements {
         let initial_power_usage = (s.population as f64) * const_funcs::calc_power_usage_per_capita(base_year);
         
         // Transform lat/lon to grid coordinates
         if let Some(grid_coord) = const_funcs::transform_lat_lon_to_grid(s.lat, s.lon) {
             let settlement = Settlement::new(s.name, grid_coord, s.population, initial_power_usage);
             settlements_vec.push(settlement);
         } else {
             eprintln!("Warning: Settlement {} has coordinates outside the valid range: ({}, {})", 
                       s.name, s.lat, s.lon);
         }
    }
    Ok(settlements_vec)
} 