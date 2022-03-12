use serde::Deserialize;
use serde_json::json;
use wither::bson::doc;
use wither::mongodb::Database;
use crate::errors::AppErrors;
use crate::forms::try_into_object_id;

#[derive(Deserialize, Clone, Debug)]
pub struct NewPositionForm {
    pub group_id: String,
    pub name: String,
    pub longitude: f64,
    pub latitude: f64,
}

impl NewPositionForm {
    pub async fn validate(&self, db: &Database) -> Result<(), AppErrors> {
        // 先离线校验
        if self.longitude < -180.0 || self.longitude > 180.0 {
            return Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "经度超出范围",
                    "en": "Longitude out of range"
                }
            })));
        }
        if self.latitude < -90.0 || self.latitude > 90.0 {
            return Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "纬度超出范围",
                    "en": "Latitude out of range"
                }
            })));
        }
        // 检查调查组是否存在
        let oid = try_into_object_id(self.group_id.to_owned())?;
        let group = db.collection("groups").find_one(doc! {
            "_id": oid
        }, None).await;
        if group.is_err() {
            return Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "调查组不存在",
                    "en": "Group not found"
                }
            })));
        }
        if group.unwrap().is_none() {
            return Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "调查组不存在",
                    "en": "Group not found"
                }
            })));
        }
        Ok(())
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct UpdatePositionForm {
    pub name: String,
    pub longitude: f64,
    pub latitude: f64,
}