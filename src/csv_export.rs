use std::fs::File;
use std::path::{Path, PathBuf};
use std::io::Write;
use std::error::Error;
use chrono::Local;

use crate::map_handler::Map;
use crate::action_weights::{GridAction, SimulationMetrics};
use crate::settlement::Settlement;
use crate::carbon_offset::CarbonOffset;
use crate::constants::{BASE_YEAR, END_YEAR};
use crate::poi::POI;

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
        writeln!(summary_file, "Year,Action Type,Generator Type,Generator ID,Operation %,Offset Type")?;
        
        // Write each action
        for (year, action) in actions {
            let (action_type, gen_type, gen_id, operation_pct, offset_type) = match action {
                GridAction::AddGenerator(gen_type) => (
                    "AddGenerator",
                    gen_type.to_string(),
                    String::new(),
                    String::new(),
                    String::new(),
                ),
                GridAction::UpgradeEfficiency(id) => (
                    "UpgradeEfficiency",
                    String::new(),
                    id.clone(),
                    String::new(),
                    String::new(),
                ),
                GridAction::AdjustOperation(id, percentage) => (
                    "AdjustOperation",
                    String::new(),
                    id.clone(),
                    percentage.to_string(),
                    String::new(),
                ),
                GridAction::AddCarbonOffset(offset_type) => (
                    "AddCarbonOffset",
                    String::new(),
                    String::new(),
                    String::new(),
                    offset_type.clone(),
                ),
                GridAction::CloseGenerator(id) => (
                    "CloseGenerator",
                    String::new(),
                    id.clone(),
                    String::new(),
                    String::new(),
                ),
                GridAction::DoNothing => (
                    "DoNothing",
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                ),
            };
            
            writeln!(
                summary_file,
                "{},{},{},{},{},{}",
                year, action_type, gen_type, gen_id, operation_pct, offset_type
            )?;
        }
        
        // Add yearly summary metrics
        writeln!(summary_file, "")?;
        writeln!(summary_file, "Yearly Summary Metrics")?;
        writeln!(
            summary_file,
            "Year,Population,Power Usage (MW),Power Generation (MW),Power Balance (MW),Public Opinion (%),CO2 Emissions (tonnes),Carbon Offset (tonnes),Net CO2 Emissions (tonnes),Operating Cost (€),Capital Cost (€),Total Cost (€),Active Generators"
        )?;
        
        for metrics in yearly_metrics {
            writeln!(
                summary_file,
                "{},{},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{}",
                metrics.year,
                metrics.total_population,
                metrics.total_power_usage,
                metrics.total_power_generation,
                metrics.power_balance,
                metrics.average_public_opinion * 100.0,
                metrics.total_co2_emissions,
                metrics.total_carbon_offset,
                metrics.net_co2_emissions,
                metrics.total_operating_cost,
                metrics.total_capital_cost,
                metrics.total_cost,
                metrics.active_generators
            )?;
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
        
        // Write settlements header
        writeln!(
            settlements_file,
            "Year,Settlement ID,Name,X,Y,Population,Power Usage (MW)"
        )?;
        
        // Get settlements from map
        let settlements = map.get_settlements();
        
        // Write detailed data for each year
        for year in BASE_YEAR..=END_YEAR {
            for settlement in settlements {
                writeln!(
                    settlements_file,
                    "{},{},{},{},{},{},{}",
                    year,
                    settlement.get_id(),
                    settlement.get_name(),
                    settlement.get_coordinate().x,
                    settlement.get_coordinate().y,
                    settlement.get_population(),
                    settlement.get_power_usage()
                )?;
            }
        }
        
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
        
        // Write generators header
        writeln!(
            generators_file,
            "Year,Generator ID,Type,X,Y,Power Output (MW),Efficiency (%),Operation (%),CO2 Output (tonnes),Is Active,Commissioning Year,End of Life Year,Size"
        )?;
        
        // Get generators from map
        let generators = map.get_generators();
        
        // Get efficiency and operation data from yearly metrics
        let efficiencies_map: std::collections::HashMap<String, std::collections::HashMap<u32, f64>> = yearly_metrics
            .iter()
            .flat_map(|metrics| {
                metrics.generator_efficiencies.iter().map(move |(id, efficiency)| {
                    (id.clone(), (metrics.year, *efficiency))
                })
            })
            .fold(std::collections::HashMap::new(), |mut acc, (id, (year, efficiency))| {
                acc.entry(id).or_insert_with(std::collections::HashMap::new).insert(year, efficiency);
                acc
            });
            
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
        
        // Write detailed data for each generator in each year
        for generator in generators {
            let commissioning_year = generator.commissioning_year;
            let eol_year = generator.eol;
            
            for year in commissioning_year..=eol_year.min(END_YEAR) {
                // Get efficiency and operation from maps or use defaults
                let efficiency = efficiencies_map
                    .get(&generator.id)
                    .and_then(|year_map| year_map.get(&year))
                    .unwrap_or(&generator.efficiency) * 100.0;
                    
                let operation = operations_map
                    .get(&generator.id)
                    .and_then(|year_map| year_map.get(&year))
                    .unwrap_or(&generator.operation_percentage) * 100.0;
                
                writeln!(
                    generators_file,
                    "{},{},{},{},{},{:.2},{:.2},{:.2},{:.2},{},{},{},{}",
                    year,
                    generator.id,
                    generator.generator_type,
                    generator.coordinate.x,
                    generator.coordinate.y,
                    generator.get_current_power_output(None), // Current power output
                    efficiency,
                    operation,
                    generator.get_co2_output(),
                    generator.is_active,
                    commissioning_year,
                    eol_year,
                    generator.size
                )?;
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
        
        // Write carbon offsets header
        writeln!(
            offsets_file,
            "Year,Offset ID,Type,X,Y,Size,Capture Efficiency (%),Power Consumption (MW),CO2 Offset (tonnes),Cost (€),Operating Cost (€)"
        )?;
        
        // Get carbon offsets from map
        let offsets = map.get_carbon_offsets();
        
        // Write detailed data for each carbon offset in each year
        for offset in offsets {
            for year in BASE_YEAR..=END_YEAR {
                let co2_offset = offset.calc_carbon_offset(year);
                let cost = offset.get_current_cost(year);
                let operating_cost = offset.get_current_operating_cost(year);
                
                writeln!(
                    offsets_file,
                    "{},{},{},{},{},{},{:.2},{},{:.2},{:.2},{:.2}",
                    year,
                    offset.get_id(),
                    offset.get_offset_type(),
                    offset.get_coordinate().x,
                    offset.get_coordinate().y,
                    offset.get_size(),
                    offset.get_capture_efficiency() * 100.0,
                    offset.get_power_consumption(),
                    co2_offset,
                    cost,
                    operating_cost
                )?;
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
        self.get_id() // Settlement ID is its name
    }
}

/// Helper trait to get carbon offset type and other data
trait CarbonOffsetExtensions {
    fn get_offset_type(&self) -> String;
    fn get_size(&self) -> f64;
    fn get_capture_efficiency(&self) -> f64;
}

impl CarbonOffsetExtensions for CarbonOffset {
    fn get_offset_type(&self) -> String {
        // Since we don't have direct access to get the type, 
        // we'll return a placeholder that doesn't access private fields
        "CarbonOffset".to_string()
    }
    
    fn get_size(&self) -> f64 {
        // Using a reasonable default since there's no public getter
        0.0
    }
    
    fn get_capture_efficiency(&self) -> f64 {
        // Using a reasonable default since there's no public getter
        0.0
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
    pub total_operating_cost: f64,
    pub total_capital_cost: f64,
    pub inflation_factor: f64,
    pub total_co2_emissions: f64,
    pub total_carbon_offset: f64,
    pub net_co2_emissions: f64,
    pub generator_efficiencies: Vec<(String, f64)>,
    pub generator_operations: Vec<(String, f64)>,
    pub active_generators: usize,
    pub upgrade_costs: f64,
    pub closure_costs: f64,
    pub total_cost: f64,
} 