// Time Constants
pub const BASE_YEAR: u32 = 2025;
pub const END_YEAR: u32 = 2050;

// Map and Grid Constants
pub const MAP_MAX_X: f64 = 50_000.0;
pub const MAP_MAX_Y: f64 = 50_000.0;
pub const GRID_CELL_SIZE: f64 = 1000.0;              // 1km grid cells

// Generator Placement Weights
pub const TRANSMISSION_LOSS_WEIGHT: f64 = 0.03;    // Weight for transmission losses in placement
pub const PUBLIC_OPINION_WEIGHT: f64 = 0.12;       // Weight for public opinion in placement
pub const CONSTRUCTION_COST_WEIGHT: f64 = 0.82;    // Weight for construction costs in placement

// Economic Constants
pub const INFLATION_RATE: f64 = 0.0185;

// Technology Cost Evolution
pub const WIND_COST_REDUCTION: f64 = 0.99;   // 5% reduction per year
pub const SOLAR_COST_REDUCTION: f64 = 0.97;  // 7% reduction per year
pub const NUCLEAR_COST_REDUCTION: f64 = 0.99; // 1% increase per year
pub const COAL_COST_INCREASE: f64 = 1.10;    // 4% increase per year
pub const GAS_COST_INCREASE: f64 = 1.04;     // 2% increase per year
pub const HYDRO_COST_INCREASE: f64 = 1.06;  // 0.5% increase per year

// Technology Efficiency Evolution
pub const WIND_EFFICIENCY_GAIN: f64 = 0.98;   // 2% improvement per year
pub const SOLAR_EFFICIENCY_GAIN: f64 = 0.97;  // 3% improvement per year
pub const NUCLEAR_EFFICIENCY_GAIN: f64 = 0.995; // 0.5% improvement per year
pub const COAL_EFFICIENCY_LOSS: f64 = 0.99;    // 4% improvement per year
pub const GAS_EFFICIENCY_LOSS: f64 = 0.99;     // 2% improvement per year
pub const HYDRO_EFFICIENCY_GAIN: f64 = 0.99;   // 1% improvement per year
pub const BIOMASS_EFFICIENCY_GAIN: f64 = 0.99;   // 1% improvement per year

// Public Opinion Base Values
pub const WIND_BASE_OPINION: f64 = 0.83;
pub const SOLAR_BASE_OPINION: f64 = 0.89;
pub const NUCLEAR_BASE_OPINION: f64 = 0.43;
pub const COAL_BASE_OPINION: f64 = 0.41;
pub const GAS_BASE_OPINION: f64 = 0.42;
pub const HYDRO_BASE_OPINION: f64 = 0.89;
pub const BIOMASS_BASE_OPINION: f64 = 0.55;
pub const PUMPED_STORAGE_BASE_OPINION: f64 = 0.75;
pub const TIDAL_BASE_OPINION: f64 = 0.80;
pub const WAVE_BASE_OPINION: f64 = 0.85;


// Public Opinion Annual Changes
pub const WIND_OPINION_CHANGE: f64 = 0.005;
pub const SOLAR_OPINION_CHANGE: f64 = 0.008;
pub const NUCLEAR_OPINION_CHANGE: f64 = 0.002;
pub const COAL_OPINION_CHANGE: f64 = -0.015;
pub const GAS_OPINION_CHANGE: f64 = -0.008;
pub const HYDRO_OPINION_CHANGE: f64 = 0.004;
pub const BIOMASS_OPINION_CHANGE: f64 = 0.001;
pub const PUMPED_STORAGE_OPINION_CHANGE: f64 = 0.002;
pub const TIDAL_OPINION_CHANGE: f64 = 0.005;
pub const WAVE_OPINION_CHANGE: f64 = 0.005;






// Generator Size and Efficiency Bounds
pub const MIN_GENERATOR_SIZE: f64 = 0.1;
pub const MAX_GENERATOR_SIZE: f64 = 1.0;
pub const BASE_EFFICIENCY: f64 = 0.99;

