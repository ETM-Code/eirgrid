use serde::{Deserialize, Serialize};
use std::fs;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

use crate::generator::{Generator, GeneratorType};
use crate::settlement::Settlement;
use crate::carbon_offset::CarbonOffset;
use crate::poi::{POI, Coordinate};
use crate::constants::{
    TRANSMISSION_LOSS_WEIGHT,
    PUBLIC_OPINION_WEIGHT,
    CONSTRUCTION_COST_WEIGHT,
    GRID_CELL_SIZE,
    MAP_MAX_X,
    MAP_MAX_Y,
};
use crate::const_funcs::is_point_inside_polygon;
use crate::simulation_config::{SimulationConfig, GeneratorConstraints};
use crate::power_storage::calculate_max_intermittent_capacity;
use crate::spatial_index::{SpatialIndex, GeneratorSuitabilityType};
use crate::metal_location_search::MetalLocationSearch;

// Static data that doesn't change during simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapStaticData {
    config: SimulationConfig,
    coastline_points: Vec<Coordinate>,
}

// Remove automatic derive for Map
#[derive(Debug, Clone)]
pub struct Map {
    static_data: Arc<MapStaticData>,
    generators: Vec<Generator>,
    settlements: Vec<Settlement>,
    carbon_offsets: Vec<CarbonOffset>,
    grid_occupancy: HashMap<(i32, i32), f64>,
    pub spatial_index: SpatialIndex,
    metal_location_search: Option<MetalLocationSearch>,
}

// Custom serialization implementation
impl Serialize for Map {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Map", 5)?;
        state.serialize_field("static_data", &*self.static_data)?;
        state.serialize_field("generators", &self.generators)?;
        state.serialize_field("settlements", &self.settlements)?;
        state.serialize_field("carbon_offsets", &self.carbon_offsets)?;
        state.serialize_field("grid_occupancy", &self.grid_occupancy)?;
        state.end()
    }
}

// Custom deserialization implementation
impl<'de> Deserialize<'de> for Map {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            static_data: MapStaticData,
            generators: Vec<Generator>,
            settlements: Vec<Settlement>,
            carbon_offsets: Vec<CarbonOffset>,
            grid_occupancy: HashMap<(i32, i32), f64>,
        }

        let helper = Helper::deserialize(deserializer)?;
        Ok(Map {
            static_data: Arc::new(helper.static_data),
            generators: helper.generators,
            settlements: helper.settlements,
            carbon_offsets: helper.carbon_offsets,
            grid_occupancy: helper.grid_occupancy,
            spatial_index: SpatialIndex::new(),
            metal_location_search: None,
        })
    }
}

impl Map {
    pub fn new(config: SimulationConfig) -> Self {
        let metal_location_search = MetalLocationSearch::new().ok();
        if metal_location_search.is_none() {
            println!("Warning: Metal-based location search not available, falling back to CPU implementation");
        }

        let coastline_json: serde_json::Value = serde_json::from_str(
            include_str!("coastline_points.json")
        ).expect("Failed to load coastline points");

        let coastline_points: Vec<Coordinate> = coastline_json["grid_coords"]
            .as_array()
            .expect("Invalid coastline format")
            .iter()
            .map(|point| {
                let coords = point.as_array().expect("Invalid point format");
                Coordinate::new(
                    coords[0].as_f64().expect("Invalid x coordinate"),
                    coords[1].as_f64().expect("Invalid y coordinate")
                )
            })
            .collect();

        let static_data = Arc::new(MapStaticData {
            config,
            coastline_points,
        });

        let mut map = Self {
            static_data,
            generators: Vec::new(),
            settlements: Vec::new(),
            carbon_offsets: Vec::new(),
            grid_occupancy: HashMap::new(),
            spatial_index: SpatialIndex::new(),
            metal_location_search,
        };

        map.initialize_spatial_index();
        map
    }

