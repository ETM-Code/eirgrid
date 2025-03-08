use std::error::Error;
use std::str::FromStr;
use rand::Rng;
use crate::map_handler::Map;
use crate::generator::{Generator, GeneratorType};
use crate::action_weights::{GridAction, SimulationMetrics};
use crate::power_storage::PowerStorageSystem as PowerStorage;
use crate::carbon_offset::{CarbonOffset, CarbonOffsetType};
use crate::poi::Coordinate;
use crate::constants::{
    DEFAULT_GENERATOR_SIZE,
    COAL_CO2_RATE,
    GAS_CC_CO2_RATE,
    GAS_PEAKER_CO2_RATE,
    BIOMASS_CO2_RATE,
    WIND_BASE_MAX_EFFICIENCY,
    UTILITY_SOLAR_BASE_MAX_EFFICIENCY,
    NUCLEAR_BASE_MAX_EFFICIENCY,
    GAS_CC_BASE_MAX_EFFICIENCY,
    HYDRO_BASE_MAX_EFFICIENCY,
    MARINE_BASE_MAX_EFFICIENCY,
    DEFAULT_BASE_MAX_EFFICIENCY,
    DEVELOPING_TECH_IMPROVEMENT_RATE,
    EMERGING_TECH_IMPROVEMENT_RATE,
    MATURE_TECH_IMPROVEMENT_RATE,
    BASE_YEAR,
    MIN_CARBON_OFFSET_SIZE,
    MAX_CARBON_OFFSET_SIZE,
    MIN_CARBON_OFFSET_EFFICIENCY,
    MAX_CARBON_OFFSET_EFFICIENCY,
    MAP_MAX_X,
    MAP_MAX_Y,
    FOREST_BASE_COST,
    WETLAND_BASE_COST,
    ACTIVE_CAPTURE_BASE_COST,
    CARBON_CREDIT_BASE_COST,
    FOREST_OPERATING_COST,
    WETLAND_OPERATING_COST,
    ACTIVE_CAPTURE_OPERATING_COST,
    CARBON_CREDIT_OPERATING_COST,
};

