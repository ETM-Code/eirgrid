//! Learning module for ActionWeights
//!
//! This module contains learning-related functionality for the ActionWeights struct.

use rand::Rng;
use crate::ai::actions::grid_action::GridAction;
use crate::ai::metrics::simulation_metrics::SimulationMetrics;
use crate::ai::learning::constants::*;
use crate::ai::score_metrics;
use super::ActionWeights;

// Add a dummy public item to ensure this file is recognized by rust-analyzer
#[allow(dead_code)]
pub const MODULE_MARKER: &str = "learning_module";

impl ActionWeights {

// This file contains extracted code from the original weights.rs file
// Appropriate imports will need to be added based on the specific requirements

    pub fn update_weights(&mut self, action: &GridAction, year: u32, improvement: f64) {
        // Ensure we have weights for this year
        if !self.weights.contains_key(&year) {
            self.weights.insert(year, self.initialize_weights());
        }
        
        let year_weights = self.weights.get_mut(&year).expect("Year weights not found");
        
        // If the action doesn't exist in weights, initialize it
        if !year_weights.contains_key(action) {
            year_weights.insert(action.clone(), DEFAULT_WEIGHT);
        }
        
        let current_weight = year_weights.get(action).expect("Weight should exist");
        
        // Get the final 2050 impact score from best metrics if available
        let final_impact_score = self.best_metrics.as_ref().map_or(ZERO_F64, |metrics| score_metrics(metrics, self.optimization_mode.as_deref()));
        
        // Calculate the relative improvement compared to the best score
        let relative_improvement = if let Some(best) = &self.best_metrics {
            let best_score = score_metrics(best, self.optimization_mode.as_deref());
            if best_score > ZERO_F64 {
                (final_impact_score - best_score) / best_score
            } else {
                final_impact_score
            }
        } else {
            final_impact_score
        };

        // Combine immediate and final impacts with adaptive weighting
        // If we're doing better than our best, weight immediate impact more
        // If we're doing worse, weight final impact more to encourage exploration
        let immediate_weight = if relative_improvement > ZERO_F64 { IMMEDIATE_WEIGHT_FACTOR_POSITIVE } else { IMMEDIATE_WEIGHT_FACTOR_NEGATIVE };
        let combined_improvement = immediate_weight * improvement + (ONE_F64 - immediate_weight) * relative_improvement;
        
        // Calculate weight adjustment
        let adjustment_factor = if combined_improvement > ZERO_F64 {
            // For improvements, increase weight proportionally to the improvement
            ONE_F64 + (self.learning_rate * combined_improvement)
        } else {
            // For deteriorations, decrease weight proportionally to how bad it was
            ONE_F64 / (ONE_F64 + (self.learning_rate * combined_improvement.abs()))
        };
        
        // Apply the adjustment with bounds
        let new_weight = (current_weight * adjustment_factor)
            .max(MIN_WEIGHT)
            .min(MAX_WEIGHT);
        
        year_weights.insert(action.clone(), new_weight);
        
        // If this was a bad outcome, slightly increase weights of other actions.
        if combined_improvement < ZERO_F64 {
            let boost_factor = ONE_F64 + (self.learning_rate * SMALL_BOOST_FACTOR); // Small boost to alternatives
            for (other_action, weight) in year_weights.iter_mut() {
                if other_action != action && matches!(other_action, GridAction::AddGenerator(_)) {
                    *weight = (*weight * boost_factor).min(MAX_WEIGHT);
                }
            }
            // If we've achieved net zero but are suffering from high costs, further boost DoNothing.
            if self.best_metrics.as_ref().map(|m| m.final_net_emissions <= ZERO_F64 && m.total_cost > MAX_ACCEPTABLE_COST * HIGH_COST_THRESHOLD_MULTIPLIER).unwrap_or(false) {
                if let Some(noop_weight) = year_weights.get_mut(&GridAction::DoNothing) {
                    *noop_weight = (*noop_weight * (ONE_F64 + self.learning_rate * NOOP_BOOST_FACTOR)).min(MAX_WEIGHT);
                }
            }
        }
    }

