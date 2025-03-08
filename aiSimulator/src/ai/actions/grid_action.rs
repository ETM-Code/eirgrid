// Grid Action module - contains the GridAction enum definition
use serde::{Serialize, Deserialize};
use crate::models::generator::GeneratorType;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum GridAction {
    AddGenerator(GeneratorType),
    UpgradeEfficiency(String),  // Generator ID
    AdjustOperation(String, u8),  // Generator ID, percentage (0-100)
    AddCarbonOffset(String),  // Offset type
    CloseGenerator(String),  // Generator ID
    DoNothing, // New no-op action
}