    pub fn new_with_static_data(static_data: Arc<MapStaticData>) -> Self {
        let metal_location_search = MetalLocationSearch::new().ok();
        if metal_location_search.is_none() {
            println!("Warning: Metal-based location search not available, falling back to CPU implementation");
        }

        Self {
            static_data,
            generators: Vec::new(),
            settlements: Vec::new(),
            carbon_offsets: Vec::new(),
            grid_occupancy: HashMap::new(),
            spatial_index: SpatialIndex::new(),
            metal_location_search,
        }
    }

    fn initialize_spatial_index(&mut self) {
        // Initialize coastal regions with a wider influence area
        for point in &self.static_data.coastline_points {
            self.spatial_index.update_region(
                point,
                8000.0,
                GeneratorSuitabilityType::Coastal,
                0.6,    // Reduced from 0.8 to allow more flexibility
            );
        }

        // Initialize urban areas based on settlements
        for settlement in &self.settlements {
            let coord = settlement.get_coordinate();
            let population = settlement.get_population();
            let radius = (population as f64).sqrt() * 15.0;
            let urban_score = (population as f64).log10() / 7.0;
            
            self.spatial_index.update_region(
                coord,
                radius,
                GeneratorSuitabilityType::Urban,
                urban_score.clamp(0.2, 0.8), // More lenient clamping
            );

            // Protected zone is now much smaller and less restrictive
            self.spatial_index.update_region(
                coord,
                radius * 0.1, // Reduced from 0.2
                GeneratorSuitabilityType::Protected,
                0.7, // Reduced from 0.9
            );
        }

        // Mark areas with existing generators as occupied
        for generator in &self.generators {
            if generator.is_active() {
                let coord = generator.get_coordinate();
                let size = generator.size;
                let radius = (size * GRID_CELL_SIZE).sqrt() * 1.2; // Reduced from 1.5
                
                let suitability_type = match generator.get_generator_type() {
                    GeneratorType::OnshoreWind => GeneratorSuitabilityType::Onshore,
                    GeneratorType::OffshoreWind => GeneratorSuitabilityType::Offshore,
                    GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => GeneratorSuitabilityType::Coastal,
                    _ => GeneratorSuitabilityType::Rural,
                };
                
                // Smaller protected area with lower protection
                self.spatial_index.update_region(
                    coord,
                    radius * 0.5, // Reduced from 1.0
                    GeneratorSuitabilityType::Protected,
                    0.6, // Reduced from 0.9
                );
                
                // Reduced impact on surrounding area
                self.spatial_index.update_region(
                    coord,
                    radius * 1.5, // Reduced from 2.0
                    suitability_type,
                    0.4, // Reduced from 0.6
                );
            }
        }

        // Initialize rural areas with lower base suitability
        self.spatial_index.update_region(
            &Coordinate::new(MAP_MAX_X / 2.0, MAP_MAX_Y / 2.0),
            (MAP_MAX_X.powi(2) + MAP_MAX_Y.powi(2)).sqrt() / 2.0,
            GeneratorSuitabilityType::Rural,
            0.5, // Reduced from 0.8 to provide more flexibility
        );

        // Initialize offshore areas with wider influence but lower base score
        for point in &self.static_data.coastline_points {
            self.spatial_index.update_region(
                point,
                20000.0,
                GeneratorSuitabilityType::Offshore,
                0.5, // Reduced from 0.7
            );
        }
    }

    pub fn get_static_data(&self) -> Arc<MapStaticData> {
        Arc::clone(&self.static_data)
    }

    pub fn set_generators(&mut self, generators: Vec<Generator>) {
        self.generators = generators;
        self.grid_occupancy.clear();
        for generator in &self.generators {
            if generator.is_active() {
                let x = (generator.get_coordinate().x / GRID_CELL_SIZE).floor() as i32;
                let y = (generator.get_coordinate().y / GRID_CELL_SIZE).floor() as i32;
                *self.grid_occupancy.entry((x, y)).or_insert(0.0) += generator.size;
            }
        }
        self.initialize_spatial_index();
    }

