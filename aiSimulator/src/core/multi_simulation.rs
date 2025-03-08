use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::path::Path;
use std::fs::File;
use std::io::Write;
use std::time::{Duration, Instant};
use rayon::prelude::*;
use parking_lot::{self, RwLock};
use crate::utils::map_handler::Map;
use crate::core::action_weights::ActionWeights;
use crate::core::action_weights::{GridAction, evaluate_action_impact};
use crate::analysis::metrics::SimulationResult;
use crate::core::iteration::run_iteration;
use crate::utils::logging;
use crate::utils::logging::OperationCategory;
use crate::data::poi::POI;
use crate::config::constants::{MAX_ACCEPTABLE_EMISSIONS, MAX_ACCEPTABLE_COST};
use crate::config::constants::{DEVELOPING_TECH_IMPROVEMENT_RATE, EMERGING_TECH_IMPROVEMENT_RATE, MATURE_TECH_IMPROVEMENT_RATE, BASE_YEAR};
use crate::models::generator::GeneratorType;
use crate::models::carbon_offset::CarbonOffsetType;
use crate::core::actions::apply_action;
use chrono::Local;
use crate::utils::csv_export;

const FULL_RUN_PERCENTAGE: usize = 10;
const REPLAY_BEST_STRATEGY_IN_FULL_RUNS: bool = true;