// Cost Reference Values (in euros)
pub const REFERENCE_ANNUAL_EXPENDITURE: f64 = 1_384_000_000.0; // 10 billion euros per year

// Operating Costs (per year)
pub const ONSHORE_WIND_OPERATING_COST: f64 = 45_000.0;
pub const OFFSHORE_WIND_OPERATING_COST: f64 = 65_000.0;

pub const DOMESTIC_SOLAR_OPERATING_COST: f64 = 200.0;
pub const UTILITY_SOLAR_OPERATING_COST: f64 = 10_000.0;

pub const NUCLEAR_OPERATING_COST: f64 = 125_000.0;

pub const COAL_OPERATING_COST: f64 = 100_000.0;
pub const GAS_CC_OPERATING_COST: f64 = 100_000.0;
pub const GAS_PEAKER_OPERATING_COST: f64 = 100_000.0;

pub const HYDRO_DAM_OPERATING_COST: f64 = 145_000.0;
pub const PUMPED_STORAGE_OPERATING_COST: f64 = 93_000.0;
pub const TIDAL_OPERATING_COST: f64 = 118_000.0;
pub const WAVE_OPERATING_COST: f64 = 145_000.0;

// Urban Placement Factors
pub const URBAN_SOLAR_BONUS: f64 = 1.1;        // 20% bonus for urban solar
pub const URBAN_PEAKER_PENALTY: f64 = 0.7;     // 10% penalty for urban gas peakers

// Water Requirement Factors
pub const COASTAL_BONUS: f64 = 1.15;           // 15% bonus for water-based generators in coastal areas
pub const RIVER_BONUS: f64 = 1.10;             // 10% bonus for water-based generators near rivers 

// Power Storage Constants
pub const MAX_INTERMITTENT_PERCENTAGE: f64 = 0.40;  // Maximum 30% intermittent without storage
pub const STORAGE_CAPACITY_FACTOR: f64 = 0.5;      // Each MW of storage allows 0.5 MW more intermittent

// Marine and Battery Storage Power Outputs
pub const MARINE_EFFICIENCY_GAIN: f64 = 0.93;      // 7% annual efficiency gain for marine tech
pub const BATTERY_EFFICIENCY_GAIN: f64 = 0.95;     // 5% annual efficiency gain for batteries

// Marine and Battery Storage Opinions
pub const MARINE_BASE_OPINION: f64 = 0.75;         // Initial public opinion of marine tech
pub const MARINE_OPINION_CHANGE: f64 = 0.005;      // Annual change in marine tech opinion
pub const BATTERY_BASE_OPINION: f64 = 0.85;        // Initial public opinion of batteries
pub const BATTERY_OPINION_CHANGE: f64 = 0.003;     // Annual change in battery opinion 

// Scoring constants
pub const MAX_ACCEPTABLE_EMISSIONS: f64 = 1_000_000.0;  // 1 million tonnes CO2 (scaled to match new emission rates)
pub const MAX_ACCEPTABLE_COST: f64 = 50_000_000_000.0;  // 50 billion euros

// Cost Opinion Constants
pub const COST_OPINION_DECAY_FACTOR: f64 = 0.5; // Decay factor for high costs
pub const COST_OPINION_EXP_SCALE: f64 = -0.5;   // Exponential scale for cost opinion curve

// Decommissioning Costs
pub const DECOMMISSION_COST_RATIO: f64 = 0.12; // 25% of base cost for decommissioning

// CO2 Emission Rates (tonnes per MW per year)
pub const COAL_CO2_RATE: f64 = 6_300.0;  // ~6,300 tonnes per MW per year (typical coal plant)
pub const GAS_CC_CO2_RATE: f64 = 3_500.0;  // ~3,500 tonnes per MW per year (combined cycle)
pub const GAS_PEAKER_CO2_RATE: f64 = 4_800.0;  // ~4,800 tonnes per MW per year (peaker plant)
pub const BIOMASS_CO2_RATE: f64 = 1_500.0;  // ~1,500 tonnes per MW per year (biomass emissions)

