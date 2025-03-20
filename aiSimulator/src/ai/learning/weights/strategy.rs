//! Strategy module for ActionWeights
//!
//! This module contains strategy-related functionality for the ActionWeights struct.

use std::collections::HashMap;
use crate::ai::actions::grid_action::GridAction;
use crate::ai::metrics::simulation_metrics::SimulationMetrics;
use crate::ai::learning::constants::*;
use crate::ai::score_metrics;
use super::ActionWeights;
use crate::utils::csv_export::ImprovementRecord;
use chrono::Local;

impl ActionWeights {

// This file contains extracted code from the original weights.rs file
// Appropriate imports will need to be added based on the specific requirements

    pub fn update_best_strategy(&mut self, mut metrics: SimulationMetrics) {
        // Add diagnostic logging for power reliability
        // println!("DIAGNOSTIC: Initial power reliability: {:.4}%", metrics.power_reliability * 100.0);
        
  
        // println!("POWER RELIABILITY FIX: Using actual calculated reliability of {:.4}%", metrics.power_reliability * 100.0);
        
        let current_score = score_metrics(&metrics, self.optimization_mode.as_deref());
        
        // Debug: Print current_run_actions info with more detailed breakdown - only if debug weights is enabled
        if crate::ai::learning::constants::is_debug_weights_enabled() {
            let total_curr_actions = self.current_run_actions.values().map(|v| v.len()).sum::<usize>();
            let years_with_curr_actions = self.current_run_actions.values().filter(|v| !v.is_empty()).count();
            // println!("DEBUG: Before update - Current run has {} actions across {} years", 
            //         total_curr_actions, years_with_curr_actions);
            
            // More detailed per-year breakdown for the current run
            // println!("Current run actions per year:");
            
            // If we have empty current_run_actions but non-empty best actions, something's wrong
            // if total_curr_actions == ZERO_USIZE && self.best_actions.is_some() {
            //     println!("⚠️ WARNING: Attempting to update best strategy with 0 actions in current run!");
            //     println!("This suggests actions aren't being recorded properly during simulation");
            // }
            
            // DIAGNOSTIC: Add logging to check metrics values
            // println!("DIAGNOSTIC: Metrics values before processing:");
            // println!("  - final_net_emissions: {}", metrics.final_net_emissions);
            // println!("  - total_cost: {}", metrics.total_cost);
            // println!("  - average_public_opinion: {}", metrics.average_public_opinion);
            // println!("  - power_reliability: {:.4}%", metrics.power_reliability * 100.0);
            // println!("  - iteration_count: {}", self.iteration_count);
        }
        
        // Increment iteration count
        self.iteration_count += 1;
        
        // Print iteration count update only if debug weights is enabled
        // if crate::ai::learning::constants::is_debug_weights_enabled() {
        //     println!("DIAGNOSTIC: Incremented iteration_count to {}", self.iteration_count);
        // }
        
        let should_update = match &self.best_metrics {
            None => true,
            Some(best) => {
                let best_score = score_metrics(best, self.optimization_mode.as_deref());
                
                // DIAGNOSTIC: Add score comparison logging - only if debug weights is enabled
                if crate::ai::learning::constants::is_debug_weights_enabled() {
                    println!("DIAGNOSTIC: Score comparison - current: {}, best: {}", current_score, best_score);
                }
                
                // If new score is better than best score, update
                current_score > best_score
            }
        };

        if should_update {
            // Create an improvement record for tracking
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let improvement_record = ImprovementRecord {
                iteration: self.iteration_count,
                score: current_score,
                net_emissions: metrics.final_net_emissions,
                total_cost: metrics.total_cost,
                public_opinion: metrics.average_public_opinion,
                power_reliability: metrics.power_reliability,
                timestamp,
            };
            
            // Add to improvement history
            self.improvement_history.push(improvement_record);
        
            // Only print improvement message if we actually had a previous best
            if let Some(best) = &self.best_metrics {
                let best_score = score_metrics(best, self.optimization_mode.as_deref());
                let improvement = ((current_score - best_score) / best_score * PERCENT_CONVERSION).abs();
                
                // Create a VERY visible message with details about the improvement
                println!("\n\n");
                println!("{}", "*".repeat(DEBUG_STAR_COUNT));
                println!("{}", "=".repeat(DEBUG_EQUALS_COUNT));
                println!("🎉🎉🎉  MAJOR STRATEGY IMPROVEMENT FOUND!  🎉🎉🎉");
                println!("{}", "=".repeat(DEBUG_EQUALS_COUNT));
                println!("{}", "*".repeat(DEBUG_STAR_COUNT));
                println!("\nScore improved by {:.2}%", improvement);
                println!("Previous best score: {:.4} → New best score: {:.4}", best_score, current_score);
                println!("Found after {} iterations without improvement", self.iterations_without_improvement);
 
                // Add more detailed metrics information with better formatting
                println!("\n📊 DETAILED METRICS COMPARISON:");
                
                // Net emissions comparison with appropriate emoji
                let emissions_change = metrics.final_net_emissions - best.final_net_emissions;
                let emissions_emoji = if emissions_change <= ZERO_F64 { "✅" } else { "⚠️" };
                println!("  {} Net emissions: {:.2} → {:.2} ({:+.2})", 
                        emissions_emoji, best.final_net_emissions, metrics.final_net_emissions, emissions_change);
                
                // Net zero status comparison
                let old_net_zero = best.final_net_emissions <= ZERO_F64;
                let new_net_zero = metrics.final_net_emissions <= ZERO_F64;
                let net_zero_emoji = if new_net_zero { "✅" } else { "⚠️" };
                println!("  {} Net zero: {} → {}", 
                        net_zero_emoji, 
                        if old_net_zero { "YES" } else { "NO" }, 
                        if new_net_zero { "YES" } else { "NO" });
                
                // Total cost comparison
                let cost_change = metrics.total_cost - best.total_cost;
                let cost_emoji = if cost_change <= ZERO_F64 { "✅" } else { "⚠️" };
                println!("  {cost_emoji} Cost: €{:.2}B/year → €{:.2}B/year ({:+.2}B)",
                    best.total_cost / BILLION_DIVISOR,
                    metrics.total_cost / BILLION_DIVISOR,
                    cost_change / BILLION_DIVISOR);
                
                // Public opinion comparison
                let opinion_change = metrics.average_public_opinion - best.average_public_opinion;
                let opinion_emoji = if opinion_change >= ZERO_F64 { "✅" } else { "⚠️" };
                println!("  {opinion_emoji} Public opinion: {:.1}% → {:.1}% ({:+.1}%)",
                    best.average_public_opinion * PERCENT_CONVERSION,
                    metrics.average_public_opinion * PERCENT_CONVERSION,
                    opinion_change * PERCENT_CONVERSION);
                
                // Power reliability comparison
                let reliability_change = metrics.power_reliability - best.power_reliability;
                let reliability_emoji = if reliability_change >= ZERO_F64 { "✅" } else { "⚠️" };
                println!("  {reliability_emoji} Power reliability: {:.1}% → {:.1}% ({:+.1}%)",
                    best.worst_power_reliability * PERCENT_CONVERSION,
                    metrics.worst_power_reliability * PERCENT_CONVERSION,
                    reliability_change * PERCENT_CONVERSION);
            } else {
                // First successful strategy found - make this VERY visible too
                println!("\n\n");
                println!("{}", "*".repeat(DEBUG_STAR_COUNT));
                println!("{}", "=".repeat(DEBUG_EQUALS_COUNT));
                println!("🎉🎉🎉  FIRST SUCCESSFUL STRATEGY FOUND!  🎉🎉🎉");
                println!("{}", "=".repeat(DEBUG_EQUALS_COUNT));
                println!("{}", "*".repeat(DEBUG_STAR_COUNT));
                println!("\nInitial score: {:.4}", current_score);
                
                // Add detailed metrics for the first strategy
                println!("\n📊 INITIAL METRICS:");
                println!("  Net emissions: {:.2} tonnes", metrics.final_net_emissions);
                println!("  Total cost: €{:.2}B/year", metrics.total_cost / BILLION_DIVISOR);
                println!("  Public opinion: {:.1}%", metrics.average_public_opinion * PERCENT_CONVERSION);
                println!("  Power reliability: {:.1}%", metrics.power_reliability * PERCENT_CONVERSION);
                
                println!("{}", "=".repeat(DEBUG_EQUALS_COUNT));
                println!("{}", "*".repeat(DEBUG_STAR_COUNT));
                println!("\n");
                
                // Log information about the actions being recorded for the first time
                let total_actions = self.current_run_actions.values().map(|v| v.len()).sum::<usize>();
                let deficit_actions = self.current_deficit_actions.values().map(|v| v.len()).sum::<usize>();
                println!("Recording first strategy with {} regular actions and {} deficit actions", 
                        total_actions, deficit_actions);
            }
            
            // DIAGNOSTIC: Log before updating best metrics
            // println!("DIAGNOSTIC: Updating best_metrics from current metrics");
            
            self.best_metrics = Some(metrics);
            self.best_weights = Some(self.weights.clone());
            
            // Always update prime weights when we find an improvement
            // This ensures prime weights always hold our very best weights
            self.prime_weights = Some(self.weights.clone());
            
            // Make sure we have entries for each year even if they're empty
            let mut complete_actions = HashMap::new();
            let mut complete_deficit_actions = HashMap::new();
            
            // Initialize empty action lists for all years
            for year in START_YEAR..=END_YEAR {
                complete_actions.insert(year, Vec::new());
                complete_deficit_actions.insert(year, Vec::new());
            }
            
            // Then copy over any actions we actually have
            for (year, actions) in &self.current_run_actions {
                if !actions.is_empty() {
                    // println!("DEBUG: Copying {} actions for year {} to best_actions", actions.len(), year);
                    complete_actions.insert(*year, actions.clone());
                }
            }
            
            for (year, actions) in &self.current_deficit_actions {
                if !actions.is_empty() {
                    // println!("DEBUG: Copying {} deficit actions for year {} to best_deficit_actions", actions.len(), year);
                    complete_deficit_actions.insert(*year, actions.clone());
                }
            }
            
            // Debug: Check if we're actually capturing any actions
            let total_complete_actions = complete_actions.values().map(|v| v.len()).sum::<usize>();
            let years_with_complete_actions = complete_actions.values().filter(|v| !v.is_empty()).count();
            // println!("DEBUG: Created best_actions map with {} actions across {} years", 
            //          total_complete_actions, years_with_complete_actions);
            
            // More detailed per-year breakdown for complete_actions
            // println!("Complete actions per year (to be stored as best):");
            // for year in START_YEAR..=END_YEAR {
            //     if let Some(actions) = complete_actions.get(&year) {
            //         if !actions.is_empty() {
            //             println!("  Year {}: {} actions", year, actions.len());
            //         }
            //     }
            // }
            
            // Store the complete maps
            self.best_actions = Some(complete_actions);
            self.best_deficit_actions = Some(complete_deficit_actions);
            
            // Debug: Check the best_actions we just stored
            if let Some(ref best_actions) = self.best_actions {
                let total_best_actions = best_actions.values().map(|v| v.len()).sum::<usize>();
                let years_with_best_actions = best_actions.values().filter(|v| !v.is_empty()).count();
                // println!("DEBUG: After update - best_actions now has {} actions across {} years", 
                //          total_best_actions, years_with_best_actions);
                
                // Detailed per-year breakdown of best actions
                // println!("Best actions per year after storage:");
                // for year in START_YEAR..=END_YEAR {
                //     if let Some(actions) = best_actions.get(&year) {
                //         if !actions.is_empty() {
                //             println!("  Year {}: {} best actions", year, actions.len());
                //         }
                //     }
                // }
            }
            
            // Reset iterations without improvement counter when we find a better strategy
            self.iterations_without_improvement = ZERO_U32;
        } else {
            // Track iterations without improvement
            self.iterations_without_improvement += 1;
            
            // Occasionally log if we have many iterations without improvement
            if self.iterations_without_improvement % SMALL_LOG_INTERVAL == ZERO_U32 {
                // println!("⏳ {} iterations without finding a better strategy", 
                //          self.iterations_without_improvement);
            }
        }
    }

