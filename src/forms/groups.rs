use serde_json::json;
use crate::errors::AppErrors;
use crate::forms::positions::UpdatePositionForm;

pub struct CreateGroupForm {
    name: String,
    points: Vec<UpdatePositionForm>,
}

impl CreateGroupForm {
    pub fn validate(&self) -> Result<(), AppErrors> {
        if self.name.is_empty() {
            return Err(AppErrors::ValidationError(json!({
                "code": 4,
                ""
            })))
        }
    }
}