// Geographic Constants
pub const IRELAND_MIN_LAT: f64 = 51.4;
pub const IRELAND_MAX_LAT: f64 = 55.4;
pub const IRELAND_MIN_LON: f64 = -10.6;
pub const IRELAND_MAX_LON: f64 = -5.9;

// Geographic Features
pub const COASTAL_THRESHOLD: f64 = 0.1; // Proportion of map width to consider coastal 

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

// Power Output Reference Values (in MW)
pub const MAX_ONSHORE_WIND_POWER: f64 = 500.0;
pub const MAX_OFFSHORE_WIND_POWER: f64 = 800.0;

// Existing power projects currently in progress (in MW)
// This represents projects that are already under construction or in planning
// and will come online during the simulation period
// This helps ensure construction times work properly for new projects
pub const EXISTING_POWER_PROJECTS: f64 = 1200.0;

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


// Additional Operating Costs
pub const BIOMASS_OPERATING_COST: f64 = 120_000.0;
pub const BATTERY_STORAGE_OPERATING_COST: f64 = 10_000_000.0; 

pub const WIND_CAPACITY_FACTOR: f64 = 0.35;  // Average wind capacity factor
pub const SOLAR_CAPACITY_FACTOR: f64 = 0.20;  // Average solar capacity factor

pub const NIGHT_START_HOUR: u8 = 6;        // Start of night period
pub const DAY_END_HOUR: u8 = 18;           // End of day period

pub const SOLAR_PEAK_HOUR: f64 = 12.0;     // Hour of peak solar output
pub const SOLAR_WINDOW: f64 = 6.0;         // Hours from peak for solar operation

pub const EFFICIENCY_UPGRADE_COST_FACTOR: f64 = 2.0;  // Multiplier for efficiency upgrade costs

// Generator Operation Percentages
pub const NUCLEAR_MIN_OPERATION: u8 = 60;  // Nuclear needs high base load
pub const HYDRO_MIN_OPERATION: u8 = 20;    // Flexible operation for hydro
pub const DEFAULT_MIN_OPERATION: u8 = 30;   // Default minimum for other types
pub const MAX_OPERATION_PERCENTAGE: u8 = 100;
pub const CLOSURE_COST_FACTOR: f64 = 0.5;  // Factor for calculating closure costs 


// Generator Default Size
pub const DEFAULT_GENERATOR_SIZE: u32 = 100;

// Generator Base Maximum Efficiencies
pub const WIND_BASE_MAX_EFFICIENCY: f64 = 0.45;
pub const UTILITY_SOLAR_BASE_MAX_EFFICIENCY: f64 = 0.40;
pub const NUCLEAR_BASE_MAX_EFFICIENCY: f64 = 0.50;
pub const GAS_CC_BASE_MAX_EFFICIENCY: f64 = 0.60;
pub const HYDRO_BASE_MAX_EFFICIENCY: f64 = 0.85;
pub const MARINE_BASE_MAX_EFFICIENCY: f64 = 0.35;
pub const DEFAULT_BASE_MAX_EFFICIENCY: f64 = 0.40;

// Technology Maturity Factors (affects cost reduction and efficiency gains)
pub const MATURE_TECH_IMPROVEMENT_RATE: f64 = 0.98;     // 2% improvement per year
pub const DEVELOPING_TECH_IMPROVEMENT_RATE: f64 = 0.95; // 5% improvement per year
pub const EMERGING_TECH_IMPROVEMENT_RATE: f64 = 0.90;   // 10% improvement per year

