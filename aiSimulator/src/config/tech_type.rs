// Tech Type module - contains TechType and BuildSpeed enums and related functions
use crate::models::generator::GeneratorType;
use crate::config::constants::BASE_YEAR;

/// Enum for generation technology types, simplified version of GeneratorType
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TechType {
    OnshoreWind,
    OffshoreWind,
    SolarPV,
    Gas,
    Coal,
    Nuclear,
    Hydro,
    Biomass,
    Tidal,
    Wave,
    Storage,
}

/// Enum for construction speed options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuildSpeed {
    Normal,   // Base speed (no acceleration)
    Fast,     // Faster construction (moderate premium)
    VeryFast, // Very fast construction (significant premium)
    Rush,     // Maximum speed rush construction (high premium)
}

impl BuildSpeed {
    /// Convert a cost multiplier (as percentage) to a BuildSpeed
    pub fn from_cost_multiplier(multiplier: u16) -> Self {
        match multiplier {
            100..=124 => BuildSpeed::Normal,
            125..=174 => BuildSpeed::Fast,
            175..=249 => BuildSpeed::VeryFast,
            _ => BuildSpeed::Rush,
        }
    }

    /// Get the display name for the build speed
    pub fn display_name(&self) -> &'static str {
        match self {
            BuildSpeed::Normal => "Normal",
            BuildSpeed::Fast => "Fast",
            BuildSpeed::VeryFast => "Very Fast",
            BuildSpeed::Rush => "Rush",
        }
    }
}

/// Map GeneratorType to simplified TechType
pub fn map_to_tech_type(gen_type: &GeneratorType) -> TechType {
    match gen_type {
        GeneratorType::OnshoreWind => TechType::OnshoreWind,
        GeneratorType::OffshoreWind => TechType::OffshoreWind,
        GeneratorType::DomesticSolar | GeneratorType::CommercialSolar | GeneratorType::UtilitySolar => TechType::SolarPV,
        GeneratorType::GasCombinedCycle | GeneratorType::GasPeaker => TechType::Gas,
        GeneratorType::CoalPlant => TechType::Coal,
        GeneratorType::Nuclear => TechType::Nuclear,
        GeneratorType::HydroDam => TechType::Hydro,
        GeneratorType::PumpedStorage | GeneratorType::BatteryStorage => TechType::Storage,
        GeneratorType::Biomass => TechType::Biomass,
        GeneratorType::TidalGenerator => TechType::Tidal,
        GeneratorType::WaveEnergy => TechType::Wave,
    }
}

/// Estimated planning duration in years for a given tech and year.
pub fn planning_duration(year: u32, tech: TechType) -> f64 {
    // Define baseline (2025) and improved (2050) planning times for each tech (in years)
    let (base_2025, min_2050) = match tech {
        TechType::OnshoreWind => {
            let base = 1.5;    // ~1.5 years (78 weeks) in 2025
            let min  = 0.5;    // ~0.5 years (26 weeks) by 2050 (streamlined)
            (base, min)
        },
        TechType::OffshoreWind => {
            let base = 3.0;    // ~3 years in 2025 (nascent permitting process, MARA etc.)
            let min  = 1.0;    // ~1 year by 2050 (improved with OPI and streamlined offshore rules)
            (base, min)
        },
        TechType::SolarPV => {
            let base = 1.0;    // ~1 year in 2025 (large solar farms often 6-12 months if appealed)
            let min  = 0.3;    // ~0.3 years (~4 months) by 2050 (smaller projects much faster)
            (base, min)
        },
        TechType::Gas | TechType::Coal => {
            let base = 2.0;    // ~2 years in 2025 (strategic infrastructure, large thermal plant)
            let min  = 1.0;    // ~1 year by 2050 (if streamlined, though coal unlikely to be built)
            (base, min)
        },
        TechType::Nuclear => {
            let base = 5.0;    // ~5 years in 2025 (would be very long due to public concern, notional)
            let min  = 3.0;    // ~3 years by 2050 (if advanced small modular reactors, faster process)
            (base, min)
        },
        TechType::Hydro => {
            let base = 2.5;    // ~2.5 years in 2025 (pumped hydro planning ~2-3 yrs if strategic)
            let min  = 1.5;    // ~1.5 years by 2050 (some improvement)
            (base, min)
        },
        TechType::Storage => {
            let base = 1.5;    // ~1.5 years in 2025 
            let min  = 0.8;    // ~0.8 years by 2050
            (base, min)
        },
        TechType::Biomass => {
            let base = 2.0;    // ~2 years in 2025
            let min  = 1.0;    // ~1 year by 2050
            (base, min)
        },
        TechType::Tidal => {
            let base = 3.0;    // ~3 years in 2025
            let min  = 1.5;    // ~1.5 years by 2050
            (base, min)
        },
        TechType::Wave => {
            let base = 3.0;    // ~3 years in 2025
            let min  = 1.5;    // ~1.5 years by 2050
            (base, min)
        },
    };
    
    // Linear interpolation between base (2025) and minimum (2050)
    let clamped_year = year.clamp(BASE_YEAR, 2050);
    let t = (clamped_year as f64 - BASE_YEAR as f64) / (2050.0 - BASE_YEAR as f64);
    
    // Ensure it doesn't go below min_2050
    let years = base_2025 + t * (min_2050 - base_2025);
    years.max(min_2050)
}

