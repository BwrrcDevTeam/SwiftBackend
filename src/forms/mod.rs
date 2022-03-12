use serde_json::json;
use wither::bson::oid::ObjectId;
use crate::errors::AppErrors;

pub mod users;
pub mod positions;
pub mod groups;

pub fn try_into_object_id(id: String) -> Result<ObjectId, AppErrors> {
    match ObjectId::with_string(&id) {
        Ok(oid) => Ok(oid),
        Err(_) => {
            Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "en": "Invalid id",
                    "cn": "无效的id"
                }
            })))
        }
    }
}