// Carbon Offset Size Range
pub const MIN_CARBON_OFFSET_SIZE: f64 = 100.0;
pub const MAX_CARBON_OFFSET_SIZE: f64 = 1000.0;

// Carbon Offset Efficiency Range
pub const MIN_CARBON_OFFSET_EFFICIENCY: f64 = 0.7;
pub const MAX_CARBON_OFFSET_EFFICIENCY: f64 = 0.95;

// Carbon Sequestration Rates (tonnes per hectare per year)
pub const FOREST_SEQUESTRATION_RATE: f64 = 25.0;  // 25 tonnes CO2 per hectare per year
pub const WETLAND_SEQUESTRATION_RATE: f64 = 40.0; // 40 tonnes CO2 per hectare per year
pub const ACTIVE_CAPTURE_MULTIPLIER: f64 = 500.0; // Each size unit captures 500 tonnes CO2 per year
pub const CARBON_CREDIT_MULTIPLIER: f64 = 100.0;  // Each size unit represents 100 tonnes CO2 offset

// Active Capture Power Consumption
pub const ACTIVE_CAPTURE_POWER_PER_UNIT: f64 = 0.5; // 0.5 MW per unit of capture capacity

// Carbon Offset Maturity Curve
pub const CARBON_OFFSET_MATURITY_FACTOR: f64 = -0.1; // Controls how quickly natural offsets mature

// Carbon Offset Base Costs
pub const FOREST_BASE_COST: f64 = 1_000_000.0;
pub const WETLAND_BASE_COST: f64 = 1_000_000.0;
pub const ACTIVE_CAPTURE_BASE_COST: f64 = 1_000_000_000.0;
pub const CARBON_CREDIT_BASE_COST: f64 = 50_000_000.0;

// Carbon Offset Operating Costs
pub const FOREST_OPERATING_COST: f64 = 10_000.0;
pub const WETLAND_OPERATING_COST: f64 = 15_000.0;
pub const ACTIVE_CAPTURE_OPERATING_COST: f64 = 100_000.0;
pub const CARBON_CREDIT_OPERATING_COST: f64 = 5_000.0;

// Carbon Credit Price Constants
pub const PRICE_BEFORE_PHASE1: f64 = 75.0;
pub const PRICE_PHASE1_START: f64 = 75.0;
pub const PRICE_PHASE1_END: f64 = 130.0;
pub const PRICE_PHASE1_START_YEAR: u32 = 2030;
pub const PRICE_PHASE1_END_YEAR: u32 = 2040;

pub const PRICE_PHASE2_START: f64 = 130.0;
pub const PRICE_PHASE2_END: f64 = 300.0;
pub const PRICE_PHASE2_START_YEAR: u32 = 2040;
pub const PRICE_PHASE2_END_YEAR: u32 = 2050;

// Transform Constants
pub const GRID_SCALE_X: f64 = 10638.297872340427;
pub const GRID_SCALE_Y: f64 = 12500.0;

// Energy Sales Constants
pub const DEFAULT_ENERGY_SALES_RATE: f64 = 50_000.0;  // €50k per GWh
pub const MW_TO_GWH_CONVERSION: f64 = 8.76;  // Convert MW (power) to GWh/year (energy), 8760 hours per year / 1000

// Planning permission and construction time constants
pub const ENABLE_CONSTRUCTION_DELAYS: bool = true;  // Default setting

// Base planning permission times (in years)
pub const ONSHORE_WIND_PLANNING_TIME: f64 = 2.0;
pub const OFFSHORE_WIND_PLANNING_TIME: f64 = 3.0;
pub const SOLAR_PLANNING_TIME: f64 = 1.0;
pub const NUCLEAR_PLANNING_TIME: f64 = 5.0;
pub const COAL_PLANNING_TIME: f64 = 3.0;
pub const GAS_PLANNING_TIME: f64 = 2.0;
pub const BIOMASS_PLANNING_TIME: f64 = 2.0;
pub const HYDRO_PLANNING_TIME: f64 = 4.0;
pub const STORAGE_PLANNING_TIME: f64 = 1.5;
pub const TIDAL_PLANNING_TIME: f64 = 3.0;
pub const WAVE_PLANNING_TIME: f64 = 3.0;

