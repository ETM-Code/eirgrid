// Serializable Action module - contains the SerializableAction struct
use serde::{Serialize, Deserialize};
use super::grid_action::GridAction;

#[derive(Serialize, Deserialize)]
pub struct SerializableAction {
    pub action_type: String,
    pub generator_type: Option<String>,
    pub generator_id: Option<String>,
    pub operation_percentage: Option<u8>,
    pub offset_type: Option<String>,
}

impl From<&GridAction> for SerializableAction {
    fn from(action: &GridAction) -> Self {
        match action {
            GridAction::AddGenerator(gen_type) => SerializableAction {
                action_type: "AddGenerator".to_string(),
                generator_type: Some(gen_type.to_string()),
                generator_id: None,
                operation_percentage: None,
                offset_type: None,
            },
            GridAction::UpgradeEfficiency(id) => SerializableAction {
                action_type: "UpgradeEfficiency".to_string(),
                generator_type: None,
                generator_id: Some(id.clone()),
                operation_percentage: None,
                offset_type: None,
            },
            GridAction::AdjustOperation(id, percentage) => SerializableAction {
                action_type: "AdjustOperation".to_string(),
                generator_type: None,
                generator_id: Some(id.clone()),
                operation_percentage: Some(*percentage),
                offset_type: None,
            },
            GridAction::AddCarbonOffset(offset_type) => SerializableAction {
                action_type: "AddCarbonOffset".to_string(),
                generator_type: None,
                generator_id: None,
                operation_percentage: None,
                offset_type: Some(offset_type.clone()),
            },
            GridAction::CloseGenerator(id) => SerializableAction {
                action_type: "CloseGenerator".to_string(),
                generator_type: None,
                generator_id: Some(id.clone()),
                operation_percentage: None,
                offset_type: None,
            },
            GridAction::DoNothing => SerializableAction {
                action_type: "DoNothing".to_string(),
                generator_type: None,
                generator_id: None,
                operation_percentage: None,
                offset_type: None,
            },
        }
    }
}
