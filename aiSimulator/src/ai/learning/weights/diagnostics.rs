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
        let total_actions = self.current_run_actions.values().map(|v| v.len()).sum::<usize>();
        let years_with_actions = self.current_run_actions.values().filter(|v| !v.is_empty()).count();
        
        println!("📊 DEBUG: Actions recorded in current run:");
        println!("  Total: {} actions across {} years", total_actions, years_with_actions);
        
        // Add per-year breakdown for easier diagnostics
        let min_year = START_YEAR;
        let max_year = END_YEAR;
        
        println!("  Per-year action counts:");
        for year in min_year..=max_year {
            if let Some(actions) = self.current_run_actions.get(&year) {
                if !actions.is_empty() {
                    println!("    Year {}: {} actions", year, actions.len());
                }
            }
        }
    }

    pub fn debug_print_deficit_actions(&self) {
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

}
