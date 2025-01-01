// Time Constants
pub const BASE_YEAR: u32 = 2025;
pub const END_YEAR: u32 = 2050;
pub const MEDIUM_TERM_YEAR: u32 = 2030;
pub const LONG_TERM_YEAR: u32 = 2040;

// Map and Grid Constants
pub const MAP_MAX_X: f64 = 100_000.0;
pub const MAP_MAX_Y: f64 = 100_000.0;
pub const GRID_CELL_SIZE: f64 = 1000.0; // 1km grid cells
pub const MAX_CUMULATIVE_GENERATOR_SIZE: f64 = 1.0;

// Transmission Loss Factors
pub const BASE_TRANSMISSION_LOSS_RATE: f64 = 0.00001; // Loss per meter of distance
pub const HIGH_VOLTAGE_LOSS_REDUCTION: f64 = 0.7;     // 30% less loss for high voltage lines
pub const URBAN_INFRASTRUCTURE_FACTOR: f64 = 0.8;     // 20% less loss in urban areas
pub const RURAL_INFRASTRUCTURE_FACTOR: f64 = 1.2;     // 20% more loss in rural areas
pub const UNDERWATER_LOSS_FACTOR: f64 = 1.5;          // 50% more loss for undersea cables
pub const MOUNTAIN_LOSS_FACTOR: f64 = 1.3;            // 30% more loss in mountainous regions

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
pub const TRANSMISSION_LOSS_WEIGHT: f64 = 0.3;    // Weight for transmission losses in placement
pub const PUBLIC_OPINION_WEIGHT: f64 = 0.3;       // Weight for public opinion in placement
pub const CONSTRUCTION_COST_WEIGHT: f64 = 0.2;    // Weight for construction costs in placement
pub const ENVIRONMENTAL_WEIGHT: f64 = 0.2;        // Weight for environmental factors

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
pub const COAL_EFFICIENCY_LOSS: f64 = 1.04;    // 4% degradation per year
pub const GAS_EFFICIENCY_LOSS: f64 = 1.02;     // 2% degradation per year
pub const HYDRO_EFFICIENCY_GAIN: f64 = 0.99;   // 1% improvement per year

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
pub const REFERENCE_LARGE_GENERATOR_COST: f64 = 1_000_000_000.0; // 1 billion euros
pub const MIN_GENERATOR_COST: f64 = 10_000_000.0;  // 10 million euros

// Power Output Reference Values (in MW)
pub const MAX_WIND_POWER: f64 = 500.0;
pub const MAX_SOLAR_POWER: f64 = 300.0;
pub const MAX_NUCLEAR_POWER: f64 = 1500.0;
pub const MAX_COAL_POWER: f64 = 1000.0;
pub const MAX_GAS_POWER: f64 = 800.0;
pub const MAX_HYDRO_POWER: f64 = 400.0;

// Size-Efficiency Relationships
pub const SIZE_EFFICIENCY_FACTOR: f64 = 0.25; // Maximum additional efficiency from size
pub const COST_EFFICIENCY_FACTOR: f64 = 0.15; // Maximum additional efficiency from cost

// Output Configuration
pub const CSV_HEADER: &str = "Year,Total Population,Total Power Usage (MW),Total Power Generation (MW),Power Balance (MW),Average Public Opinion"; 