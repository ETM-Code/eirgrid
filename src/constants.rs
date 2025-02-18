// Time Constants
pub const BASE_YEAR: u32 = 2025;
pub const END_YEAR: u32 = 2050;
pub const MEDIUM_TERM_YEAR: u32 = 2030;
pub const LONG_TERM_YEAR: u32 = 2040;

// Map and Grid Constants
pub const MAP_MAX_X: f64 = 50_000.0;
pub const MAP_MAX_Y: f64 = 50_000.0;
pub const GRID_CELL_SIZE: f64 = 1000.0; // 1km grid cells
pub const MAX_CUMULATIVE_GENERATOR_SIZE: f64 = 1.0;

// Transmission Loss Factors
pub const BASE_TRANSMISSION_LOSS_RATE: f64 = 0.00001; // Loss per meter of distance
pub const HIGH_VOLTAGE_LOSS_REDUCTION: f64 = 0.7;     // 30% less loss for high voltage lines
pub const URBAN_INFRASTRUCTURE_FACTOR: f64 = 0.8;     // 20% less loss in urban areas
pub const RURAL_INFRASTRUCTURE_FACTOR: f64 = 1.2;     // 20% more loss in rural areas
pub const UNDERWATER_LOSS_FACTOR: f64 = 1.5;          // 50% more loss for undersea cables - unlikely to be used
pub const MOUNTAIN_LOSS_FACTOR: f64 = 1.3;            // 30% more loss in mountainous regions - unlikely to be used

// Industrial Power Usage
pub const INDUSTRY_POWER_FACTOR: f64 = 1.8;  // Additional 80% power usage for industrial areas
pub const INDUSTRY_THRESHOLD_POP: u32 = 100_000; // Population threshold for significant industry
pub const DATA_CENTER_POWER_FACTOR: f64 = 1.5; // Additional 50% power for areas with data centers
pub const COMMERCIAL_POWER_FACTOR: f64 = 1.3;  // Additional 30% for commercial districts

// Grid Infrastructure Evolution
pub const GRID_IMPROVEMENT_RATE: f64 = 0.01;  // 1% annual improvement in transmission efficiency
pub const SMART_GRID_FACTOR: f64 = 0.85;      // 15% reduction in losses with smart grid
pub const SMART_GRID_ADOPTION_YEAR: u32 = 2035;

// Generator Placement Weights
pub const TRANSMISSION_LOSS_WEIGHT: f64 = 0.03;    // Weight for transmission losses in placement
pub const PUBLIC_OPINION_WEIGHT: f64 = 0.12;       // Weight for public opinion in placement
pub const CONSTRUCTION_COST_WEIGHT: f64 = 0.82;    // Weight for construction costs in placement
pub const ENVIRONMENTAL_WEIGHT: f64 = 0.03;        // Weight for environmental factors

// Settlement Power Distribution
pub const RESIDENTIAL_POWER_RATIO: f64 = 0.35;    // 35% of power for residential use
pub const COMMERCIAL_POWER_RATIO: f64 = 0.25;     // 25% of power for commercial use
pub const INDUSTRIAL_POWER_RATIO: f64 = 0.40;     // 40% of power for industrial use

// Settlement Size Classifications (population)
pub const URBAN_POPULATION_THRESHOLD: u32 = 50_000;
pub const MEDIUM_SETTLEMENT_THRESHOLD: u32 = 10_000;
pub const LARGE_CITY_REFERENCE: f64 = 1_000_000.0;

// Economic Constants
pub const SHORT_TERM_INFLATION: f64 = 0.020;  // ECB target
pub const MEDIUM_TERM_INFLATION: f64 = 0.018; // Stabilization period
pub const LONG_TERM_INFLATION: f64 = 0.015;   // Long-term projection

// Population Growth Rates (from CSO)
pub const SHORT_TERM_GROWTH: f64 = 0.0115;
pub const MEDIUM_TERM_GROWTH: f64 = 0.0085;
pub const LONG_TERM_GROWTH: f64 = 0.006;

