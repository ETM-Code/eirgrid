use serde::{Deserialize, Serialize};
use crate::data::poi::{POI, Coordinate};
// use crate::config::const_funcs::{calc_population_growth, calc_power_usage_per_capita};
// use crate::config::constants::{MAP_MAX_X, MAP_MAX_Y};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementData {
    name: String,
    coordinate: Coordinate,
    base_population: u32,
    base_power_usage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementState {
    current_population: u32,
    current_power_usage: f64,
}

// Custom serialization for Settlement to handle Arc
#[derive(Debug, Clone)]
pub struct Settlement {
    data: Arc<SettlementData>,
    state: SettlementState,
}

// Implement custom serialization
impl Serialize for Settlement {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Settlement", 2)?;
        state.serialize_field("data", &*self.data)?;
        state.serialize_field("state", &self.state)?;
        state.end()
    }
}

// Implement custom deserialization
impl<'de> Deserialize<'de> for Settlement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            data: SettlementData,
            state: SettlementState,
        }

        let helper = Helper::deserialize(deserializer)?;
        Ok(Settlement {
            data: Arc::new(helper.data),
            state: helper.state,
        })
    }
}

impl Settlement {
    pub fn new(name: String, coordinate: Coordinate, population: u32, power_usage: f64) -> Self {
        let data = Arc::new(SettlementData {
            name,
            coordinate,
            base_population: population,
            base_power_usage: power_usage,
        });
        
        let state = SettlementState {
            current_population: population,
            current_power_usage: power_usage,
        };

        Settlement { data, state }
    }

    pub fn get_name(&self) -> &str {
        &self.data.name
    }

    pub fn get_coordinate(&self) -> &Coordinate {
        &self.data.coordinate
    }

    pub fn get_population(&self) -> u32 {
        self.state.current_population
    }

    pub fn get_power_usage(&self) -> f64 {
        self.state.current_power_usage
    }

    pub fn update_population(&mut self, new_population: u32) {
        self.state.current_population = new_population;
    }

    pub fn update_power_usage(&mut self, new_usage: f64) {
        self.state.current_power_usage = new_usage;
    }

    pub fn calc_range_opinion(&self, generator_coord: &Coordinate) -> f64 {
        let distance = self.data.coordinate.distance_to(generator_coord);
        1.0 / (1.0 + distance / 10000.0)
    }
}

impl POI for Settlement {
    fn get_coordinate(&self) -> &Coordinate {
        &self.data.coordinate
    }

    fn get_id(&self) -> &str {
        &self.data.name
    }
} 