    pub fn set_settlements(&mut self, settlements: Vec<Settlement>) {
        self.settlements = settlements;
        self.initialize_spatial_index();
    }

    pub fn set_carbon_offsets(&mut self, offsets: Vec<CarbonOffset>) {
        self.carbon_offsets = offsets;
    }

    pub fn load_coastline(&mut self, coastline_points: Vec<Coordinate>) {
        self.static_data = Arc::new(MapStaticData {
            config: self.static_data.config.clone(),
            coastline_points,
        });
    }

    pub fn load_from_json(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let map: Map = serde_json::from_str(&content)?;
        Ok(map)
    }

    pub fn save_to_json(&self, path: &str) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn add_generator(&mut self, generator: Generator) {
        let coord = generator.get_coordinate();
        let size = generator.size;
        
        // Update grid occupancy
        let grid_x = (coord.x / GRID_CELL_SIZE).floor() as i32;
        let grid_y = (coord.y / GRID_CELL_SIZE).floor() as i32;
        *self.grid_occupancy.entry((grid_x, grid_y)).or_insert(0.0) += size;

        // Update spatial index
        let radius = (size * GRID_CELL_SIZE).sqrt() * 1.5; // Reduced from 2.0
        
        // Create a smaller protected zone with lower protection value
        self.spatial_index.update_region(
            coord,
            radius * 0.5, // Smaller protected radius
            GeneratorSuitabilityType::Protected,
            0.7, // Lower protection value to allow some flexibility
        );
        
        // Update suitability for the generator type in surrounding area
        let (suitability_type, base_score) = match generator.get_generator_type() {
            GeneratorType::OnshoreWind => (GeneratorSuitabilityType::Onshore, 0.4),
            GeneratorType::OffshoreWind => (GeneratorSuitabilityType::Offshore, 0.5),
            GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => (GeneratorSuitabilityType::Coastal, 0.5),
            _ => (GeneratorSuitabilityType::Rural, 0.3),
        };
        
        // Create graduated zones of influence
        let zones = [
            (1.0, base_score * 0.8),    // Inner zone
            (2.0, base_score * 0.6),    // Middle zone
            (3.0, base_score * 0.4),    // Outer zone
        ];

        for (radius_mult, score) in zones.iter() {
            self.spatial_index.update_region(
                coord,
                radius * radius_mult,
                suitability_type,
                *score,
            );
        }

        self.generators.push(generator);
    }

    pub fn remove_generator(&mut self, id: &str) -> Option<Generator> {
        if let Some(index) = self.generators.iter().position(|g| g.get_id() == id) {
            let generator = self.generators.remove(index);
            let coord = generator.get_coordinate();
            let size = generator.size;
            
            // Update grid occupancy
            let grid_x = (coord.x / GRID_CELL_SIZE).floor() as i32;
            let grid_y = (coord.y / GRID_CELL_SIZE).floor() as i32;
            if let Some(occupancy) = self.grid_occupancy.get_mut(&(grid_x, grid_y)) {
                *occupancy = (*occupancy - size).max(0.0);
            }

            // Update spatial index
            let radius = (size * GRID_CELL_SIZE).sqrt() * 2.0;
            
            // Remove Protected status from the immediate area
            self.spatial_index.update_region(
                coord,
                radius,
                GeneratorSuitabilityType::Protected,
                0.0,
            );
            
            // Restore suitability in the surrounding area
            let suitability_type = match generator.get_generator_type() {
                GeneratorType::OnshoreWind => GeneratorSuitabilityType::Onshore,
                GeneratorType::OffshoreWind => GeneratorSuitabilityType::Offshore,
                GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => GeneratorSuitabilityType::Coastal,
                _ => GeneratorSuitabilityType::Rural,
            };
            
            self.spatial_index.update_region(
                coord,
                radius * 3.0,
                suitability_type,
                1.0, // Restore full suitability
            );

            Some(generator)
        } else {
            None
        }
    }

