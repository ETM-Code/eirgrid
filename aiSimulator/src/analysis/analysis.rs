use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::path::Path;
use crate::models::generator::GeneratorType;
use crate::data::poi::Coordinate;

pub trait LocationAnalysisSource {
    fn calculate_generator_suitability(&self, coordinate: &Coordinate, generator_type: &GeneratorType) -> f64;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationSuitability {
    pub coordinate: Coordinate,
    pub suitability_scores: HashMap<GeneratorType, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationAnalysis {
    pub locations: Vec<LocationSuitability>,
    pub type_counts: HashMap<GeneratorType, usize>,
    pub multi_type_locations: Vec<(Coordinate, Vec<GeneratorType>)>,
    remaining_spaces: HashMap<GeneratorType, usize>,
}

impl LocationAnalysis {
    pub fn new(
        locations: Vec<LocationSuitability>,
        type_counts: HashMap<GeneratorType, usize>,
        multi_type_locations: Vec<(Coordinate, Vec<GeneratorType>)>,
    ) -> Self {
        let remaining_spaces = type_counts.clone();
        Self {
            locations,
            type_counts,
            multi_type_locations,
            remaining_spaces,
        }
    }

    pub fn try_reserve_space(&mut self, generator_type: &GeneratorType) -> bool {
        if let Some(count) = self.remaining_spaces.get_mut(generator_type) {
            if *count > 0 {
                *count -= 1;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn reset_space_counts(&mut self) {
        self.remaining_spaces = self.type_counts.clone();
    }

    pub fn get_remaining_spaces(&self, generator_type: &GeneratorType) -> usize {
        self.remaining_spaces.get(generator_type).copied().unwrap_or(0)
    }

    pub fn print_summary(&self) {
        println!("\nLocation Analysis Summary:");
        println!("-------------------------");
        println!("\nTotal suitable locations found: {}", self.locations.len());
        
        println!("\nSuitable locations by generator type:");
        for (gen_type, count) in &self.type_counts {
            println!("{}: {} locations", gen_type, count);
        }
        
        println!("\nMulti-type locations: {}", self.multi_type_locations.len());
        
        // Print most common multi-type combinations
        let mut combination_counts = HashMap::new();
        for (_, types) in &self.multi_type_locations {
            let mut type_names: Vec<String> = types.iter().map(|t| t.to_string()).collect();
            type_names.sort();
            let key = type_names.join(", ");
            *combination_counts.entry(key).or_insert(0) += 1;
        }
        
        println!("\nMost common multi-type combinations:");
        let mut combinations: Vec<_> = combination_counts.iter().collect();
        combinations.sort_by(|a, b| b.1.cmp(a.1));
        for (types, count) in combinations.iter().take(5) {
            println!("{}: {} locations", types, count);
        }
    }

    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::Write;
        
        let mut file = File::create(path)?;
        
        // Write header
        writeln!(file, "Location Analysis Results")?;
        writeln!(file, "========================\n")?;
        
        // Write summary statistics
        writeln!(file, "Total suitable locations: {}", self.locations.len())?;
        writeln!(file, "Multi-type locations: {}\n", self.multi_type_locations.len())?;
        
        // Write type counts
        writeln!(file, "Locations by Generator Type:")?;
        writeln!(file, "--------------------------")?;
        for (gen_type, count) in &self.type_counts {
            writeln!(file, "{}: {}", gen_type, count)?;
        }
        
        // Write detailed location data
        writeln!(file, "\nDetailed Location Data:")?;
        writeln!(file, "---------------------")?;
        for location in &self.locations {
            writeln!(file, "\nCoordinate: ({}, {})", 
                location.coordinate.x, location.coordinate.y)?;
            for (gen_type, score) in &location.suitability_scores {
                writeln!(file, "  {}: {:.3}", gen_type, score)?;
            }
        }
        
        Ok(())
    }

    pub fn save_cache(&self, cache_dir: &str) -> std::io::Result<()> {
        std::fs::create_dir_all(cache_dir)?;
        let cache_path = Path::new(cache_dir).join("location_analysis.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(cache_path, json)?;
        Ok(())
    }

    pub fn load_cache(cache_dir: &str) -> std::io::Result<Option<Self>> {
        let cache_path = Path::new(cache_dir).join("location_analysis.json");
        if cache_path.exists() {
            let content = std::fs::read_to_string(cache_path)?;
            let analysis: Self = serde_json::from_str(&content)?;
            Ok(Some(analysis))
        } else {
            Ok(None)
        }
    }
} 