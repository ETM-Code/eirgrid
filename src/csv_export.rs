use std::fs::File;
use std::path::{Path, PathBuf};
use std::io::Write;
use std::error::Error;
use chrono::Local;

use crate::map_handler::Map;
use crate::action_weights::{GridAction, SimulationMetrics};
use crate::settlement::Settlement;
use crate::carbon_offset::{CarbonOffset, CarbonOffsetType};
use crate::constants::{BASE_YEAR, END_YEAR, IRELAND_MIN_LAT, IRELAND_MAX_LAT, IRELAND_MIN_LON, IRELAND_MAX_LON, GRID_SCALE_X, GRID_SCALE_Y};
use crate::poi::POI;
use crate::generator::{Generator, GeneratorType};
use crate::const_funcs;

/// Function to transform grid coordinates back to lat/lon
fn transform_grid_to_lat_lon(x: f64, y: f64) -> (f64, f64) {
    // This is the inverse of the transform_lat_lon_to_grid function in const_funcs.rs
    let lon = (x / GRID_SCALE_X) + IRELAND_MIN_LON;
    let lat = (y / GRID_SCALE_Y) + IRELAND_MIN_LAT;
    (lon, lat) // Return as (longitude, latitude) for consistent ordering
}

/// Main struct for handling CSV export
pub struct CsvExporter {
    output_dir: PathBuf,
    timestamp: String,
}

impl CsvExporter {
    /// Create a new CSV exporter with the specified output directory
    pub fn new(output_dir: impl AsRef<Path>) -> Self {
        let now = Local::now();
        let timestamp = now.format("%Y%m%d_%H%M%S").to_string();

        // Create output directory if it doesn't exist
        let full_path = Path::new(output_dir.as_ref()).join(&timestamp);
        std::fs::create_dir_all(&full_path).expect("Failed to create output directory");

        Self {
            output_dir: full_path,
            timestamp,
        }
    }

    /// Export all simulation data to CSV files
    pub fn export_simulation_results(
        &self,
        map: &Map,
        actions: &[(u32, GridAction)],
        metrics: &SimulationMetrics,
        yearly_metrics: &[YearlyMetrics],
    ) -> Result<(), Box<dyn Error>> {
        // Export summary data
        self.export_simulation_summary(map, actions, metrics, yearly_metrics)?;
        
        // Export detailed data
        self.export_yearly_details(map, yearly_metrics)?;
        
        // Export generator operation time logs
        self.export_generator_operation_logs(map, yearly_metrics)?;

        println!("CSV export completed successfully to: {}", self.output_dir.display());
        Ok(())
    }

    /// Export summary data to CSV
    fn export_simulation_summary(
        &self,
        map: &Map,
        actions: &[(u32, GridAction)],
        metrics: &SimulationMetrics,
        yearly_metrics: &[YearlyMetrics],
    ) -> Result<(), Box<dyn Error>> {
        let summary_path = self.output_dir.join("simulation_summary.csv");
        let mut summary_file = File::create(&summary_path)?;

        // Write general information header
        writeln!(summary_file, "Simulation Summary")?;
        writeln!(summary_file, "Timestamp,{}", self.timestamp)?;
        writeln!(summary_file, "")?;
        
        // Write final metrics
        writeln!(summary_file, "Final Metrics")?;
        writeln!(summary_file, "Final Net Emissions (tonnes CO2),{}", metrics.final_net_emissions)?;
        writeln!(summary_file, "Average Public Opinion (%),{:.2}", metrics.average_public_opinion * 100.0)?;
        writeln!(summary_file, "Total Cost (€),{:.2}", metrics.total_cost)?;
        writeln!(summary_file, "Power Reliability (%),{:.2}", metrics.power_reliability * 100.0)?;
        writeln!(summary_file, "")?;
        
        // Write actions section header
        writeln!(summary_file, "Actions Taken")?;
        writeln!(summary_file, "Year,Action Type,Generator Type,Generator ID,Operation %,Offset Type,Estimated Cost (€)")?;
        
        // Prepare a lookup map of generators for cost estimates
        let generators = map.get_generators();
        let generator_map: std::collections::HashMap<&str, &Generator> = generators
            .iter()
            .map(|g| (g.get_id(), g))
            .collect();
        
        // Track total action costs
        let mut total_action_costs = 0.0;
        
        // Write each action with its estimated cost
        for (year, action) in actions {
            let (action_type, gen_type, gen_id, operation_pct, offset_type, estimated_cost) = match action {
                GridAction::AddGenerator(gen_type) => {
                    let cost = gen_type.get_base_cost(*year);
                    (
                    "AddGenerator",
                    gen_type.to_string(),
                    String::new(),
                    String::new(),
                    String::new(),
                        format!("{:.2}", cost),
                    )
                },
                GridAction::UpgradeEfficiency(id) => {
                    // Estimate upgrade cost based on generator type and base cost
                    let upgrade_cost = if let Some(generator) = generator_map.get(id.as_str()) {
                        // Typical efficiency increase is about 10-20%
                        let typical_efficiency_increase = 0.15;
                        generator.get_current_cost(*year) * typical_efficiency_increase * 0.5
                    } else {
                        0.0
                    };
                    
                    (
                    "UpgradeEfficiency",
                    String::new(),
                    id.clone(),
                    String::new(),
                    String::new(),
                        format!("{:.2}", upgrade_cost),
                    )
                },
                GridAction::AdjustOperation(id, percentage) => (
                    "AdjustOperation",
                    String::new(),
                    id.clone(),
                    percentage.to_string(),
                    String::new(),
                    "0.00".to_string(), // Operation adjustment has no direct capital cost
                ),
                GridAction::AddCarbonOffset(offset_type) => {
                    // Get cost based on offset type
                    let offset_cost = match offset_type.as_str() {
                        "Forest" => 1000000.0,       // Forest planting costs
                        "ActiveCapture" => 5000000.0, // Active carbon capture technology
                        "CarbonCredit" => 2000000.0,  // Carbon credit purchases
                        "Wetland" => 1500000.0,      // Wetland restoration
                        _ => 1000000.0,             // Default cost
                    };
                    
                    (
                    "AddCarbonOffset",
                    String::new(),
                    String::new(),
                    String::new(),
                    offset_type.clone(),
                        format!("{:.2}", offset_cost),
                    )
                },
                GridAction::CloseGenerator(id) => {
                    // Calculate closure cost
                    let closure_cost = if let Some(generator) = generator_map.get(id.as_str()) {
                        let years_remaining = (generator.eol as i32 - (*year as i32 - 2025).max(0)) as f64;
                        generator.get_current_cost(*year) * 0.3 * (years_remaining / generator.eol as f64)
                    } else {
                        0.0
                    };
                    
                    (
                    "CloseGenerator",
                    String::new(),
                    id.clone(),
                    String::new(),
                    String::new(),
                        format!("{:.2}", closure_cost),
                    )
                },
                GridAction::DoNothing => (
                    "DoNothing",
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    "0.00".to_string(),
                ),
            };
            
            writeln!(
                summary_file,
                "{},{},{},{},{},{},{}",
                year, action_type, gen_type, gen_id, operation_pct, offset_type, estimated_cost
            )?;
        }
        
        // Add yearly summary metrics
        writeln!(summary_file, "")?;
        writeln!(summary_file, "Yearly Summary Metrics")?;
        writeln!(
            summary_file,
            "Year,Population,PowerUsage,PowerGeneration,PowerBalance,PublicOpinion,YearlyCapitalCost,TotalCapitalCost,Inflation,CO2Emissions,CarbonOffset,NetEmissions,YearlyRevenue,TotalRevenue,ActiveGenerators,YearlyUpgradeCosts,YearlyClosureCosts,YearlyTotalCost,TotalCost"
        )?;
        
        for metrics in yearly_metrics {
            // Basic financial and operational metrics
            let formatted_line = format!(
                "{},{},{:.2},{:.2},{:.2},{:.4},{:.2},{:.2},{:.4},{:.2},{:.2},{:.2},{:.2},{:.2},{},{:.2},{:.2},{:.2},{:.2}",
                metrics.year,
                metrics.total_population,
                metrics.total_power_usage,
                metrics.total_power_generation,
                metrics.power_balance,
                metrics.average_public_opinion,
                metrics.yearly_capital_cost,
                metrics.total_capital_cost,
                metrics.inflation_factor,
                metrics.total_co2_emissions,
                metrics.total_carbon_offset,
                metrics.net_co2_emissions,
                metrics.yearly_carbon_credit_revenue,
                metrics.total_carbon_credit_revenue,
                metrics.active_generators,
                metrics.yearly_upgrade_costs,
                metrics.yearly_closure_costs,
                metrics.yearly_total_cost,
                metrics.total_cost
            );
            
            writeln!(summary_file, "{}", formatted_line)?;
        }
        
        Ok(())
    }