    pub fn add_settlement(&mut self, settlement: Settlement) {
        self.settlements.push(settlement);
    }

    pub fn add_carbon_offset(&mut self, offset: CarbonOffset) {
        self.carbon_offsets.push(offset);
    }

    pub fn calc_total_population(&self, year: u32) -> u32 {
        self.settlements.iter()
            .map(|s| s.get_population())
            .sum()
    }

    pub fn calc_total_power_usage(&self, year: u32) -> f64 {
        // Base power usage from settlements
        let settlement_usage = self.settlements.iter()
            .map(|s| s.get_power_usage())
            .sum::<f64>();

        // Add growth factor based on year
        settlement_usage * (1.0 + (year as f64 - 2024.0) * 0.02)
    }

    pub fn get_storage_generators(&mut self) -> Vec<&mut Generator> {
        self.generators.iter_mut()
            .filter(|g| g.get_generator_type().is_storage())
            .collect()
    }

    pub fn get_storage_generators_mut(&mut self) -> Vec<&mut Generator> {
        self.generators.iter_mut()
            .filter(|g| g.get_generator_type().is_storage())
            .collect()
    }

    pub fn calc_total_power_generation(&self, year: u32, hour: Option<u8>) -> f64 {
        let mut total_generation = 0.0;
        let mut excess_intermittent = 0.0;
        let mut storage_capacity = 0.0;
        
        // First, calculate total storage capacity
        for generator in &self.generators {
            if generator.get_generator_type().is_storage() {
                storage_capacity += generator.get_storage_capacity();
            }
        }
        
        // Calculate total power needed for proper intermittent limits
        let total_power_needed = self.calc_total_power_usage(year);
        let max_intermittent = calculate_max_intermittent_capacity(total_power_needed, storage_capacity);
        
        // Calculate generation from each source
        let mut intermittent_generation = 0.0;
        let mut storage_generation = 0.0;
        
        for generator in &self.generators {
            let output = generator.get_current_power_output(hour);
            
            if generator.get_generator_type().is_intermittent() {
                intermittent_generation += output;
                if intermittent_generation > max_intermittent {
                    excess_intermittent += output;
                }
            } else if generator.get_generator_type().is_storage() {
                storage_generation += output;
            } else {
                total_generation += output;
            }
        }
        
        total_generation + intermittent_generation + storage_generation
    }

    pub fn handle_power_deficit(&mut self, deficit: f64, hour: Option<u8>) -> f64 {
        let mut remaining_deficit = deficit;
        
        // First try to use stored power
        for generator in &mut self.generators {
            if !generator.get_generator_type().is_storage() {
                continue;
            }
            
            if let Some(storage) = &mut generator.storage {
                let discharged = storage.discharge(remaining_deficit);
                remaining_deficit -= discharged;
                
                if remaining_deficit <= 0.0 {
                    break;
                }
            }
        }
        
        remaining_deficit
    }

    pub fn get_total_storage_capacity(&self) -> f64 {
        self.generators.iter()
            .filter(|g| g.get_generator_type().is_storage())
            .map(|g| g.get_storage_capacity())
            .sum()
    }

    pub fn get_current_storage_level(&self) -> f64 {
        self.generators.iter()
            .filter(|g| g.get_generator_type().is_storage())
            .filter_map(|g| g.storage.as_ref())
            .map(|s| s.current_charge)
            .sum()
    }

    pub fn calc_total_co2_emissions(&self) -> f64 {
        self.generators.iter()
            .filter(|g| g.is_active())
            .map(|g| g.get_co2_output())
            .sum()
    }

    pub fn calc_total_carbon_offset(&self, year: u32) -> f64 {
        self.carbon_offsets.iter()
            .map(|offset| offset.calc_carbon_offset(year))
            .sum()
    }