    pub fn update_action_count_weights(&mut self, year: u32, action_count: u32, improvement: f64) {
        if let Some(year_counts) = self.action_count_weights.get_mut(&year) {
            if let Some(weight) = year_counts.get_mut(&action_count) {
                // Amplify the improvement based on how low the action count is
                // Lower action counts get more positive reinforcement for success
                let action_count_bonus = if improvement > 0.0 {
                    // Apply additional bonus for lower action counts when successful
                    // This gives stronger positive reinforcement for strategies with fewer actions
                    1.0 + (MAX_ACTION_COUNT as f64 - action_count as f64) / MAX_ACTION_COUNT as f64
                } else {
                    1.0 // No bonus for negative improvements
                };
                
                // Apply the action count bonus to the improvement
                let adjusted_improvement = improvement * action_count_bonus;
                
                // Similar to action weight updates, but with the adjusted improvement
                let adjustment_factor = if adjusted_improvement > 0.0 {
                    1.0 + (self.learning_rate * adjusted_improvement)
                } else {
                    1.0 / (1.0 + (self.learning_rate * adjusted_improvement.abs()))
                };
                
                // Apply the adjustment
                *weight = (*weight * adjustment_factor).max(MIN_ACTION_WEIGHT).min(ONE_F64);
                
                // Print information about significant weight updates only if debug weights is enabled
                if improvement.abs() > 0.05 && crate::ai::learning::constants::is_debug_weights_enabled() {
                    println!("Updated action count weight for {} actions in year {}: {:.4} (improvement: {:.4}, adjusted: {:.4})",
                             action_count, year, *weight, improvement, adjusted_improvement);
                }
                
                // Normalize weights
                let total: f64 = year_counts.values().sum();
                for w in year_counts.values_mut() {
                    *w /= total;
                }
            }
        }
    }