    pub fn record_action(&mut self, year: u32, action: GridAction) {
        self.current_run_actions.entry(year)
            .or_insert_with(Vec::new)
            .push(action);
    }

    pub fn get_best_actions_for_year(&self, year: u32) -> Option<&Vec<GridAction>> {
        if let Some(ref best_actions) = self.best_actions {
            // If we have best_actions but not for this specific year, return empty vec instead of None
            return best_actions.get(&year).or_else(|| {
                // println!("⚠️ WARNING: Best actions exist but none for year {}", year);
                None
            });
        }
        None
    }

    pub fn get_current_run_actions_for_year(&self, year: u32) -> Option<&Vec<GridAction>> {
        self.current_run_actions.get(&year)
    }
    
    pub fn get_recorded_actions(&self) -> Vec<(u32, &GridAction)> {
        let mut actions = Vec::new();
        
        // Collect actions from current run
        for (year, year_actions) in &self.current_run_actions {
            for action in year_actions {
                actions.push((*year, action));
            }
        }
        
        // Also collect deficit actions
        for (year, year_actions) in &self.current_deficit_actions {
            for action in year_actions {
                actions.push((*year, action));
            }
        }
        
        // Sort by year (important for correct simulation order)
        actions.sort_by_key(|(year, _)| *year);
        
        actions
    }