    pub fn calc_net_co2_emissions(&self, year: u32) -> f64 {
        self.calc_total_co2_emissions() - self.calc_total_carbon_offset(year)
    }

    pub fn calc_new_generator_opinion(
        &self,
        coordinate: &Coordinate,
        generator: &Generator,
        year: u32,
    ) -> f64 {
        let settlement_opinions: f64 = self
            .settlements
            .iter()
            .map(|s| s.calc_range_opinion(coordinate))
            .sum();

        let avg_settlement_opinion = if !self.settlements.is_empty() {
            settlement_opinions / self.settlements.len() as f64
        } else {
            1.0
        };

        let type_opinion = generator.calc_type_opinion(year);
        let cost_opinion = generator.calc_cost_opinion(year);

        TRANSMISSION_LOSS_WEIGHT * avg_settlement_opinion +
        PUBLIC_OPINION_WEIGHT * type_opinion +
        CONSTRUCTION_COST_WEIGHT * cost_opinion
    }

    pub fn calc_total_operating_cost(&self, year: u32) -> f64 {
        let generator_costs = self.generators.iter()
            .map(|g| g.get_current_operating_cost(year))
            .sum::<f64>();

        let offset_costs = self.carbon_offsets.iter()
            .map(|o| o.get_current_operating_cost(year))
            .sum::<f64>();

        generator_costs + offset_costs
    }

    pub fn calc_total_capital_cost(&self, year: u32) -> f64 {
        let generator_costs = self.generators.iter()
            .map(|g| g.get_current_cost(year))
            .sum::<f64>();

        let offset_costs = self.carbon_offsets.iter()
            .map(|o| o.get_current_cost(year))
            .sum::<f64>();

        generator_costs + offset_costs
    }

    pub fn get_generators(&self) -> &[Generator] {
        &self.generators
    }

    pub fn get_generator_mut(&mut self, id: &str) -> Option<&mut Generator> {
        self.generators.iter_mut().find(|g| g.get_id() == id)
    }

    pub fn get_generator_count(&self) -> usize {
        self.generators.len()
    }

    pub fn get_carbon_offset_count(&self) -> usize {
        self.carbon_offsets.len()
    }

    pub fn get_generator_constraints(&self) -> &GeneratorConstraints {
        &self.static_data.config.generator_constraints
    }

    pub fn get_settlements(&self) -> &Vec<Settlement> {
        &self.settlements
    }

    pub fn get_carbon_offsets(&self) -> &[CarbonOffset] {
        &self.carbon_offsets
    }

    pub fn get_generator_grid_occupancy(&self) -> &HashMap<(i32, i32), f64> {
        &self.grid_occupancy
    }

    pub fn update_grid_occupancy(&mut self) {
        self.grid_occupancy.clear();
        for generator in &self.generators {
            if generator.is_active() {
                let x = (generator.get_coordinate().x / GRID_CELL_SIZE).floor() as i32;
                let y = (generator.get_coordinate().y / GRID_CELL_SIZE).floor() as i32;
                *self.grid_occupancy.entry((x, y)).or_insert(0.0) += generator.size;
            }
        }
    }

    // Add a method to handle generator state changes
    pub fn handle_generator_state_change(&mut self) {
        self.update_grid_occupancy();
    }

    // Add method to be called after generator modifications
    pub fn after_generator_modification(&mut self) {
        self.update_grid_occupancy();
    }

