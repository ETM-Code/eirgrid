use serde::{Deserialize, Serialize};
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
    pub fn new(capacity: f64) -> Self {
        Self {
            capacity,
            current_charge: 0.0,
            charge_rate: capacity * 0.25,      // Typical charge rate is 25% of capacity per hour
            discharge_rate: capacity * 0.25,    // Typical discharge rate is 25% of capacity per hour
            efficiency: 0.85, // Default efficiency for storage systems
        }
    }

    pub fn discharge(&mut self, amount: f64) -> f64 {
        let actual_discharge = amount.min(self.current_charge);
        self.current_charge -= actual_discharge;
        actual_discharge * self.efficiency
    }
}

pub fn calculate_max_intermittent_capacity(total_power_needed: f64, storage_capacity: f64) -> f64 {
    // Without storage, limit intermittent sources to a percentage of total power needed
    let base_limit = total_power_needed * MAX_INTERMITTENT_PERCENTAGE;
    
    // Storage allows exceeding this limit
    let storage_bonus = storage_capacity * STORAGE_CAPACITY_FACTOR;
    
    base_limit + storage_bonus
} 