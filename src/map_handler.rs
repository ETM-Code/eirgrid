use serde::{Deserialize, Serialize};
use std::fs;
use anyhow::Result;

use crate::generator::Generator;
use crate::settlement::Settlement;
use crate::carbon_offset::CarbonOffset;
use crate::poi::{POI, Coordinate};
use crate::constants::{
    TRANSMISSION_LOSS_WEIGHT,
    PUBLIC_OPINION_WEIGHT,
    CONSTRUCTION_COST_WEIGHT,
};
use crate::simulation_config::{SimulationConfig, GeneratorConstraints};
use crate::power_storage::calculate_max_intermittent_capacity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Map {
    generators: Vec<Generator>,
    settlements: Vec<Settlement>,
    carbon_offsets: Vec<CarbonOffset>,
    config: SimulationConfig,
}

impl Map {
    pub fn new(config: SimulationConfig) -> Self {
        Self {
            generators: Vec::new(),
            settlements: Vec::new(),
            carbon_offsets: Vec::new(),
            config,
        }
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
        self.generators.push(generator);
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

        // Additional power usage from active carbon capture
        let carbon_capture_usage = self.carbon_offsets.iter()
            .map(|offset| offset.get_power_consumption())
            .sum::<f64>();

        settlement_usage + carbon_capture_usage
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
        &self.config.generator_constraints
    }

    pub fn get_settlements(&self) -> &[Settlement] {
        &self.settlements
    }

    pub fn get_carbon_offsets(&self) -> &[CarbonOffset] {
        &self.carbon_offsets
    }
}