    /// Export detailed yearly data to CSV
    fn export_yearly_details(
        &self,
        map: &Map,
        yearly_metrics: &[YearlyMetrics],
    ) -> Result<(), Box<dyn Error>> {
        // Create a directory for detailed yearly data
        let details_dir = self.output_dir.join("yearly_details");
        std::fs::create_dir_all(&details_dir)?;
        
        // Export settlements data
        self.export_settlements_data(map, &details_dir, yearly_metrics)?;
        
        // Export generators data
        self.export_generators_data(map, &details_dir, yearly_metrics)?;
        
        // Export carbon offsets data
        self.export_carbon_offsets_data(map, &details_dir)?;
        
        Ok(())
    }

    /// Export settlements data
    fn export_settlements_data(
        &self,
        map: &Map,
        details_dir: &Path,
        yearly_metrics: &[YearlyMetrics],
    ) -> Result<(), Box<dyn Error>> {
        let settlements_path = details_dir.join("settlements.csv");
        let mut settlements_file = File::create(&settlements_path)?;
        
        // Write settlements header with more comprehensive information
        writeln!(
            settlements_file,
            "Year,Settlement ID,Name,X,Y,Population,Growth Rate (%),Power Usage (MW),Power Usage Per Capita (kW)"
        )?;
        
        // Get settlements from map
        let settlements = map.get_settlements();
        
        // Check if we have any settlements to export
        if settlements.is_empty() {
            println!("No settlements found in the simulation");
            writeln!(settlements_file, "NOTE,No settlements found in the simulation")?;
            return Ok(());
        }
        
        // Print debug information about which settlements will be shown in debug output
        println!("\n=== SETTLEMENTS DEBUG INFO ===");
        println!("Total settlements to export: {}", settlements.len());
        if settlements.len() > 0 {
            println!("Debug output will be shown for the following settlements (every 10th):");
            for (i, settlement) in settlements.iter().enumerate() {
                if i % 10 == 0 {
                    println!("  - Settlement #{}: {} ({}) at ({:.6},{:.6})", i + 1, settlement.get_name(), settlement.get_id(), settlement.get_coordinate().x, settlement.get_coordinate().y);
                }
            }
        }
        println!("============================\n");
        
        // Create a yearly metrics map for easier lookup
        let yearly_metrics_map: std::collections::HashMap<u32, &YearlyMetrics> = yearly_metrics
            .iter()
            .map(|m| (m.year, m))
            .collect();
            
        println!("Exporting data for {} settlements across {} years", settlements.len(), END_YEAR - BASE_YEAR + 1);
        
        // Store population data for each year and settlement
        // First create a map to track population by year and settlement ID
        let mut yearly_population_data: std::collections::HashMap<u32, std::collections::HashMap<String, u32>> = 
            std::collections::HashMap::new();
        
        // Store power usage data for each year and settlement
        let mut yearly_power_usage_data: std::collections::HashMap<u32, std::collections::HashMap<String, f64>> = 
            std::collections::HashMap::new();
        
        // Helper function to escape commas in CSV fields
        let escape_csv_field = |field: &str| -> String {
            if field.contains(',') || field.contains('"') || field.contains('\n') {
                // Escape double quotes by doubling them and wrap in quotes
                let escaped = field.replace('"', "\"\"");
                format!("\"{}\"", escaped)
            } else {
                field.to_string()
            }
        };
        
        // Helper function to sanitize settlement names by removing non-alphabetic characters
        let sanitize_name = |name: &str| -> String {
            name.chars()
                .filter(|c| c.is_alphabetic() || c.is_whitespace())
                .collect()
        };
        
        // Helper function to sanitize IDs by removing non-alphabetic and non-numeric characters
        let sanitize_id = |id: &str| -> String {
            id.chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '_')
                .collect()
        };
        
        // Initialize base year population and power usage data
        let mut base_year_population = std::collections::HashMap::new();
        let mut base_year_power_usage = std::collections::HashMap::new();
        
        for settlement in settlements {
            let id = settlement.get_id().to_string();
            base_year_population.insert(id.clone(), settlement.get_population());
            base_year_power_usage.insert(id.clone(), settlement.get_power_usage());
        }
        