// Urbanization Factors
pub const URBAN_GROWTH_RATE: f64 = 0.005;
pub const RURAL_DECLINE_RATE: f64 = 0.003;
pub const MAX_URBAN_BOOST_YEARS: f64 = 15.0;
pub const MAX_RURAL_DECLINE_YEARS: f64 = 20.0;
pub const MAX_URBAN_BOOST: f64 = 0.20;  // 20% maximum boost for large cities
pub const MAX_RURAL_DECLINE: f64 = 0.15; // 15% maximum decline for small towns

// Economic Growth Rates
pub const SHORT_TERM_ECONOMIC_GROWTH: f64 = 0.025;
pub const MEDIUM_TERM_ECONOMIC_GROWTH: f64 = 0.020;
pub const LONG_TERM_ECONOMIC_GROWTH: f64 = 0.015;

// Energy Efficiency Improvements
pub const SHORT_TERM_EFFICIENCY_GAIN: f64 = 0.020;
pub const MEDIUM_TERM_EFFICIENCY_GAIN: f64 = 0.015;
pub const LONG_TERM_EFFICIENCY_GAIN: f64 = 0.010;

// Technology Cost Evolution
pub const WIND_COST_REDUCTION: f64 = 0.95;   // 5% reduction per year
pub const SOLAR_COST_REDUCTION: f64 = 0.93;  // 7% reduction per year
pub const NUCLEAR_COST_INCREASE: f64 = 1.01; // 1% increase per year
pub const COAL_COST_INCREASE: f64 = 1.04;    // 4% increase per year
pub const GAS_COST_INCREASE: f64 = 1.02;     // 2% increase per year
pub const HYDRO_COST_INCREASE: f64 = 1.005;  // 0.5% increase per year

// Technology Efficiency Evolution
pub const WIND_EFFICIENCY_GAIN: f64 = 0.98;   // 2% improvement per year
pub const SOLAR_EFFICIENCY_GAIN: f64 = 0.97;  // 3% improvement per year
pub const NUCLEAR_EFFICIENCY_GAIN: f64 = 0.995; // 0.5% improvement per year
pub const COAL_EFFICIENCY_LOSS: f64 = 0.99;    // 4% improvement per year
pub const GAS_EFFICIENCY_LOSS: f64 = 0.99;     // 2% improvement per year
pub const HYDRO_EFFICIENCY_GAIN: f64 = 0.99;   // 1% improvement per year
pub const BIOMASS_EFFICIENCY_GAIN: f64 = 0.99;   // 1% improvement per year
// Public Opinion Base Values
pub const WIND_BASE_OPINION: f64 = 0.85;
pub const SOLAR_BASE_OPINION: f64 = 0.90;
pub const NUCLEAR_BASE_OPINION: f64 = 0.30;
pub const COAL_BASE_OPINION: f64 = 0.25;
pub const GAS_BASE_OPINION: f64 = 0.45;
pub const HYDRO_BASE_OPINION: f64 = 0.75;

// Public Opinion Annual Changes
pub const WIND_OPINION_CHANGE: f64 = 0.003;
pub const SOLAR_OPINION_CHANGE: f64 = 0.002;
pub const NUCLEAR_OPINION_CHANGE: f64 = 0.008;
pub const COAL_OPINION_CHANGE: f64 = -0.015;
pub const GAS_OPINION_CHANGE: f64 = -0.008;
pub const HYDRO_OPINION_CHANGE: f64 = 0.001;

// Infrastructure Impact Factors
pub const DATA_CENTER_GROWTH: f64 = 0.03;     // 3% annual growth
pub const MAX_DATA_CENTER_YEARS: f64 = 10.0;  // Impact plateaus after 10 years
pub const ELECTRIFICATION_GROWTH: f64 = 0.02; // 2% annual growth
pub const MAX_ELECTRIFICATION_YEARS: f64 = 15.0;