pub fn apply_action(map: &mut Map, action: &GridAction, year: u32) -> Result<(), Box<dyn Error + Send + Sync>> {
    match action {
        GridAction::AddGenerator(gen_type) => {
            let gen_size = DEFAULT_GENERATOR_SIZE;
            match map.find_best_generator_location(gen_type, gen_size as f64 / 100.0) {
                Some(location) => {
                    let base_efficiency = gen_type.get_base_efficiency(year);
                    let initial_co2_output = match gen_type {
                        GeneratorType::CoalPlant => COAL_CO2_RATE,
                        GeneratorType::GasCombinedCycle => GAS_CC_CO2_RATE,
                        GeneratorType::GasPeaker => GAS_PEAKER_CO2_RATE,
                        GeneratorType::Biomass => BIOMASS_CO2_RATE,
                        _ => 0.0,  // All other types have zero direct CO2 emissions
                    } * (gen_size as f64 / 100.0);  // Scale by size
                     
                    let generator = Generator::new(
                        format!("Gen_{}_{}_{}", gen_type.to_string(), year, map.get_generator_count()),
                        location,
                        gen_type.clone(),
                        gen_type.get_base_cost(year),
                        gen_type.get_base_power(year),
                        gen_type.get_operating_cost(year),
                        gen_type.get_lifespan(),
                        gen_size as f64 / 100.0,
                        initial_co2_output,
                        base_efficiency,
                    );
                    map.add_generator(generator);
                    Ok(())
                },
                None => {
                    // Fallback: Try a different generator type
                    let fallback_type = match gen_type {
                        GeneratorType::Nuclear => GeneratorType::GasCombinedCycle,
                        GeneratorType::HydroDam | GeneratorType::PumpedStorage => GeneratorType::GasPeaker,
                        GeneratorType::OffshoreWind => GeneratorType::OnshoreWind,
                        GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => GeneratorType::OffshoreWind,
                        _ => GeneratorType::GasPeaker, // Default fallback
                    };
                     
                    println!("Falling back to {:?} generator instead of {:?}", fallback_type, gen_type);
                    apply_action(map, &GridAction::AddGenerator(fallback_type), year)
                }
            }
        },
        GridAction::UpgradeEfficiency(id) => {
            if let Some(generator) = map.get_generator_mut(id) {
                if generator.is_active() {
                    let base_max = match generator.get_generator_type() {
                        GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => WIND_BASE_MAX_EFFICIENCY,
                        GeneratorType::UtilitySolar => UTILITY_SOLAR_BASE_MAX_EFFICIENCY,
                        GeneratorType::Nuclear => NUCLEAR_BASE_MAX_EFFICIENCY,
                        GeneratorType::GasCombinedCycle => GAS_CC_BASE_MAX_EFFICIENCY,
                        GeneratorType::HydroDam | GeneratorType::PumpedStorage => HYDRO_BASE_MAX_EFFICIENCY,
                        GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => MARINE_BASE_MAX_EFFICIENCY,
                        _ => DEFAULT_BASE_MAX_EFFICIENCY,
                    };
                     
                    let tech_improvement = match generator.get_generator_type() {
                        GeneratorType::OnshoreWind | GeneratorType::OffshoreWind |
                        GeneratorType::UtilitySolar => DEVELOPING_TECH_IMPROVEMENT_RATE,
                        GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => EMERGING_TECH_IMPROVEMENT_RATE,
                        _ => MATURE_TECH_IMPROVEMENT_RATE,
                    }.powi((year - BASE_YEAR) as i32);
                     
                    let max_efficiency = base_max * (1.0 + (1.0 - tech_improvement));
                    generator.upgrade_efficiency(year, max_efficiency);
                }
            }
            map.after_generator_modification();
            Ok(())
        },
        GridAction::AdjustOperation(id, percentage) => {
            let constraints = map.get_generator_constraints().clone();
            if let Some(generator) = map.get_generator_mut(id) {
                if generator.is_active() {
                    generator.adjust_operation(*percentage, &constraints);
                }
            }
            map.after_generator_modification();
            Ok(())
        },
        GridAction::AddCarbonOffset(offset_type) => {
            let offset_size = rand::thread_rng().gen_range(MIN_CARBON_OFFSET_SIZE..MAX_CARBON_OFFSET_SIZE);
            let base_efficiency = rand::thread_rng().gen_range(MIN_CARBON_OFFSET_EFFICIENCY..MAX_CARBON_OFFSET_EFFICIENCY);
             
            let location = Coordinate::new(
                rand::thread_rng().gen_range(0.0..MAP_MAX_X),
                rand::thread_rng().gen_range(0.0..MAP_MAX_Y),
            );
             
            let offset = CarbonOffset::new(
                format!("Offset_{}_{}_{}", offset_type, year, map.get_carbon_offset_count()),
                location,
                CarbonOffsetType::from_str(offset_type).unwrap_or(CarbonOffsetType::Forest),
                match CarbonOffsetType::from_str(offset_type).unwrap_or(CarbonOffsetType::Forest) {
                    CarbonOffsetType::Forest => FOREST_BASE_COST,
                    CarbonOffsetType::Wetland => WETLAND_BASE_COST,
                    CarbonOffsetType::ActiveCapture => ACTIVE_CAPTURE_BASE_COST,
                    CarbonOffsetType::CarbonCredit => CARBON_CREDIT_BASE_COST,
                },
                match CarbonOffsetType::from_str(offset_type).unwrap_or(CarbonOffsetType::Forest) {
                    CarbonOffsetType::Forest => FOREST_OPERATING_COST,
                    CarbonOffsetType::Wetland => WETLAND_OPERATING_COST,
                    CarbonOffsetType::ActiveCapture => ACTIVE_CAPTURE_OPERATING_COST,
                    CarbonOffsetType::CarbonCredit => CARBON_CREDIT_OPERATING_COST,
                },
                offset_size,
                base_efficiency,
            );
             
            map.add_carbon_offset(offset);
            Ok(())
        },
        GridAction::CloseGenerator(id) => {
            if let Some(generator) = map.get_generator_mut(id) {
                if generator.is_active() {
                    let age = year - generator.commissioning_year;
                    let min_age = match generator.get_generator_type() {
                        GeneratorType::Nuclear => 1,
                        GeneratorType::HydroDam => 1,
                        GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => 1,
                        GeneratorType::UtilitySolar => 1,
                        _ => 1,
                    };
                     
                    if age >= min_age {
                        generator.close_generator(year);
                    }
                }
            }
            map.after_generator_modification();
            Ok(())
        },
        GridAction::DoNothing => {
            Ok(())
        },
    }
}