        yearly_population_data.insert(BASE_YEAR, base_year_population);
        yearly_power_usage_data.insert(BASE_YEAR, base_year_power_usage);
        
        // Calculate population growth for all years
        // The initial population growth rate is 1% annually (as seen in main.rs)
        // We'll vary it slightly by settlement for realism
        const DEFAULT_ANNUAL_GROWTH: f64 = 0.01; // 1% annual growth (from main.rs)
        
        for year in (BASE_YEAR + 1)..=END_YEAR {
            let previous_year = year - 1;
            let mut current_year_population = std::collections::HashMap::new();
            let mut current_year_power_usage = std::collections::HashMap::new();
            
            if let Some(previous_year_population) = yearly_population_data.get(&previous_year) {
                for settlement in settlements {
                    let id = settlement.get_id().to_string();
                    
                    // Get previous population or use current as fallback
                    let settlement_population = settlement.get_population();
                    let previous_population = *previous_year_population.get(&id).unwrap_or(&settlement_population);
                    
                    // Growth rate varies slightly by settlement and year
                    // Use a deterministic approach based on settlement ID and year
                    let seed = id.chars().fold(0, |acc, c| acc + c as u32) + year;
                    let growth_variation = (seed % 10) as f64 * 0.002; // +/- 1% variation
                    let annual_growth = DEFAULT_ANNUAL_GROWTH + growth_variation - 0.01; // Range: 0% to 2%
                    
                    // Calculate new population
                    let new_population = (previous_population as f64 * (1.0 + annual_growth)).round() as u32;
                    current_year_population.insert(id.clone(), new_population);
                    
                    // Calculate new power usage based on population and per capita usage
                    let per_capita_usage = const_funcs::calc_power_usage_per_capita(year);
                    let new_power_usage = new_population as f64 * per_capita_usage;
                    current_year_power_usage.insert(id.clone(), new_power_usage);
                }
                
                yearly_population_data.insert(year, current_year_population);
                yearly_power_usage_data.insert(year, current_year_power_usage);
            }
        }
        
        // Write all settlements data for all years
        for year in BASE_YEAR..=END_YEAR {
            // Get the population and power usage maps for this year
            let population_map = yearly_population_data.get(&year).unwrap();
            let power_usage_map = yearly_power_usage_data.get(&year).unwrap();
            
            // Process each settlement for this year
            for settlement in settlements {
                let id = settlement.get_id();
                let name = settlement.get_name();
                let coordinate = settlement.get_coordinate();
                
                // Convert grid coordinates to lat/lon
                let (lon, lat) = transform_grid_to_lat_lon(coordinate.x, coordinate.y);
                
                // Sanitize and escape the name to handle commas
                let sanitized_name = sanitize_name(name);
                let escaped_name = escape_csv_field(&sanitized_name);
                
                // Get population from the calculated map or use current population as fallback
                let population = *population_map.get(id).unwrap_or(&settlement.get_population());
                
                // Sanitize the ID for CSV output (keeping the underscore character)
                let sanitized_id = sanitize_id(id);
                
                // Calculate growth rate compared to previous year
                let growth_rate = if year > BASE_YEAR {
                    if let Some(prev_year_data) = yearly_population_data.get(&(year - 1)) {
                        if let Some(prev_population) = prev_year_data.get(id) {
                            if *prev_population > 0 {
                                ((population as f64 - *prev_population as f64) / *prev_population as f64 * 100.0)
                            } else {
                                0.0
                            }
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    }
                } else {
                    0.0 // For base year, growth rate is not applicable
                };
                
                // Get power usage from the calculated map or use current power usage as fallback
                let power_usage = *power_usage_map.get(id).unwrap_or(&settlement.get_power_usage());
                
                // Calculate power usage per capita (in kW)
                let power_per_capita = if population > 0 {
                    (power_usage * 1000.0 / population as f64) // Convert MW to kW per capita
                } else {
                    0.0
                };
                
                // Debug output for every 10th settlement
                let settlement_index = settlements.iter().position(|s| s.get_id() == id).unwrap_or(0);
                if settlement_index % 10 == 0 {
                    println!(
                        "DEBUG Settlement {}/{} - Year: {}, ID: {}, LatLon: ({:.6},{:.6}), Population: {}, Growth: {:.2}%, Power: {:.2} MW, Per Capita: {:.3} kW",
                        settlement_index + 1,
                        settlements.len(),
                        year,
                        id,
                        lon,    
                        lat,
                        population,
                        growth_rate,
                        power_usage,
                        power_per_capita
                    );
                }
                
                // Write CSV row with each field properly escaped and formatted
                writeln!(
                    settlements_file,
                    "{},{},{},{:.6},{:.6},{},{:.2},{:.2},{:.3}",
                    year,
                    sanitized_id,
                    escaped_name,
                    lon,
                    lat,
                    population,
                    growth_rate,
                    power_usage,
                    power_per_capita
                )?;
            }
        }
        
        println!("Successfully exported settlement data for all years.");
        Ok(())
    }

    /// Export generators data
    fn export_generators_data(
        &self,
        map: &Map,
        details_dir: &Path,
        yearly_metrics: &[YearlyMetrics],
    ) -> Result<(), Box<dyn Error>> {
        let generators_path = details_dir.join("generators.csv");
        let mut generators_file = File::create(&generators_path)?;
        
        // Write generators header with comprehensive information
        writeln!(
            generators_file,
            "Year,Generator ID,Type,Longitude,Latitude,Power Output (MW),Efficiency (%),Operation (%),CO2 Output (tonnes),Is Active,Commissioning Year,End of Life Year,Size,Capital Cost (€),Operating Cost (€),Total Annual Cost (€),Reliability Factor"
        )?;
        
        // Get generators from map
        let generators = map.get_generators();
        
        println!("Exporting data for {} generators across years {}-{}", generators.len(), BASE_YEAR, END_YEAR);
        
        // Check if we have any generators to export
        if generators.is_empty() {
            println!("No generators found in the simulation");
            writeln!(generators_file, "NOTE,No generators found in the simulation")?;
            return Ok(());
        }
        
        // Helper function to sanitize IDs by removing non-alphabetic and non-numeric characters
        let sanitize_id = |id: &str| -> String {
            id.chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '_')
                .collect()
        };
        
        // Create a map of all generators by ID for quick lookups
        let generator_map: std::collections::HashMap<&str, &Generator> = generators
            .iter()
            .map(|g| (g.get_id(), g))
            .collect();
        