// Generator Size and Efficiency Bounds
pub const MIN_GENERATOR_SIZE: f64 = 0.1;
pub const MAX_GENERATOR_SIZE: f64 = 1.0;
pub const BASE_EFFICIENCY: f64 = 0.35;
pub const MAX_EFFICIENCY: f64 = 0.60;

// Cost Reference Values (in euros)
pub const REFERENCE_ANNUAL_EXPENDITURE: f64 = 10_000_000_000.0; // 10 billion euros per year
pub const MIN_ANNUAL_EXPENDITURE: f64 = 500_000_000.0;  // 500 million euros per year

// Power Output Reference Values (in MW)
pub const MAX_ONSHORE_WIND_POWER: f64 = 500.0;
pub const MAX_OFFSHORE_WIND_POWER: f64 = 800.0;

pub const MAX_DOMESTIC_SOLAR_POWER: f64 = 10.0;
pub const MAX_COMMERCIAL_SOLAR_POWER: f64 = 50.0;
pub const MAX_UTILITY_SOLAR_POWER: f64 = 300.0;

pub const MAX_NUCLEAR_POWER: f64 = 1500.0;
pub const MAX_COAL_POWER: f64 = 1000.0;
pub const MAX_GAS_CC_POWER: f64 = 800.0;
pub const MAX_GAS_PEAKER_POWER: f64 = 400.0;

pub const MAX_HYDRO_DAM_POWER: f64 = 1200.0;
pub const MAX_PUMPED_STORAGE_POWER: f64 = 600.0;
pub const MAX_TIDAL_POWER: f64 = 200.0;
pub const MAX_WAVE_POWER: f64 = 100.0;
pub const MAX_BATTERY_STORAGE_POWER: f64 = 500.0;

pub const MAX_BIOMASS_POWER: f64 = 50.0;

// Size-Efficiency Relationships
pub const SIZE_EFFICIENCY_FACTOR: f64 = 0.25; // Maximum additional efficiency from size
pub const COST_EFFICIENCY_FACTOR: f64 = 0.15; // Maximum additional efficiency from cost

// Output Configuration
pub const CSV_HEADER: &str = "Year,Total Population,Total Power Usage (MW),Total Power Generation (MW),Power Balance (MW),Average Public Opinion"; 

// Base Costs (in euros)
pub const BASE_ONSHORE_WIND_COST: f64 = 2_000_000.0;  // Per MW
pub const BASE_OFFSHORE_WIND_COST: f64 = 4_000_000.0;  // Per MW

pub const BASE_DOMESTIC_SOLAR_COST: f64 = 10_000.0;    // Per installation
pub const BASE_COMMERCIAL_SOLAR_COST: f64 = 800_000.0; // Per MW
pub const BASE_UTILITY_SOLAR_COST: f64 = 600_000.0;    // Per MW

pub const BASE_NUCLEAR_COST: f64 = 6_000_000.0;        // Per MW

pub const BASE_COAL_COST: f64 = 2_000_000.0;          // Per MW
pub const BASE_GAS_CC_COST: f64 = 1_000_000.0;        // Per MW
pub const BASE_GAS_PEAKER_COST: f64 = 500_000.0;      // Per MW

pub const BASE_HYDRO_DAM_COST: f64 = 5_000_000.0;     // Per MW
pub const BASE_PUMPED_STORAGE_COST: f64 = 2_500_000.0; // Per MW
pub const BASE_TIDAL_COST: f64 = 8_000_000.0;         // Per MW (initially very expensive)
pub const BASE_WAVE_COST: f64 = 10_000_000.0;         // Per MW (initially extremely expensive)

// Operating Costs (per MW per year)
pub const ONSHORE_WIND_OPERATING_COST: f64 = 50_000.0;
pub const OFFSHORE_WIND_OPERATING_COST: f64 = 100_000.0;

