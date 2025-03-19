// Scoring module - contains functions for evaluating simulation metrics
use super::simulation_metrics::{SimulationMetrics, ActionResult};
use crate::ai::learning::constants::*;

pub fn score_metrics(metrics: &SimulationMetrics, optimization_mode: Option<&str>) -> f64 {
    // Check for cost-only optimization mode
    if let Some(mode) = optimization_mode {
        if mode == "cost_only" {
            // In cost-only mode, only consider cost improvements regardless of emissions state
            // Normalize and invert cost so lower costs give higher scores
            let normalized_cost = (metrics.total_cost / MAX_ACCEPTABLE_COST).max(ONE_F64);
            let log_cost = normalized_cost.ln();
            let max_expected_log_cost = (MAX_ACCEPTABLE_COST * MAX_BUDGET_MULTIPLIER / MAX_ACCEPTABLE_COST).ln(); // Assume 100x budget is max
            return MAX_SCORE_RANGE - (log_cost / max_expected_log_cost).min(ONE_F64); // Return value between 1.0 and 2.0
        }
    }

    // Tier 1: Power reliability up to 95% (score 0-0.8)
    // Only consider power reliability in this tier
    if metrics.worst_power_reliability < 0.95 {
        // Scale power reliability to 0-0.8 range
        metrics.worst_power_reliability * 0.8 / 0.95
    }
    // Tier 2: Power reliability >= 95% but still emitting (score 1.0-1.8)
    // Only consider emissions reduction in this tier
    else if metrics.final_net_emissions > 0.0 {
        // Base score of 1.0 for achieving 95% power reliability
        let base_score = 1.0;
        
        // Emissions component - scale remaining emissions to 0-0.8 range
        // Score based on progress towards net zero
        let emissions_score = if metrics.final_net_emissions > ZERO_F64 {
            // Calculate how far we are from net zero as a percentage
            // Use a reference point of 200M tonnes (typical starting point) to normalize
            let reference_emissions = 200_000_000.0; // 200M tonnes
            let progress = 1.0 - (metrics.final_net_emissions / reference_emissions).min(1.0);
            
            // Use a power function to make the scoring more sensitive to improvements
            // This means reducing from 200M to 150M gives a bigger score boost than
            // reducing from 50M to 0M
            progress.powf(0.5) // Square root makes it more sensitive to early improvements
        } else {
            ONE_F64
        };
        
        // Add diagnostic output
        // println!("TIER 2 SCORING:");
        // println!("  - Current emissions: {:.1}M tonnes", metrics.final_net_emissions / 1_000_000.0);
        // println!("  - Reference emissions: 200.0M tonnes");
        // println!("  - Progress towards net zero: {:.2}%", (1.0 - (metrics.final_net_emissions / 200_000_000.0).min(1.0)) * 100.0);
        // println!("  - Emissions score: {:.4}", emissions_score);
        // println!("  - Final score: {:.4}", base_score + (emissions_score * 0.8));
        
        base_score + (emissions_score * 0.8)
    }
    // Tier 3: Power reliability >= 95% and negative emissions (score 2.0-3.0)
    // Only consider cost and public opinion in this tier
    else {
        // Base score of 2.0 for achieving both power reliability and negative emissions
        let base_score = 2.0;
        
        // Cost component - normalized and inverted so lower costs give higher scores
        // Only consider cost and public opinion, ignoring other metrics
        let normalized_cost = (metrics.total_cost / MAX_ACCEPTABLE_COST).max(ONE_F64);
        let log_cost = normalized_cost.ln();
        let max_expected_log_cost = (MAX_ACCEPTABLE_COST * MAX_BUDGET_MULTIPLIER / MAX_ACCEPTABLE_COST).ln();
        let cost_score = ONE_F64 - (log_cost / max_expected_log_cost).min(ONE_F64);
        
        // Public opinion component
        let opinion_score = metrics.average_public_opinion;
        
        // Weight cost more heavily until we're under 200% of budget
        let cost_weight = if normalized_cost > 2.0 { 0.8 } else { 0.6 };
        let opinion_weight = ONE_F64 - cost_weight;
        
        base_score + (cost_score * cost_weight + opinion_score * opinion_weight)
    }
}
pub fn evaluate_action_impact(
    current_state: &ActionResult,
    new_state: &ActionResult,
    optimization_mode: Option<&str>,
) -> f64 {
    // Check for cost-only optimization mode
    if let Some(mode) = optimization_mode {
        if mode == "cost_only" {
            // In cost-only mode, only consider cost improvements regardless of emissions state
            let cost_change = new_state.total_cost - current_state.total_cost;
            return -cost_change / current_state.total_cost.abs().max(ONE_F64);
        }
    }

    // Default evaluation logic
    if current_state.net_emissions > ZERO_F64 {
        // First priority: If we haven't achieved net zero, only consider emissions
        let emissions_improvement = (current_state.net_emissions - new_state.net_emissions) / 
                                  current_state.net_emissions.abs().max(ONE_F64);
        emissions_improvement
    }
    else {
        // If we've achieved net zero, consider both cost and opinion improvements
        
        // Cost improvement (negative is better)
        let cost_change = new_state.total_cost - current_state.total_cost;
        let cost_improvement = -cost_change / current_state.total_cost.abs().max(ONE_F64);
        
        // Opinion improvement
        let opinion_improvement = (new_state.public_opinion - current_state.public_opinion) /
                                current_state.public_opinion.abs().max(ONE_F64);
        
        // Weight cost more heavily if it's very high
        let cost_weight = if current_state.total_cost > MAX_ACCEPTABLE_COST * HIGH_COST_THRESHOLD_MULTIPLIER { HIGH_COST_WEIGHT } else { NORMAL_COST_WEIGHT };
        let opinion_weight = ONE_F64 - cost_weight;
        
        // Combined improvement score
        cost_improvement * cost_weight + opinion_improvement * opinion_weight
    }
}