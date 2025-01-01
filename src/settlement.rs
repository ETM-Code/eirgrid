use serde::{Deserialize, Serialize};
use crate::poi::{POI, Coordinate};
use crate::const_funcs::{calc_population_growth, calc_power_usage_per_capita};

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

    pub fn calc_pop(&mut self, year: u32) {
        self.current_pop = calc_population_growth(self.initial_pop, year);
    }

    pub fn calc_range_opinion(&self, coordinate: &Coordinate) -> f64 {
        let distance = self.coordinate.distance_to(coordinate);
        let max_distance = (MAP_MAX_X.powi(2) + MAP_MAX_Y.powi(2)).sqrt();
        (1.0 - distance / max_distance).max(0.0)
    }

    pub fn calc_power_usage(&mut self, year: u32) {
        let per_capita_usage = calc_power_usage_per_capita(
            self.initial_power_usage / self.initial_pop as f64,
            year
        );
        self.current_power_usage = per_capita_usage * self.current_pop as f64;
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