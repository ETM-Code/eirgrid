use serde::{Deserialize, Serialize};
use crate::config::constants::{MAP_MAX_X, MAP_MAX_Y};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coordinate {
    pub x: f64,
    pub y: f64,
}

impl Coordinate {
    pub fn new(x: f64, y: f64) -> Self {
        let x = x.clamp(0.0, MAP_MAX_X);
        let y = y.clamp(0.0, MAP_MAX_Y);
        Self { x, y }
    }

    pub fn distance_to(&self, other: &Coordinate) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

pub trait POI {
    fn get_coordinate(&self) -> &Coordinate;
    fn get_id(&self) -> &str;
} 