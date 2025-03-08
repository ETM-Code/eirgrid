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

    // Default scoring logic - First priority: Reach net zero emissions
    if metrics.final_net_emissions > ZERO_F64 {
        // If we haven't achieved net zero, only focus on reducing emissions
        ONE_F64 - (metrics.final_net_emissions / MAX_ACCEPTABLE_EMISSIONS).min(ONE_F64)
    }
    // Second priority: Optimize costs after achieving net zero
    else {
        // Base score of 1.0 for achieving net zero
        let base_score = BASE_NET_ZERO_SCORE;
        
        // Cost component - normalized and inverted so lower costs give higher scores
        // Use log scale to differentiate between very high costs
        let normalized_cost = (metrics.total_cost / MAX_ACCEPTABLE_COST).max(ONE_F64);
        let log_cost = normalized_cost.ln();
        let max_expected_log_cost = (MAX_ACCEPTABLE_COST * MAX_BUDGET_MULTIPLIER / MAX_ACCEPTABLE_COST).ln(); // Assume 100x budget is max
        let cost_score = ONE_F64 - (log_cost / max_expected_log_cost).min(ONE_F64);
        
        // Public opinion component
        let opinion_score = metrics.average_public_opinion;
        
        // Combine scores with appropriate weights
        // Cost is higher priority until it's reasonable
        let cost_weight = if normalized_cost > HIGH_COST_THRESHOLD_MULTIPLIER { HIGH_COST_WEIGHT } else { NORMAL_COST_WEIGHT };
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