// Base construction times (in years)
pub const ONSHORE_WIND_CONSTRUCTION_TIME: f64 = 1.0;
pub const OFFSHORE_WIND_CONSTRUCTION_TIME: f64 = 2.0;
pub const SOLAR_CONSTRUCTION_TIME: f64 = 0.5;
pub const NUCLEAR_CONSTRUCTION_TIME: f64 = 7.0;
pub const COAL_CONSTRUCTION_TIME: f64 = 4.0;
pub const GAS_CONSTRUCTION_TIME: f64 = 2.0;
pub const BIOMASS_CONSTRUCTION_TIME: f64 = 2.0;
pub const HYDRO_CONSTRUCTION_TIME: f64 = 5.0;
pub const STORAGE_CONSTRUCTION_TIME: f64 = 1.0;
pub const TIDAL_CONSTRUCTION_TIME: f64 = 2.0;
pub const WAVE_CONSTRUCTION_TIME: f64 = 2.0;

// Carbon offset planning and construction times
pub const FOREST_PLANNING_TIME: f64 = 1.0;
pub const WETLAND_PLANNING_TIME: f64 = 1.5;
pub const ACTIVE_CAPTURE_PLANNING_TIME: f64 = 2.0;
pub const CARBON_CREDIT_PLANNING_TIME: f64 = 0.5;

pub const FOREST_CONSTRUCTION_TIME: f64 = 1.0;
pub const WETLAND_CONSTRUCTION_TIME: f64 = 2.0;
pub const ACTIVE_CAPTURE_CONSTRUCTION_TIME: f64 = 3.0;
pub const CARBON_CREDIT_CONSTRUCTION_TIME: f64 = 0.2;

// Time reduction factors
pub const PLANNING_TIME_OPINION_FACTOR: f64 = 0.5;  // How much opinion affects planning time (0.0-1.0)
pub const PLANNING_TIME_YEAR_REDUCTION: f64 = 0.02; // Annual reduction in planning time (2% per year)
pub const CONSTRUCTION_TIME_YEAR_REDUCTION: f64 = 0.03; // Annual reduction in construction time (3% per year)

// Minimum times (years) regardless of other factors
pub const MIN_PLANNING_TIME: f64 = 0.25;  // Minimum 3 months for planning
pub const MIN_CONSTRUCTION_TIME: f64 = 0.1;  // Minimum ~1 month for construction

// Cost-time tradeoff constants
pub const MIN_CONSTRUCTION_COST_MULTIPLIER: f64 = 1.0;  // Base cost (no speedup)
pub const MAX_CONSTRUCTION_COST_MULTIPLIER: f64 = 5.0;  // Maximum cost multiplier allowed
pub const CONSTRUCTION_TIME_REDUCTION_FACTOR: f64 = 0.5;  // How much time is reduced with increased cost (logarithmic)
pub const PLANNING_TIME_REDUCTION_FACTOR: f64 = 0.25;  // Planning time reduces at half the rate of construction time

// Cost multiplier constants for AI decision making
pub const DEFAULT_COST_MULTIPLIER: u16 = 100;  // 100% = normal cost
pub const FAST_COST_MULTIPLIER: u16 = 120;     // 120% = faster construction
pub const VERY_FAST_COST_MULTIPLIER: u16 = 150; // 150% = very fast construction
pub const RUSH_COST_MULTIPLIER: u16 = 200;     // 200% = rush construction