    pub fn apply_contrast_learning(&mut self, current_metrics: &SimulationMetrics) {
        // Only apply contrast learning if we have a best run to compare against
        if let (Some(best_metrics), Some(best_actions)) = (&self.best_metrics, &self.best_actions) {
            let best_score = score_metrics(best_metrics, self.optimization_mode.as_deref());
            let current_score = score_metrics(current_metrics, self.optimization_mode.as_deref());
            
            // Calculate how much worse the current run is compared to the best
            let deterioration = if best_score > ZERO_F64 {
                (best_score - current_score) / best_score
            } else {
                ZERO_F64
            };
            
            // Only apply contrast learning if the current run is significantly worse (>3%)
            if deterioration > DIVERGENCE_FOR_NEGATIVE_WEIGHT {
                // Calculate stagnation penalty with exponential scaling
                // For stagnation, we want more iterations to have a stronger effect, so we use a power > 1
                let stagnation_iterations = self.iterations_without_improvement as f64 / STAGNATION_ITERATIONS_DIVISOR;
                let stagnation_factor = ONE_F64 + (STAGNATION_PENALTY_FACTOR * stagnation_iterations.powf(STAGNATION_EXPONENT));
                
                // Fix the divergence scaling - for values between 0 and 1, using a power < 1 makes them larger
                // This ensures that worse divergence (higher values) results in stronger penalties
                let scaled_deterioration = deterioration.powf(DIVERGENCE_EXPONENT);
                
                // Calculate the combined penalty multiplier
                let combined_penalty = scaled_deterioration * stagnation_factor;
                
                // Enhanced adaptive learning rate based on stagnation and performance degradation
                let adaptive_learning_rate = self.learning_rate * (ONE_F64 + ADAPTIVE_LEARNING_RATE_FACTOR * self.iterations_without_improvement as f64);
                
                // Log the contrast learning application with more detailed information
                println!("\n🔄 Applying enhanced contrast learning:");
                println!("   - Current run is {:.1}% worse than best", deterioration * PERCENT_CONVERSION);
                println!("   - Iterations without improvement: {}", self.iterations_without_improvement);
                println!("   - Raw deterioration: {:.4}, Scaled: {:.4}", deterioration, scaled_deterioration);
                println!("   - Stagnation factor: {:.2}x", stagnation_factor);
                println!("   - Combined penalty multiplier: {:.4}", combined_penalty);
                println!("   - Adaptive learning rate: {:.4} (base: {:.4})", adaptive_learning_rate, self.learning_rate);
                
                // Calculate the penalty factor - more severe for worse runs and after more stagnation
                let penalty_factor = ONE_F64 / (ONE_F64 + adaptive_learning_rate * PENALTY_MULTIPLIER * combined_penalty);
                
                // Calculate the boost factor for best actions - increases with stagnation
                let best_boost_factor = ONE_F64 + (adaptive_learning_rate * BOOST_MULTIPLIER * stagnation_factor);
                
                println!("   - Penalty factor: {:.8}", penalty_factor);
                println!("   - Best action boost factor: {:.8}", best_boost_factor);
                
                // Debug - show an example of penalty effect on a typical weight
                let example_weight = 0.1;
                let penalized_weight = (example_weight * penalty_factor).max(MIN_WEIGHT);
                println!("   - Example: Weight of 0.1 becomes {:.8} after penalty", penalized_weight);
                
                // Track how many weights are at minimum value
                let mut min_weight_count = 0;
                let mut total_weights = 0;
                
                // For each year, compare actions in the current run with the best run
                for (year, best_year_actions) in best_actions {
                    // Get regular and deficit actions for the current run
                    let current_regular_actions = self.current_run_actions.get(year).cloned().unwrap_or_default();
                    let current_deficit_actions = self.current_deficit_actions.get(year).cloned().unwrap_or_default();
                    
                    // Combine current regular and deficit actions for comparison
                    let mut current_year_actions = current_regular_actions.clone();
                    current_year_actions.extend(current_deficit_actions.clone());
                    
                    // Get best deficit actions for this year if they exist
                    let best_deficit_actions = self.best_deficit_actions.as_ref()
                        .and_then(|bda| bda.get(year))
                        .cloned()
                        .unwrap_or_default();
                    
                    // Combine best regular and deficit actions
                    let mut complete_best_actions = best_year_actions.clone();
                    complete_best_actions.extend(best_deficit_actions);
                    
                    // Identify actions in the current run that differ from the best run
                    if let Some(year_weights) = self.weights.get_mut(year) {
                        // Store which actions got boosted vs penalized for normalization
                        let mut penalized_actions = Vec::new();
                        let mut boosted_actions = Vec::new();
                        let reward_actions: Vec<GridAction> = Vec::new();
                        
                        // First, STRONGLY boost all best actions, regardless of whether they appeared in current run
                        for best_action in &complete_best_actions {
                            if let Some(weight) = year_weights.get_mut(best_action) {
                                // Very strong boost to best actions, especially with high stagnation
                                *weight = (*weight * best_boost_factor).min(MAX_WEIGHT);
                                boosted_actions.push(best_action.clone());
                            }
                        }
                        
                        // Now process all current actions to possibly penalize
                        for (i, current_action) in current_year_actions.iter().enumerate() {
                            // Only penalize if this action doesn't appear in the best strategy at all
                            if !complete_best_actions.contains(current_action) {
                                if let Some(weight) = year_weights.get_mut(current_action) {
                                    *weight = (*weight * penalty_factor).max(MIN_WEIGHT);
                                    if *weight <= MIN_WEIGHT + WEIGHT_PRECISION_THRESHOLD {
                                        min_weight_count += 1;
                                    }
                                    total_weights += 1;
                                    penalized_actions.push(current_action.clone());
                                }
                            } else {
                                // This action appears in best strategy but might be at wrong time
                                // Check if it's at the same position
                                if i < complete_best_actions.len() && current_action != &complete_best_actions[i] {
                                    // It's in the best strategy but in the wrong order - mild penalty
                                    if let Some(weight) = year_weights.get_mut(current_action) {
                                        let mild_penalty = ONE_F64 / (ONE_F64 + adaptive_learning_rate * combined_penalty * MILD_PENALTY_FACTOR);
                                        *weight = (*weight * mild_penalty).max(MIN_WEIGHT);
                                        total_weights += 1;
                                    }
                                }
                            }
                        }
                        
                        // Log a summary of changes for this year
                        if !penalized_actions.is_empty() || !boosted_actions.is_empty() {
                            println!("   Year {}: Penalized {} actions, boosted {} actions, rewarded {} actions", 
                                    year, penalized_actions.len(), boosted_actions.len(), reward_actions.len());
                        }
                    }
                }
                
                // Log summary information about the weight changes
                println!("   - Applied enhanced contrast learning to deficit handling actions");

                // Show stats on how many weights were affected
                if total_weights > 0 {
                    println!("   - {}/{} weights ({:.1}%) reduced to minimum value", 
                            min_weight_count, total_weights, (min_weight_count as f64 / total_weights as f64) * 100.0);
                }

                // If we've been stagnating for a very long time, also apply some randomization
                // to break out of local optima
                if self.iterations_without_improvement > ITERATIONS_FOR_RANDOMIZATION {
                    println!("   - Applying weight randomization to break stagnation after {} iterations", 
                            self.iterations_without_improvement);
                    
                    let randomization_factor = RANDOMIZATION_FACTOR; // 10% random variation
                    let mut rng = rand::thread_rng();
                    
                    for year_weights in self.weights.values_mut() {
                        for weight in year_weights.values_mut() {
                            let random_factor = ONE_F64 + randomization_factor * (rng.gen::<f64>() * RANDOM_RANGE_MULTIPLIER - ONE_F64);
                            *weight = (*weight * random_factor).clamp(MIN_WEIGHT, MAX_WEIGHT);
                        }
                    }
                }
            }
        }
    }

