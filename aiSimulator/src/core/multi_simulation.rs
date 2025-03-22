use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use rayon::prelude::*;
use crate::models::carbon_offset::CarbonOffsetType;
use crate::models::generator::GeneratorType;
use crate::utils::csv_export::{self, CsvExporter};
use crate::config::constants::{
    FOREST_BASE_COST, WETLAND_BASE_COST, ACTIVE_CAPTURE_BASE_COST, CARBON_CREDIT_BASE_COST,
    FOREST_OPERATING_COST, WETLAND_OPERATING_COST, ACTIVE_CAPTURE_OPERATING_COST, CARBON_CREDIT_OPERATING_COST, MAX_ACCEPTABLE_EMISSIONS, MAX_ACCEPTABLE_COST,
    DEVELOPING_TECH_IMPROVEMENT_RATE, EMERGING_TECH_IMPROVEMENT_RATE, MATURE_TECH_IMPROVEMENT_RATE, BASE_YEAR,
    COAL_CO2_RATE, GAS_CC_CO2_RATE, GAS_PEAKER_CO2_RATE, BIOMASS_CO2_RATE,
    END_YEAR, MAP_MAX_X, MAP_MAX_Y,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use parking_lot::RwLock;
use crate::utils::map_handler::Map;
use crate::ai::learning::weights::ActionWeights;
use crate::core::action_weights::GridAction;
use crate::analysis::metrics::SimulationResult;
use crate::core::iteration::run_iteration;
use crate::utils::logging;
use crate::utils::logging::OperationCategory;
use crate::data::poi::POI;
use crate::core::action_weights::evaluate_action_impact;
use crate::core::actions::apply_action;
use chrono::Local;
use serde::{Serialize, Deserialize};
use crate::core::action_weights::SimulationMetrics;
use crate::config::simulation_config::SimulationConfig;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde_json;

const FULL_RUN_PERCENTAGE: usize = 10;
const REPLAY_BEST_STRATEGY_IN_FULL_RUNS: bool = true;

// Constants for the full simulation continuation prompt
const FULL_SIM_INTERVAL: usize = 10000;  // Ask after this many full simulations
const FULL_SIM_THRESHOLD_PERCENT: f64 = 5.0;  // Target percentage of best score (5%)

// Generator efficiency constants
const ONSHORE_OFFSHORE_WIND_EFFICIENCY: f64 = 0.45;
const UTILITY_SOLAR_EFFICIENCY: f64 = 0.40;
const NUCLEAR_EFFICIENCY: f64 = 0.50;
const GAS_COMBINED_CYCLE_EFFICIENCY: f64 = 0.60;
const HYDRO_PUMPED_STORAGE_EFFICIENCY: f64 = 0.85;
const TIDAL_WAVE_EFFICIENCY: f64 = 0.35;
const DEFAULT_EFFICIENCY: f64 = 0.40;

// Import metrics_to_action_result from main module as it's defined there
fn metrics_to_action_result(metrics: &crate::core::action_weights::SimulationMetrics) -> crate::core::action_weights::ActionResult {
    crate::core::action_weights::ActionResult {
        net_emissions: metrics.final_net_emissions,
        public_opinion: metrics.average_public_opinion,
        power_balance: 0.0, // Not directly available in SimulationMetrics
        total_cost: metrics.total_cost,
    }
}

// Add this helper function to prompt the user
fn prompt_continue_full_simulations(best_score: f64, current_score: f64) -> bool {
    let percent_of_best = (current_score / best_score) * 100.0;
    let percent_diff = 100.0 - percent_of_best;
    
    println!("\n======================================================");
    println!("FULL SIMULATION CONTINUATION PROMPT");
    println!("======================================================");
    println!("Current best score from exploration: {:.4}", best_score);
    println!("Best score from full simulations: {:.4}", current_score);
    println!("Full simulation is {:.2}% of the way to the best score", percent_of_best);
    println!("({:.2}% difference)", percent_diff.abs());
    println!("Target: Get within {:.1}% of the best score", FULL_SIM_THRESHOLD_PERCENT);
    println!("------------------------------------------------------");
    println!("Would you like to continue running full simulations?");
    print!("Enter 'y' to continue or any other key to stop: ");
    
    // Flush to ensure the prompt is displayed
    std::io::stdout().flush().unwrap();
    
    // Read user input
    let mut input = String::new();
    match std::io::stdin().read_line(&mut input) {
        Ok(_) => {},
        Err(_) => return false, // If reading fails, default to stopping
    }
    
    // Return true if user wants to continue
    input.trim().to_lowercase() == "y"
}

pub fn run_multi_simulation(
    base_map: &Map,
    num_iterations: usize,
    parallel: bool,
    continue_from_checkpoint: bool,
    checkpoint_dir: &str,
    checkpoint_interval: usize,
    progress_interval: usize,
    cache_dir: &str,
    force_full_simulation: bool,
    seed: Option<u64>,
    verbose_logging: bool,
    optimization_mode: Option<&str>,
    enable_energy_sales: bool,
    enable_csv_export: bool,
    debug_weights: bool,
    enable_construction_delays: bool,
    track_weight_history: bool,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Configure debug weights output
    crate::ai::learning::constants::set_debug_weights(debug_weights);
    
    let _timing = logging::start_timing("run_multi_simulation", OperationCategory::Simulation);
    
    // Define a struct to track full simulation data
    struct FullSimData {
        count: usize,
        best_score: f64,
        continue_sims: bool,
        should_prompt: bool,
    }
    
    // Create a shared tracking struct for full simulations
    let full_sim_tracking = if parallel {
        Arc::new(Mutex::new(FullSimData {
            count: 0,
            best_score: 0.0,
            continue_sims: true,
            should_prompt: false,
        }))
    } else {
        // For sequential execution, we'll track this differently
        Arc::new(Mutex::new(FullSimData {
            count: 0,
            best_score: 0.0,
            continue_sims: true,
            should_prompt: false,
        }))
    };
    
    let result = (|| {
        // Create checkpoint directory if it doesn't exist
        std::fs::create_dir_all(checkpoint_dir)?;
         
        // Load location analysis cache
        let mut base_map = base_map.clone();
        let cache_loaded = base_map.load_location_analysis(cache_dir)?;
        if !cache_loaded {
            println!("Warning: Location analysis cache not found in {}. All simulations will use full mode.", cache_dir);
        }

        // Initialize progress tracking
        let completed_iterations = Arc::new(AtomicUsize::new(0));
        let start_time = Instant::now();
         
        // Create timestamp directory after loading weights
        let now = Local::now();
        let timestamp = format!("2024{}", now.format("%m%d_%H%M%S"));
        let run_dir = format!("{}/{}", checkpoint_dir, timestamp);
        std::fs::create_dir_all(&run_dir)?;
        
        // Create weight history file if tracking is enabled
        let weight_history_path = if track_weight_history {
            let history_path = Path::new(&run_dir).join("weight_history.json");
            if !history_path.exists() {
                let mut file = File::create(&history_path)?;
                file.write_all(b"[]")?;
            }
            Some(history_path)
        } else {
            None
        };

        // Function to save weight history
        let save_weight_history = |weights: &ActionWeights, iteration: usize| -> Result<(), Box<dyn Error + Send + Sync>> {
            if let Some(history_path) = &weight_history_path {
                let mut history: Vec<serde_json::Value> = if history_path.exists() {
                    let contents = std::fs::read_to_string(history_path)?;
                    if contents.trim().is_empty() {
                        Vec::new()
                    } else {
                        serde_json::from_str(&contents)?
                    }
                } else {
                    Vec::new()
                };
                
                // Create a snapshot of the current weights
                let snapshot = serde_json::json!({
                    "iteration": iteration,
                    "timestamp": Local::now().to_rfc3339(),
                    "weights": weights.to_json(),
                    "best_score": weights.get_best_metrics().map(|(score, _)| score).unwrap_or(0.0),
                });
                
                history.push(snapshot);
                
                // Write back to file
                let mut file = File::create(history_path)?;
                file.write_all(serde_json::to_string_pretty(&history)?.as_bytes())?;
            }
            Ok(())
        };

        // Load or create initial weights
        let initial_weights = if continue_from_checkpoint {
            // Try to find the most recent checkpoint
            let entries: Vec<_> = std::fs::read_dir(checkpoint_dir)?
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_dir())
                .collect();
             
            let latest_dir = entries.iter()
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| {
                    // Filter out directories with invalid format
                    if name.len() != 15 || !name.chars().all(|c| c.is_ascii_digit() || c == '_') {
                        return false;
                    }
                    // Parse the date from the directory name (format: YYYYMMDD_HHMMSS)
                    let year = name[0..4].parse::<i32>().unwrap_or(9999);
                    let month = name[4..6].parse::<u32>().unwrap_or(99);
                    let day = name[6..8].parse::<u32>().unwrap_or(99);
                     
                    // Accept 2025 and earlier, but ensure month and day are valid
                    if year > 2025 || month > 12 || day > 31 {
                        return false;
                    }
                    true
                })
                .max();
             
            if let Some(latest) = latest_dir {
                let checkpoint_dir = Path::new(checkpoint_dir).join(&latest);
                println!("Checking for weights in: {:?}", checkpoint_dir);
                 
                // Load and merge all thread weights
                let mut merged_weights = ActionWeights::new();
                let mut found_weights = false;
                 
                // First load the shared weights if they exist
                let shared_weights_path = checkpoint_dir.join("latest_weights.json");
                if shared_weights_path.exists() {
                    println!("Loading shared weights from: {:?}", shared_weights_path);
                    if let Ok(weights) = ActionWeights::load_from_file(shared_weights_path.to_str().unwrap()) {
                        merged_weights = weights;
                        found_weights = true;
                    }
                }
                 
                // Then load and merge all thread-specific weights
                for entry in std::fs::read_dir(&checkpoint_dir)? {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                            if filename.starts_with("thread_") && filename.ends_with("_weights.json") {
                                println!("Loading thread weights from: {:?}", path);
                                if let Ok(thread_weights) = ActionWeights::load_from_file(path.to_str().unwrap()) {
                                    merged_weights.update_weights_from(&thread_weights);
                                    found_weights = true;
                                }
                            }
                        }
                    }
                }
                 
                if found_weights {
                    if let Some((best_score, _)) = merged_weights.get_best_metrics() {
                        println!("\n{}", "=".repeat(80));
                        println!("📊 LOADED AND MERGED WEIGHTS FROM PREVIOUS RUNS 📊");
                        println!("Best score from loaded weights: {:.4}", best_score);
                        println!("{}", "=".repeat(80));
                    }
                    merged_weights
                } else {
                    println!("No weights found in latest directory, starting fresh");
                    ActionWeights::new()
                }
            } else {
                println!("No checkpoint directories found, starting fresh");
                ActionWeights::new()
            }
        } else {
            println!("Starting fresh simulation (--no-continue specified)");
            ActionWeights::new()
        };

        // Create a clone of initial weights for later use in sequential mode
        let initial_weights_clone = initial_weights.clone();
         
        // Create shared weights
        let action_weights = Arc::new(RwLock::new(initial_weights));
         
        // Spawn progress monitoring thread
        let progress_counter = completed_iterations.clone();
        let total_iterations = num_iterations;
        let action_weights_for_progress: Arc<RwLock<ActionWeights>> = Arc::clone(&action_weights);
         
        std::thread::spawn(move || {
            while progress_counter.load(Ordering::Relaxed) < total_iterations {
                std::thread::sleep(Duration::from_secs(progress_interval as u64));
                let completed = progress_counter.load(Ordering::Relaxed);
                let elapsed = start_time.elapsed();
                let iterations_per_second = completed as f64 / elapsed.as_secs_f64();
                let remaining = total_iterations - completed;
                let eta_seconds = if iterations_per_second > 0.0 {
                    remaining as f64 / iterations_per_second
                } else {
                    0.0
                };

                // Get best score and metrics from the shared weights
                let weights = action_weights_for_progress.read();
                let (best_score, is_net_zero) = weights.get_best_metrics()
                    .unwrap_or((0.0, false));
                 
                // Get emissions metrics from the best metrics
                let metrics_info = if let Some(best) = weights.get_simulation_metrics() {
                    let emissions_status = if best.final_net_emissions <= 0.0 {
                        "✅ NET ZERO ACHIEVED".to_string()
                    } else {
                        format!("⚠ {:.1}% above target",
                            (best.final_net_emissions / MAX_ACCEPTABLE_EMISSIONS) * 100.0)
                    };
                     
                    let cost_status = if best.total_cost <= MAX_ACCEPTABLE_COST {
                        "✅ WITHIN BUDGET".to_string()
                    } else {
                        format!("❌ {:.1}% OVER BUDGET",
                            ((best.total_cost - MAX_ACCEPTABLE_COST) / MAX_ACCEPTABLE_COST) * 100.0)
                    };
                     
                    format!("\nMetrics Status:\n\
                    - Emissions: {} ({:.1} tonnes)\n\
                    - Cost: {} (€{:.1}B accumulated)\n\
                    - Public Opinion: {:.1}%\n\
                    - Power Reliability: {:.1}%",
                    emissions_status,
                    best.final_net_emissions,
                    cost_status,
                    best.total_cost / 1_000_000_000.0,
                    best.average_public_opinion * 100.0,
                    best.power_reliability * 100.0)
                } else {
                    "\nMetrics Status: No data yet".to_string()
                };
                 
                println!(
                    "\n{}\n\
                    📈 PROGRESS UPDATE 📈\n\
                    {}\n\
                    Iterations: {}/{} ({:.1}%)\n\
                    Speed: {:.1} iterations/sec\n\
                    ETA: {:.1} minutes\n\
                    \n\
                    Best Score: {:.9} {}\n\
                    Target: 1.0000 (Net Zero + Max Public Opinion){}\n\
                    \n\
                    Score Explanation:\n\
                    - Score < 1.0000: Working on reducing emissions\n\
                    - Score = 0.0000: Emissions at or above maximum\n\
                    - Score > 0.0000: Making progress on emissions\n\
                    - [NET ZERO]: Achieved net zero, score is now public opinion\n\
                    {}",
                    "=".repeat(80),
                    "=".repeat(80),
                    completed,
                    total_iterations,
                    (completed as f64 / total_iterations as f64) * 100.0,
                    iterations_per_second,
                    eta_seconds / 60.0,
                    best_score,
                    if is_net_zero { "✅ [NET ZERO]" } else { "" },
                    metrics_info,
                    "=".repeat(80)
                );
            }
        });

        let mut best_result: Option<SimulationResult> = None;
        let start_iteration = if continue_from_checkpoint {
            let entries: Vec<_> = std::fs::read_dir(checkpoint_dir)?
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_dir())
                .collect();
             
            if let Some(latest_dir) = entries.iter()
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| name.chars().all(|c| c.is_ascii_digit() || c == '_'))
                .max()
            {
                let iteration_path = Path::new(checkpoint_dir)
                    .join(&latest_dir)
                    .join("checkpoint_iteration.txt");
                 
                if iteration_path.exists() {
                    std::fs::read_to_string(iteration_path)?
                        .trim()
                        .parse::<usize>()
                        .unwrap_or(0)
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        };
         
        println!("Starting multi-simulation optimization with {} iterations ({} completed, {} remaining) in directory {}",
            num_iterations,
            start_iteration,
            num_iterations - start_iteration,
            run_dir);
         
        // Create a clone of the base map's static data once
        let static_data = base_map.get_static_data();
         
        if parallel {
            let results: Vec<_> = (start_iteration..num_iterations)
                .into_par_iter()
                .map(|i| -> Result<SimulationResult, Box<dyn Error + Send + Sync>> {
                    // Create a new map instance with shared static data
                    let mut map_clone = Map::new_with_static_data(static_data.clone());
                     
                    // Clone only the dynamic data
                    map_clone.set_generators(base_map.get_generators().to_vec());
                    map_clone.set_settlements(base_map.get_settlements().to_vec());
                    map_clone.set_carbon_offsets(base_map.get_carbon_offsets().to_vec());

                    // Set simulation mode based on global progress
                    let final_full_sim_count = (num_iterations * FULL_RUN_PERCENTAGE) / 100;
                    let total_completed = completed_iterations.load(Ordering::Relaxed);
                     
                    // A run should be a full run if:
                    // 1. Full simulation is forced by command line
                    // 2. We're in the final X% of iterations (based on FULL_RUN_PERCENTAGE)
                    // 3. Cache isn't loaded (forcing full sim)
                    let is_full_run = force_full_simulation ||
                        !cache_loaded ||
                        total_completed >= num_iterations.saturating_sub(final_full_sim_count);
                     
                    // Set simulation mode
                    map_clone.set_simulation_mode(!is_full_run);
                     
                    // if total_completed == num_iterations.saturating_sub(final_full_sim_count) {
                    //     println!("\nSwitching to full simulation mode for final {} iterations ({:.1}% of total)",
                    //         final_full_sim_count, FULL_RUN_PERCENTAGE as f64);
                    // }
                     
                    // Create local weights and immediately drop the read lock
                    let mut local_weights = {
                        let weights = action_weights.read();
                        weights.clone()
                    }; // Read lock is dropped here
                     
                    // Determine if we should replay the best strategy
                    let replay_best_strategy = is_full_run &&
                        REPLAY_BEST_STRATEGY_IN_FULL_RUNS &&
                        local_weights.has_best_actions();
                     
                    // Log if we're replaying the best strategy
                    // if replay_best_strategy {
                    //     println!("🔁 Iteration {} is replaying the best strategy for thorough analysis", i + 1);
                    // }
                     
                    let result = run_iteration(i, &mut map_clone, &mut local_weights, replay_best_strategy, seed, verbose_logging, optimization_mode, enable_energy_sales, enable_construction_delays)?;
                     
                    // Track full simulation results for the user prompt functionality
                    if is_full_run {
                        let mut full_sim_data = full_sim_tracking.lock().unwrap();
                        full_sim_data.count += 1;
                        
                        // Get the score for this simulation
                        let score = crate::ai::score_metrics(&result.metrics, optimization_mode);
                        
                        // Update the best full sim score if this is better
                        if score > full_sim_data.best_score {
                            full_sim_data.best_score = score;
                        }
                        
                        // Check if we need to prompt the user to continue
                        if full_sim_data.count % FULL_SIM_INTERVAL == 0 {
                            full_sim_data.should_prompt = true;
                        }
                    }
                     
                    // Update best metrics immediately - changed order to transfer actions first
                    let best_metrics_after_update = {
                        let mut weights = action_weights.write();
                        weights.transfer_recorded_actions_from(&local_weights);
                        
                        // Apply contrast learning before updating best strategy
                        weights.apply_contrast_learning(&result.metrics);
                        
                        // After applying contrast learning, update the best strategy if this one is better
                        weights.update_best_strategy(result.metrics.clone());
                        
                        // If we're handling deficit actions, also apply deficit contrast learning
                        weights.apply_deficit_contrast_learning();
                        
                        weights.get_simulation_metrics().cloned()
                    };
                    
                    // Print iteration results at the end of the iteration
                    let current_score = crate::ai::score_metrics(&result.metrics, optimization_mode);
                    let thread_id = rayon::current_thread_index().unwrap_or(0);
                    if let Some(best_metrics) = &best_metrics_after_update {
                        let best_score = crate::ai::score_metrics(best_metrics, optimization_mode);
                        if (i + 1) % 100 == 0 {  // Only print every 100 iterations
                            println!("\n🔄 Iteration {} completed (Thread {}): ", i + 1, thread_id);
                            println!("  Current result: Score {:.6} (Emissions: {:.1} tonnes, Cost: €{:.1}B, Opinion: {:.1}%)",
                                current_score,
                                result.metrics.final_net_emissions,
                                result.metrics.total_cost / 1_000_000_000.0,
                                result.metrics.average_public_opinion * 100.0);
                            println!("  Best result so far: Score {:.6} (Emissions: {:.1} tonnes, Cost: €{:.1}B, Opinion: {:.1}%)",
                                best_score,
                                best_metrics.final_net_emissions,
                                best_metrics.total_cost / 1_000_000_000.0,
                                best_metrics.average_public_opinion * 100.0);
                        }
                    } else {
                        if (i + 1) % 100 == 0 {  // Only print every 100 iterations
                            println!("\n🔄 Iteration {} completed (Thread {}): ", i + 1, thread_id);
                            println!("  Current result: Score {:.6} (Emissions: {:.1} tonnes, Cost: €{:.1}B, Opinion: {:.1}%)",
                                current_score,
                                result.metrics.final_net_emissions,
                                result.metrics.total_cost / 1_000_000_000.0,
                                result.metrics.average_public_opinion * 100.0);
                            println!("  Best result so far: No best result yet");
                        }
                    }
                     
                    // Increment completed iterations counter
                    completed_iterations.fetch_add(1, Ordering::Relaxed);
                     
                    // Save checkpoint at intervals
                    if (i + 1) % checkpoint_interval == 0 {
                        let thread_id = rayon::current_thread_index().unwrap_or(0);
                        let weights = action_weights.write();
                         
                        // Save thread-specific weights
                        let thread_weights_path = Path::new(&run_dir)
                            .join(format!("thread_{}_weights.json", thread_id));
                        local_weights.save_to_file(thread_weights_path.to_str().unwrap())?;
                         
                        // Save shared weights
                        let checkpoint_path = Path::new(&run_dir).join("latest_weights.json");
                        weights.save_to_file(checkpoint_path.to_str().unwrap())?;
                         
                        // Save weight history if enabled
                        if track_weight_history {
                            save_weight_history(&weights, i)?;
                        }
                         
                        // Save iteration number
                        let iteration_path = Path::new(&run_dir).join("checkpoint_iteration.txt");
                        std::fs::write(iteration_path, (i + 1).to_string())?;
                         
                        // println!("Saved checkpoint at iteration {} in {} (thread {})", i + 1, run_dir, thread_id);
                    }
                     
                    // Check if we need to prompt the user about continuing with full simulations
                    let should_prompt_now = {
                        let full_sim_data = full_sim_tracking.lock().unwrap();
                        full_sim_data.should_prompt
                    };
                    
                    if should_prompt_now {
                        // Only one thread should handle the prompt, so we'll acquire an exclusive lock
                        let mut full_sim_data = full_sim_tracking.lock().unwrap();
                        
                        // Double-check that the prompt is still needed (another thread may have handled it)
                        if full_sim_data.should_prompt {
                            // Get the current best score from the weights
                            let best_score = {
                                let weights = action_weights.read();
                                if let Some(best_metrics) = weights.get_simulation_metrics() {
                                    crate::ai::score_metrics(best_metrics, optimization_mode)
                                } else {
                                    // If no best metrics yet, use a default score of 1.0
                                    1.0
                                }
                            };
                            
                            // If we're within threshold, mark that we're done with full sims
                            let percent_diff: f64 = 100.0 - ((full_sim_data.best_score / best_score) * 100.0);
                            if percent_diff.abs() <= FULL_SIM_THRESHOLD_PERCENT {
                                println!("\nFull simulation results within {}% of best score, continuing with final batch", 
                                        FULL_SIM_THRESHOLD_PERCENT);
                                full_sim_data.continue_sims = false;
                            } else {
                                // Otherwise, prompt the user
                                full_sim_data.continue_sims = prompt_continue_full_simulations(best_score, full_sim_data.best_score);
                            }
                            
                            // Mark that we've handled the prompt
                            full_sim_data.should_prompt = false;
                        }
                    }
                     
                    // Return the result (clone not needed anymore since we're returning it)
                    Ok(result)
                })
                .collect::<Result<Vec<_>, _>>()?;
             
            // Find the best result from all results AFTER the parallel execution completes
            for result in results {
                if best_result.as_ref().map_or(true, |best| {
                    evaluate_action_impact(&metrics_to_action_result(&result.metrics), &metrics_to_action_result(&best.metrics), optimization_mode) > 0.0
                }) {
                    best_result = Some(result);
                }
            }
        } else {
            // Sequential implementation
            let action_weights = Arc::new(parking_lot::RwLock::new(ActionWeights::new()));
            
            // Load initial weights from the shared weights that were loaded earlier
            {
                let mut weights = action_weights.write();
                // Use the initial weights that were loaded earlier in the function
                *weights = initial_weights_clone.clone();
            }
            
            // Create a timestamp directory for this run
            let run_dir = format!("{}/{}", checkpoint_dir, Local::now().format("%Y%m%d_%H%M%S"));
            std::fs::create_dir_all(&run_dir)?;
            
            for i in start_iteration..num_iterations {
                // Check if we should continue with full simulations
                if force_full_simulation {
                    let full_sim_data = full_sim_tracking.lock().unwrap();
                    if !full_sim_data.continue_sims && full_sim_data.count >= FULL_SIM_INTERVAL {
                        println!("User requested to stop full simulations. Ending multi-simulation.");
                        break;
                    }
                }
                
                // Create a clone of the base map for this iteration
                let mut map_clone = base_map.clone();
                 
                // Clone only the dynamic data
                map_clone.set_generators(base_map.get_generators().to_vec());
                map_clone.set_settlements(base_map.get_settlements().to_vec());
                map_clone.set_carbon_offsets(base_map.get_carbon_offsets().to_vec());

                // Set simulation mode based on global progress
                let final_full_sim_count = (num_iterations * FULL_RUN_PERCENTAGE) / 100;
                let total_completed = completed_iterations.load(Ordering::Relaxed);
                 
                // A run should be a full run if:
                // 1. Full simulation is forced by command line
                // 2. We're in the final X% of iterations (based on FULL_RUN_PERCENTAGE)
                // 3. Cache isn't loaded (forcing full sim)
                let is_full_run = force_full_simulation ||
                    !cache_loaded ||
                    total_completed >= num_iterations.saturating_sub(final_full_sim_count);
                 
                // Set simulation mode
                map_clone.set_simulation_mode(!is_full_run);
                 
                // if total_completed == num_iterations.saturating_sub(final_full_sim_count) {
                //     println!("\nSwitching to full simulation mode for final {} iterations ({:.1}% of total)",
                //         final_full_sim_count, FULL_RUN_PERCENTAGE as f64);
                // }
                 
                // Create local weights and immediately drop the read lock
                let mut local_weights = {
                    let weights = action_weights.read();
                    weights.clone()
                }; // Read lock is dropped here
                 
                // Determine if we should replay the best strategy
                let replay_best_strategy = is_full_run &&
                    REPLAY_BEST_STRATEGY_IN_FULL_RUNS &&
                    local_weights.has_best_actions();
                 
                // // Log if we're replaying the best strategy
                // if replay_best_strategy {
                //     println!("🔁 Iteration {} is replaying the best strategy for thorough analysis", i + 1);
                // }
                 
                let result = run_iteration(i, &mut map_clone, &mut local_weights, replay_best_strategy, seed, verbose_logging, optimization_mode, enable_energy_sales, enable_construction_delays)?;
                
                // Track full simulation results and prompt if needed
                if is_full_run {
                    let mut full_sim_data = full_sim_tracking.lock().unwrap();
                    full_sim_data.count += 1;
                    
                    // Get the score for this simulation
                    let score = crate::ai::score_metrics(&result.metrics, optimization_mode);
                    
                    // Update the best full sim score if this is better
                    if score > full_sim_data.best_score {
                        full_sim_data.best_score = score;
                    }
                    
                    // Check if we need to prompt the user to continue
                    if full_sim_data.count % FULL_SIM_INTERVAL == 0 {
                        // Get the current best score from the weights
                        let best_score = {
                            let weights = action_weights.read();
                            if let Some(best_metrics) = weights.get_simulation_metrics() {
                                crate::ai::score_metrics(best_metrics, optimization_mode)
                            } else {
                                // If no best metrics yet, use a default score of 1.0
                                1.0
                            }
                        };
                        
                        // If we're within threshold, mark that we're done with full sims
                        let percent_of_best = (full_sim_data.best_score / best_score) * 100.0;
                        let percent_diff: f64 = 100.0 - percent_of_best;
                        if percent_diff.abs() <= FULL_SIM_THRESHOLD_PERCENT {
                            println!("\nFull simulation results within {}% of best score, continuing with final batch", 
                                    FULL_SIM_THRESHOLD_PERCENT);
                            // Continue to finish the current batch
                        } else {
                            // Otherwise, prompt the user
                            full_sim_data.continue_sims = prompt_continue_full_simulations(best_score, full_sim_data.best_score);
                            
                            if !full_sim_data.continue_sims {
                                // Finish the current batch of FULL_SIM_INTERVAL before stopping
                                println!("Will stop after current batch of simulations completes.");
                            }
                        }
                    }
                }
                
                // Update best metrics and get the current best
                let best_metrics_after_update = {
                    let mut weights = action_weights.write();
                    weights.transfer_recorded_actions_from(&local_weights);
                    
                    // Apply contrast learning before updating best strategy
                    weights.apply_contrast_learning(&result.metrics);
                    
                    // After applying contrast learning, update the best strategy if this one is better
                    weights.update_best_strategy(result.metrics.clone());
                    
                    // If we're handling deficit actions, also apply deficit contrast learning
                    weights.apply_deficit_contrast_learning();
                    
                    weights.get_simulation_metrics().cloned()
                };
                
                // Print iteration results at the end of the iteration
                let current_score = crate::ai::score_metrics(&result.metrics, optimization_mode);
                let thread_id = 0;  // In sequential mode, thread is always 0
                if let Some(best_metrics) = &best_metrics_after_update {
                    let best_score = crate::ai::score_metrics(best_metrics, optimization_mode);
                    if (i + 1) % 100 == 0 {  // Only print every 100 iterations
                        println!("\n🔄 Iteration {} completed (Thread {}): ", i + 1, thread_id);
                        println!("  Current result: Score {:.6} (Emissions: {:.1} tonnes, Cost: €{:.1}B, Opinion: {:.1}%)",
                            current_score,
                            result.metrics.final_net_emissions,
                            result.metrics.total_cost / 1_000_000_000.0,
                            result.metrics.average_public_opinion * 100.0);
                        println!("  Best result so far: Score {:.6} (Emissions: {:.1} tonnes, Cost: €{:.1}B, Opinion: {:.1}%)",
                            best_score,
                            best_metrics.final_net_emissions,
                            best_metrics.total_cost / 1_000_000_000.0,
                            best_metrics.average_public_opinion * 100.0);
                    }
                } else {
                    if (i + 1) % 100 == 0 {  // Only print every 100 iterations
                        println!("\n🔄 Iteration {} completed (Thread {}): ", i + 1, thread_id);
                        println!("  Current result: Score {:.6} (Emissions: {:.1} tonnes, Cost: €{:.1}B, Opinion: {:.1}%)",
                            current_score,
                            result.metrics.final_net_emissions,
                            result.metrics.total_cost / 1_000_000_000.0,
                            result.metrics.average_public_opinion * 100.0);
                        println!("  Best result so far: No best result yet");
                    }
                }
                 
                // Store each result for later comparison
                let curr_result = result.clone();
                 
                // Increment completed iterations counter
                completed_iterations.fetch_add(1, Ordering::Relaxed);
                 
                // Save checkpoint at intervals
                if (i + 1) % checkpoint_interval == 0 {
                    let checkpoint_path = Path::new(&run_dir).join("latest_weights.json");
                     
                    // Get a write lock to save the weights
                    {
                        let weights = action_weights.write();
                        weights.save_to_file(checkpoint_path.to_str().unwrap())?;
                        
                        // Save weight history if enabled
                        if track_weight_history {
                            save_weight_history(&weights, i)?;
                        }
                    }
                     
                    // Save iteration number
                    let iteration_path = Path::new(&run_dir).join("checkpoint_iteration.txt");
                    std::fs::write(iteration_path, (i + 1).to_string())?;
                     
                    // println!("Saved checkpoint at iteration {} in {}", i + 1, run_dir);
                }
                 
                // Check if this result is better than our best result
                if best_result.as_ref().map_or(true, |best| {
                    evaluate_action_impact(&metrics_to_action_result(&curr_result.metrics), &metrics_to_action_result(&best.metrics), optimization_mode) > 0.0
                }) {
                    best_result = Some(curr_result);
                }
            }
        }
         
        if let Some(best) = best_result {
            println!("\n{}", "=".repeat(80));
            println!("🏆 BEST SIMULATION RESULTS SUMMARY 🏆");
            println!("{}", "=".repeat(80));
            println!("Final net emissions: {:.2} tonnes", best.metrics.final_net_emissions);
             
            // Add emoji indicators for success/failure
            let emissions_status = if best.metrics.final_net_emissions <= 0.0 {
                "✅ NET ZERO ACHIEVED".to_string()
            } else {
                "❌ NET ZERO NOT ACHIEVED".to_string()
            };
             
            let cost_status = if best.metrics.total_cost <= MAX_ACCEPTABLE_COST {
                "✅ WITHIN BUDGET".to_string()
            } else {
                format!("❌ {:.1}% OVER BUDGET",
                    ((best.metrics.total_cost - MAX_ACCEPTABLE_COST) / (MAX_ACCEPTABLE_COST)) * 100.0)
            };
             
            println!("Emissions Status: {}", emissions_status);
            println!("Average public opinion: {:.1}%", best.metrics.average_public_opinion * 100.0);
             
            // Display total cost in billions
            let total_cost_billions = best.metrics.total_cost / 1_000_000_000.0;
            println!("Total cost: €{:.2} billion accumulated ({})",
                total_cost_billions, cost_status);
             
            println!("Power reliability: {:.1}%", best.metrics.power_reliability * 100.0);
            println!("{}", "=".repeat(80));
             
            // Use our enhanced CSV exporter for more detailed data export
            let csv_export_dir = Path::new(&run_dir).join("enhanced_csv");
            std::fs::create_dir_all(&csv_export_dir)?;
            
            if enable_csv_export {
                // Create a CSV exporter instance
                let _timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
                let csv_exporter = csv_export::CsvExporter::new(&csv_export_dir, verbose_logging);
                
                // Create a new map and apply all the best actions to it
                let mut final_map = base_map.clone();
                
                // Ensure the map is in full simulation mode for accurate generator placement
                final_map.set_simulation_mode(false);
                
                // Verify all generators have reasonable coordinates for CSV export
                let generators_needing_location = final_map.get_generators()
                    .iter()
                    .filter(|g| {
                        let x = g.get_coordinate().x;
                        let y = g.get_coordinate().y;
                        
                        // Check if coordinates are very near corners which suggests default placement
                        (x < 1000.0 && y < 1000.0) || 
                        (x > MAP_MAX_X - 1000.0 && y < 1000.0) ||
                        (x < 1000.0 && y > MAP_MAX_Y - 1000.0) ||
                        (x > MAP_MAX_X - 1000.0 && y > MAP_MAX_Y - 1000.0)
                    })
                    .count();

                if generators_needing_location > 0 {
                    println!("WARNING: Found {} generators with suspicious coordinates. Coordinates may not be accurate in CSV export.", 
                        generators_needing_location);
                }

                println!("Applying all actions to map for CSV export...");
                for (year, action) in &best.actions {
                    if let Err(e) = apply_action(&mut final_map, action, *year) {
                        println!("Warning: Failed to apply action {:?} for year {}: {}", action, year, e);
                    }
                }
                
                // Export all simulation results with detailed data using the final map
                println!("DIAGNOSTIC: Metrics before CSV export:");
                println!("  - final_net_emissions: {}", best.metrics.final_net_emissions);
                println!("  - total_cost: {}", best.metrics.total_cost);
                println!("  - average_public_opinion: {}", best.metrics.average_public_opinion);
                println!("  - power_reliability: {}", best.metrics.power_reliability);
                println!("  - Number of yearly metrics: {}", best.yearly_metrics.len());
                
                if let Ok(()) = csv_exporter.export_simulation_results(
                    &final_map,
                    &best.actions,
                    &best.metrics,
                    &csv_export::convert_yearly_metrics(&best.yearly_metrics),
                ) {
                    println!("\nEnhanced simulation results exported to: {}", csv_export_dir.display());
                    println!("Use these files for detailed analysis and visualization.");
                    
                    // Export improvement history
                    // Clone the improvement history to avoid holding the lock during export
                    let improvement_history = {
                        let weights = action_weights.read();
                        weights.get_improvement_history().to_vec()
                    };
                    
                    // Export the improvement history to CSV
                    if let Ok(()) = csv_exporter.export_improvement_history(&improvement_history) {
                        println!("Improvement history exported with {} records", improvement_history.len());
                    }
                } else {
                    // Fallback to basic export if the enhanced export fails
                    let csv_filename = Path::new(&run_dir).join("best_simulation.csv");
                    let mut file = File::create(&csv_filename)?;
                    file.write_all(best.output.as_bytes())?;
                    println!("\nBasic simulation results saved to: {}", csv_filename.display());
                    
                    // Still create basic actions file as fallback
                    let actions_filename = Path::new(&run_dir).join("best_simulation_actions.csv");
                    let mut actions_file = File::create(&actions_filename)?;
                    actions_file.write_all(
                        "Year,Action Type,Details,Capital Cost (EUR),Operating Cost (EUR),\
                        Location X,Location Y,Generator Type,Power Output (MW),Efficiency,\
                        CO2 Output (tonnes),Operation Percentage,Lifespan (years),\
                        Previous State,Impact Description\n".as_bytes()
                    )?;
                    
                    // Write actions in basic format as fallback
                    for (year, action) in &best.actions {
                        let (
                            action_type, details, capital_cost, operating_cost,
                            location_x, location_y, gen_type, power_output, efficiency,
                            co2_output, operation_percentage, lifespan, prev_state, impact
                        ) = match action {
                            GridAction::AddGenerator(gen_type, cost_multiplier) => {
                                // Get base cost for this generator type
                                let base_cost = gen_type.get_base_cost(*year);
                                
                                // Apply cost multiplier
                                let cost = base_cost * (*cost_multiplier as f64 / 100.0);
                                
                                // Calculate estimated CO2 output based on generator type
                                let co2_output = match gen_type {
                                    GeneratorType::CoalPlant => COAL_CO2_RATE,
                                    GeneratorType::GasCombinedCycle => GAS_CC_CO2_RATE,
                                    GeneratorType::GasPeaker => GAS_PEAKER_CO2_RATE,
                                    GeneratorType::Biomass => BIOMASS_CO2_RATE,
                                    _ => 0.0,  // All other types have zero direct CO2 emissions
                                } * gen_type.get_base_power(*year); // Scale by power output
                                
                                (
                                    String::from("AddGenerator"),
                                    gen_type.to_string(),
                                    cost,                           // capital cost
                                    gen_type.get_operating_cost(*year), // operating cost
                                    0.0,                            // location_x (will be set during actual creation)
                                    0.0,                            // location_y (will be set during actual creation)
                                    gen_type.to_string(),           // generator type
                                    gen_type.get_base_power(*year), // power output
                                    gen_type.get_base_efficiency(*year), // efficiency 
                                    co2_output,                     // calculated co2 output
                                    100,                            // Default to 100% operation
                                    gen_type.get_lifespan(),        // lifespan
                                    String::from("New Generator"),  // previous state
                                    format!("Added new {} generator", gen_type.to_string()) // impact
                                )
                            },
                            GridAction::UpgradeEfficiency(id) => {
                                let generator = base_map.get_generators().iter().find(|g| g.get_id() == id);
                                if let Some(gen) = generator {
                                    let base_max = match gen.get_generator_type() {
                                        GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => ONSHORE_OFFSHORE_WIND_EFFICIENCY,
                                        GeneratorType::UtilitySolar => UTILITY_SOLAR_EFFICIENCY,
                                        GeneratorType::Nuclear => NUCLEAR_EFFICIENCY,
                                        GeneratorType::GasCombinedCycle => GAS_COMBINED_CYCLE_EFFICIENCY,
                                        GeneratorType::HydroDam | GeneratorType::PumpedStorage => HYDRO_PUMPED_STORAGE_EFFICIENCY,
                                        GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => TIDAL_WAVE_EFFICIENCY,
                                        _ => DEFAULT_EFFICIENCY,
                                    };
                                    let tech_improvement = match gen.get_generator_type() {
                                        GeneratorType::OnshoreWind | GeneratorType::OffshoreWind |
                                        GeneratorType::UtilitySolar => DEVELOPING_TECH_IMPROVEMENT_RATE,
                                        GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => EMERGING_TECH_IMPROVEMENT_RATE,
                                        _ => MATURE_TECH_IMPROVEMENT_RATE,
                                    }.powi((year - BASE_YEAR) as i32);
                                    let max_efficiency = base_max * (1.0 + (1.0 - tech_improvement));
                                    let upgrade_cost = gen.get_current_cost(*year) * (max_efficiency - gen.get_efficiency()) * 2.0;
                                    
                                    (
                                        String::from("Upgrade Efficiency"),
                                        gen.get_generator_type().to_string(),
                                        upgrade_cost,            // capital cost
                                        gen.get_current_operating_cost(*year), // operating cost
                                        gen.get_coordinate().x,             // location_x
                                        gen.get_coordinate().y,             // location_y
                                        gen.get_generator_type().to_string(), // generator type
                                        gen.get_current_power_output(None), // power output
                                        max_efficiency,          // new efficiency
                                        gen.get_co2_output(),      // co2 output
                                        gen.get_operation_percentage() as i32, // operation percentage
                                        gen.eol,                 // lifespan
                                        String::from("Previous Efficiency"), // previous state
                                        format!("Upgraded efficiency from {:.1}% to {:.1}%",
                                            gen.get_efficiency() * 100.0, max_efficiency * 100.0) // impact
                                    )
                                } else {
                                    continue; // Skip if generator not found
                                }
                            },
                            GridAction::AdjustOperation(id, percentage) => {
                                let generator = base_map.get_generators().iter().find(|g| g.get_id() == id);
                                if let Some(gen) = generator {
                                    (
                                        String::from("Adjust Operation"),
                                        gen.get_generator_type().to_string(),
                                        0.0,                     // capital cost (no cost for adjustment)
                                        gen.get_current_operating_cost(*year), // operating cost
                                        gen.get_coordinate().x,             // location_x
                                        gen.get_coordinate().y,             // location_y
                                        gen.get_generator_type().to_string(), // generator type
                                        gen.get_current_power_output(None), // power output
                                        gen.get_efficiency(),    // efficiency
                                        gen.get_co2_output(),    // co2 output
                                        *percentage as i32,      // new operation percentage
                                        gen.eol,                 // lifespan
                                        String::from("Previous Operation"), // previous state
                                        format!("Adjusted operation from {}% to {}%",
                                            gen.get_operation_percentage(), percentage) // impact
                                    )
                                } else {
                                    continue; // Skip if generator not found
                                }
                            },
                            GridAction::AddCarbonOffset(offset_type, cost_multiplier) => {
                                // Use the offset type directly
                                let base_cost = match offset_type {
                                    CarbonOffsetType::Forest => FOREST_BASE_COST,
                                    CarbonOffsetType::Wetland => WETLAND_BASE_COST,
                                    CarbonOffsetType::ActiveCapture => ACTIVE_CAPTURE_BASE_COST,
                                    CarbonOffsetType::CarbonCredit => CARBON_CREDIT_BASE_COST,
                                };
                                
                                // Apply cost multiplier
                                let cost = base_cost * (*cost_multiplier as f64 / 100.0);
                                let operating_cost = match offset_type {
                                    CarbonOffsetType::Forest => FOREST_OPERATING_COST,
                                    CarbonOffsetType::Wetland => WETLAND_OPERATING_COST,
                                    CarbonOffsetType::ActiveCapture => ACTIVE_CAPTURE_OPERATING_COST,
                                    CarbonOffsetType::CarbonCredit => CARBON_CREDIT_OPERATING_COST,
                                };
                                
                                // Use a default location since we don't know exact placement yet
                                (
                                    String::from("Add Carbon Offset"),
                                    offset_type.to_string(),
                                    cost,                    // capital cost
                                    operating_cost,          // operating cost
                                    0.0,                     // location_x (not known yet)
                                    0.0,                     // location_y (not known yet)
                                    String::from("Carbon Offset"), // type
                                    0.0,                     // No power output
                                    0.85,                    // Default efficiency (as seen in actions.rs)
                                    0.0,                     // No direct CO2 output (it reduces CO2)
                                    100,                     // Always fully operational
                                    30,                      // Standard offset lifespan
                                    String::from("New Offset"), // previous state
                                    format!("Added new {} carbon offset", offset_type.to_string()) // impact
                                )
                            },
                            GridAction::CloseGenerator(id) => {
                                // Find the generator in the map
                                if let Some(gen) = base_map.get_generators().iter().find(|g| g.get_id() == id) {
                                    (
                                        String::from("CloseGenerator"),
                                        gen.get_generator_type().to_string(),
                                        gen.decommission_cost,    // capital cost (decommission cost)
                                        0.0,                      // operating cost (none after closure)
                                        gen.get_coordinate().x,   // location_x
                                        gen.get_coordinate().y,   // location_y
                                        gen.get_generator_type().to_string(), // generator type
                                        0.0,                      // power output (closed)
                                        0.0,                      // efficiency (closed)
                                        0.0,                      // co2 output (closed)
                                        0,                        // operation percentage (closed)
                                        gen.eol,                  // lifespan
                                        String::from("Previous Operation"), // previous state
                                        format!("Closed generator {}", gen.get_id()) // impact
                                    )
                                } else {
                                    continue; // Skip if generator not found
                                }
                            },
                            GridAction::DoNothing => {
                                (
                                    String::from("Do Nothing"),
                                    String::new(), // no details
                                    0.0,             // capital cost
                                    0.0,             // operating cost
                                    0.0,             // location_x
                                    0.0,             // location_y
                                    String::new(),   // generator type
                                    0.0,             // power output
                                    0.0,             // efficiency
                                    0.0,             // co2 output
                                    0,               // operation percentage
                                    0,               // lifespan
                                    String::new(),   // previous state
                                    String::new()    // impact description
                                )
                            },
                        };
                         
                        actions_file.write_all(format!(
                            "{},{:?},\"{}\",{:.2},{:.2},{:.1},{:.1},\"{}\",{:.2},{:.3},{:.2},{},{},\"{}\",\"{}\"\n",
                            year,
                            action_type,
                            details,
                            capital_cost,
                            operating_cost,
                            location_x,
                            location_y,
                            gen_type,
                            power_output,
                            efficiency,
                            co2_output,
                            operation_percentage,
                            lifespan,
                            prev_state,
                            impact
                        ).as_bytes())?;
                    }
                     
                    println!("Basic action history saved to: {}", actions_filename.display());
                }
            } else {
                println!("\nCSV export is disabled. Use --enable-csv-export to generate detailed CSV files.");
                
                // Create a minimal results file with basic info even when CSV export is disabled
                let csv_filename = Path::new(&run_dir).join("simulation_summary.csv");
                let mut file = File::create(&csv_filename)?;
                file.write_all(format!("Simulation Summary\n").as_bytes())?;
                file.write_all(format!("Final Net Emissions (tonnes CO2),{}\n", best.metrics.final_net_emissions).as_bytes())?;
                file.write_all(format!("Average Public Opinion (%),{:.2}\n", best.metrics.average_public_opinion * 100.0).as_bytes())?;
                file.write_all(format!("Total Cost (€),{:.2}\n", best.metrics.total_cost).as_bytes())?;
                file.write_all(format!("Power Reliability (%),{:.2}\n", best.metrics.power_reliability * 100.0).as_bytes())?;
                println!("Basic simulation summary saved to: {}", csv_filename.display());
            }
            
            // Save final weights in the run directory
            let final_weights_path = Path::new(&run_dir).join("best_weights.json");
            let weights = action_weights.write();
            weights.save_to_file(final_weights_path.to_str().unwrap())?;
            println!("Final weights saved to: {}", final_weights_path.display());
            
            // After all iterations, check if we need to run additional full simulations
            if force_full_simulation {
                let full_sim_data = full_sim_tracking.lock().unwrap();
                
                // Get the current best score from the weights
                let best_score = {
                    if let Some(best_metrics) = weights.get_simulation_metrics() {
                        crate::ai::score_metrics(best_metrics, optimization_mode)
                    } else {
                        // If no best metrics yet, use a default score of 1.0
                        1.0
                    }
                };
                
                // If we're not within threshold, prompt the user to run more full simulations
                let percent_of_best = (full_sim_data.best_score / best_score) * 100.0;
                let percent_diff: f64 = 100.0 - percent_of_best;
                
                if percent_diff.abs() > FULL_SIM_THRESHOLD_PERCENT && full_sim_data.count > 0 {
                    println!("\nFull simulation results are not within {}% of best score.", FULL_SIM_THRESHOLD_PERCENT);
                    println!("Current difference: {:.2}%", percent_diff.abs());
                    
                    // Prompt user to run additional full simulations
                    let continue_sims = prompt_continue_full_simulations(best_score, full_sim_data.best_score);
                    
                    if continue_sims {
                        println!("\nRunning additional full simulations...");
                        
                        // Release the lock before running additional simulations
                        drop(full_sim_data);
                        
                        // Run additional batch of full simulations
                        let additional_iterations = FULL_SIM_INTERVAL;
                        println!("Running {} additional full simulations...", additional_iterations);
                        
                        // Create a new run directory for the additional simulations
                        let additional_run_dir = format!("{}/{}_additional", checkpoint_dir, Local::now().format("%Y%m%d_%H%M%S"));
                        std::fs::create_dir_all(&additional_run_dir)?;
                        
                        // Clone the base map for the additional simulations
                        let base_map_clone = base_map.clone();
                        
                        // Run the additional simulations with force_full_simulation set to true
                        return run_multi_simulation(
                            &base_map_clone,
                            additional_iterations,
                            parallel,
                            true, // continue from checkpoint
                            checkpoint_dir,
                            checkpoint_interval,
                            progress_interval,
                            cache_dir,
                            true, // force full simulation
                            seed,
                            verbose_logging,
                            optimization_mode,
                            enable_energy_sales,
                            enable_csv_export,
                            debug_weights,
                            enable_construction_delays,
                            track_weight_history,
                        );
                    }
                }
            }
        }
         
        Ok(())
    })();
     
    // Print final timing report
    if logging::is_timing_enabled() {
        logging::print_timing_report();
    }
     
    result
}