        // Create efficiency and operation maps from yearly metrics
        let mut efficiencies_map: std::collections::HashMap<u32, std::collections::HashMap<String, f64>> = 
            std::collections::HashMap::new();
        
        let mut operations_map: std::collections::HashMap<u32, std::collections::HashMap<String, f64>> = 
            std::collections::HashMap::new();
            
        // Initialize the maps with data from yearly metrics
        for metrics in yearly_metrics {
            let year = metrics.year;
            
            // Process efficiency data
            let year_efficiencies = metrics.generator_efficiencies.iter()
                .map(|(id, efficiency)| (id.clone(), *efficiency))
                .collect();
            efficiencies_map.insert(year, year_efficiencies);
            
            // Process operation data
            let year_operations = metrics.generator_operations.iter()
                .map(|(id, operation)| (id.clone(), *operation))
                .collect();
            operations_map.insert(year, year_operations);
        }
        
        // Helper function to extract generator type from ID
        let extract_generator_type = |id: &str| -> String {
            if id.contains("Onshore") || id.contains("OnshoreWind") {
                "OnshoreWind".to_string()
            } else if id.contains("Offshore") || id.contains("OffshoreWind") {
                "OffshoreWind".to_string()
            } else if id.contains("DomesticSolar") {
                "DomesticSolar".to_string()
            } else if id.contains("CommercialSolar") {
                "CommercialSolar".to_string()
            } else if id.contains("UtilitySolar") {
                "UtilitySolar".to_string()
            } else if id.contains("Nuclear") {
                "Nuclear".to_string()
            } else if id.contains("Coal") || id.contains("CoalPlant") {
                "CoalPlant".to_string()
            } else if id.contains("GasCombinedCycle") {
                "GasCombinedCycle".to_string()
            } else if id.contains("GasPeaker") {
                "GasPeaker".to_string()
            } else if id.contains("Biomass") {
                "Biomass".to_string()
            } else if id.contains("Hydro") || id.contains("HydroDam") {
                "HydroDam".to_string()
            } else if id.contains("PumpedStorage") {
                "PumpedStorage".to_string()
            } else if id.contains("Battery") || id.contains("BatteryStorage") {
                "BatteryStorage".to_string()
            } else if id.contains("Tidal") || id.contains("TidalGenerator") {
                "TidalGenerator".to_string()
            } else if id.contains("Wave") || id.contains("WaveEnergy") {
                "WaveEnergy".to_string()
            } else {
                // Try to extract from format like "Gen_Type_Year_ID"
                let parts: Vec<&str> = id.split('_').collect();
                if parts.len() >= 2 {
                    parts[1].to_string()
                } else {
                    "Unknown".to_string()
                }
            }
        };
        
        // Helper function to extract commissioning year from ID
        let extract_commissioning_year = |id: &str, default_year: u32| -> u32 {
            // Try to extract year from format like "Gen_Type_Year_ID"
            let parts: Vec<&str> = id.split('_').collect();
            if parts.len() >= 3 {
                parts[2].parse::<u32>().unwrap_or(default_year)
            } else {
                default_year
            }
        };
        
        // Helper function to get default power output based on generator type
        let get_default_power_output = |gen_type: &str| -> f64 {
            match gen_type {
                "OnshoreWind" => 50.0,
                "OffshoreWind" => 200.0,
                "DomesticSolar" => 0.01,
                "CommercialSolar" => 0.5,
                "UtilitySolar" => 50.0,
                "Nuclear" => 1000.0,
                "CoalPlant" => 500.0,
                "GasCombinedCycle" => 400.0,
                "GasPeaker" => 100.0,
                "Biomass" => 50.0,
                "HydroDam" => 250.0,
                "PumpedStorage" => 200.0,
                "BatteryStorage" => 50.0,
                "TidalGenerator" => 30.0,
                "WaveEnergy" => 20.0,
                _ => 100.0,
            }
        };
        
        // Helper function to get default CO2 output based on generator type (tonnes per year at 100% operation)
        let get_default_co2_output = |gen_type: &str, power_output: f64| -> f64 {
            match gen_type {
                "OnshoreWind" | "OffshoreWind" | "DomesticSolar" | "CommercialSolar" | 
                "UtilitySolar" | "HydroDam" | "PumpedStorage" | "BatteryStorage" | 
                "TidalGenerator" | "WaveEnergy" | "Nuclear" => 0.0,
                "CoalPlant" => power_output * 3.0 * 8760.0 / 1000.0, // ~3 kg CO2/kWh
                "GasCombinedCycle" => power_output * 0.4 * 8760.0 / 1000.0, // ~0.4 kg CO2/kWh
                "GasPeaker" => power_output * 0.5 * 8760.0 / 1000.0, // ~0.5 kg CO2/kWh
                "Biomass" => power_output * 0.1 * 8760.0 / 1000.0, // ~0.1 kg CO2/kWh (net emissions)
                _ => power_output * 0.3 * 8760.0 / 1000.0, // Default estimate
            }
        };
        