pub const DOMESTIC_SOLAR_OPERATING_COST: f64 = 200.0;
pub const COMMERCIAL_SOLAR_OPERATING_COST: f64 = 20_000.0;
pub const UTILITY_SOLAR_OPERATING_COST: f64 = 15_000.0;

pub const NUCLEAR_OPERATING_COST: f64 = 250_000.0;

pub const COAL_OPERATING_COST: f64 = 150_000.0;
pub const GAS_CC_OPERATING_COST: f64 = 100_000.0;
pub const GAS_PEAKER_OPERATING_COST: f64 = 80_000.0;

pub const HYDRO_DAM_OPERATING_COST: f64 = 60_000.0;
pub const PUMPED_STORAGE_OPERATING_COST: f64 = 40_000.0;
pub const TIDAL_OPERATING_COST: f64 = 200_000.0;
pub const WAVE_OPERATING_COST: f64 = 250_000.0;

// Technology Maturity Factors (affects cost reduction and efficiency gains)
pub const MATURE_TECH_IMPROVEMENT_RATE: f64 = 0.98;     // 2% improvement per year
pub const DEVELOPING_TECH_IMPROVEMENT_RATE: f64 = 0.95; // 5% improvement per year
pub const EMERGING_TECH_IMPROVEMENT_RATE: f64 = 0.90;   // 10% improvement per year

// Urban Placement Factors
pub const URBAN_SOLAR_BONUS: f64 = 1.2;        // 20% bonus for urban solar
pub const URBAN_PEAKER_PENALTY: f64 = 0.9;     // 10% penalty for urban gas peakers

// Water Requirement Factors
pub const COASTAL_BONUS: f64 = 1.15;           // 15% bonus for water-based generators in coastal areas
pub const RIVER_BONUS: f64 = 1.10;             // 10% bonus for water-based generators near rivers 

// Power Storage Constants
pub const MAX_INTERMITTENT_PERCENTAGE: f64 = 0.30;  // Maximum 30% intermittent without storage
pub const STORAGE_CAPACITY_FACTOR: f64 = 0.5;      // Each MW of storage allows 0.5 MW more intermittent

// Pumped Storage Parameters
pub const PUMPED_STORAGE_CAPACITY: f64 = 8000.0;      // MWh of storage
pub const PUMPED_STORAGE_CHARGE_RATE: f64 = 300.0;    // MW charging rate
pub const PUMPED_STORAGE_DISCHARGE_RATE: f64 = 300.0;  // MW discharge rate
pub const PUMPED_STORAGE_EFFICIENCY: f64 = 0.75;       // 75% round-trip efficiency

// Battery Storage Parameters
pub const BATTERY_STORAGE_CAPACITY: f64 = 500.0;      // MWh of storage
pub const BATTERY_STORAGE_CHARGE_RATE: f64 = 100.0;   // MW charging rate
pub const BATTERY_STORAGE_DISCHARGE_RATE: f64 = 100.0; // MW discharge rate
pub const BATTERY_STORAGE_EFFICIENCY: f64 = 0.85;      // 85% round-trip efficiency

// Time-based Generation Factors
pub const SOLAR_PEAK_HOURS: u8 = 5;  // Number of peak solar hours per day
pub const WIND_PEAK_HOURS: u8 = 8;   // Number of peak wind hours per day

// Weather Impact Factors
pub const CLOUDY_DAY_FACTOR: f64 = 0.3;  // Solar output on cloudy days
pub const CALM_DAY_FACTOR: f64 = 0.2;    // Wind output on calm days

// Storage Cost Parameters
pub const BATTERY_STORAGE_COST_PER_MWH: f64 = 500_000.0;  // Cost per MWh of battery storage
pub const PUMPED_STORAGE_COST_PER_MWH: f64 = 200_000.0;   // Cost per MWh of pumped storage 

