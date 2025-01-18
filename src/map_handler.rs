use serde::{Deserialize, Serialize};
use std::fs;
use anyhow::Result;

use crate::generator::Generator;
use crate::settlement::Settlement;
use crate::carbon_offset::CarbonOffset;
use crate::poi::{POI, Coordinate};
use crate::constants::{
    DISTANCE_OPINION_WEIGHT,
    TYPE_OPINION_WEIGHT,
    COST_OPINION_WEIGHT,
};
use crate::simulation_config::{SimulationConfig, GeneratorConstraints};

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

    pub fn calc_total_population(&mut self, year: u32) -> u32 {
        for settlement in &mut self.settlements {
            settlement.calc_pop(year);
        }
        self.settlements.iter().map(|s| s.current_pop).sum()
    }

    pub fn calc_total_power_usage(&mut self, year: u32) -> f64 {
        // Base power usage from settlements
        let settlement_usage = {
            for settlement in &mut self.settlements {
                settlement.calc_power_usage(year);
            }
            self.settlements.iter().map(|s| s.current_power_usage).sum::<f64>()
        };

        // Additional power usage from active carbon capture
        let carbon_capture_usage = self.carbon_offsets.iter()
            .map(|offset| offset.get_power_consumption())
            .sum::<f64>();

        settlement_usage + carbon_capture_usage
    }

    pub fn calc_total_power_generation(&self, hour: Option<u8>) -> f64 {
        let mut total_generation = 0.0;
        let mut excess_intermittent = 0.0;
        let mut storage_capacity = 0.0;
        
        // First, calculate total storage capacity
        for generator in &self.generators {
            if generator.get_type().is_storage() {
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
            
            if generator.get_type().is_intermittent() {
                intermittent_generation += output;
                if intermittent_generation > max_intermittent {
                    excess_intermittent += output;
                }
            } else if generator.get_type().is_storage() {
                storage_generation += output;
            } else {
                total_generation += output;
            }
        }
        
        // Handle excess intermittent power with storage
        if excess_intermittent > 0.0 {
            let mut stored_power = 0.0;
            for generator in &mut self.generators {
                if generator.get_type().is_storage() {
                    if let Some(storage) = generator.get_storage_system() {
                        stored_power += storage.charge(excess_intermittent);
                    }
                }
            }
            // Only count intermittent power that's either within limits or can be stored
            intermittent_generation = max_intermittent + stored_power;
        }
        
        total_generation + intermittent_generation + storage_generation
    }

    pub fn handle_power_deficit(&mut self, deficit: f64, hour: Option<u8>) -> f64 {
        let mut remaining_deficit = deficit;
        
        // First try to use stored power
        for generator in &mut self.generators {
            if generator.get_type().is_storage() {
                if let Some(storage) = generator.get_storage_system() {
                    let discharged = storage.discharge(remaining_deficit);
                    remaining_deficit -= discharged;
                    
                    if remaining_deficit <= 0.0 {
                        break;
                    }
                }
            }
        }
        
        remaining_deficit
    }

    pub fn get_total_storage_capacity(&self) -> f64 {
        self.generators.iter()
            .filter(|g| g.get_type().is_storage())
            .map(|g| g.get_storage_capacity())
            .sum()
    }

    pub fn get_current_storage_level(&self) -> f64 {
        self.generators.iter()
            .filter(|g| g.get_type().is_storage())
            .filter_map(|g| g.get_storage_system())
            .map(|s| s.current_charge)
            .sum()
    }

    pub fn calc_total_co2_emissions(&self) -> f64 {
        self.generators.iter()
            .map(|g| g.get_current_co2_output())
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

        DISTANCE_OPINION_WEIGHT * avg_settlement_opinion +
        TYPE_OPINION_WEIGHT * type_opinion +
        COST_OPINION_WEIGHT * cost_opinion
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
}

//     pub fn balance_power_generation(&mut self, year: u32) -> Result<(f64, f64)> {
//         let required_power = self.calc_total_power_usage(year);
//         let current_generation = self.calc_total_power_generation();
//         let power_deficit = required_power - current_generation;

//         if power_deficit > 0.0 {
//             // Need to increase generation
//             self.increase_generation(power_deficit, year)?;
//         } else if power_deficit < 0.0 {
//             // Need to decrease generation
//             self.decrease_generation(-power_deficit, year)?;
//         }

//         Ok((required_power, self.calc_total_power_generation()))
//     }

//     fn increase_generation(&mut self, deficit: f64, year: u32) -> Result<()> {
//         // First, try to increase operation percentage of existing generators
//         let mut remaining_deficit = deficit;
        
//         for generator in &mut self.generators {
//             if generator.is_active() && generator.get_operation_percentage() < 1.0 {
//                 let current_output = generator.get_current_power_output();
//                 let max_additional = generator.power_out - current_output;
                
//                 if max_additional > 0.0 {
//                     let increase = remaining_deficit.min(max_additional);
//                     let new_percentage = (current_output + increase) / generator.power_out;
//                     generator.adjust_operation(new_percentage, &self.config.generator_constraints);
//                     remaining_deficit -= increase;
//                 }
//             }
//         }

//         Ok(())
//     }

//     fn decrease_generation(&mut self, surplus: f64, year: u32) -> Result<()> {
//         // Reduce operation of fossil fuel plants first
//         let mut remaining_surplus = surplus;
        
//         // Sort generators by CO2 output per MW
//         let mut generator_indices: Vec<usize> = (0..self.generators.len()).collect();
//         generator_indices.sort_by(|&a, &b| {
//             let gen_a = &self.generators[a];
//             let gen_b = &self.generators[b];
//             let co2_per_mw_a = gen_a.get_current_co2_output() / gen_a.get_current_power_output();
//             let co2_per_mw_b = gen_b.get_current_co2_output() / gen_b.get_current_power_output();
//             co2_per_mw_b.partial_cmp(&co2_per_mw_a).unwrap()
//         });

//         for &idx in &generator_indices {
//             if remaining_surplus <= 0.0 {
//                 break;
//             }

//             let generator = &mut self.generators[idx];
//             if generator.is_active() {
//                 let current_output = generator.get_current_power_output();
//                 let min_output = generator.power_out * self.config.generator_constraints.min_operation_percentage;
//                 let reducible_output = (current_output - min_output).max(0.0);
                
//                 if reducible_output > 0.0 {
//                     let reduction = remaining_surplus.min(reducible_output);
//                     let new_percentage = (current_output - reduction) / generator.power_out;
//                     generator.adjust_operation(new_percentage, &self.config.generator_constraints);
//                     remaining_surplus -= reduction;
//                 }
//             }
//         }

//         Ok(())
//     }



//     pub fn optimize_for_net_zero(&mut self, year: u32) -> Result<()> {
//         if !self.config.target_net_zero_2050 || year != 2050 {
//             return Ok(());
//         }

//         let net_emissions = self.calc_net_co2_emissions(year);
//         if net_emissions <= 0.0 {
//             return Ok(());
//         }

//         // Try efficiency upgrades first
//         for generator in &mut self.generators {
//             if generator.is_active() && generator.can_upgrade_efficiency(year, &self.config.generator_constraints) {
//                 let current_max = self.config.generator_constraints.max_efficiency_by_year
//                     .iter()
//                     .filter(|(y, _)| *y <= year)
//                     .map(|(_, e)| *e)
//                     .max_by(|a, b| a.partial_cmp(b).unwrap())
//                     .unwrap_or(0.4);
                
//                 generator.upgrade_efficiency(year, current_max);
//             }
//         }

//         // If still not at net zero, reduce operation of highest CO2 emitters
//         if self.calc_net_co2_emissions(year) > 0.0 {
//             let mut generator_indices: Vec<usize> = (0..self.generators.len()).collect();
//             generator_indices.sort_by(|&a, &b| {
//                 let gen_a = &self.generators[a];
//                 let gen_b = &self.generators[b];
//                 let co2_per_mw_a = gen_a.get_current_co2_output() / gen_a.get_current_power_output();
//                 let co2_per_mw_b = gen_b.get_current_co2_output() / gen_b.get_current_power_output();
//                 co2_per_mw_b.partial_cmp(&co2_per_mw_a).unwrap()
//             });

//             for &idx in &generator_indices {
//                 if self.calc_net_co2_emissions(year) <= 0.0 {
//                     break;
//                 }

//                 let generator = &mut self.generators[idx];
//                 if generator.is_active() && generator.get_current_co2_output() > 0.0 {
//                     if self.config.allow_generator_closure {
//                         generator.close_generator(year);
//                     } else if self.config.allow_operation_adjustment {
//                         generator.adjust_operation(
//                             self.config.generator_constraints.min_operation_percentage,
//                             &self.config.generator_constraints
//                         );
//                     }
//                 }
//             }
//         }

//         Ok(())
//     }
// } 