    pub fn apply_deficit_contrast_learning(&mut self) {
        // Only apply contrast learning if we have a best run to compare against
        if let (Some(best_metrics), Some(best_deficit_actions)) = (&self.best_metrics, &self.best_deficit_actions) {
            let __best_score = score_metrics(best_metrics, self.optimization_mode.as_deref());
            // We don't have a current metrics specific to deficit actions, but we can use the deterioration
            // from the regular contrast learning as an approximation
            let deterioration = self.iterations_without_improvement as f64 / STAGNATION_ITERATIONS_DIVISOR; // Use iterations as a proxy for deterioration
            
            // Only apply contrast learning if there's some deterioration
            if deterioration > ZERO_F64 {
                // Calculate stagnation penalty with exponential scaling
                let stagnation_iterations = self.iterations_without_improvement as f64 / STAGNATION_ITERATIONS_DIVISOR;
                let stagnation_factor = ONE_F64 + (STAGNATION_PENALTY_FACTOR * stagnation_iterations.powf(STAGNATION_EXPONENT));
                
                // Scale the deterioration like in regular contrast learning
                let scaled_deterioration = deterioration.powf(DIVERGENCE_EXPONENT);
                
                // Calculate the combined penalty multiplier
                let combined_penalty = scaled_deterioration * stagnation_factor;
                
                // Calculate adaptive learning rate as in regular contrast learning
                let adaptive_learning_rate = self.learning_rate * (ONE_F64 + ADAPTIVE_LEARNING_RATE_FACTOR * self.iterations_without_improvement as f64);
                
                // Log the contrast learning application with more detailed information
                println!("\n🔄 Applying enhanced contrast learning to deficit handling actions:");
                println!("   - Iterations without improvement: {}", self.iterations_without_improvement);
                println!("   - Proxy deterioration: {:.4}, Scaled: {:.4}", deterioration, scaled_deterioration);
                println!("   - Stagnation factor: {:.2}x", stagnation_factor);
                println!("   - Combined penalty multiplier: {:.4}", combined_penalty);
                
                // Calculate the penalty factor - more severe for worse runs and after more stagnation
                let penalty_factor = ONE_F64 / (ONE_F64 + adaptive_learning_rate * PENALTY_MULTIPLIER * combined_penalty);
                let best_boost_factor = ONE_F64 + (adaptive_learning_rate * BOOST_MULTIPLIER * stagnation_factor);
                
                println!("   - Penalty factor: {:.8}", penalty_factor);
                println!("   - Best boost factor: {:.8}", best_boost_factor);
                
                // For each year, compare deficit actions with the best deficit actions
                for (year, best_year_actions) in best_deficit_actions {
                    let current_year_actions = self.current_deficit_actions.get(year).cloned().unwrap_or_default();
                    
                    if let Some(year_weights) = self.deficit_weights.get_mut(year) {
                        // First, boost all best deficit actions
                        for best_action in best_year_actions {
                            if let Some(weight) = year_weights.get_mut(best_action) {
                                *weight = (*weight * best_boost_factor).min(MAX_WEIGHT);
                            }
                        }
                        
                        // Then penalize current actions that don't appear in best actions
                        for current_action in &current_year_actions {
                            if !best_year_actions.contains(current_action) {
                                if let Some(weight) = year_weights.get_mut(current_action) {
                                    *weight = (*weight * penalty_factor).max(MIN_WEIGHT);
                                }
                            }
                        }
                    }
                }
                
                // Log summary information about the weight changes
                println!("   - Applied enhanced contrast learning to deficit handling actions");

                // If we've been stagnating for a very long time, also apply some randomization
                // to break out of local optima
                if self.iterations_without_improvement > ITERATIONS_FOR_RANDOMIZATION {
                    println!("   - Applying weight randomization to deficit weights after {} iterations", 
                            self.iterations_without_improvement);
                    
                    let mut rng = rand::thread_rng();
                    
                    for year_weights in self.deficit_weights.values_mut() {
                        for weight in year_weights.values_mut() {
                            let random_factor = ONE_F64 + RANDOMIZATION_FACTOR * (rng.gen::<f64>() * 2.0 - 1.0);
                            *weight = (*weight * random_factor).clamp(MIN_WEIGHT, MAX_WEIGHT);
                        }
                    }
                }
            }
        }
    }

    pub fn restore_best_weights(&mut self, best_weight_factor: f64) {
        if let Some(best_weights) = &self.best_weights {
            // Mix best weights with current weights using the specified factor
            for (year, best_year_weights) in best_weights {
                if let Some(current_year_weights) = self.weights.get_mut(year) {
                    for (action, &best_weight) in best_year_weights {
                        if let Some(current_weight) = current_year_weights.get_mut(action) {
                            // Mix weights
                            *current_weight = best_weight * best_weight_factor + 
                                            *current_weight * (ONE_F64 - best_weight_factor);
                        } else {
                            // Action exists in best but not in current, add it
                            current_year_weights.insert(action.clone(), best_weight);
                        }
                    }
                }
            }
            
            println!("   - Restored weights with {:.0}% best weights / {:.0}% current weights", 
                    best_weight_factor * PERCENT_CONVERSION, (ONE_F64 - best_weight_factor) * PERCENT_CONVERSION);
        }
    }

}