// Generator Reliability Factors
pub const ONSHORE_WIND_RELIABILITY: f64 = 0.35;
pub const OFFSHORE_WIND_RELIABILITY: f64 = 0.35;
pub const DOMESTIC_SOLAR_RELIABILITY: f64 = 0.25;
pub const COMMERCIAL_SOLAR_RELIABILITY: f64 = 0.25;
pub const UTILITY_SOLAR_RELIABILITY: f64 = 0.25;
pub const NUCLEAR_RELIABILITY: f64 = 0.95;
pub const COAL_RELIABILITY: f64 = 0.90;
pub const GAS_CC_RELIABILITY: f64 = 0.85;
pub const GAS_PEAKER_RELIABILITY: f64 = 0.90;
pub const BIOMASS_RELIABILITY: f64 = 0.80;
pub const HYDRO_RELIABILITY: f64 = 0.75;
pub const PUMPED_STORAGE_RELIABILITY: f64 = 0.95;
pub const BATTERY_STORAGE_RELIABILITY: f64 = 0.98;
pub const TIDAL_RELIABILITY: f64 = 0.45;
pub const WAVE_RELIABILITY: f64 = 0.40;

// Operation Percentage Constants
pub const OPERATION_PERCENTAGE_SCALE: f64 = 100.0;

// Default Metric Values
pub const DEFAULT_OPINION: f64 = 1.0;
pub const DEFAULT_POWER: f64 = 0.0;
pub const DEFAULT_COST: f64 = 0.0;
pub const DEFAULT_EMISSIONS: f64 = 0.0;

// Spatial Index Constants
pub const COASTAL_INFLUENCE_RADIUS: f64 = 8000.0;
pub const COASTAL_INFLUENCE_SCORE: f64 = 0.6;
pub const OFFSHORE_INFLUENCE_RADIUS: f64 = 20000.0;
pub const OFFSHORE_INFLUENCE_SCORE: f64 = 0.5;

pub const URBAN_RADIUS_FACTOR: f64 = 15.0;
pub const URBAN_SCORE_DIVISOR: f64 = 7.0;
pub const URBAN_MIN_SCORE: f64 = 0.2;
pub const URBAN_MAX_SCORE: f64 = 0.8;

pub const PROTECTED_URBAN_RADIUS_FACTOR: f64 = 0.1;
pub const PROTECTED_URBAN_SCORE: f64 = 0.7;

pub const GENERATOR_RADIUS_FACTOR: f64 = 1.2;
pub const GENERATOR_PROTECTED_RADIUS_FACTOR: f64 = 0.5;
pub const GENERATOR_PROTECTED_SCORE: f64 = 0.6;
pub const GENERATOR_INFLUENCE_RADIUS_FACTOR: f64 = 1.5;
pub const GENERATOR_INFLUENCE_SCORE: f64 = 0.4;

pub const RURAL_BASE_SCORE: f64 = 0.5;

// Public Opinion Calculation Constants
pub const OPINION_SEARCH_RADIUS: f64 = 10000.0;
pub const OPINION_DISTANCE_FACTOR: f64 = 5000.0;
pub const OPINION_MIN: f64 = 0.0;
pub const OPINION_MAX: f64 = 1.0;
pub const OPINION_BASE_WEIGHT: f64 = 1.0;

// Power Reliability Calculation Constants
pub const RELIABILITY_THRESHOLD: f64 = 0.9; // Threshold for switching between calculation modes
pub const LOW_SUPPLY_WEIGHT: f64 = 0.7;     // Weight for supply adequacy when supply is low
pub const LOW_SUPPLY_MIX_WEIGHT: f64 = 0.3; // Weight for reliability mix when supply is low
pub const HIGH_SUPPLY_WEIGHT: f64 = 0.4;    // Weight for supply adequacy when supply is good
pub const HIGH_SUPPLY_MIX_WEIGHT: f64 = 0.6; // Weight for reliability mix when supply is good
pub const RELIABILITY_ADJUSTMENT: f64 = 0.08; // 8% flat adjustment to account for inaccuracies in low fidelity simulations