    pub fn calculate_suitability(&self, location: &Coordinate, generator_type: &GeneratorType) -> f64 {
        let mut score = match generator_type {
            GeneratorType::OnshoreWind => {
                let base = if self.is_coastal_region(location) { 0.7 } else { 0.5 };
                if self.is_urban_area(location) { base * 0.3 } else { base }
            },
            GeneratorType::OffshoreWind => {
                if self.is_offshore_region(location) { 0.8 } else { 0.0 }
            },
            GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => {
                if self.is_offshore_region(location) { 0.8 } else { 0.0 }
            },
            GeneratorType::Nuclear => {
                if self.is_urban_area(location) { 0.0 } else { 0.7 }
            },
            GeneratorType::DomesticSolar | GeneratorType::CommercialSolar => {
                if self.is_urban_area(location) { 0.6 } else { 0.4 }
            },
            GeneratorType::UtilitySolar => {
                if self.is_urban_area(location) { 0.3 } else { 0.5 }
            },
            GeneratorType::HydroDam | GeneratorType::PumpedStorage => {
                if self.is_near_water(location) { 0.7 } else { 0.0 }
            },
            _ => {
                if self.is_urban_area(location) { 0.3 } else { 0.5 }
            }
        };

        // Reduce score based on proximity to other generators
        let nearby_generators = self.get_nearby_generators(location, 5000.0);
        if !nearby_generators.is_empty() {
            score *= 0.8;
        }

        // Add bonus for coastal regions for certain types
        if self.is_coastal_region(location) {
            match generator_type {
                GeneratorType::OnshoreWind => score *= 1.2,
                GeneratorType::OffshoreWind | GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => score *= 1.3,
                _ => {}
            }
        }

        // Adjust score based on terrain suitability
        score *= self.get_terrain_suitability(location, generator_type);

        score
    }

    pub fn find_best_generator_location(&self, generator_type: &GeneratorType, size: f64) -> Option<Coordinate> {
        // Try Metal-based search first if available
        if let Some(metal_search) = &self.metal_location_search {
            if let Some(location) = metal_search.find_best_location(
                generator_type,
                &self.settlements,
                &self.generators,
                &self.static_data.coastline_points,
                size as f32,
            ) {
                return Some(location);
            }
        }

        // Fall back to CPU implementation if Metal search fails or is unavailable
        let initial_min_score = match generator_type {
            GeneratorType::OnshoreWind => 0.2,
            GeneratorType::OffshoreWind => 0.3,
            GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => 0.35,
            GeneratorType::Nuclear => 0.4,
            GeneratorType::DomesticSolar | GeneratorType::CommercialSolar => 0.2,
            GeneratorType::UtilitySolar => 0.3,
            GeneratorType::HydroDam | GeneratorType::PumpedStorage => 0.35,
            _ => 0.15,
        };

        let reduction_steps = [1.0, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3];
        let size_penalty = 0.03 * size;

        for reduction in reduction_steps.iter() {
            let min_score = initial_min_score * reduction;
            if let Some(location) = self.find_location_with_min_score(generator_type.clone(), min_score, size_penalty) {
                if *reduction < 1.0 {
                    println!("Found location for {} generator with {:.1}% of original requirements (score: {:.2}, size factor: {:.2})",
                        generator_type, reduction * 100.0, min_score, size_penalty);
                }
                return Some(location);
            } else {
                println!("Failed to find location for {} at {:.1}% requirements (min score: {:.3})",
                    generator_type, reduction * 100.0, min_score);
            }
        }

        None
    }

    // Add helper methods for location checks
    fn is_coastal_region(&self, location: &Coordinate) -> bool {
        // Check if within 8km of coastline
        let coastal_distance = 8000.0;
        for x in -1..=1 {
            for y in -1..=1 {
                let check_point = Coordinate::new(
                    location.x + (x as f64 * coastal_distance),
                    location.y + (y as f64 * coastal_distance)
                );
                if self.is_water_tile(&check_point) {
                    return true;
                }
            }
        }
        false
    }

    fn is_offshore_region(&self, location: &Coordinate) -> bool {
        self.is_water_tile(location)
    }

    fn is_urban_area(&self, location: &Coordinate) -> bool {
        // Check if location is within urban area bounds
        for settlement in &self.settlements {
            let distance = settlement.get_coordinate().distance_to(location);
            let radius = (settlement.get_population() as f64).sqrt() * 5.0; // Scale radius with population
            if distance < radius {
                return true;
            }
        }
        false
    }