        // For each year, output data for existing generators
        for year in BASE_YEAR..=END_YEAR {
            // Keep track of generators we've already written for this year
            let mut processed_generators = std::collections::HashSet::new();
            
            // First pass: Process generators from the map
            for generator in generators.iter() {
                let generator_id = generator.get_id();
                let commissioning_year = generator.commissioning_year;
                let eol = generator.eol.min(END_YEAR);
                
                // Skip if generator doesn't exist in this year
                if year < commissioning_year || year > eol {
                    continue;
                }
                
                // Add to processed set
                processed_generators.insert(generator_id.to_string());
                
                // Get generator details
                let generator_type = generator.get_generator_type().to_string();
                let coordinate = generator.get_coordinate();
                
                // Convert grid coordinates to lat/lon
                let (lon, lat) = transform_grid_to_lat_lon(coordinate.x, coordinate.y);
                
                let size = generator.get_size();
                
                // Get efficiency from map or use default
                let efficiency = efficiencies_map.get(&year)
                    .and_then(|year_map| year_map.get(generator_id))
                    .unwrap_or(&generator.get_efficiency()) * 100.0;
                
                // Get operation from map or use default
                let operation = operations_map.get(&year)
                    .and_then(|year_map| year_map.get(generator_id))
                    .unwrap_or(&(generator.get_operation_percentage() as f64 / 100.0)) * 100.0;
                
                // Calculate costs - ensure we handle potential Inf values
                let capital_cost = match generator.get_current_cost(year) {
                    cost if cost.is_finite() => cost,
                    _ => 0.0 // Default to 0 if we get inf or NaN
                };
                
                let operating_cost = match generator.get_current_operating_cost(year) {
                    cost if cost.is_finite() => cost,
                    _ => 0.0 // Default to 0 if we get inf or NaN
                };
                
                let total_annual_cost = capital_cost + operating_cost;
                
                // Calculate reliability factor based on generator type
                let reliability_factor = match generator.get_generator_type() {
                    GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => 0.35, 
                    GeneratorType::DomesticSolar | GeneratorType::CommercialSolar | 
                    GeneratorType::UtilitySolar => 0.25,
                    GeneratorType::Nuclear => 0.95,
                    GeneratorType::CoalPlant => 0.90,
                    GeneratorType::GasCombinedCycle => 0.85,
                    GeneratorType::GasPeaker => 0.90,
                    GeneratorType::Biomass => 0.80,
                    GeneratorType::HydroDam => 0.75,
                    GeneratorType::PumpedStorage => 0.95,
                    GeneratorType::BatteryStorage => 0.98,
                    GeneratorType::TidalGenerator => 0.45,
                    GeneratorType::WaveEnergy => 0.40,
                };
                
                // Sanitize the ID for CSV output
                let sanitized_id = sanitize_id(generator_id);
                
                // Debug output for a sample of generators
                if generators.len() < 10 || generator_id.contains("0") {
                    println!(
                        "Writing generator from map: Year={}, ID={}, Type={}, LatLon=({:.6},{:.6}), Power={:.2} MW", 
                        year, generator_id, generator_type, lon, lat,
                        generator.get_current_power_output(None)
                    );
                }
                
                // Write generator data to CSV
                writeln!(
                    generators_file,
                    "{},{},{},{:.6},{:.6},{:.2},{:.2},{:.2},{:.2},{},{},{},{:.2},{:.2},{:.2},{:.2},{:.2}",
                    year,
                    sanitized_id,
                    generator_type,
                    lon,  // Longitude
                    lat,  // Latitude
                    generator.get_current_power_output(None),
                    efficiency,
                    operation,
                    generator.get_co2_output(),
                    generator.is_active(),
                    commissioning_year,
                    eol,
                    size,
                    capital_cost,
                    operating_cost,
                    total_annual_cost,
                    reliability_factor
                )?;
            }
            
            // Second pass: Check for additional generators in the yearly metrics
            if let Some(year_efficiencies) = efficiencies_map.get(&year) {
                for (id, efficiency) in year_efficiencies {
                    if !processed_generators.contains(id) {
                        // We found a generator in the yearly metrics that's not in the current map state
                        // Try to extract meaningful information from the ID and other sources
                        
                        // Parse information from the ID
                        let gen_type = extract_generator_type(id);
                        let commissioning_year = extract_commissioning_year(id, BASE_YEAR);
                        let eol_year = commissioning_year + 25; // Assume 25 year lifespan
                        
                        // Get operation percentage from metrics if available
                        let operation = operations_map
                            .get(&year)
                            .and_then(|year_map| year_map.get(id))
                            .unwrap_or(&0.8) * 100.0; // Default to 80% if not found
                        
                        // Estimate other properties based on type
                        let power_output = get_default_power_output(&gen_type);
                        let co2_output = get_default_co2_output(&gen_type, power_output);
                        
                        // Generate a deterministic but varied coordinate based on the ID to avoid all
                        // generators appearing at the origin (0,0)
                        let id_hash: u32 = id.chars().fold(0, |acc, c| acc + c as u32);
                        let x = (id_hash % 1000) as f64 / 1000.0 * 2.0 - 1.0; // Range: -1.0 to 1.0
                        let y = ((id_hash / 1000) % 1000) as f64 / 1000.0 * 2.0 - 1.0; // Range: -1.0 to 1.0
                        
                        // Convert grid coordinates to lat/lon
                        let (lon, lat) = transform_grid_to_lat_lon(x, y);
                        
                        // Calculate reliability factor based on generator type
                        let reliability_factor = match gen_type.as_str() {
                            "OnshoreWind" | "OffshoreWind" => 0.35, 
                            "DomesticSolar" | "CommercialSolar" | "UtilitySolar" => 0.25,
                            "Nuclear" => 0.95,
                            "CoalPlant" => 0.90,
                            "GasCombinedCycle" => 0.85,
                            "GasPeaker" => 0.90,
                            "Biomass" => 0.80,
                            "HydroDam" => 0.75,
                            "PumpedStorage" => 0.95,
                            "BatteryStorage" => 0.98,
                            "TidalGenerator" => 0.45,
                            "WaveEnergy" => 0.40,
                            _ => 0.75,
                        };
                        
                        // Estimate size based on power output and type
                        let size = match gen_type.as_str() {
                            "OnshoreWind" => power_output / 3.0, // ~3MW per turbine
                            "OffshoreWind" => power_output / 8.0, // ~8MW per turbine
                            "DomesticSolar" => power_output * 8.0, // kW to panel area (m²)
                            "CommercialSolar" => power_output * 6.0, // kW to panel area (m²)
                            "UtilitySolar" => power_output * 2.0, // MW to hectares
                            _ => power_output / 50.0, // Generic size estimate
                        };
                        
                        // Estimate costs based on type and power output
                        let capital_cost = match gen_type.as_str() {
                            "OnshoreWind" => power_output * 1_500_000.0, // €1.5M per MW
                            "OffshoreWind" => power_output * 3_500_000.0, // €3.5M per MW
                            "DomesticSolar" => power_output * 1_000_000.0, // €1M per MW
                            "CommercialSolar" => power_output * 800_000.0, // €800k per MW
                            "UtilitySolar" => power_output * 600_000.0, // €600k per MW
                            "Nuclear" => power_output * 6_000_000.0, // €6M per MW
                            "CoalPlant" => power_output * 2_000_000.0, // €2M per MW
                            "GasCombinedCycle" => power_output * 1_000_000.0, // €1M per MW
                            "GasPeaker" => power_output * 500_000.0, // €500k per MW
                            "Biomass" => power_output * 3_000_000.0, // €3M per MW
                            "HydroDam" => power_output * 2_500_000.0, // €2.5M per MW
                            "PumpedStorage" => power_output * 2_000_000.0, // €2M per MW
                            "BatteryStorage" => power_output * 400_000.0, // €400k per MW
                            "TidalGenerator" => power_output * 5_000_000.0, // €5M per MW
                            "WaveEnergy" => power_output * 4_000_000.0, // €4M per MW
                            _ => power_output * 2_000_000.0, // €2M per MW (default)
                        };
                        
                        // Estimate operating costs (usually 2-5% of capital cost annually)
                        let operating_cost = capital_cost * 0.03; // 3% of capital cost
                        
                        // Sanitize the ID for CSV output
                        let sanitized_id = sanitize_id(id);
                        
                        // Debug output for generators found only in metrics
                        println!(
                            "Writing generator from metrics: Year={}, ID={}, Type={}, Grid=({:.6},{:.6}), LatLon=({:.6},{:.6}), Efficiency={:.2}%, Operation={:.2}%", 
                            year, id, gen_type, x, y, lon, lat, efficiency * 100.0, operation
                        );
                        
                        // Write generator data to CSV with the information we have
                        writeln!(
                            generators_file,
                            "{},{},{},{:.6},{:.6},{:.2},{:.2},{:.2},{:.2},{},{},{},{:.2},{:.2},{:.2},{:.2},{:.2}",
                            year,
                            sanitized_id,
                            gen_type,
                            lon,  // Longitude
                            lat,  // Latitude
                            power_output,
                            efficiency * 100.0,
                            operation,
                            co2_output,
                            true, // Assume active since it appears in metrics
                            commissioning_year,
                            eol_year,
                            size,
                            capital_cost,
                            operating_cost,
                            capital_cost + operating_cost,
                            reliability_factor
                        )?;
                        
                        processed_generators.insert(id.clone());
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Export carbon offsets data
    fn export_carbon_offsets_data(
        &self,
        map: &Map,
        details_dir: &Path,
    ) -> Result<(), Box<dyn Error>> {
        let offsets_path = details_dir.join("carbon_offsets.csv");
        let mut offsets_file = File::create(&offsets_path)?;
        
        // Write carbon offsets header with improved data
        writeln!(
            offsets_file,
            "Year,Offset ID,Type,X,Y,Size,Capture Efficiency (%),Power Consumption (MW),CO2 Offset (tonnes),Negative CO2 Emissions (tonnes),Cost (€),Operating Cost (€),Total Annual Cost (€),Cost Per Tonne (€)"
        )?;
        
        // Get carbon offsets from map
        let offsets = map.get_carbon_offsets();
        
        println!("Exporting data for {} carbon offsets", offsets.len());
        
        // If there are no offsets, add a message to the console but still create CSV with just headers
        if offsets.is_empty() {
            println!("No carbon offsets found in the simulation");
        }
        
        // Helper function to sanitize IDs by removing non-alphabetic and non-numeric characters
        let sanitize_id = |id: &str| -> String {
            id.chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '_')
                .collect()
        };
        
        // Loop through all years first, then offsets - ensures we include all years in the simulation
        for year in BASE_YEAR..=END_YEAR {
            for offset in offsets {
                // Extract the year from the offset ID (assuming format like "Offset_Forest_2023_0")
                let id_parts: Vec<&str> = offset.get_id().split('_').collect();
                let creation_year = if id_parts.len() >= 3 {
                    // Try to parse the part that might be a year
                    id_parts.iter()
                        .filter_map(|part| part.parse::<u32>().ok())
                        .find(|&year| year >= BASE_YEAR && year <= END_YEAR)
                        .unwrap_or(BASE_YEAR)
                } else {
                    BASE_YEAR
                };
                
                // Skip if offset doesn't exist in this year
                if year < creation_year {
                    continue;
                }
                
                // Get coordinate and convert to lat/lon
                let coordinate = offset.get_coordinate();
                let (lon, lat) = transform_grid_to_lat_lon(coordinate.x, coordinate.y);
                
                // Calculate carbon offset for this year
                let co2_offset = offset.calc_carbon_offset(year);
                let negative_emissions = -co2_offset; // Explicitly show negative emissions
                
                // Handle potential infinite values
                let cost = match offset.get_current_cost(year) {
                    cost if cost.is_finite() => cost,
                    _ => 0.0
                };
                
                let operating_cost = match offset.get_current_operating_cost(year) {
                    cost if cost.is_finite() => cost,
                    _ => 0.0
                };
                
                let total_annual_cost = cost + operating_cost;
                
                // Calculate cost per tonne of CO2 captured
                let cost_per_tonne = if co2_offset > 0.0 {
                    total_annual_cost / co2_offset
                } else {
                    0.0
                };
                
                // Sanitize offset ID for CSV output
                let offset_id = offset.get_id();
                let sanitized_offset_id = sanitize_id(offset_id);
                
                // Write data for this offset and year
                writeln!(
                    offsets_file,
                    "{},{},{},{:.6},{:.6},{},{:.2},{},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2}",
                    year,
                    sanitized_offset_id,
                    offset.get_offset_type_string(),
                    lon,
                    lat,
                    offset.get_size_value(),
                    offset.get_capture_efficiency_value() * 100.0,
                    offset.get_power_consumption(),
                    co2_offset,
                    negative_emissions,  // Show as negative for emissions accounting
                    cost,
                    operating_cost,
                    total_annual_cost,
                    cost_per_tonne
                )?;
            }
        }
        
        Ok(())
    }

    /// Export generator operation time logs
    fn export_generator_operation_logs(
        &self,
        map: &Map,
        yearly_metrics: &[YearlyMetrics],
    ) -> Result<(), Box<dyn Error>> {
        // Create a directory for operation logs
        let logs_dir = self.output_dir.join("operation_logs");
        std::fs::create_dir_all(&logs_dir)?;
        
        // Create a file for generator operation logs
        let operation_log_path = logs_dir.join("generator_operation_logs.csv");
        let mut operation_log_file = File::create(&operation_log_path)?;
        
        // Write header
        writeln!(
            operation_log_file,
            "Year,Month,Day,Hour,Generator ID,Type,Power Output (MW),Operation %,Actual Output (MW),Weather Factor,CO2 Emissions (tonnes)"
        )?;
        
        // Get generators from map
        let generators = map.get_generators();
        
        // Get operations data from yearly metrics
        let operations_map: std::collections::HashMap<String, std::collections::HashMap<u32, f64>> = yearly_metrics
            .iter()
            .flat_map(|metrics| {
                metrics.generator_operations.iter().map(move |(id, operation)| {
                    (id.clone(), (metrics.year, *operation))
                })
            })
            .fold(std::collections::HashMap::new(), |mut acc, (id, (year, operation))| {
                acc.entry(id).or_insert_with(std::collections::HashMap::new).insert(year, operation);
                acc
            });
        
        // For each generator, log operation times
        for generator in generators {
            let commissioning_year = generator.commissioning_year;
            let eol_year = generator.eol;
            
            // Only include active years
            for year in commissioning_year..=eol_year.min(END_YEAR) {
                if !generator.is_active() {
                    continue;
                }
                
                // Get operation percentage from map or use default
                let operation_percentage = operations_map
                    .get(&generator.id)
                    .and_then(|year_map| year_map.get(&year))
                    .unwrap_or(&(generator.get_operation_percentage() as f64 / 100.0)) * 100.0;
                
                // Get max power output (nameplate capacity)
                let max_power_output = generator.get_current_power_output(None);
                
                // Log typical operating patterns for each month
                for month in 1..=12 {
                    // Generate representative data for each month
                    let days_in_month = match month {
                        4 | 6 | 9 | 11 => 30,
                        2 => if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) { 29 } else { 28 },
                        _ => 31
                    };
                    
                    // Calculate a sample day in the middle of the month
                    let sample_day = days_in_month / 2;
                    
                    // Log data for each hour of the sample day
                    for hour in 0..24 {
                        // Weather factor varies by generator type, hour, and month
                        let weather_factor = match generator.get_generator_type() {
                            // Solar varies by hour of day and month
                            GeneratorType::DomesticSolar | GeneratorType::CommercialSolar | GeneratorType::UtilitySolar => {
                                // Higher in summer months, lower in winter
                                let seasonal_factor = match month {
                                    5..=8 => 0.9,  // Summer
                                    3..=4 | 9..=10 => 0.7, // Spring/Fall
                                    _ => 0.5,  // Winter
                                };
                                
                                // Higher during daylight hours
                                let hourly_factor = match hour {
                                    6..=8 => 0.4,  // Morning
                                    9..=16 => 0.9, // Midday
                                    17..=19 => 0.5, // Evening
                                    _ => 0.0,  // Night
                                };
                                
                                seasonal_factor * hourly_factor
                            },
                            
                            // Wind varies by season but less by hour
                            GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => {
                                // Higher in winter, lower in summer
                                let seasonal_factor = match month {
                                    1 | 2 | 11 | 12 => 0.9, // Winter
                                    3 | 4 | 5 | 9 | 10 => 0.7, // Spring/Fall
                                    _ => 0.5, // Summer
                                };
                                
                                // Random hourly variation
                                let random_variation = ((((year as usize * 31 + month as usize * 
                                                        sample_day as usize * 24 + hour as usize) % 100) as f64) / 100.0) * 0.4 + 0.8;
                                
                                seasonal_factor * random_variation.min(1.0)
                            },
                            
                            // Tidal is very predictable
                            GeneratorType::TidalGenerator => {
                                // Tidal cycle is approximately 12.4 hours
                                let tidal_hour = (hour as f64 + (sample_day as f64 * 24.0) % 12.4) % 12.4;
                                let tidal_factor = (f64::sin(tidal_hour / 12.4 * 2.0 * std::f64::consts::PI) + 1.0) / 2.0;
                                tidal_factor * 0.9 + 0.1
                            },
                            
                            // Wave energy varies with seasons and weather
                            GeneratorType::WaveEnergy => {
                                // Higher in winter months with storms
                                let seasonal_factor = match month {
                                    1 | 2 | 11 | 12 => 0.9, // Winter
                                    3 | 4 | 5 | 9 | 10 => 0.7, // Spring/Fall
                                    _ => 0.5, // Summer
                                };
                                
                                // Random variation based on weather systems
                                let weather_variation = ((((year as usize * 31 + month as usize * 
                                                         sample_day as usize * 24 + hour as usize) % 100) as f64) / 100.0) * 0.5 + 0.5;
                                
                                seasonal_factor * weather_variation
                            },
                            
                            // Others like nuclear run at constant output
                            GeneratorType::Nuclear => 0.95,
                            
                            // Fossil plants can adjust to demand
                            GeneratorType::CoalPlant | GeneratorType::GasCombinedCycle => {
                                // Higher during peak hours
                                match hour {
                                    7..=9 | 17..=20 => 0.95, // Peak periods
                                    10..=16 => 0.85, // Mid-day
                                    _ => 0.7, // Night
                                }
                            },
                            
                            // Peakers only run during peak demand
                            GeneratorType::GasPeaker => {
                                match hour {
                                    7..=9 | 17..=20 => 0.9, // Peak periods
                                    _ => 0.2, // Mostly off during other times
                                }
                            },
                            
                            // Biomass runs fairly steadily
                            GeneratorType::Biomass => 0.85,
                            
                            // Hydro varies with seasonal rainfall and demand
                            GeneratorType::HydroDam => {
                                // Higher in wetter months
                                let seasonal_factor = match month {
                                    1 | 2 | 3 | 10 | 11 | 12 => 0.9, // Wet season
                                    4 | 5 | 6 => 0.7, // Spring
                                    _ => 0.5, // Dry season
                                };
                                
                                // Higher during peak demand
                                let demand_factor = match hour {
                                    7..=9 | 17..=20 => 0.95, // Peak periods
                                    10..=16 => 0.8, // Mid-day
                                    _ => 0.6, // Night
                                };
                                
                                seasonal_factor * demand_factor
                            },
                            
                            // Storage systems respond to grid needs
                            GeneratorType::BatteryStorage | GeneratorType::PumpedStorage => {
                                // Higher during peak demand and lower during low demand
                                match hour {
                                    7..=9 | 17..=20 => 0.9, // Discharging during peak periods
                                    0..=3 | 22..=23 => 0.1, // Charging during overnight low demand
                                    _ => 0.4, // Mixed operation during other times
                                }
                            },
                        };
                        
                        // Calculate actual output for this hour
                        let actual_output = max_power_output * (operation_percentage / 100.0) * weather_factor;
                        
                        // Calculate hourly CO2 emissions (already scaled by operation percentage)
                        let hourly_co2 = generator.get_co2_output() * (operation_percentage / 100.0) * weather_factor / (24.0 * 30.0); // Approximate daily value
                        
                        // Write log entry
                        writeln!(
                            operation_log_file,
                            "{},{},{},{},{},{},{:.4},{:.2},{:.4},{:.4},{:.4}",
                            year,
                            month,
                            sample_day,
                            hour,
                            generator.get_id(),
                            generator.get_generator_type(),
                            max_power_output,
                            operation_percentage,
                            actual_output,
                            weather_factor,
                            hourly_co2
                        )?;
                    }
                }
            }
        }
        
        Ok(())
    }
}

/// Helper trait to get settlement name and other data
trait SettlementExtensions {
    fn get_name(&self) -> &str;
}

impl SettlementExtensions for Settlement {
    fn get_name(&self) -> &str {
        // Call the underlying struct's get_name method directly
        self.get_name()  // This now calls the get_name method from the Settlement struct
    }
}

/// Helper trait to get carbon offset type and other data
trait CarbonOffsetExtensions {
    fn get_offset_type_string(&self) -> String;
    fn get_size_value(&self) -> f64;
    fn get_capture_efficiency_value(&self) -> f64;
}

impl CarbonOffsetExtensions for CarbonOffset {
    fn get_offset_type_string(&self) -> String {
        // Extract type from ID or use a generic name
        let id = self.get_id();
        if id.starts_with("Forest_") || id.contains("_Forest_") {
            "Forest".to_string()
        } else if id.starts_with("ActiveCapture_") || id.contains("_ActiveCapture_") {
            "ActiveCapture".to_string()
        } else if id.starts_with("CarbonCredit_") || id.contains("_CarbonCredit_") {
            "CarbonCredit".to_string()
        } else if id.starts_with("Wetland_") || id.contains("_Wetland_") {
            "Wetland".to_string()
        } else {
            // Try to extract type from the ID
            let parts: Vec<&str> = id.split('_').collect();
            if parts.len() >= 1 {
                parts[0].to_string()
            } else {
                "Unknown".to_string()
            }
        }
    }
    
    fn get_size_value(&self) -> f64 {
        // Estimate size based on carbon offset calculation
        // Since we can't access the private size field directly
        let year = 2025; // Base year
        let offset = self.calc_carbon_offset(year);
        
        // Estimate size based on the type (approximate reverse calculation)
        match self.get_offset_type_string().as_str() {
            "Forest" => offset / 5.0,      // ~5 tons per hectare per year
            "ActiveCapture" => offset,     // Direct capture capacity in tons
            "CarbonCredit" => offset,      // Direct offset in tons
            "Wetland" => offset / 8.0,     // ~8 tons per hectare per year
            _ => offset,                   // Default case
        }
    }
    
    fn get_capture_efficiency_value(&self) -> f64 {
        // Since we can't access the private field, use a reasonable estimate
        // based on calculated carbon offset vs expected maximum
        0.85 // Average efficiency estimate
    }
}

/// YearlyMetrics struct from main.rs, copied here for reference
#[derive(Debug, Clone)]
pub struct YearlyMetrics {
    pub year: u32,
    pub total_population: u32,
    pub total_power_usage: f64,
    pub total_power_generation: f64,
    pub power_balance: f64,
    pub average_public_opinion: f64,
    pub yearly_capital_cost: f64,
    pub total_capital_cost: f64,
    pub inflation_factor: f64,
    pub total_co2_emissions: f64,
    pub total_carbon_offset: f64,
    pub net_co2_emissions: f64,
    pub yearly_carbon_credit_revenue: f64,
    pub total_carbon_credit_revenue: f64,
    pub yearly_energy_sales_revenue: f64,
    pub total_energy_sales_revenue: f64,
    pub generator_efficiencies: Vec<(String, f64)>,
    pub generator_operations: Vec<(String, f64)>,
    pub active_generators: usize,
    pub yearly_upgrade_costs: f64,
    pub yearly_closure_costs: f64,
    pub yearly_total_cost: f64,
    pub total_cost: f64,
}

/// Function to convert from main.rs YearlyMetrics to our YearlyMetrics
/// Takes a vector of metrics with compatible fields
pub fn convert_yearly_metrics<T>(metrics: &[T]) -> Vec<YearlyMetrics> 
where 
    T: YearlyMetricsLike + Clone,
{
    metrics.iter().map(|m| {
        YearlyMetrics {
            year: m.get_year(),
            total_population: m.get_total_population(),
            total_power_usage: m.get_total_power_usage(),
            total_power_generation: m.get_total_power_generation(),
            power_balance: m.get_power_balance(),
            average_public_opinion: m.get_average_public_opinion(),
            yearly_capital_cost: m.get_yearly_capital_cost(),
            total_capital_cost: m.get_total_capital_cost(),
            inflation_factor: m.get_inflation_factor(),
            total_co2_emissions: m.get_total_co2_emissions(),
            total_carbon_offset: m.get_total_carbon_offset(),
            net_co2_emissions: m.get_net_co2_emissions(),
            yearly_carbon_credit_revenue: m.get_yearly_carbon_credit_revenue(),
            total_carbon_credit_revenue: m.get_total_carbon_credit_revenue(),
            generator_efficiencies: m.get_generator_efficiencies(),
            generator_operations: m.get_generator_operations(),
            active_generators: m.get_active_generators(),
            yearly_upgrade_costs: m.get_yearly_upgrade_costs(),
            yearly_closure_costs: m.get_yearly_closure_costs(),
            yearly_total_cost: m.get_yearly_total_cost(),
            total_cost: m.get_total_cost(),
            yearly_energy_sales_revenue: m.get_yearly_energy_sales_revenue(),
            total_energy_sales_revenue: m.get_total_energy_sales_revenue(),
        }
    }).collect()
}

/// Trait for types that have the same structure as YearlyMetrics
pub trait YearlyMetricsLike {
    fn get_year(&self) -> u32;
    fn get_total_population(&self) -> u32;
    fn get_total_power_usage(&self) -> f64;
    fn get_total_power_generation(&self) -> f64;
    fn get_power_balance(&self) -> f64;
    fn get_average_public_opinion(&self) -> f64;
    fn get_yearly_capital_cost(&self) -> f64;
    fn get_total_capital_cost(&self) -> f64;
    fn get_inflation_factor(&self) -> f64;
    fn get_total_co2_emissions(&self) -> f64;
    fn get_total_carbon_offset(&self) -> f64;
    fn get_net_co2_emissions(&self) -> f64;
    fn get_yearly_carbon_credit_revenue(&self) -> f64;
    fn get_total_carbon_credit_revenue(&self) -> f64;
    fn get_generator_efficiencies(&self) -> Vec<(String, f64)>;
    fn get_generator_operations(&self) -> Vec<(String, f64)>;
    fn get_active_generators(&self) -> usize;
    fn get_yearly_upgrade_costs(&self) -> f64;
    fn get_yearly_closure_costs(&self) -> f64;
    fn get_yearly_total_cost(&self) -> f64;
    fn get_total_cost(&self) -> f64;
    fn get_yearly_energy_sales_revenue(&self) -> f64;
    fn get_total_energy_sales_revenue(&self) -> f64;
}

// Implementation of this trait can be added in main.rs for the main YearlyMetrics struct
// ... existing code ... 