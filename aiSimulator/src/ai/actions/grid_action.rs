// Grid Action module - contains the GridAction enum definition
use serde::{Serialize, Deserialize};
use crate::models::generator::GeneratorType;
use crate::models::carbon_offset::CarbonOffsetType;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum GridAction {
    // Add generator with type and construction cost multiplier (as percentage: 100-500%)
    AddGenerator(GeneratorType, u16),
    UpgradeEfficiency(String),  // Generator ID
    AdjustOperation(String, u8),  // Generator ID, percentage (0-100)
    // Add carbon offset with type and construction cost multiplier (as percentage: 100-500%)
    AddCarbonOffset(CarbonOffsetType, u16),
    CloseGenerator(String),  // Generator ID
    DoNothing, // New no-op action
}

impl std::fmt::Display for GridAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GridAction::AddGenerator(gen_type, cost_multiplier) => {
                write!(f, "AddGenerator({}, {}%)", gen_type, cost_multiplier)
            },
            GridAction::UpgradeEfficiency(id) => {
                write!(f, "UpgradeEfficiency({})", id)
            },
            GridAction::AdjustOperation(id, percentage) => {
                write!(f, "AdjustOperation({}, {}%)", id, percentage)
            },
            GridAction::AddCarbonOffset(offset_type, cost_multiplier) => {
                write!(f, "AddCarbonOffset({}, {}%)", offset_type, cost_multiplier)
            },
            GridAction::CloseGenerator(id) => {
                write!(f, "CloseGenerator({})", id)
            },
            GridAction::DoNothing => {
                write!(f, "DoNothing")
            },
        }
    }
}