    fn is_near_water(&self, location: &Coordinate) -> bool {
        // Check if within 5km of water (rivers, lakes, sea)
        let water_distance = 5000.0;
        for x in -1..=1 {
            for y in -1..=1 {
                let check_point = Coordinate::new(
                    location.x + (x as f64 * water_distance),
                    location.y + (y as f64 * water_distance)
                );
                if self.is_water_tile(&check_point) {
                    return true;
                }
            }
        }
        false
    }

    fn get_terrain_suitability(&self, location: &Coordinate, generator_type: &GeneratorType) -> f64 {
        match generator_type {
            GeneratorType::OnshoreWind => {
                // Prefer elevated areas for wind
                let elevation = self.get_elevation(location);
                if elevation > 200.0 { 1.2 }
                else if elevation > 100.0 { 1.1 }
                else { 1.0 }
            },
            GeneratorType::UtilitySolar => {
                // Prefer flat, open areas
                let elevation = self.get_elevation(location);
                if elevation < 100.0 && !self.is_near_water(location) { 1.2 }
                else { 1.0 }
            },
            GeneratorType::Nuclear => {
                // Must be on stable ground, near water for cooling
                if self.is_near_water(location) && !self.is_coastal_region(location) { 1.2 }
                else { 0.8 }
            },
            _ => 1.0
        }
    }

    fn get_nearby_generators(&self, location: &Coordinate, radius: f64) -> Vec<&Generator> {
        self.generators.iter()
            .filter(|g| g.get_coordinate().distance_to(location) < radius)
            .collect()
    }

    fn get_elevation(&self, location: &Coordinate) -> f64 {
        // Simple elevation check based on terrain data
        // This would need actual terrain data in a real implementation
        0.0 // Placeholder
    }

    fn get_ireland_bounds(&self) -> Bounds {
        // Return bounds for Ireland's territory
        Bounds {
            min: Coordinate::new(-100000.0, -100000.0),
            max: Coordinate::new(100000.0, 100000.0)
        }
    }

    fn is_water_tile(&self, location: &Coordinate) -> bool {
        // Use the point-in-polygon algorithm to check if the point is inside Ireland's landmass
        // If the point is inside the polygon formed by coastline points, it's land
        // If it's outside, it's water
        !is_point_inside_polygon(location, &self.static_data.coastline_points)
    }

    pub fn find_location_with_min_score(&self, generator_type: GeneratorType, min_score: f64, size_penalty: f64) -> Option<Coordinate> {
        let mut best_location = None;
        let mut best_score = min_score;

        let bounds = self.get_ireland_bounds();
        let step_size = 1000.0; // Use a fixed step size for grid search
        
        let x_steps = ((bounds.max.x - bounds.min.x) / step_size).ceil() as i32;
        let y_steps = ((bounds.max.y - bounds.min.y) / step_size).ceil() as i32;

        for i in 0..=x_steps {
            let x = bounds.min.x + (i as f64 * step_size);
            for j in 0..=y_steps {
                let y = bounds.min.y + (j as f64 * step_size);
                let coordinate = Coordinate::new(x, y);
                let base_score = self.calculate_generator_suitability(&coordinate, &generator_type);
                
                // Apply size penalty based on generator type
                let size_factor = match generator_type {
                    GeneratorType::Nuclear => 0.8,
                    GeneratorType::CoalPlant | GeneratorType::GasCombinedCycle => 0.6,
                    GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => 0.4,
                    _ => 0.3,
                };
                
                let final_score = base_score - (size_factor * size_penalty);

                if final_score > best_score {
                    best_score = final_score;
                    best_location = Some(coordinate);
                }
            }
        }

        best_location
    }