// Marine and Battery Storage Power Outputs
pub const MARINE_EFFICIENCY_GAIN: f64 = 0.93;      // 7% annual efficiency gain for marine tech
pub const BATTERY_EFFICIENCY_GAIN: f64 = 0.95;     // 5% annual efficiency gain for batteries

// Marine and Battery Storage Opinions
pub const MARINE_BASE_OPINION: f64 = 0.75;         // Initial public opinion of marine tech
pub const MARINE_OPINION_CHANGE: f64 = 0.005;      // Annual change in marine tech opinion
pub const BATTERY_BASE_OPINION: f64 = 0.85;        // Initial public opinion of batteries
pub const BATTERY_OPINION_CHANGE: f64 = 0.003;     // Annual change in battery opinion 

// Scoring constants
pub const MAX_ACCEPTABLE_EMISSIONS: f64 = 10_160_470.0;  // 10 million tonnes CO2 (converted from 10 million tons)
pub const MAX_ACCEPTABLE_COST: f64 = 10_000_000_000.0;  // 10 billion euros 

// Decommissioning Costs
pub const DECOMMISSION_COST_RATIO: f64 = 0.25; // 25% of base cost for decommissioning

// CO2 Emission Rates (tonnes per MW)
pub const COAL_CO2_RATE: f64 = 1016.047;  // ~1016 tonnes per MW (converted from 1000 tons)
pub const GAS_CC_CO2_RATE: f64 = 508.023;  // ~508 tonnes per MW (converted from 500 tons)
pub const GAS_PEAKER_CO2_RATE: f64 = 711.233;  // ~711 tonnes per MW (converted from 700 tons)
pub const BIOMASS_CO2_RATE: f64 = 50.802;  // ~51 tonnes per MW (converted from 50 tons)

// Geographic Constants
pub const IRELAND_MIN_LAT: f64 = 51.4;
pub const IRELAND_MAX_LAT: f64 = 55.4;
pub const IRELAND_MIN_LON: f64 = -10.5;
pub const IRELAND_MAX_LON: f64 = -5.4;

// Geographic Features
pub const COASTAL_THRESHOLD: f64 = 0.1; // Proportion of map width to consider coastal 

// Power Distribution Evolution Rates
pub const RESIDENTIAL_EFFICIENCY_GAIN: f64 = 0.02;  // 2% annual efficiency improvement
pub const COMMERCIAL_GROWTH_RATE: f64 = 0.015;      // 1.5% annual growth in commercial power usage
pub const INDUSTRIAL_EVOLUTION_RATE: f64 = -0.01;   // 1% annual reduction due to efficiency improvements 

// Generator Operation Percentages
pub const NUCLEAR_MIN_OPERATION: u8 = 60;  // Nuclear needs high base load
pub const HYDRO_MIN_OPERATION: u8 = 20;    // Flexible operation for hydro
pub const DEFAULT_MIN_OPERATION: u8 = 30;   // Default minimum for other types
pub const MAX_OPERATION_PERCENTAGE: u8 = 100;

// Time-based Operation Factors
pub const NIGHT_WIND_FACTOR: f64 = 1.2;    // Higher wind output at night
pub const DAY_WIND_FACTOR: f64 = 0.8;      // Lower wind output during day
pub const NIGHT_START_HOUR: u8 = 6;        // Start of night period
pub const DAY_END_HOUR: u8 = 18;           // End of day period
pub const SOLAR_PEAK_HOUR: f64 = 12.0;     // Hour of peak solar output
pub const SOLAR_WINDOW: f64 = 6.0;         // Hours from peak for solar operation

// Generator Efficiency Factors
pub const WIND_CAPACITY_FACTOR: f64 = 0.35;  // Average wind capacity factor
pub const SOLAR_CAPACITY_FACTOR: f64 = 0.20;  // Average solar capacity factor
pub const EFFICIENCY_UPGRADE_COST_FACTOR: f64 = 2.0;  // Multiplier for efficiency upgrade costs
pub const CLOSURE_COST_FACTOR: f64 = 0.5;  // Factor for calculating closure costs 