// Import metrics_to_action_result from main module as it's defined there
fn metrics_to_action_result(metrics: &crate::core::action_weights::SimulationMetrics) -> crate::core::action_weights::ActionResult {
    crate::core::action_weights::ActionResult {
        net_emissions: metrics.final_net_emissions,
        public_opinion: metrics.average_public_opinion,
        power_balance: 0.0, // Not directly available in SimulationMetrics
        total_cost: metrics.total_cost,
    }
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
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let _timing = logging::start_timing("run_multi_simulation", OperationCategory::Simulation);
     
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

        // Create timestamp directory after loading weights
        let now = Local::now();
        let timestamp = format!("2024{}", now.format("%m%d_%H%M%S"));
        let run_dir = format!("{}/{}", checkpoint_dir, timestamp);
        std::fs::create_dir_all(&run_dir)?;
         
        // Create shared weights
        let action_weights = Arc::new(RwLock::new(initial_weights));
         
        // Spawn progress monitoring thread
        let progress_counter = completed_iterations.clone();
        let total_iterations = num_iterations;
        let action_weights_for_progress = Arc::clone(&action_weights);
         
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
                     
                    if total_completed == num_iterations.saturating_sub(final_full_sim_count) {
                        println!("\nSwitching to full simulation mode for final {} iterations ({:.1}% of total)",
                            final_full_sim_count, FULL_RUN_PERCENTAGE as f64);
                    }
                     
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
                    if replay_best_strategy {
                        println!("🔁 Iteration {} is replaying the best strategy for thorough analysis", i + 1);
                    }
                     
                    let result = run_iteration(i, &mut map_clone, &mut local_weights, replay_best_strategy, seed, verbose_logging, optimization_mode, enable_energy_sales)?;
                     
                    // Update best metrics immediately - changed order to transfer actions first
                    {
                        let mut weights = parking_lot::RwLock::write(&*action_weights);
                        // First transfer weights and recorded actions from the local instance
                        weights.update_weights_from(&local_weights);
                        // Then update best strategy after we have the actions
                        weights.update_best_strategy(result.metrics.clone());
                    }
                     
                    // Increment completed iterations counter
                    completed_iterations.fetch_add(1, Ordering::Relaxed);
                     
                    // Save checkpoint at intervals
                    if (i + 1) % checkpoint_interval == 0 {
                        let thread_id = rayon::current_thread_index().unwrap_or(0);
                        let weights = parking_lot::RwLock::write(&*action_weights);
                         
                        // Save thread-specific weights
                        let thread_weights_path = Path::new(&run_dir)
                            .join(format!("thread_{}_weights.json", thread_id));
                        local_weights.save_to_file(thread_weights_path.to_str().unwrap())?;
                         
                        // Save shared weights
                        let checkpoint_path = Path::new(&run_dir).join("latest_weights.json");
                        weights.save_to_file(checkpoint_path.to_str().unwrap())?;
                         
                        // Save iteration number
                        let iteration_path = Path::new(&run_dir).join("checkpoint_iteration.txt");
                        std::fs::write(iteration_path, (i + 1).to_string())?;
                         
                        println!("Saved checkpoint at iteration {} in {} (thread {})", i + 1, run_dir, thread_id);
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
            for i in start_iteration..num_iterations {
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
                 
                if total_completed == num_iterations.saturating_sub(final_full_sim_count) {
                    println!("\nSwitching to full simulation mode for final {} iterations ({:.1}% of total)",
                        final_full_sim_count, FULL_RUN_PERCENTAGE as f64);
                }
                 
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
                if replay_best_strategy {
                    println!("🔁 Iteration {} is replaying the best strategy for thorough analysis", i + 1);
                }
                 
                let result = run_iteration(i, &mut map_clone, &mut local_weights, replay_best_strategy, seed, verbose_logging, optimization_mode, enable_energy_sales)?;
                 
                // Update best metrics immediately - changed order to transfer actions first
                {
                    let mut weights = parking_lot::RwLock::write(&*action_weights);
                    // First transfer weights and recorded actions from the local instance
                    weights.update_weights_from(&local_weights);
                    // Then update best strategy after we have the actions
                    weights.update_best_strategy(result.metrics.clone());
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
                        let weights = parking_lot::RwLock::write(&*action_weights);
                        weights.save_to_file(checkpoint_path.to_str().unwrap())?;
                    }
                     
                    // Save iteration number
                    let iteration_path = Path::new(&run_dir).join("checkpoint_iteration.txt");
                    std::fs::write(iteration_path, (i + 1).to_string())?;
                     
                    println!("Saved checkpoint at iteration {} in {}", i + 1, run_dir);
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
             
            // Create a CSV exporter instance
            let _timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
            let csv_exporter = csv_export::CsvExporter::new(&csv_export_dir);
             
            // Create a new map and apply all the best actions to it
            let mut final_map = base_map.clone();
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
                        GridAction::AddGenerator(gen_type) => {
                            let location = if let Some(gen) = base_map.get_generators().iter()
                                .find(|g| g.get_id().contains(&format!("{}_{}", gen_type.to_string(), year))) {
                                (gen.coordinate.x, gen.coordinate.y)
                            } else {
                                (0.0, 0.0) // Default if not found - fixed tuple type
                            };
                            (
                                "Add Generator",
                                gen_type.to_string(),
                                gen_type.get_base_cost(*year),
                                gen_type.get_operating_cost(*year),
                                location.0,
                                location.1,
                                gen_type.to_string(),
                                gen_type.get_base_power(*year),
                                gen_type.get_base_efficiency(*year),
                                match gen_type {
                                    GeneratorType::CoalPlant => 1000.0,
                                    GeneratorType::GasCombinedCycle => 500.0,
                                    GeneratorType::GasPeaker => 700.0,
                                    GeneratorType::Biomass => 50.0,
                                    _ => 0.0,
                                },
                                100, // New generators start at 100%
                                gen_type.get_lifespan(),
                                "New Installation".to_string(),
                                format!("Added new {} power generation capacity", gen_type.to_string())
                            )
                        },
                        GridAction::UpgradeEfficiency(id) => {
                            let generator = base_map.get_generators().iter().find(|g| g.get_id() == id);
                            if let Some(gen) = generator {
                                let base_max = match gen.get_generator_type() {
                                    GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => 0.45,
                                    GeneratorType::UtilitySolar => 0.40,
                                    GeneratorType::Nuclear => 0.50,
                                    GeneratorType::GasCombinedCycle => 0.60,
                                    GeneratorType::HydroDam | GeneratorType::PumpedStorage => 0.85,
                                    GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => 0.35,
                                    _ => 0.40,
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
                                    "Upgrade Efficiency",
                                    id.clone(),
                                    upgrade_cost,
                                    0.0,
                                    gen.coordinate.x,
                                    gen.coordinate.y,
                                    gen.get_generator_type().to_string(),
                                    gen.power_out,
                                    max_efficiency,
                                    gen.co2_out as f64,
                                    gen.get_operation_percentage() as i32, // Use the generator's operation percentage
                                    gen.eol,
                                    "Previous Efficiency".to_string(),
                                    format!("Upgraded efficiency from {:.1}% to {:.1}%",
                                        gen.get_efficiency() * 100.0, max_efficiency * 100.0)
                                )
                            } else {
                                continue; // Skip if generator not found
                            }
                        },
                        GridAction::AdjustOperation(id, percentage) => {
                            let generator = base_map.get_generators().iter().find(|g| g.get_id() == id);
                            if let Some(gen) = generator {
                                (
                                    "Adjust Operation",
                                    format!("{} to {}%", id, percentage),
                                    0.0,
                                    0.0,
                                    gen.coordinate.x,
                                    gen.coordinate.y,
                                    gen.get_generator_type().to_string(),
                                    gen.power_out,
                                    gen.get_efficiency(),
                                    gen.co2_out as f64,
                                    gen.get_operation_percentage() as i32, // Use the generator's operation percentage
                                    gen.eol,
                                    "Previous Operation".to_string(),
                                    format!("Adjusted operation from {}% to {}%",
                                        gen.get_operation_percentage(), percentage)
                                )
                            } else {
                                continue; // Skip if generator not found
                            }
                        },
                        GridAction::AddCarbonOffset(offset_type) => {
                            // Simple string to enum conversion
                            let offset_type = match offset_type.as_str() {
                                "Forest" => CarbonOffsetType::Forest,
                                "Wetland" => CarbonOffsetType::Wetland,
                                "ActiveCapture" => CarbonOffsetType::ActiveCapture,
                                "CarbonCredit" => CarbonOffsetType::CarbonCredit,
                                _ => CarbonOffsetType::Forest, // Default
                            };
                            let base_cost = match offset_type {
                                CarbonOffsetType::Forest => 10_000_000.0,
                                CarbonOffsetType::Wetland => 15_000_000.0,
                                CarbonOffsetType::ActiveCapture => 100_000_000.0,
                                CarbonOffsetType::CarbonCredit => 5_000_000.0,
                            };
                            let operating_cost = match offset_type {
                                CarbonOffsetType::Forest => 500_000.0,
                                CarbonOffsetType::Wetland => 750_000.0,
                                CarbonOffsetType::ActiveCapture => 5_000_000.0,
                                CarbonOffsetType::CarbonCredit => 250_000.0,
                            };
                            // Find the offset in the map to get its location
                            let offset = base_map.get_carbon_offsets().iter()
                                .find(|o| o.get_id().contains(&format!("{}_{}", offset_type.to_string(), year)));
                            let location = if let Some(o) = offset {
                                (o.get_coordinate().x, o.get_coordinate().y)
                            } else {
                                (0.0, 0.0)
                            };
                            (
                                "Add Carbon Offset",
                                offset_type.to_string(),
                                base_cost,
                                operating_cost,
                                location.0,
                                location.1,
                                "Carbon Offset".to_string(),
                                0.0, // No power output
                                0.0, // No efficiency
                                0.0, // No direct CO2 output
                                100, // Always fully operational
                                30, // Standard offset lifespan
                                "New Offset".to_string(),
                                format!("Added new {} carbon offset project", offset_type.to_string())
                            )
                        },
                        GridAction::CloseGenerator(id) => {
                            let generator = base_map.get_generators().iter().find(|g| g.get_id() == id);
                            if let Some(gen) = generator {
                                let years_remaining = (gen.eol as i32 - (year - 2025) as i32).max(0) as f64;
                                let closure_cost = gen.get_current_cost(*year) * 0.5 * (years_remaining / gen.eol as f64);
                                (
                                    "Close Generator",
                                    id.clone(),
                                    closure_cost,
                                    0.0,
                                    gen.coordinate.x,
                                    gen.coordinate.y,
                                    gen.get_generator_type().to_string(),
                                    gen.power_out,
                                    gen.get_efficiency(),
                                    gen.co2_out as f64,
                                    0i32, // Closed generators have 0% operation
                                    gen.eol,
                                    "Previously Active".to_string(),
                                    format!("Closed {} generator after {} years of operation",
                                        gen.get_generator_type().to_string(), year - gen.commissioning_year)
                                )
                            } else {
                                continue; // Skip if generator not found
                            }
                        },
                        GridAction::DoNothing => {
                            (
                                "Do Nothing",
                                "".to_string(), // no details
                                0.0,             // capital cost
                                0.0,             // operating cost
                                0.0,             // location_x
                                0.0,             // location_y
                                "".to_string(), // generator type
                                0.0,             // power output
                                0.0,             // efficiency
                                0.0,             // co2 output
                                0,              // operation percentage
                                0,              // lifespan
                                "".to_string(), // previous state
                                "".to_string()  // impact description
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
             
            // Save final weights in the run directory
            let final_weights_path = Path::new(&run_dir).join("best_weights.json");
            let weights = parking_lot::RwLock::write(&*action_weights);
            weights.save_to_file(final_weights_path.to_str().unwrap())?;
            println!("Final weights saved to: {}", final_weights_path.display());
        }
         
        Ok(())
    })();
     
    // Print final timing report
    if logging::is_timing_enabled() {
        logging::print_timing_report();
    }
     
    result
}