    pub fn calculate_generator_suitability(&self, coordinate: &Coordinate, generator_type: &GeneratorType) -> f64 {
        match generator_type {
            GeneratorType::OnshoreWind => {
                let base_score = if self.is_urban_area(coordinate) {
                    0.0
                } else if self.is_coastal_region(coordinate) {
                    0.7
                } else {
                    0.5
                };
                
                let nearby_penalty = self.get_nearby_generators(coordinate, 3000.0)
                    .iter()
                    .map(|g| 0.1 / (1.0 + g.get_coordinate().distance_to(coordinate)))
                    .sum::<f64>();

                base_score - nearby_penalty
            },
            GeneratorType::OffshoreWind | GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => {
                if !self.is_offshore_region(coordinate) {
                    return 0.0;
                }
                
                let depth_factor = if self.is_water_tile(coordinate) { 0.8 } else { 0.0 };
                let shore_distance = self.get_distance_to_nearest_land(coordinate);
                let distance_factor = if shore_distance < 2000.0 { 
                    0.3 
                } else if shore_distance > 10000.0 { 
                    0.5 
                } else {
                    0.7
                };
                
                depth_factor * distance_factor
            },
            GeneratorType::Nuclear => {
                if self.is_urban_area(coordinate) || self.is_offshore_region(coordinate) {
                    return 0.0;
                }
                
                let water_proximity = if self.is_near_water(coordinate) { 0.3 } else { 0.0 };
                let population_factor = if self.get_nearby_population(coordinate, 5000.0) < 10000 { 0.7 } else { 0.0 };
                
                0.4 * water_proximity + 0.6 * population_factor
            },
            GeneratorType::UtilitySolar | GeneratorType::DomesticSolar | GeneratorType::CommercialSolar => {
                if self.is_offshore_region(coordinate) {
                    return 0.0;
                }
                
                let terrain_score = self.get_terrain_suitability(coordinate, generator_type);
                let sunlight_factor = 0.8; // Ireland has relatively uniform sunlight patterns
                
                0.6 * terrain_score + 0.4 * sunlight_factor
            },
            GeneratorType::HydroDam | GeneratorType::PumpedStorage => {
                if !self.is_near_water(coordinate) || self.is_urban_area(coordinate) {
                    return 0.0;
                }
                
                let elevation = self.get_elevation(coordinate);
                let water_proximity = if self.is_near_water(coordinate) { 0.8 } else { 0.0 };
                
                0.5 * elevation + 0.5 * water_proximity
            },
            _ => {
                if self.is_offshore_region(coordinate) || self.is_urban_area(coordinate) {
                    return 0.0;
                }
                
                let terrain_score = self.get_terrain_suitability(coordinate, generator_type);
                0.7 * terrain_score + 0.3 * 0.5 // Use a default accessibility score of 0.5
            }
        }
    }

    fn get_distance_to_nearest_land(&self, coordinate: &Coordinate) -> f64 {
        let mut min_distance = f64::MAX;
        let search_radius = 10;
        let step = 1000.0; // Step size in meters
        
        let bounds = self.get_ireland_bounds();
        for i in -search_radius..=search_radius {
            for j in -search_radius..=search_radius {
                let x = coordinate.x + (i as f64 * step);
                let y = coordinate.y + (j as f64 * step);
                
                if x >= bounds.min.x && x <= bounds.max.x && 
                   y >= bounds.min.y && y <= bounds.max.y {
                    let test_coord = Coordinate::new(x, y);
                    if !self.is_water_tile(&test_coord) {
                        let distance = coordinate.distance_to(&test_coord);
                        min_distance = min_distance.min(distance);
                    }
                }
            }
        }
        
        min_distance
    }

    fn get_nearby_population(&self, coordinate: &Coordinate, radius: f64) -> u32 {
        let mut total_population = 0;
        
        for settlement in &self.settlements {
            if settlement.get_coordinate().distance_to(coordinate) <= radius {
                total_population += settlement.get_population();
            }
        }
        
        total_population
    }
}

// Add the Bounds struct if it doesn't exist
#[derive(Debug, Clone)]
pub struct Bounds {
    pub min: Coordinate,
    pub max: Coordinate,
}