    pub fn update_weights_from(&mut self, other: &ActionWeights) {
        // Copy weights
        for (year, other_year_weights) in &other.weights {
            let year_weights = self.weights.entry(*year).or_insert_with(HashMap::new);
            for (action, weight) in other_year_weights {
                year_weights.insert(action.clone(), *weight);
            }
        }
        
        // Copy deficit weights
        for (year, other_deficit_weights) in &other.deficit_weights {
            let deficit_weights = self.deficit_weights.entry(*year).or_insert_with(HashMap::new);
            for (action, weight) in other_deficit_weights {
                deficit_weights.insert(action.clone(), *weight);
            }
        }
        
        // Copy action count weights
        for (year, other_count_weights) in &other.action_count_weights {
            let count_weights = self.action_count_weights.entry(*year).or_insert_with(HashMap::new);
            for (count, weight) in other_count_weights {
                count_weights.insert(*count, *weight);
            }
        }
        
        // Update performance counters
        self.iteration_count = std::cmp::max(self.iteration_count, other.iteration_count);
        
        // Transfer recorded actions from the other instance
        self.transfer_recorded_actions_from(other);
    }

    pub fn transfer_recorded_actions_from(&mut self, other: &ActionWeights) {
        // Clear our current run actions first
        self.current_run_actions.clear();
        
        // Copy all actions from the other instance's current run
        for (year, actions) in &other.current_run_actions {
            if !actions.is_empty() {
                // Clone the vector of actions for the current year
                self.current_run_actions.insert(*year, actions.clone());
            }
        }
        
        // Do the same for deficit actions
        self.current_deficit_actions.clear();
        for (year, actions) in &other.current_deficit_actions {
            if !actions.is_empty() {
                self.current_deficit_actions.insert(*year, actions.clone());
            }
        }
        
        // Print debug info only if debug weights is enabled
        if crate::ai::learning::constants::is_debug_weights_enabled() {
            // Debug output to verify actions were transferred
            let total_actions = self.current_run_actions.values().map(|v| v.len()).sum::<usize>();
            let years_with_actions = self.current_run_actions.values().filter(|v| !v.is_empty()).count();
            
            // println!("DEBUG: Transferred {} actions across {} years from local weights", 
            //          total_actions, years_with_actions);
        }
    }

}