// Generator Size Constraints
pub const ONSHORE_WIND_MIN_SIZE: f64 = 0.2;
pub const OFFSHORE_WIND_MIN_SIZE: f64 = 0.5;
pub const DOMESTIC_SOLAR_MIN_SIZE: f64 = 0.001;
pub const DOMESTIC_SOLAR_MAX_SIZE: f64 = 0.01;
pub const COMMERCIAL_SOLAR_MIN_SIZE: f64 = 0.01;
pub const COMMERCIAL_SOLAR_MAX_SIZE: f64 = 0.1;
pub const UTILITY_SOLAR_MIN_SIZE: f64 = 0.2;
pub const NUCLEAR_MIN_SIZE: f64 = 0.8;
pub const COAL_MIN_SIZE: f64 = 0.6;
pub const GAS_CC_MIN_SIZE: f64 = 0.4;
pub const GAS_PEAKER_MIN_SIZE: f64 = 0.2;
pub const GAS_PEAKER_MAX_SIZE: f64 = 0.6;
pub const BIOMASS_MIN_SIZE: f64 = 0.4;
pub const HYDRO_MIN_SIZE: f64 = 0.5;
pub const PUMPED_STORAGE_MIN_SIZE: f64 = 0.4;
pub const PUMPED_STORAGE_MAX_SIZE: f64 = 0.8;
pub const TIDAL_MIN_SIZE: f64 = 0.1;
pub const TIDAL_MAX_SIZE: f64 = 0.5;
pub const WAVE_MIN_SIZE: f64 = 0.05;
pub const WAVE_MAX_SIZE: f64 = 0.3;
pub const BATTERY_MIN_SIZE: f64 = 0.1;
pub const BATTERY_MAX_SIZE: f64 = 0.5;

// Cost Evolution Rates
pub const ONSHORE_WIND_COST_EVOLUTION: f64 = 0.97;      // 3% reduction per year
pub const OFFSHORE_WIND_COST_EVOLUTION: f64 = 0.95;     // 5% reduction per year
pub const DOMESTIC_SOLAR_COST_EVOLUTION: f64 = 0.93;    // 7% reduction per year
pub const COMMERCIAL_SOLAR_COST_EVOLUTION: f64 = 0.93;  // 7% reduction per year
pub const UTILITY_SOLAR_COST_EVOLUTION: f64 = 0.92;     // 8% reduction per year
pub const GAS_CC_COST_EVOLUTION: f64 = 1.02;            // 2% increase per year
pub const GAS_PEAKER_COST_EVOLUTION: f64 = 1.02;        // 2% increase per year
pub const BIOMASS_COST_EVOLUTION: f64 = 0.98;           // 2% reduction per year
pub const PUMPED_STORAGE_COST_EVOLUTION: f64 = 1.01;    // 1% increase per year
pub const BATTERY_COST_EVOLUTION: f64 = 0.90;           // 10% reduction per year
pub const TIDAL_COST_EVOLUTION: f64 = 0.90;             // 10% reduction per year
pub const WAVE_COST_EVOLUTION: f64 = 0.88;              // 12% reduction per year

// Base Opinion Values
pub const ONSHORE_WIND_OPINION: f64 = 0.75;
pub const OFFSHORE_WIND_OPINION: f64 = 0.85;
pub const DOMESTIC_SOLAR_OPINION: f64 = 0.95;
pub const COMMERCIAL_SOLAR_OPINION: f64 = 0.90;
pub const UTILITY_SOLAR_OPINION: f64 = 0.85;
pub const BIOMASS_OPINION: f64 = 0.55;
pub const PUMPED_STORAGE_OPINION: f64 = 0.75;
pub const BATTERY_OPINION: f64 = 0.85;
pub const TIDAL_OPINION: f64 = 0.80;
pub const WAVE_OPINION: f64 = 0.85;

