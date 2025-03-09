use std::error::Error;
use rand::Rng;
use crate::utils::map_handler::Map;
use crate::models::generator::{Generator, GeneratorType};
use super::action_weights::GridAction;
use crate::models::carbon_offset::{CarbonOffset, CarbonOffsetType};
use crate::data::poi::Coordinate;
use crate::config::constants::{
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
    MIN_CONSTRUCTION_COST_MULTIPLIER,
    MAX_CONSTRUCTION_COST_MULTIPLIER,
};
use crate::config::const_funcs::calc_decommission_cost;

pub fn apply_action(map: &mut Map, action: &GridAction, year: u32) -> Result<(), Box<dyn Error + Send + Sync>> {
    match action {
        GridAction::AddGenerator(gen_type, cost_multiplier_percent) => {
            let gen_size = DEFAULT_GENERATOR_SIZE;
            let cost_multiplier = (*cost_multiplier_percent as f64 / 100.0)
                .clamp(MIN_CONSTRUCTION_COST_MULTIPLIER, MAX_CONSTRUCTION_COST_MULTIPLIER);
                
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
                     
                    let mut generator = Generator::new(
                        format!("Gen_{}_{}_{}", gen_type.to_string(), year, map.get_generator_count()),
                        location,
                        gen_type.clone(),
                        gen_type.get_base_cost(year),
                        gen_type.get_base_power(year),
                        gen_type.get_operating_cost(year),
                        gen_type.get_lifespan(),
                        gen_size as f64 / 100.0,
                        initial_co2_output,
                        calc_decommission_cost(gen_type.get_base_cost(year)),
                    );
                    
                    // Set the construction cost multiplier
                    generator.set_construction_cost_multiplier(cost_multiplier);
                    
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
                    apply_action(map, &GridAction::AddGenerator(fallback_type, *cost_multiplier_percent), year)
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
        GridAction::AddCarbonOffset(offset_type, cost_multiplier_percent) => {
            let cost_multiplier = (*cost_multiplier_percent as f64 / 100.0)
                .clamp(MIN_CONSTRUCTION_COST_MULTIPLIER, MAX_CONSTRUCTION_COST_MULTIPLIER);
                
            // Create a new carbon offset with the specified type and cost multiplier
            let offset_size = match offset_type {
                CarbonOffsetType::Forest => 500.0, // 500 hectares
                CarbonOffsetType::Wetland => 300.0, // 300 hectares
                CarbonOffsetType::ActiveCapture => 100.0, // 100 tons capacity
                CarbonOffsetType::CarbonCredit => 1000.0, // 1000 tons of credits
            };
            
            // Get a random location within the map bounds
            let mut rng = rand::thread_rng();
            let x = rng.gen_range(0.0..MAP_MAX_X);
            let y = rng.gen_range(0.0..MAP_MAX_Y);
            let location = Coordinate { x, y };
            
            // Calculate base cost based on type
            let base_cost = match offset_type {
                CarbonOffsetType::Forest => FOREST_BASE_COST,
                CarbonOffsetType::Wetland => WETLAND_BASE_COST,
                CarbonOffsetType::ActiveCapture => ACTIVE_CAPTURE_BASE_COST,
                CarbonOffsetType::CarbonCredit => CARBON_CREDIT_BASE_COST,
            };
            
            // Calculate operating cost based on type
            let operating_cost = match offset_type {
                CarbonOffsetType::Forest => FOREST_OPERATING_COST,
                CarbonOffsetType::Wetland => WETLAND_OPERATING_COST,
                CarbonOffsetType::ActiveCapture => ACTIVE_CAPTURE_OPERATING_COST,
                CarbonOffsetType::CarbonCredit => CARBON_CREDIT_OPERATING_COST,
            };
            
            // Create the carbon offset
            let mut offset = CarbonOffset::new(
                format!("Offset_{}_{}_{}", offset_type.to_string(), year, map.get_carbon_offset_count()),
                location,
                offset_type.clone(),
                base_cost,
                operating_cost,
                offset_size,
                0.85, // Default efficiency
            );
            
            // Set the construction cost multiplier
            offset.set_construction_cost_multiplier(cost_multiplier);
            
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