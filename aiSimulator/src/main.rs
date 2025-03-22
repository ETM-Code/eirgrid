#[macro_use]
extern crate lazy_static;

use std::error::Error;

use clap::Parser;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

// Import using updated module structure
use eirgrid::core::multi_simulation::run_multi_simulation;
use eirgrid::core::action_weights::{ SimulationMetrics, ActionResult};

use eirgrid::models::generator::{Generator, GeneratorType};
use eirgrid::models::settlement::Settlement;

use eirgrid::config::simulation_config::SimulationConfig;

use eirgrid::data::settlements_loader;
use eirgrid::data::generators_loader;
use eirgrid::data::poi::Coordinate;

use eirgrid::utils::map_handler::Map;
use eirgrid::utils::logging::{self, OperationCategory, FileIOType};
use eirgrid::cli::cli::Args;

// Constants
const SIMULATION_START_YEAR: u32 = 2025;
const SIMULATION_END_YEAR: u32 = 2050;

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Parse command line arguments
    let args = Args::parse();
     
    // Initialize logging with timing and debug logging parameters
    logging::init_logging(args.enable_timing(), args.debug_logging());
     
    println!("EirGrid Power System Simulator (2025-2050)");
    println!("Debug logging: {}, CSV export: {}, Weights debugging: {}", 
             if args.debug_logging() { "enabled" } else { "disabled" },
             if args.enable_csv_export() { "enabled" } else { "disabled" },
             if args.debug_weights() { "enabled" } else { "disabled" });
     
    let config = SimulationConfig::default();
    let mut map = Map::new(config);
     
    // Initialize the map, now with seed support
    initialize_map(&mut map, args.seed());
     
    run_multi_simulation(
        &map,
        args.iterations(),
        args.parallel(),
        !args.no_continue(),
        args.checkpoint_dir(),
        args.checkpoint_interval(),
        args.progress_interval(),
        args.cache_dir(),
        args.force_full_simulation(),
        args.seed(),
        args.verbose_state_logging(),
        if args.cost_only() { Some("cost_only") } else { None },
        args.enable_energy_sales(),
        args.enable_csv_export(),
        args.debug_weights(),
        args.enable_construction_delays(),
        args.track_weight_history(),
    )?;

    Ok(())
}

// Modified to accept a seed parameter
fn initialize_map(map: &mut Map, seed: Option<u64>) {
    let _timing = logging::start_timing("initialize_map",
        OperationCategory::FileIO { subcategory: FileIOType::DataLoad });
     
    // Create a deterministic RNG if seed is provided
    let mut seeded_rng = seed.map(StdRng::seed_from_u64);
     
    // Load settlements
    match settlements_loader::load_settlements("aiSimulator/assets/settlements.json", SIMULATION_START_YEAR) {
        Ok(settlements) => {
            for settlement in settlements {
                map.add_settlement(settlement);
            }
        },
        Err(e) => {
            eprintln!("Failed to load settlements from JSON: {}. Using fallback settlements.", e);
            map.add_settlement(Settlement::new(
                "Dublin".to_string(),
                Coordinate::new(70000.0, 70000.0),
                1_200_000,
                2000.0,
            ));
            map.add_settlement(Settlement::new(
                "Cork".to_string(),
                Coordinate::new(50000.0, 30000.0),
                190_000,
                350.0,
            ));
            map.add_settlement(Settlement::new(
                "Galway".to_string(),
                Coordinate::new(20000.0, 60000.0),
                80_000,
                150.0,
            ));
            map.add_settlement(Settlement::new(
                "Limerick".to_string(),
                Coordinate::new(30000.0, 40000.0),
                94_000,
                180.0,
            ));
        }
    }
     
    // Load existing generators from CSV, with deterministic fallbacks if needed
    match generators_loader::load_generators("aiSimulator/assets/ireland_generators.csv", SIMULATION_START_YEAR) {
        Ok(loaded_generators) => {
            let num_generators = loaded_generators.len();
            for generator in loaded_generators {
                map.add_generator(generator.clone());  // Clone each generator before adding
            }
            println!("Successfully loaded {} generators from CSV", num_generators);
        },
        Err(e) => {
            eprintln!("Failed to load generators from CSV: {}. Using fallback generators.", e);
             
            // When using a seed, we can generate deterministic locations instead of fixed ones
            if let Some(rng) = &mut seeded_rng {
                // Use seeded RNG for deterministic but varied placement
                let x1 = rng.gen_range(20000.0..40000.0);
                let y1 = rng.gen_range(40000.0..60000.0);
                 
                map.add_generator(Generator::new(
                    "Moneypoint".to_string(),
                    Coordinate::new(x1, y1),
                    GeneratorType::CoalPlant,
                    800_000_000.0,
                    915.0,
                    50_000_000.0,
                    40,
                    1.0,
                    2_000_000.0,
                    0.37,
                ));
                 
                let x2 = rng.gen_range(65000.0..75000.0);
                let y2 = rng.gen_range(65000.0..75000.0);
                 
                map.add_generator(Generator::new(
                    "Dublin Bay".to_string(),
                    Coordinate::new(x2, y2),
                    GeneratorType::GasCombinedCycle,
                    400_000_000.0,
                    415.0,
                    20_000_000.0,
                    30,
                    0.8,
                    800_000.0,
                    0.45,
                ));
            } else {
                // No seed, use fixed positions
                map.add_generator(Generator::new(
                    "Moneypoint".to_string(),
                    Coordinate::new(30000.0, 50000.0),
                    GeneratorType::CoalPlant,
                    800_000_000.0,
                    915.0,
                    50_000_000.0,
                    40,
                    1.0,
                    2_000_000.0,
                    0.37,
                ));
                 
                map.add_generator(Generator::new(
                    "Dublin Bay".to_string(),
                    Coordinate::new(72000.0, 72000.0),
                    GeneratorType::GasCombinedCycle,
                    400_000_000.0,
                    415.0,
                    20_000_000.0,
                    30,
                    0.8,
                    800_000.0,
                    0.45,
                ));
            }
        }
    }
}

// Fix the helper function for converting SimulationMetrics to ActionResult
fn metrics_to_action_result(metrics: &SimulationMetrics) -> ActionResult {
    ActionResult {
        net_emissions: metrics.final_net_emissions,
        public_opinion: metrics.average_public_opinion,
        power_balance: 0.0, // Not directly available in SimulationMetrics
        total_cost: metrics.total_cost
    }
}
