// Deficit handling for ActionWeights

use std::collections::HashMap;
use crate::models::generator::GeneratorType;
use crate::models::carbon_offset::CarbonOffsetType;
use crate::ai::actions::grid_action::GridAction;
use crate::ai::learning::constants::*;
use crate::config::constants::DEFAULT_COST_MULTIPLIER;
use super::ActionWeights;

// Add a dummy public item to ensure this file is recognized by rust-analyzer
#[allow(dead_code)]
pub const MODULE_MARKER: &str = "deficit_module";

impl ActionWeights {

// This file contains extracted code from the original weights.rs file
// Appropriate imports will need to be added based on the specific requirements

    pub fn update_best_deficit_actions(&mut self) {
        if self.iterations_without_improvement == ZERO_U32 {
            // We just found a better strategy, so update best deficit actions
            // Make sure we have entries for each year even if they're empty
            let mut complete_deficit_actions = HashMap::new();
            
            // Initialize empty action lists for all years
            for year in START_YEAR..=END_YEAR {
                complete_deficit_actions.insert(year, Vec::new());
            }
            
            // Then copy over any deficit actions we actually have
            for (year, actions) in &self.current_deficit_actions {
                if !actions.is_empty() {
                    // println!("DEBUG: Copying {} deficit actions for year {} to best_deficit_actions", 
                    //          actions.len(), year);
                    complete_deficit_actions.insert(*year, actions.clone());
                }
            }
            
            // Debug: Check if we're actually capturing any deficit actions
            let total_complete_actions = complete_deficit_actions.values().map(|v| v.len()).sum::<usize>();
            let years_with_actions = complete_deficit_actions.values().filter(|v| !v.is_empty()).count();
            println!("DEBUG: Created best_deficit_actions map with {} actions across {} years", 
                     total_complete_actions, years_with_actions);
            
            // Store the complete map
            self.best_deficit_actions = Some(complete_deficit_actions);
        }
    }

    pub fn has_deficit_actions_for_year(&self, year: u32) -> bool {
        self.current_deficit_actions.get(&year)
            .map_or(false, |actions| !actions.is_empty())
    }

    pub fn get_deficit_actions_for_year(&self, year: u32) -> Option<Vec<GridAction>> {
        self.current_deficit_actions.get(&year)
            .map(|actions| actions.clone())
    }

    pub fn get_best_deficit_actions_for_year(&self, year: u32) -> Option<&Vec<GridAction>> {
        if let Some(ref best_deficit_actions) = self.best_deficit_actions {
            // If we have best_deficit_actions but not for this specific year, return empty vec instead of None
            return best_deficit_actions.get(&year).or_else(|| {
                println!("⚠️ WARNING: Best deficit actions exist but none for year {}", year);
                None
            });
        }
        None
    }

    pub fn get_current_deficit_actions_for_year(&self, year: u32) -> Option<&Vec<GridAction>> {
        self.current_deficit_actions.get(&year)
    }

    pub fn record_deficit_action(&mut self, year: u32, action: GridAction) {
        self.current_deficit_actions.entry(year)
            .or_insert_with(Vec::new)
            .push(action);
    }

    pub fn update_deficit_weights(&mut self, action: &GridAction, year: u32, improvement: f64) {
        // Ensure we have weights for this year
        if !self.deficit_weights.contains_key(&year) {
            // Initialize with defaults biased toward fast-responding generators
            let mut deficit_year_weights = HashMap::new();
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::GasPeaker, DEFAULT_COST_MULTIPLIER), DEFICIT_GAS_PEAKER_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::GasCombinedCycle, DEFAULT_COST_MULTIPLIER), DEFICIT_GAS_COMBINED_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::BatteryStorage, DEFAULT_COST_MULTIPLIER), DEFICIT_BATTERY_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::PumpedStorage, DEFAULT_COST_MULTIPLIER), DEFICIT_PUMPED_STORAGE_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::Biomass, DEFAULT_COST_MULTIPLIER), DEFICIT_BIOMASS_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::OnshoreWind, DEFAULT_COST_MULTIPLIER), DEFICIT_ONSHORE_WIND_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::OffshoreWind, DEFAULT_COST_MULTIPLIER), DEFICIT_OFFSHORE_WIND_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::UtilitySolar, DEFAULT_COST_MULTIPLIER), DEFICIT_UTILITY_SOLAR_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::HydroDam, DEFAULT_COST_MULTIPLIER), DEFICIT_HYDRO_DAM_WEIGHT);
            deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::Nuclear, DEFAULT_COST_MULTIPLIER), DEFICIT_NUCLEAR_WEIGHT);
            deficit_year_weights.insert(GridAction::DoNothing, DEFICIT_DO_NOTHING_WEIGHT);
            self.deficit_weights.insert(year, deficit_year_weights);
        }
        
        let year_weights = self.deficit_weights.get_mut(&year).expect("Year weights not found");
        
        // If the action doesn't exist in weights, initialize it
        if !year_weights.contains_key(action) {
            year_weights.insert(action.clone(), DEFAULT_WEIGHT);
        }
        
        let current_weight = year_weights.get(action).expect("Weight should exist");
        
        // Calculate adjustment factor similar to normal action weights
        let adjustment_factor = if improvement > ZERO_F64 {
            // For improvements, increase weight proportionally to the improvement
            ONE_F64 + (self.learning_rate * improvement * DEFICIT_REINFORCEMENT_MULTIPLIER)
        } else {
            // For deteriorations, decrease weight proportionally to how bad it was
            ONE_F64 / (ONE_F64 + (self.learning_rate * improvement.abs() * DEFICIT_REINFORCEMENT_MULTIPLIER))
        };
        
        // Apply the adjustment with bounds
        let new_weight = (current_weight * adjustment_factor)
            .max(MIN_WEIGHT)
            .min(MAX_WEIGHT);
        
        year_weights.insert(action.clone(), new_weight);
        
        // If this was a bad outcome, slightly increase weights of other generator types
        if improvement < ZERO_F64 {
            let boost_factor = ONE_F64 + (self.learning_rate * SMALL_BOOST_FACTOR); // Small boost to alternatives
            for (other_action, weight) in year_weights.iter_mut() {
                if other_action != action && matches!(other_action, GridAction::AddGenerator(_, _)) {
                    *weight = (*weight * boost_factor).min(MAX_WEIGHT);
                }
            }
        }
    }

}
