use serde::{Deserialize, Serialize};
use crate::poi::{POI, Coordinate};
use crate::const_funcs::{calc_population_growth, calc_power_usage_per_capita};
use crate::constants::{MAP_MAX_X, MAP_MAX_Y};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settlement {
    id: String,
    coordinate: Coordinate,
    initial_pop: u32,
    current_pop: u32,
    initial_power_usage: f64,
    current_power_usage: f64,
}

impl Settlement {
    pub fn new(
        id: String,
        coordinate: Coordinate,
        initial_pop: u32,
        initial_power_usage: f64,
    ) -> Self {
        Self {
            id,
            coordinate,
            initial_pop,
            current_pop: initial_pop,
            initial_power_usage,
            current_power_usage: initial_power_usage,
        }
    }

    pub fn get_population(&self) -> u32 {
        calc_population_growth(self.initial_pop, 2025)
    }

    pub fn get_power_usage(&self) -> f64 {
        let pop = self.get_population();
        let usage_per_capita = calc_power_usage_per_capita(2025);
        pop as f64 * usage_per_capita
    }

    pub fn calc_range_opinion(&self, coordinate: &Coordinate) -> f64 {
        let distance = self.coordinate.distance_to(coordinate);
        let max_distance = (MAP_MAX_X.powi(2) + MAP_MAX_Y.powi(2)).sqrt();
        (1.0 - distance / max_distance).max(0.0)
    }
}

impl POI for Settlement {
    fn get_coordinate(&self) -> &Coordinate {
        &self.coordinate
    }

    fn get_id(&self) -> &str {
        &self.id
    }
} 