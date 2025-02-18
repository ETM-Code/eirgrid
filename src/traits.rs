// Define a trait to abstract the functionality needed for location analysis

use crate::poi::Coordinate;
use crate::generator::GeneratorType;

pub trait LocationAnalysisSource {
    fn calculate_generator_suitability(&self, coordinate: &Coordinate, generator_type: &GeneratorType) -> f64;
} 