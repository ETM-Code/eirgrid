// Diagnostic and debugging functions for ActionWeights

use crate::ai::learning::constants::*;
use super::ActionWeights;

// Add a dummy public item to ensure this file is recognized by rust-analyzer
#[allow(dead_code)]
pub const MODULE_MARKER: &str = "diagnostics_module";

impl ActionWeights {

// This file contains extracted code from the original weights.rs file
// Appropriate imports will need to be added based on the specific requirements

    pub fn print_top_actions(&self, year: u32, n: usize) {
        if let Some(year_weights) = self.weights.get(&year) {
            let mut actions: Vec<_> = year_weights.iter().collect();
            actions.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
            
            println!("\nTop {} actions for year {}:", n, year);
            for (i, (action, weight)) in actions.iter().take(n).enumerate() {
                println!("{}. {:?}: {:.3}", i + 1, action, weight);
            }
        }
    }

    pub fn diagnose_best_actions(&self) {
        println!("✅ Best actions recorded: {} across {} years", 
            self.best_actions.as_ref().map_or(0, |actions| actions.values().map(|v| v.len()).sum::<usize>()),
            self.best_actions.as_ref().map_or(0, |actions| actions.values().filter(|v| !v.is_empty()).count()));
            
        println!("✅ Best deficit actions recorded: {} across {} years",
            self.best_deficit_actions.as_ref().map_or(0, |actions| actions.values().map(|v| v.len()).sum::<usize>()),
            self.best_deficit_actions.as_ref().map_or(0, |actions| actions.values().filter(|v| !v.is_empty()).count()));
        
        // Print the distribution of actions per year
        println!("Action distribution per year:");
        
        // If we have best metrics, also show those
        if let Some(ref metrics) = self.best_metrics {
            println!("✅ Best metrics recorded:");
            println!("  Net emissions: {:.2} tonnes", metrics.final_net_emissions);
            println!("  Is net zero: {}", if metrics.final_net_emissions <= ZERO_F64 { "true" } else { "false" });
            println!("  Total cost: €{:.2}B", metrics.total_cost / BILLION_DIVISOR);
            println!("  Public opinion: {:.1}%", metrics.average_public_opinion * PERCENT_CONVERSION);
            println!("  Power reliability: {:.1}%", metrics.power_reliability * PERCENT_CONVERSION);
        } else {
            println!("❌ No best metrics recorded yet");
        }
        
        println!("Total iterations: {}", self.iteration_count);
        println!("Iterations without improvement: {}", self.iterations_without_improvement);
    }

    pub fn debug_print_recorded_actions(&self) {
        println!("DEBUG: Current run has {} actions recorded", 
                self.current_run_actions.values().map(|v| v.len()).sum::<usize>());
    }
    
    /// Prints detailed information about the current run actions
    pub fn debug_print_current_run_actions(&self) {
        let total_actions = self.current_run_actions.values().map(|v| v.len()).sum::<usize>();
        let years_with_actions = self.current_run_actions.values().filter(|v| !v.is_empty()).count();
        
        println!("📊 CURRENT RUN ACTIONS: {} actions across {} years", total_actions, years_with_actions);
        
        // Print actions per year
        let mut years: Vec<_> = self.current_run_actions.keys().cloned().collect();
        years.sort();
        for year in years {
            if let Some(actions) = self.current_run_actions.get(&year) {
                if !actions.is_empty() {
                    println!("  Year {}: {} actions", year, actions.len());
                }
            }
        }
        
        // Print deficit actions
        let total_deficit_actions = self.current_deficit_actions.values().map(|v| v.len()).sum::<usize>();
        let years_with_deficit_actions = self.current_deficit_actions.values().filter(|v| !v.is_empty()).count();
        
        println!("📊 CURRENT RUN DEFICIT ACTIONS: {} actions across {} years", 
                total_deficit_actions, years_with_deficit_actions);
    }

    pub fn debug_print_deficit_actions(&self) {
        // Only print debug info if weights debugging is enabled
        if !crate::ai::learning::constants::is_debug_weights_enabled() {
            return;
        }
        
        let total_actions = self.current_deficit_actions.values().map(|v| v.len()).sum::<usize>();
        let years_with_actions = self.current_deficit_actions.values().filter(|v| !v.is_empty()).count();
        
        println!("📊 DEBUG: Deficit actions recorded in current run:");
        println!("  Total: {} deficit actions across {} years", total_actions, years_with_actions);
        
        // Add per-year breakdown for easier diagnostics
        let min_year = START_YEAR;
        let max_year = END_YEAR;
        
        println!("  Per-year deficit action counts:");
        for year in min_year..=max_year {
            if let Some(actions) = self.current_deficit_actions.get(&year) {
                if !actions.is_empty() {
                    println!("    Year {}: {} deficit actions", year, actions.len());
                }
            }
        }
    }

    pub fn print_action_count_weights(&self, year: u32) {
        if let Some(year_counts) = self.action_count_weights.get(&year) {
            println!("\n📊 Action Count Weights for Year {}:", year);
            
            // Get the counts and sort them
            let mut counts: Vec<u32> = year_counts.keys().cloned().collect();
            counts.sort();
            
            // Find the maximum weight for scaling the bar chart
            let max_weight = *year_counts.values().max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(&0.0);
            
            // Print a simple bar chart for visualization
            for count in counts {
                if let Some(&weight) = year_counts.get(&count) {
                    let bar_length = ((weight / max_weight) * 50.0).round() as usize;
                    let bar = "#".repeat(bar_length);
                    println!("{:2} actions: {:6.4} | {}", count, weight, bar);
                }
            }
            println!();
        } else {
            println!("\n❌ No action count weights found for year {}", year);
        }
    }

}
