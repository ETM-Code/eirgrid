use serde::{Serialize, Deserialize};
use crate::generator::{Generator, GeneratorType};
use crate::constants::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerStorageSystem {
    pub capacity: f64,          // Maximum storage capacity in MWh
    pub current_charge: f64,    // Current stored energy in MWh
    pub charge_rate: f64,       // Maximum rate of charging in MW
    pub discharge_rate: f64,    // Maximum rate of discharging in MW
    pub efficiency: f64,        // Round-trip efficiency
}

impl PowerStorageSystem {
    pub fn new(capacity: f64, charge_rate: f64, discharge_rate: f64, efficiency: f64) -> Self {
        Self {
            capacity,
            current_charge: 0.0,
            charge_rate,
            discharge_rate,
            efficiency,
        }
    }

    pub fn charge(&mut self, available_power: f64) -> f64 {
        let space_available = self.capacity - self.current_charge;
        let max_charge = self.charge_rate.min(space_available);
        let actual_charge = available_power.min(max_charge);
        
        self.current_charge += actual_charge * self.efficiency.sqrt(); // Square root because efficiency is round-trip
        actual_charge
    }

    pub fn discharge(&mut self, power_needed: f64) -> f64 {
        let max_discharge = self.discharge_rate.min(self.current_charge);
        let actual_discharge = power_needed.min(max_discharge);
        
        self.current_charge -= actual_discharge / self.efficiency.sqrt();
        actual_discharge
    }
}

pub fn calculate_intermittent_output(generator: &Generator, hour: u8) -> f64 {
    let base_output = generator.get_current_power_output();
    
    match generator.get_type() {
        GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => {
            // Wind tends to be stronger at night
            let time_factor = match hour {
                0..=6 => 0.8,   // Early morning: moderate
                7..=11 => 0.6,  // Morning: lower
                12..=17 => 0.5, // Afternoon: lowest
                18..=21 => 0.7, // Evening: picking up
                _ => 0.8,       // Night: stronger
            };
            
            // Add some randomness to simulate wind variability
            let variability = rand::thread_rng().gen_range(0.4..1.2);
            base_output * time_factor * variability
        },
        
        GeneratorType::UtilitySolar | GeneratorType::CommercialSolar | GeneratorType::DomesticSolar => {
            // Solar follows a daily curve
            let time_factor = match hour {
                0..=5 => 0.0,   // Night: no generation
                6..=7 => 0.2,   // Dawn: starting up
                8..=9 => 0.5,   // Morning: ramping up
                10..=14 => 1.0, // Midday: peak
                15..=16 => 0.5, // Afternoon: ramping down
                17..=18 => 0.2, // Dusk: winding down
                _ => 0.0,       // Night: no generation
            };
            
            // Add weather variability
            let weather_factor = rand::thread_rng().gen_range(0.6..1.0);
            base_output * time_factor * weather_factor
        },
        
        _ => base_output, // Non-intermittent sources
    }
}

pub fn get_storage_capacity(gen_type: &GeneratorType) -> Option<PowerStorageSystem> {
    match gen_type {
        GeneratorType::PumpedStorage => Some(PowerStorageSystem::new(
            PUMPED_STORAGE_CAPACITY,
            PUMPED_STORAGE_CHARGE_RATE,
            PUMPED_STORAGE_DISCHARGE_RATE,
            PUMPED_STORAGE_EFFICIENCY,
        )),
        GeneratorType::BatteryStorage => Some(PowerStorageSystem::new(
            BATTERY_STORAGE_CAPACITY,
            BATTERY_STORAGE_CHARGE_RATE,
            BATTERY_STORAGE_DISCHARGE_RATE,
            BATTERY_STORAGE_EFFICIENCY,
        )),
        _ => None,
    }
}

pub fn calculate_max_intermittent_capacity(total_power_needed: f64, storage_capacity: f64) -> f64 {
    // Without storage, limit intermittent sources to a percentage of total power needed
    let base_limit = total_power_needed * MAX_INTERMITTENT_PERCENTAGE;
    
    // Storage allows exceeding this limit
    let storage_bonus = storage_capacity * STORAGE_CAPACITY_FACTOR;
    
    base_limit + storage_bonus
} 