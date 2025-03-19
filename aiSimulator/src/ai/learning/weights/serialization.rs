//! Serialization module for ActionWeights
//!
//! This module contains serialization-related functionality for the ActionWeights struct.

use std::path::Path;
use std::str::FromStr;
use std::collections::HashMap;
use crate::models::generator::GeneratorType;
use crate::ai::actions::grid_action::GridAction;
use crate::ai::actions::serializable_action::SerializableAction;
use crate::ai::learning::constants::*;
use crate::ai::learning::serialization::SerializableWeights;
use super::{ActionWeights, FILE_MUTEX};
use crate::models::carbon_offset::CarbonOffsetType;
use crate::config::constants::DEFAULT_COST_MULTIPLIER;
use crate::ai::metrics::simulation_metrics::SimulationMetrics;

// Add a dummy public item to ensure this file is recognized by rust-analyzer
#[allow(dead_code)]
pub const MODULE_MARKER: &str = "serialization_module";

impl ActionWeights {

// This file contains extracted code from the original weights.rs file
// Appropriate imports will need to be added based on the specific requirements

    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        // Acquire lock for file operations
        let _lock = FILE_MUTEX.lock().map_err(|e| {
            println!("Error acquiring file lock for saving: {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, "Failed to acquire file lock for saving")
        })?;
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = Path::new(path).parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                println!("Error creating directory {}: {}", parent.display(), e);
                return Err(e);
            }
        }

        // Convert to serializable format
        let mut serializable_weights = HashMap::new();
        for (year, year_weights) in &self.weights {
            let mut serializable_year_weights = Vec::new();
            for (action, &weight) in year_weights {
                serializable_year_weights.push((
                    SerializableAction::from(action),
                    weight,
                ));
            }
            serializable_weights.insert(*year, serializable_year_weights);
        }
        
        // Convert deficit weights to serializable format
        let mut serializable_deficit_weights = HashMap::new();
        for (year, year_weights) in &self.deficit_weights {
            let mut serializable_year_weights = Vec::new();
            for (action, &weight) in year_weights {
                serializable_year_weights.push((
                    SerializableAction::from(action),
                    weight,
                ));
            }
            serializable_deficit_weights.insert(*year, serializable_year_weights);
        }
        
        // Convert best weights to serializable format
        let serializable_best_weights = self.best_weights.as_ref().map(|best_weights| {
            let mut serializable = HashMap::new();
            for (year, year_weights) in best_weights {
                let mut serializable_year_weights = Vec::new();
                for (action, &weight) in year_weights {
                    serializable_year_weights.push((
                        SerializableAction::from(action),
                        weight,
                    ));
                }
                serializable.insert(*year, serializable_year_weights);
            }
            serializable
        });
        
        // Convert prime weights to serializable format
        let serializable_prime_weights = self.prime_weights.as_ref().map(|prime_weights| {
            let mut serializable = HashMap::new();
            for (year, year_weights) in prime_weights {
                let mut serializable_year_weights = Vec::new();
                for (action, &weight) in year_weights {
                    serializable_year_weights.push((
                        SerializableAction::from(action),
                        weight,
                    ));
                }
                serializable.insert(*year, serializable_year_weights);
            }
            serializable
        });
        
        // Convert best actions to serializable format
        let serializable_best_actions = self.best_actions.as_ref().map(|best_actions| {
            let mut serializable = HashMap::new();
            for (year, actions) in best_actions {
                let mut serializable_actions = Vec::new();
                for action in actions {
                    serializable_actions.push(SerializableAction::from(action));
                }
                serializable.insert(*year, serializable_actions);
            }
            serializable
        });

        // Convert best deficit actions to serializable format
        let serializable_best_deficit_actions = self.best_deficit_actions.as_ref().map(|best_actions| {
            let mut serializable = HashMap::new();
            for (year, actions) in best_actions {
                let mut serializable_actions = Vec::new();
                for action in actions {
                    serializable_actions.push(SerializableAction::from(action));
                }
                serializable.insert(*year, serializable_actions);
            }
            serializable
        });

        let serializable = SerializableWeights {
            weights: serializable_weights,
            learning_rate: self.learning_rate,
            best_metrics: self.best_metrics.clone(),
            best_weights: serializable_best_weights,
            best_actions: serializable_best_actions,
            iteration_count: self.iteration_count,
            iterations_without_improvement: self.iterations_without_improvement,
            exploration_rate: self.exploration_rate,
            deficit_weights: serializable_deficit_weights,
            best_deficit_actions: serializable_best_deficit_actions,
            optimization_mode: self.optimization_mode.clone(),
            improvement_history: if !self.improvement_history.is_empty() {
                Some(self.improvement_history.iter().map(|record| {
                    crate::ai::learning::serialization::SerializableImprovementRecord::from(record)
                }).collect())
            } else {
                None
            },
            prime_weights: serializable_prime_weights,
        };
        
        let json = serde_json::to_string_pretty(&serializable)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        std::fs::write(path, json)?;
        
        Ok(())
    }

    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        let _lock = FILE_MUTEX.lock().map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to acquire file lock for loading: {}", e))
        })?;
        
        let json = std::fs::read_to_string(path)?;
        let serializable: SerializableWeights = serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        // Convert serializable weights to actual weights
        let mut weights = HashMap::new();
        for (year, serializable_year_weights) in &serializable.weights {
            let mut year_weights = HashMap::new();
            for (serializable_action, weight) in serializable_year_weights {
                let action = match serializable_action.action_type.as_str() {
                    "AddGenerator" => {
                        if let Some(gen_type_str) = &serializable_action.generator_type {
                            let gen_type = GeneratorType::from_str(gen_type_str)
                                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                            let cost_multiplier = serializable_action.cost_multiplier.unwrap_or(DEFAULT_COST_MULTIPLIER);
                            GridAction::AddGenerator(gen_type, cost_multiplier)
                        } else {
                            GridAction::AddGenerator(GeneratorType::GasPeaker, DEFAULT_COST_MULTIPLIER)
                        }
                    },
                    "UpgradeEfficiency" => {
                        if let Some(id) = &serializable_action.generator_id {
                            GridAction::UpgradeEfficiency(id.clone())
                        } else {
                            GridAction::UpgradeEfficiency(String::new())
                        }
                    },
                    "AdjustOperation" => {
                        let id = serializable_action.generator_id.clone().unwrap_or_default();
                        let percentage = serializable_action.operation_percentage.unwrap_or(0);
                        // Skip legacy empty ID operations
                        if id.is_empty() {
                            continue;
                        }
                        // Validate generator type exists
                        if let Some(gen_type_str) = &serializable_action.generator_type {
                            if GeneratorType::from_str(gen_type_str).is_err() {
                                continue;
                            }
                        }
                        GridAction::AdjustOperation(id, percentage)
                    },
                    "AddCarbonOffset" => {
                        if let Some(offset_type_str) = &serializable_action.offset_type {
                            let offset_type = match offset_type_str.as_str() {
                                "Forest" => CarbonOffsetType::Forest,
                                "Wetland" => CarbonOffsetType::Wetland,
                                "ActiveCapture" => CarbonOffsetType::ActiveCapture,
                                "CarbonCredit" => CarbonOffsetType::CarbonCredit,
                                _ => CarbonOffsetType::Forest,
                            };
                            let cost_multiplier = serializable_action.cost_multiplier.unwrap_or(DEFAULT_COST_MULTIPLIER);
                            GridAction::AddCarbonOffset(offset_type, cost_multiplier)
                        } else {
                            GridAction::AddCarbonOffset(CarbonOffsetType::Forest, DEFAULT_COST_MULTIPLIER)
                        }
                    },
                    "CloseGenerator" => {
                        let id = serializable_action.generator_id.clone().unwrap_or_default();
                        // Skip legacy empty ID closures
                        if id.is_empty() {
                            continue;
                        }
                        // Validate generator type exists
                        if let Some(gen_type_str) = &serializable_action.generator_type {
                            if GeneratorType::from_str(gen_type_str).is_err() {
                                continue;
                            }
                        }
                        GridAction::CloseGenerator(id)
                    },
                    "DoNothing" => GridAction::DoNothing,
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Unknown action type: {}", serializable_action.action_type),
                        ));
                    }
                };
                year_weights.insert(action, *weight);
            }
            weights.insert(*year, year_weights);
        }
        
        // Convert serializable deficit weights to actual deficit weights
        let mut deficit_weights = HashMap::new();
        for (year, serializable_year_weights) in &serializable.deficit_weights {
            let mut year_weights = HashMap::new();
            for (serializable_action, weight) in serializable_year_weights {
                let action = match serializable_action.action_type.as_str() {
                    "AddGenerator" => {
                        if let Some(gen_type_str) = &serializable_action.generator_type {
                            match GeneratorType::from_str(gen_type_str) {
                                Ok(gen_type) => {
                                    let cost_multiplier = serializable_action.cost_multiplier.unwrap_or(DEFAULT_COST_MULTIPLIER);
                                    GridAction::AddGenerator(gen_type, cost_multiplier)
                                },
                                Err(_) => continue,
                            }
                        } else {
                            continue;
                        }
                    },
                    "UpgradeEfficiency" => {
                        GridAction::UpgradeEfficiency(serializable_action.generator_id.clone().unwrap_or_default())
                    },
                    "AdjustOperation" => {
                        let id = serializable_action.generator_id.clone().unwrap_or_default();
                        let percentage = serializable_action.operation_percentage.unwrap_or(0);
                        // Skip legacy empty ID operations
                        if id.is_empty() {
                            continue;
                        }
                        // Validate generator type exists
                        if let Some(gen_type_str) = &serializable_action.generator_type {
                            if GeneratorType::from_str(gen_type_str).is_err() {
                                continue;
                            }
                        }
                        GridAction::AdjustOperation(id, percentage)
                    },
                    "AddCarbonOffset" => {
                        if let Some(offset_type_str) = &serializable_action.offset_type {
                            let offset_type = match offset_type_str.as_str() {
                                "Forest" => CarbonOffsetType::Forest,
                                "Wetland" => CarbonOffsetType::Wetland,
                                "ActiveCapture" => CarbonOffsetType::ActiveCapture,
                                "CarbonCredit" => CarbonOffsetType::CarbonCredit,
                                _ => CarbonOffsetType::Forest,
                            };
                            let cost_multiplier = serializable_action.cost_multiplier.unwrap_or(DEFAULT_COST_MULTIPLIER);
                            GridAction::AddCarbonOffset(offset_type, cost_multiplier)
                        } else {
                            GridAction::AddCarbonOffset(CarbonOffsetType::Forest, DEFAULT_COST_MULTIPLIER)
                        }
                    },
                    "CloseGenerator" => {
                        GridAction::CloseGenerator(serializable_action.generator_id.clone().unwrap_or_default())
                    },
                    "DoNothing" => GridAction::DoNothing,
                    _ => continue,
                };
                year_weights.insert(action, *weight);
            }
            deficit_weights.insert(*year, year_weights);
        }
        
        // If no deficit weights were found in the file, initialize them with defaults
        if deficit_weights.is_empty() {
            for year in START_YEAR..=END_YEAR {
                let mut deficit_year_weights = HashMap::new();
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::GasPeaker, DEFAULT_COST_MULTIPLIER), DEFICIT_GAS_PEAKER_WEIGHT);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::GasCombinedCycle, DEFAULT_COST_MULTIPLIER), DEFICIT_GAS_COMBINED_WEIGHT);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::BatteryStorage, DEFAULT_COST_MULTIPLIER), DEFICIT_BATTERY_WEIGHT);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::PumpedStorage, DEFAULT_COST_MULTIPLIER), DEFICIT_PUMPED_STORAGE_WEIGHT);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::Biomass, DEFAULT_COST_MULTIPLIER), DEFICIT_BIOMASS_WEIGHT);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::OnshoreWind, DEFAULT_COST_MULTIPLIER), DEFICIT_ONSHORE_WIND_WEIGHT);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::OffshoreWind, DEFAULT_COST_MULTIPLIER), DEFICIT_OFFSHORE_WIND_WEIGHT);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::UtilitySolar, DEFAULT_COST_MULTIPLIER), DEFICIT_UTILITY_SOLAR_WEIGHT);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::HydroDam, DEFAULT_COST_MULTIPLIER), DEFICIT_HYDRO_DAM_WEIGHT);
                deficit_year_weights.insert(GridAction::AddGenerator(GeneratorType::Nuclear, DEFAULT_COST_MULTIPLIER), DEFICIT_NUCLEAR_WEIGHT);
                deficit_year_weights.insert(GridAction::DoNothing, DEFICIT_DO_NOTHING_WEIGHT);
                deficit_weights.insert(year, deficit_year_weights);
            }
        }
        
        // Convert serializable best weights to actual best weights
        let best_weights = serializable.best_weights.map(|serializable_best_weights| {
            serializable_best_weights.iter()
                .map(|(year, serializable_year_weights)| {
                    let mut year_weights = HashMap::new();
                    for (serializable_action, weight) in serializable_year_weights {
                        let action = match serializable_action.action_type.as_str() {
                            "AddGenerator" => {
                                if let Some(gen_type_str) = &serializable_action.generator_type {
                                    match GeneratorType::from_str(gen_type_str) {
                                        Ok(gen_type) => {
                                            let cost_multiplier = serializable_action.cost_multiplier.unwrap_or(DEFAULT_COST_MULTIPLIER);
                                            GridAction::AddGenerator(gen_type, cost_multiplier)
                                        },
                                        Err(_) => continue,
                                    }
                                } else {
                                    continue;
                                }
                            },
                            "UpgradeEfficiency" => {
                                GridAction::UpgradeEfficiency(serializable_action.generator_id.clone().unwrap_or_default())
                            },
                            "AdjustOperation" => {
                                let id = serializable_action.generator_id.clone().unwrap_or_default();
                                let percentage = serializable_action.operation_percentage.unwrap_or(0);
                                // Skip legacy empty ID operations
                                if id.is_empty() {
                                    continue;
                                }
                                // Validate generator type exists
                                if let Some(gen_type_str) = &serializable_action.generator_type {
                                    if GeneratorType::from_str(gen_type_str).is_err() {
                                        continue;
                                    }
                                }
                                GridAction::AdjustOperation(id, percentage)
                            },
                            "AddCarbonOffset" => {
                                if let Some(offset_type_str) = &serializable_action.offset_type {
                                    let offset_type = match offset_type_str.as_str() {
                                        "Forest" => CarbonOffsetType::Forest,
                                        "Wetland" => CarbonOffsetType::Wetland,
                                        "ActiveCapture" => CarbonOffsetType::ActiveCapture,
                                        "CarbonCredit" => CarbonOffsetType::CarbonCredit,
                                        _ => CarbonOffsetType::Forest,
                                    };
                                    let cost_multiplier = serializable_action.cost_multiplier.unwrap_or(DEFAULT_COST_MULTIPLIER);
                                    GridAction::AddCarbonOffset(offset_type, cost_multiplier)
                                } else {
                                    GridAction::AddCarbonOffset(CarbonOffsetType::Forest, DEFAULT_COST_MULTIPLIER)
                                }
                            },
                            "CloseGenerator" => {
                                GridAction::CloseGenerator(serializable_action.generator_id.clone().unwrap_or_default())
                            },
                            "DoNothing" => GridAction::DoNothing,
                            _ => continue,
                        };
                        year_weights.insert(action, *weight);
                    }
                    (*year, year_weights)
                })
                .collect()
        });
        
        // Convert serializable best actions to actual best actions
        let best_actions = serializable.best_actions.map(|serializable_best_actions| {
            let mut best_actions = HashMap::new();
            for (year, serializable_actions) in &serializable_best_actions {
                let mut actions = Vec::new();
                for serializable_action in serializable_actions {
                    let action = match serializable_action.action_type.as_str() {
                        "AddGenerator" => {
                            if let Some(gen_type_str) = &serializable_action.generator_type {
                                match GeneratorType::from_str(gen_type_str) {
                                    Ok(gen_type) => {
                                        let cost_multiplier = serializable_action.cost_multiplier.unwrap_or(DEFAULT_COST_MULTIPLIER);
                                        GridAction::AddGenerator(gen_type, cost_multiplier)
                                    },
                                    Err(_) => continue,
                                }
                            } else {
                                continue;
                            }
                        },
                        "UpgradeEfficiency" => {
                            GridAction::UpgradeEfficiency(serializable_action.generator_id.clone().unwrap_or_default())
                        },
                        "AdjustOperation" => {
                            let id = serializable_action.generator_id.clone().unwrap_or_default();
                            let percentage = serializable_action.operation_percentage.unwrap_or(0);
                            // Skip legacy empty ID operations
                            if id.is_empty() {
                                continue;
                            }
                            // Validate generator type exists
                            if let Some(gen_type_str) = &serializable_action.generator_type {
                                if GeneratorType::from_str(gen_type_str).is_err() {
                                    continue;
                                }
                            }
                            GridAction::AdjustOperation(id, percentage)
                        },
                        "AddCarbonOffset" => {
                            if let Some(offset_type_str) = &serializable_action.offset_type {
                                let offset_type = match offset_type_str.as_str() {
                                    "Forest" => CarbonOffsetType::Forest,
                                    "Wetland" => CarbonOffsetType::Wetland,
                                    "ActiveCapture" => CarbonOffsetType::ActiveCapture,
                                    "CarbonCredit" => CarbonOffsetType::CarbonCredit,
                                    _ => CarbonOffsetType::Forest,
                                };
                                let cost_multiplier = serializable_action.cost_multiplier.unwrap_or(DEFAULT_COST_MULTIPLIER);
                                GridAction::AddCarbonOffset(offset_type, cost_multiplier)
                            } else {
                                GridAction::AddCarbonOffset(CarbonOffsetType::Forest, DEFAULT_COST_MULTIPLIER)
                            }
                        },
                        "CloseGenerator" => {
                            GridAction::CloseGenerator(serializable_action.generator_id.clone().unwrap_or_default())
                        },
                        "DoNothing" => GridAction::DoNothing,
                        _ => continue,
                    };
                    actions.push(action);
                }
                best_actions.insert(*year, actions);
            }
            best_actions
        });

        // Convert serializable best deficit actions to actual best deficit actions
        let best_deficit_actions = serializable.best_deficit_actions.map(|serializable_best_deficit_actions| {
            let mut best_deficit_actions = HashMap::new();
            for (year, serializable_actions) in &serializable_best_deficit_actions {
                let mut actions = Vec::new();
                for serializable_action in serializable_actions {
                    let action = match serializable_action.action_type.as_str() {
                        "AddGenerator" => {
                            if let Some(gen_type_str) = &serializable_action.generator_type {
                                match GeneratorType::from_str(gen_type_str) {
                                    Ok(gen_type) => {
                                        let cost_multiplier = serializable_action.cost_multiplier.unwrap_or(DEFAULT_COST_MULTIPLIER);
                                        GridAction::AddGenerator(gen_type, cost_multiplier)
                                    },
                                    Err(_) => continue,
                                }
                            } else {
                                continue;
                            }
                        },
                        "UpgradeEfficiency" => {
                            GridAction::UpgradeEfficiency(serializable_action.generator_id.clone().unwrap_or_default())
                        },
                        "AdjustOperation" => {
                            let id = serializable_action.generator_id.clone().unwrap_or_default();
                            let percentage = serializable_action.operation_percentage.unwrap_or(0);
                            // Skip legacy empty ID operations
                            if id.is_empty() {
                                continue;
                            }
                            // Validate generator type exists
                            if let Some(gen_type_str) = &serializable_action.generator_type {
                                if GeneratorType::from_str(gen_type_str).is_err() {
                                    continue;
                                }
                            }
                            GridAction::AdjustOperation(id, percentage)
                        },
                        "AddCarbonOffset" => {
                            if let Some(offset_type_str) = &serializable_action.offset_type {
                                let offset_type = match offset_type_str.as_str() {
                                    "Forest" => CarbonOffsetType::Forest,
                                    "Wetland" => CarbonOffsetType::Wetland,
                                    "ActiveCapture" => CarbonOffsetType::ActiveCapture,
                                    "CarbonCredit" => CarbonOffsetType::CarbonCredit,
                                    _ => CarbonOffsetType::Forest,
                                };
                                let cost_multiplier = serializable_action.cost_multiplier.unwrap_or(DEFAULT_COST_MULTIPLIER);
                                GridAction::AddCarbonOffset(offset_type, cost_multiplier)
                            } else {
                                GridAction::AddCarbonOffset(CarbonOffsetType::Forest, DEFAULT_COST_MULTIPLIER)
                            }
                        },
                        "CloseGenerator" => {
                            GridAction::CloseGenerator(serializable_action.generator_id.clone().unwrap_or_default())
                        },
                        "DoNothing" => GridAction::DoNothing,
                        _ => continue,
                    };
                    actions.push(action);
                }
                best_deficit_actions.insert(*year, actions);
            }
            best_deficit_actions
        });

        // Convert serializable improvement history to actual improvement history
        let improvement_history = serializable.improvement_history
            .map(|records| {
                records.into_iter()
                    .map(|record| {
                        crate::utils::csv_export::ImprovementRecord {
                            iteration: record.iteration,
                            score: record.score,
                            net_emissions: record.net_emissions,
                            total_cost: record.total_cost,
                            public_opinion: record.public_opinion,
                            power_reliability: record.power_reliability,
                            timestamp: record.timestamp,
                        }
                    })
                    .collect()
            })
            .unwrap_or_else(Vec::new);

        // Convert serializable prime weights to actual prime weights
        let prime_weights = serializable.prime_weights.map(|serializable_prime_weights| {
            serializable_prime_weights.iter()
                .map(|(year, serializable_year_weights)| {
                    let mut year_weights = HashMap::new();
                    for (serializable_action, weight) in serializable_year_weights {
                        let action = match serializable_action.action_type.as_str() {
                            "AddGenerator" => {
                                if let Some(gen_type_str) = &serializable_action.generator_type {
                                    match GeneratorType::from_str(gen_type_str) {
                                        Ok(gen_type) => {
                                            let cost_multiplier = serializable_action.cost_multiplier.unwrap_or(DEFAULT_COST_MULTIPLIER);
                                            GridAction::AddGenerator(gen_type, cost_multiplier)
                                        },
                                        Err(_) => continue,
                                    }
                                } else {
                                    continue;
                                }
                            },
                            "UpgradeEfficiency" => {
                                GridAction::UpgradeEfficiency(serializable_action.generator_id.clone().unwrap_or_default())
                            },
                            "AdjustOperation" => {
                                let id = serializable_action.generator_id.clone().unwrap_or_default();
                                let percentage = serializable_action.operation_percentage.unwrap_or(0);
                                // Skip legacy empty ID operations
                                if id.is_empty() {
                                    continue;
                                }
                                // Validate generator type exists
                                if let Some(gen_type_str) = &serializable_action.generator_type {
                                    if GeneratorType::from_str(gen_type_str).is_err() {
                                        continue;
                                    }
                                }
                                GridAction::AdjustOperation(id, percentage)
                            },
                            "AddCarbonOffset" => {
                                if let Some(offset_type_str) = &serializable_action.offset_type {
                                    let offset_type = match offset_type_str.as_str() {
                                        "Forest" => CarbonOffsetType::Forest,
                                        "Wetland" => CarbonOffsetType::Wetland,
                                        "ActiveCapture" => CarbonOffsetType::ActiveCapture,
                                        "CarbonCredit" => CarbonOffsetType::CarbonCredit,
                                        _ => CarbonOffsetType::Forest,
                                    };
                                    let cost_multiplier = serializable_action.cost_multiplier.unwrap_or(DEFAULT_COST_MULTIPLIER);
                                    GridAction::AddCarbonOffset(offset_type, cost_multiplier)
                                } else {
                                    GridAction::AddCarbonOffset(CarbonOffsetType::Forest, DEFAULT_COST_MULTIPLIER)
                                }
                            },
                            "CloseGenerator" => {
                                GridAction::CloseGenerator(serializable_action.generator_id.clone().unwrap_or_default())
                            },
                            "DoNothing" => GridAction::DoNothing,
                            _ => continue,
                        };
                        year_weights.insert(action, *weight);
                    }
                    (*year, year_weights)
                })
                .collect()
        });

        Ok(Self {
            weights,
            action_count_weights: HashMap::new(),
            learning_rate: serializable.learning_rate,
            best_metrics: serializable.best_metrics,
            best_weights,
            prime_weights,
            best_actions,
            iteration_count: serializable.iteration_count,
            iterations_without_improvement: serializable.iterations_without_improvement,
            exploration_rate: serializable.exploration_rate,
            current_run_actions: HashMap::new(),
            force_best_actions: false,
            deficit_weights,
            current_deficit_actions: HashMap::new(),
            best_deficit_actions,
            deterministic_rng: None,
            guaranteed_best_actions: false,
            optimization_mode: serializable.optimization_mode,
            replay_index: HashMap::new(),
            improvement_history,
            current_metrics: None,
        })
    }

}