/// Estimated construction duration in years for a given tech and year, assuming normal (non-rush) pace.
pub fn construction_duration(year: u32, tech: TechType) -> f64 {
    // Baseline (2025) and improved (2050) construction times for each tech (in years)
    let (base_2025, improv_2050) = match tech {
        TechType::OnshoreWind => {
            let base = 1.25;  // ~1.25 years (15 months) in 2025
            let imp  = 0.75;  // ~0.75 years (9 months) by 2050 (faster installs, better processes)
            (base, imp)
        },
        TechType::OffshoreWind => {
            let base = 3.0;   // ~3 years in 2025 (for first large projects)
            let imp  = 2.0;   // ~2 years by 2050 (improved installation tech)
            (base, imp)
        },
        TechType::SolarPV => {
            let base = 0.5;   // ~0.5 years (6 months) in 2025 (many solar built within months)
            let imp  = 0.25;  // ~0.25 years (3 months) by 2050 (automation, modular install)
            (base, imp)
        },
        TechType::Gas => {
            let base = 2.5;   // ~2.5 years in 2025 (CCGT plant ~30 months)
            let imp  = 2.0;   // ~2.0 years by 2050 (some efficiency gains, modular components)
            (base, imp)
        },
        TechType::Coal => {
            let base = 3.0;   // ~3 years in 2025 (large coal plant, though none likely built)
            let imp  = 3.0;   // ~3 years by 2050 (no significant improvement assumed)
            (base, imp)
        },
        TechType::Nuclear => {
            let base = 7.0;   // ~7 years in 2025 (global avg ~6-8 years)
            let imp  = 4.0;   // ~4 years by 2050 (assuming SMRs improve build time)
            (base, imp)
        },
        TechType::Hydro => {
            let base = 4.0;   // ~4 years in 2025 (pumped hydro major civil works)
            let imp  = 3.5;   // ~3.5 years by 2050 (slight improvement with better tunneling tech)
            (base, imp)
        },
        TechType::Storage => {
            let base = 1.0;   // ~1 year in 2025
            let imp  = 0.5;   // ~0.5 years by 2050
            (base, imp)
        },
        TechType::Biomass => {
            let base = 2.0;   // ~2 years in 2025
            let imp  = 1.5;   // ~1.5 years by 2050
            (base, imp)
        },
        TechType::Tidal => {
            let base = 2.0;   // ~2 years in 2025
            let imp  = 1.5;   // ~1.5 years by 2050
            (base, imp)
        },
        TechType::Wave => {
            let base = 2.0;   // ~2 years in 2025
            let imp  = 1.5;   // ~1.5 years by 2050
            (base, imp)
        },
    };
    
    // Linear interpolation for improvement over time
    let clamped_year = year.clamp(BASE_YEAR, 2050);
    let t = (clamped_year as f64 - BASE_YEAR as f64) / (2050.0 - BASE_YEAR as f64);
    let years = base_2025 + t * (improv_2050 - base_2025);
    years.max(improv_2050)
}

/// Cost multiplier for a given construction speed option.
/// Returns a value that can be used as a multiplier for construction costs.
pub fn cost_multiplier(speed: BuildSpeed) -> f64 {
    match speed {
        BuildSpeed::Normal   => 1.0,   // base cost (no acceleration) 
        BuildSpeed::Fast     => 1.2,   // ~20% cost increase for faster delivery
        BuildSpeed::VeryFast => 1.5,   // ~50% cost increase for significantly faster delivery
        BuildSpeed::Rush     => 2.0    // ~100% cost increase for maximum speed (crash program)
    }
}

/// Convert a u16 percentage cost multiplier (100-300) to a f64 multiplier (1.0-3.0)
pub fn convert_cost_multiplier(percentage: u16) -> f64 {
    (percentage as f64) / 100.0
} 