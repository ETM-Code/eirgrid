use std::error::Error;
use clap::Parser;
use eirgrid::map_handler::Map;
use eirgrid::simulation_config::SimulationConfig;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = 0.3)]
    min_suitability: f64,

    #[arg(short, long, default_value = "location_analysis.txt")]
    output_file: String,

    #[arg(short, long, default_value = "cache")]
    cache_dir: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Initialize map with default configuration
    let config = SimulationConfig::default();
    let map = Map::new(config);

    println!("Starting location analysis...");
    println!("Minimum suitability threshold: {}", args.min_suitability);

    // Perform analysis using the map's analyze_locations method
    let analysis = map.analyze_locations(args.min_suitability);

    // Print summary to console
    analysis.print_summary();

    // Save detailed results to file
    println!("\nSaving detailed results to {}...", args.output_file);
    analysis.save_to_file(&args.output_file)?;

    // Save cache for fast simulation
    println!("Saving location analysis cache to {}...", args.cache_dir);
    analysis.save_cache(&args.cache_dir)?;

    println!("Analysis complete!");

    Ok(())
} 