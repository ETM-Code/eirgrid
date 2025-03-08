// Define a trait to abstract the functionality needed for location analysis

use crate::data::poi::Coordinate;
use crate::models::generator::GeneratorType;

pub trait LocationAnalysisSource {
    fn calculate_generator_suitability(&self, coordinate: &Coordinate, generator_type: &GeneratorType) -> f64;
} 