// Opinion Change Rates
pub const ONSHORE_WIND_OPINION_CHANGE: f64 = 0.002;
pub const OFFSHORE_WIND_OPINION_CHANGE: f64 = 0.003;
pub const DOMESTIC_SOLAR_OPINION_CHANGE: f64 = 0.001;
pub const COMMERCIAL_SOLAR_OPINION_CHANGE: f64 = 0.002;
pub const UTILITY_SOLAR_OPINION_CHANGE: f64 = 0.002;
pub const BIOMASS_OPINION_CHANGE: f64 = 0.001;
pub const PUMPED_STORAGE_OPINION_CHANGE: f64 = 0.002;
pub const TIDAL_OPINION_CHANGE: f64 = 0.005;
pub const WAVE_OPINION_CHANGE: f64 = 0.005;

// Additional Operating Costs
pub const BIOMASS_OPERATING_COST: f64 = 120_000.0;
pub const BATTERY_STORAGE_OPERATING_COST: f64 = 10_000_000.0; 

// Generator Default Size
pub const DEFAULT_GENERATOR_SIZE: u32 = 100;

// Carbon Offset Size Range
pub const MIN_CARBON_OFFSET_SIZE: f64 = 100.0;
pub const MAX_CARBON_OFFSET_SIZE: f64 = 1000.0;

// Carbon Offset Efficiency Range
pub const MIN_CARBON_OFFSET_EFFICIENCY: f64 = 0.7;
pub const MAX_CARBON_OFFSET_EFFICIENCY: f64 = 0.95;

// Generator Minimum Ages for Closure
pub const NUCLEAR_MIN_CLOSURE_AGE: u32 = 30;
pub const HYDRO_DAM_MIN_CLOSURE_AGE: u32 = 40;
pub const WIND_MIN_CLOSURE_AGE: u32 = 15;
pub const SOLAR_MIN_CLOSURE_AGE: u32 = 20;
pub const DEFAULT_MIN_CLOSURE_AGE: u32 = 25;

// Carbon Offset Base Costs
pub const FOREST_BASE_COST: f64 = 10_000_000.0;
pub const WETLAND_BASE_COST: f64 = 15_000_000.0;
pub const ACTIVE_CAPTURE_BASE_COST: f64 = 100_000_000.0;
pub const CARBON_CREDIT_BASE_COST: f64 = 5_000_000.0;

// Carbon Offset Operating Costs
pub const FOREST_OPERATING_COST: f64 = 500_000.0;
pub const WETLAND_OPERATING_COST: f64 = 750_000.0;
pub const ACTIVE_CAPTURE_OPERATING_COST: f64 = 5_000_000.0;
pub const CARBON_CREDIT_OPERATING_COST: f64 = 250_000.0;

// Generator Base Maximum Efficiencies
pub const WIND_BASE_MAX_EFFICIENCY: f64 = 0.45;
pub const UTILITY_SOLAR_BASE_MAX_EFFICIENCY: f64 = 0.40;
pub const NUCLEAR_BASE_MAX_EFFICIENCY: f64 = 0.50;
pub const GAS_CC_BASE_MAX_EFFICIENCY: f64 = 0.60;
pub const HYDRO_BASE_MAX_EFFICIENCY: f64 = 0.85;
pub const MARINE_BASE_MAX_EFFICIENCY: f64 = 0.35;
pub const DEFAULT_BASE_MAX_EFFICIENCY: f64 = 0.40;

// // Metal Location Search Constants
// pub const MAP_MAX_X: f64 = 50_000.0;
// pub const MAP_MAX_Y: f64 = 50_000.0;
// pub const GRID_CELL_SIZE: f64 = 1000.0;
// pub const COASTAL_THRESHOLD: f64 = 0.1; 