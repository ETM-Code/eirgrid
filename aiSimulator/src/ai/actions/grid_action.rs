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
