// Action sampling for ActionWeights

use std::collections::HashMap;
use rand::Rng;
use crate::models::generator::GeneratorType;
use crate::ai::actions::grid_action::GridAction;
use crate::ai::learning::constants::*;
use super::ActionWeights;

// Add a dummy public item to ensure this file is recognized by rust-analyzer
#[allow(dead_code)]
pub const MODULE_MARKER: &str = "sampling_module";

impl ActionWeights {

// This file contains extracted code from the original weights.rs file
// Appropriate imports will need to be added based on the specific requirements

    // Initialize weights for a single year
    pub fn initialize_weights(&self) -> HashMap<GridAction, f64> {
        let mut year_weights = HashMap::new();
        year_weights.insert(GridAction::AddGenerator(GeneratorType::OnshoreWind), ONSHORE_WIND_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::OffshoreWind), OFFSHORE_WIND_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::DomesticSolar), DOMESTIC_SOLAR_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::CommercialSolar), COMMERCIAL_SOLAR_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::UtilitySolar), UTILITY_SOLAR_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::Nuclear), NUCLEAR_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::CoalPlant), COAL_PLANT_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::GasCombinedCycle), GAS_COMBINED_CYCLE_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::GasPeaker), GAS_PEAKER_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::Biomass), BIOMASS_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::HydroDam), HYDRO_DAM_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::PumpedStorage), PUMPED_STORAGE_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::BatteryStorage), BATTERY_STORAGE_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::TidalGenerator), TIDAL_GENERATOR_WEIGHT);
        year_weights.insert(GridAction::AddGenerator(GeneratorType::WaveEnergy), WAVE_ENERGY_WEIGHT);
        year_weights.insert(GridAction::UpgradeEfficiency(String::new()), UPGRADE_EFFICIENCY_WEIGHT);
        year_weights.insert(GridAction::AdjustOperation(String::new(), OPERATION_PERCENTAGE_MIN), ADJUST_OPERATION_WEIGHT);
        year_weights.insert(GridAction::AddCarbonOffset("Forest".to_string()), CARBON_OFFSET_WEIGHT);
        year_weights.insert(GridAction::AddCarbonOffset("Wetland".to_string()), CARBON_OFFSET_WEIGHT);
        year_weights.insert(GridAction::AddCarbonOffset("ActiveCapture".to_string()), CARBON_OFFSET_WEIGHT);
        year_weights.insert(GridAction::AddCarbonOffset("CarbonCredit".to_string()), CARBON_OFFSET_WEIGHT);
        year_weights.insert(GridAction::CloseGenerator(String::new()), CLOSE_GENERATOR_WEIGHT);
        // Initialize DoNothing with a base weight
        year_weights.insert(GridAction::DoNothing, DO_NOTHING_WEIGHT);
        year_weights
    }

    pub fn sample_action(&mut self, year: u32) -> GridAction {
        // If we're forcing replay of best actions and we have them, use those
        if self.force_best_actions {
            if let Some(best_actions) = &self.best_actions {
                if let Some(year_actions) = best_actions.get(&year) {
                    // Get the current replay index for this year, or initialize it to 0
                    let current_index = *self.replay_index.entry(year).or_insert(0);
                    
                    if current_index < year_actions.len() {
                        let action = year_actions[current_index].clone();
                        
                        // Only print debug info if debug weights is enabled
                        if crate::ai::learning::constants::is_debug_weights_enabled() {
                            println!("🔄 REPLAY: Using best action #{} for year {}: {:?}", 
                                    current_index + ONE_USIZE, year, action);
                        }
                        
                        // Increment the replay index for this year
                        self.replay_index.insert(year, current_index + 1);
                        
                        // Make sure to record the replayed action in the current run
                        self.current_run_actions.entry(year)
                            .or_insert_with(Vec::new)
                            .push(action.clone());
                        
                        return action;
                    } else {
                        // Only print debug info if debug weights is enabled
                        if crate::ai::learning::constants::is_debug_weights_enabled() {
                            println!("⚠️ REPLAY FALLBACK: Ran out of best actions for year {} (needed action #{}, have {})", 
                                    year, current_index + ONE_USIZE, year_actions.len());
                        }
                        
                        // Add smart fallback for when we run out of actions
                        let fallback_action = self.generate_smart_fallback_action(year, "ran out of best actions");
                        
                        // Also record this fallback action in the current run
                        self.current_run_actions.entry(year)
                            .or_insert_with(Vec::new)
                            .push(fallback_action.clone());
                        
                        return fallback_action;
                    }
                } else {
                    println!("⚠️ REPLAY FALLBACK: No best actions recorded for year {}", year);
                    
                    // Add smart fallback for when no actions exist for this year
                    let fallback_action = self.generate_smart_fallback_action(year, "no best actions for year");
                    
                    // Also record this fallback action in the current run
                    self.current_run_actions.entry(year)
                        .or_insert_with(Vec::new)
                        .push(fallback_action.clone());
                    
                    return fallback_action;
                }
            } else {
                println!("⚠️ REPLAY FALLBACK: No best actions recorded for any year");
                
                // Add smart fallback for when no best actions exist at all
                let fallback_action = self.generate_smart_fallback_action(year, "no best actions at all");
                
                // Also record this fallback action in the current run
                self.current_run_actions.entry(year)
                    .or_insert_with(Vec::new)
                    .push(fallback_action.clone());
                
                return fallback_action;
            }
        }
        
        // Normal action selection logic
        let year_weights = self.weights.get(&year).expect("Year weights not found");

        // Calculate a dynamic exploration rate that decreases when we're stuck
        let current_exploration = if self.iterations_without_improvement > LOW_ITERATION_THRESHOLD {
            // Reduce exploration drastically after being stuck for a while to focus on best known actions
            self.exploration_rate * (ONE_F64 / (ONE_F64 + EXPLORATION_DECAY_FACTOR * self.iterations_without_improvement as f64))
        } else {
            self.exploration_rate
        };

        // Determine if we should explore based on the deterministic RNG or thread_rng
        let should_explore = match &mut self.deterministic_rng {
            Some(rng) => rng.gen::<f64>() < current_exploration,
            None => rand::thread_rng().gen::<f64>() < current_exploration,
        };

        // Epsilon-greedy exploration with dynamic rate
        if should_explore {
            // Random exploration
            let actions: Vec<_> = year_weights.keys().collect();
            if actions.is_empty() {
                // Fallback to a safe default action if no actions are available
                return GridAction::AddGenerator(GeneratorType::GasPeaker);
            }
            
            let random_idx = match &mut self.deterministic_rng {
                Some(rng) => rng.gen_range(ZERO_USIZE..actions.len()),
                None => rand::thread_rng().gen_range(ZERO_USIZE..actions.len()),
            };
            
            return actions[random_idx].clone();
        }

        // Exploitation - weighted selection
        let total_weight: f64 = year_weights.values().sum();
        if total_weight <= ZERO_F64 {
            // If all weights are zero or negative, fall back to a safe default
            return GridAction::AddGenerator(GeneratorType::GasPeaker);
        }

        // When stuck for many iterations, use a more aggressive selection strategy
        // by applying a power scaling to the weights, making higher weights even more likely
        if self.iterations_without_improvement > MID_ITERATION_THRESHOLD {
            // Extract actions and weights
            let mut actions_with_weights: Vec<_> = year_weights.iter().collect();
            // Sort by weight in descending order
            actions_with_weights.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
            
            // Calculate how aggressive the selection should be based on stagnation
            let stagnation_factor = (self.iterations_without_improvement as f64 / STAGNATION_DIVISOR).min(STAGNATION_SCALE_MAX);
            let power_scaling = STAGNATION_SCALE_MIN + (STAGNATION_SCALE_FACTOR * stagnation_factor); // Ranges from 1.0 to 3.0
            
            // Use exponentiated weights for selection
            let total_scaled_weight: f64 = actions_with_weights.iter()
                .map(|(_, &w)| w.powf(power_scaling))
                .sum();
            
            let mut random_val = match &mut self.deterministic_rng {
                Some(rng) => rng.gen::<f64>() * total_scaled_weight,
                None => rand::thread_rng().gen::<f64>() * total_scaled_weight,
            };
            
            for (action, &weight) in &actions_with_weights {
                let scaled_weight = weight.powf(power_scaling);
                random_val -= scaled_weight;
                if random_val <= ZERO_F64 {
                    return (*action).clone();
                }
            }
            
            // Fallback to the highest weight action
            return actions_with_weights.first().map(|(a, _)| (*a).clone())
                .unwrap_or(GridAction::AddGenerator(GeneratorType::GasPeaker));
        } else {
            // Standard weighted selection for normal operation
            let mut random_val = match &mut self.deterministic_rng {
                Some(rng) => rng.gen::<f64>() * total_weight,
                None => rand::thread_rng().gen::<f64>() * total_weight,
            };
            
            for (action, weight) in year_weights {
                random_val -= weight;
                if random_val <= ZERO_F64 {
                    return action.clone();
                }
            }
        }
        
        // Fallback to a safe default if no action was selected
        GridAction::AddGenerator(GeneratorType::GasPeaker)
    }

    pub fn sample_deficit_action(&mut self, year: u32) -> GridAction {
        // If we're forcing replay of best actions and we have best deficit actions, use those
        if self.force_best_actions {
            if let Some(best_deficit_actions) = &self.best_deficit_actions {
                if let Some(year_deficit_actions) = best_deficit_actions.get(&year) {
                    // Use a separate key format for deficit replay index to avoid conflicts with regular actions
                    let deficit_year_key = year + 10000; // Add 10000 to distinguish from regular action years
                    let current_index = *self.replay_index.entry(deficit_year_key).or_insert(0);
                    
                    if current_index < year_deficit_actions.len() {
                        let action = year_deficit_actions[current_index].clone();
                        
                        // Only print debug info if debug weights is enabled
                        if crate::ai::learning::constants::is_debug_weights_enabled() {
                            println!("🔄 DEFICIT REPLAY: Using best deficit action #{} for year {}: {:?}", 
                                    current_index + ONE_USIZE, year, action);
                        }
                        
                        // Increment the deficit replay index for this year
                        self.replay_index.insert(deficit_year_key, current_index + 1);
                        
                        // Make sure to record this replayed deficit action
                        self.current_deficit_actions.entry(year)
                            .or_insert_with(Vec::new)
                            .push(action.clone());
                        
                        return action;
                    } else {
                        // Only print debug info if debug weights is enabled
                        if crate::ai::learning::constants::is_debug_weights_enabled() {
                            println!("⚠️ DEFICIT REPLAY FALLBACK: Ran out of best deficit actions for year {} (needed action #{}, have {})",
                                    year, current_index + ONE_USIZE, year_deficit_actions.len());
                        }
                        
                        // Smart fallback for deficit
                        let fallback_action = self.generate_smart_deficit_fallback_action(year);
                        
                        // Record this fallback action
                        self.current_deficit_actions.entry(year)
                            .or_insert_with(Vec::new)
                            .push(fallback_action.clone());
                        
                        return fallback_action;
                    }
                } else {
                    // Only print debug info if debug weights is enabled
                    if crate::ai::learning::constants::is_debug_weights_enabled() {
                        println!("⚠️ DEFICIT REPLAY FALLBACK: No best deficit actions recorded for year {}", year);
                    }
                    
                    // Smart fallback for deficit
                    let fallback_action = self.generate_smart_deficit_fallback_action(year);
                    
                    // Record this fallback action
                    self.current_deficit_actions.entry(year)
                        .or_insert_with(Vec::new)
                        .push(fallback_action.clone());
                    
                    return fallback_action;
                }
            } else {
                println!("⚠️ DEFICIT REPLAY FALLBACK: No best deficit actions recorded for any year");
                
                // Smart fallback for deficit
                let fallback_action = self.generate_smart_deficit_fallback_action(year);
                
                // Record this fallback action
                self.current_deficit_actions.entry(year)
                    .or_insert_with(Vec::new)
                    .push(fallback_action.clone());
                    
                return fallback_action;
            }
        }
        
        // Continue with normal deficit action selection
        // Default to normal deficit weights
        let year_weights = match self.deficit_weights.get(&year) {
            Some(weights) => weights,
            None => {
                // Fallback to initialize weights for this year if missing
                return GridAction::AddGenerator(GeneratorType::GasPeaker);
            }
        };
        
        // Determine if we should explore based on the deterministic RNG or thread_rng
        let should_explore = match &mut self.deterministic_rng {
            Some(rng) => rng.gen::<f64>() < self.exploration_rate,
            None => rand::thread_rng().gen::<f64>() < self.exploration_rate,
        };
        
        // Apply epsilon-greedy strategy similar to main action sampling
        if should_explore {
            // Random exploration
            let actions: Vec<_> = year_weights.keys()
                .filter(|action| matches!(action, GridAction::AddGenerator(_)))
                .collect();
            
            if actions.is_empty() {
                // Fallback to a reliable generator if no AddGenerator actions
                return GridAction::AddGenerator(GeneratorType::GasPeaker);
            }
            
            let random_idx = match &mut self.deterministic_rng {
                Some(rng) => rng.gen_range(ZERO_USIZE..actions.len()),
                None => rand::thread_rng().gen_range(ZERO_USIZE..actions.len()),
            };
            
            return actions[random_idx].clone();
        }
        
        // Exploitation - weighted selection of generator actions
        let total_weight: f64 = year_weights.iter()
            .filter(|(action, _)| matches!(action, GridAction::AddGenerator(_)))
            .map(|(_, &weight)| weight)
            .sum();
        
        if total_weight <= ZERO_F64 {
            // If all weights are zero or negative, fall back to a reliable generator
            return GridAction::AddGenerator(GeneratorType::GasPeaker);
        }
        
        let mut random_val = match &mut self.deterministic_rng {
            Some(rng) => rng.gen::<f64>() * total_weight,
            None => rand::thread_rng().gen::<f64>() * total_weight,
        };
        
        for (action, weight) in year_weights {
            if matches!(action, GridAction::AddGenerator(_)) {
                random_val -= weight;
                if random_val <= ZERO_F64 {
                    return action.clone();
                }
            }
        }
        
        // Fallback to a reliable generator if selection fails
        GridAction::AddGenerator(GeneratorType::GasPeaker)
    }

    pub fn sample_additional_actions(&mut self, year: u32) -> u32 {
        // First, check how many deficit actions we already have for this year
        let deficit_actions_count = self.current_deficit_actions.get(&year)
            .map_or(0, |actions| actions.len() as u32);
            
        // Calculate how many regular actions we can have, ensuring total is capped at MAX_ACTION_COUNT
        let max_possible_actions = if deficit_actions_count >= MAX_ACTION_COUNT {
            0 // If deficit actions alone exceed MAX_ACTION_COUNT, don't allow any more actions
        } else {
            MAX_ACTION_COUNT - deficit_actions_count
        };
        
        // If we can't add any more actions, return 0
        if max_possible_actions == 0 {
            println!("WARNING: Already have {} deficit actions for year {}, capping additional actions at 0", 
                    deficit_actions_count, year);
            return 0;
        }
        
        let random_val = match &mut self.deterministic_rng {
            Some(rng) => rng.gen::<f64>(),
            None => rand::thread_rng().gen::<f64>(),
        };
        
        if let Some(year_counts) = self.action_count_weights.get(&year) {
            // Use weighted sampling based on historical data
            let total_weight: f64 = year_counts.values().sum();
            if total_weight <= ZERO_F64 {
                return 0;
            }
            
            let mut random_choice = random_val * total_weight;
            
            // Sample from weights but ensure we don't exceed max_possible_actions
            for (count, weight) in year_counts {
                random_choice -= weight;
                if random_choice <= ZERO_F64 {
                    return (*count).min(max_possible_actions);
                }
            }
            
            // Fallback to a reasonable default if sampling fails, but still respect the cap
            return 5.min(max_possible_actions);
        } else {
            // Fallback to simple heuristic if no historical data
            let scaled_exploration = self.exploration_rate.powf(0.5); // Square root to increase base value
            let min_actions = (EXPLORATION_DIVISOR / scaled_exploration).round() as u32;
            let max_actions = (MAX_ACTIONS_MULTIPLIER / scaled_exploration).round() as u32;
            
            // Cap the maximum actions to respect our limit
            let capped_max_actions = max_actions.min(max_possible_actions);
            let capped_min_actions = min_actions.min(capped_max_actions);
            
            // If capped_min_actions equals capped_max_actions, return that value
            if capped_min_actions == capped_max_actions {
                return capped_min_actions;
            }
            
            match &mut self.deterministic_rng {
                Some(rng) => rng.gen_range(capped_min_actions..=capped_max_actions),
                None => rand::thread_rng().gen_range(capped_min_actions..=capped_max_actions),
            }
        }
    }

    pub fn generate_smart_fallback_action(&self, year: u32, fallback_reason: &str) -> GridAction {
        println!("🔧 SMART FALLBACK: Generating strategic action for year {} (reason: {})", year, fallback_reason);
        
        // The year will influence what kind of actions are taken
        // Early years: Focus on establishing renewable infrastructure
        // Middle years: Balance cost and emissions reduction
        // Late years: Focus heavily on carbon offsets and storage for net zero

        // Create weighted action pools for different years
        let mut action_pool = Vec::new();
        
        // Basic renewables always have some representation
        action_pool.push((GridAction::AddGenerator(GeneratorType::OnshoreWind), ONSHORE_WIND_FALLBACK_WEIGHT as u32));
        action_pool.push((GridAction::AddGenerator(GeneratorType::OffshoreWind), OFFSHORE_WIND_FALLBACK_WEIGHT as u32));
        action_pool.push((GridAction::AddGenerator(GeneratorType::UtilitySolar), UTILITY_SOLAR_FALLBACK_WEIGHT as u32));
        
        // Storage becomes more important in middle and late years
        let storage_weight = if year < MID_YEAR_THRESHOLD { STORAGE_WEIGHT_EARLY } else { STORAGE_WEIGHT_LATE };
        action_pool.push((GridAction::AddGenerator(GeneratorType::BatteryStorage), storage_weight as u32));
        
        // Carbon offsets become crucial in later years
        let offset_weight = if year < MID_YEAR_THRESHOLD { OFFSET_WEIGHT_EARLY } else if year < LATE_YEAR_THRESHOLD { OFFSET_WEIGHT_MID } else { OFFSET_WEIGHT_LATE };
        action_pool.push((GridAction::AddCarbonOffset("Forest".to_string()), offset_weight as u32));
        action_pool.push((GridAction::AddCarbonOffset("ActiveCapture".to_string()), offset_weight as u32));
        
        // Gas for reliable power - more important in early years, less in later
        let gas_weight = if year < MID_YEAR_THRESHOLD { GAS_WEIGHT_EARLY } else if year < LATE_YEAR_THRESHOLD { GAS_WEIGHT_MID } else { GAS_WEIGHT_LATE };
        action_pool.push((GridAction::AddGenerator(GeneratorType::GasCombinedCycle), gas_weight as u32));
        
        // Calculate total weight
        let total_weight: u32 = action_pool.iter().map(|(_, w)| w).sum();
        
        // Select an action based on weighted random choice
        let mut rng = rand::thread_rng();
        let mut choice = rng.gen_range(0..total_weight);
        
        for (action, weight) in action_pool {
            if choice < weight {
                return action;
            }
            choice -= weight;
        }
        
        // Fallback to a safe default if something went wrong
        GridAction::AddGenerator(GeneratorType::BatteryStorage)
    }

    pub fn generate_smart_deficit_fallback_action(&self, year: u32) -> GridAction {
        println!("🔧 SMART DEFICIT FALLBACK: Generating strategic deficit action for year {}", year);
        
        // For deficit handling, we need to prioritize reliable power generation
        // that can be deployed quickly and provide consistent output
        
        let mut action_pool = Vec::new();
        
        // Immediate response options get highest priority
        action_pool.push((GridAction::AddGenerator(GeneratorType::GasPeaker), DEFICIT_GAS_PEAKER_FALLBACK_WEIGHT as u32));
        action_pool.push((GridAction::AddGenerator(GeneratorType::BatteryStorage), DEFICIT_BATTERY_FALLBACK_WEIGHT as u32));
        
        // Medium-term reliable options
        action_pool.push((GridAction::AddGenerator(GeneratorType::GasCombinedCycle), DEFICIT_GAS_COMBINED_FALLBACK_WEIGHT as u32));
        
        // Renewables - lower priority for deficit but still included
        action_pool.push((GridAction::AddGenerator(GeneratorType::OnshoreWind), DEFICIT_ONSHORE_WIND_FALLBACK_WEIGHT as u32));
        action_pool.push((GridAction::AddGenerator(GeneratorType::OffshoreWind), (DEFICIT_OFFSHORE_WIND_WEIGHT * RENEWABLE_FALLBACK_WEIGHT_FACTOR) as u32));
        action_pool.push((GridAction::AddGenerator(GeneratorType::UtilitySolar), (DEFICIT_UTILITY_SOLAR_WEIGHT * RENEWABLE_FALLBACK_WEIGHT_FACTOR * PERCENT_CONVERSION) as u32));
        
        // Calculate total weight
        let total_weight: u32 = action_pool.iter().map(|(_, w)| w).sum();
        
        // Select an action based on weighted random choice
        let mut rng = rand::thread_rng();
        let mut choice = rng.gen_range(0..total_weight);
        
        for (action, weight) in action_pool {
            if choice < weight {
                return action;
            }
            choice -= weight;
        }
        
        // Fallback to a reliable default if something went wrong
        GridAction::AddGenerator(GeneratorType::BatteryStorage)
    }

}
