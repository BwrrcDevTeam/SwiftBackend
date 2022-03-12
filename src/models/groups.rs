use wither::bson::DateTime;
use wither::bson::oid::ObjectId;
use crate::models::SearchById;
use wither::Model;
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};


#[derive(Model, Debug, Clone, Serialize, Deserialize)]
#[model(collection_name = "groups")]
pub struct Group {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub name: String,
    pub created_at: DateTime,
    pub managers: Vec<String>,
    pub cover: Option<String>,
}

impl SearchById for Group {}

impl Group {
    pub fn to_response(self) -> Value {
        json!({
            "id": self.id.unwrap().to_hex(),
            "name": self.name,
            "created_at": self.created_at.timestamp(),
            "managers": self.managers,
            "cover": self.cover,
        })
    }
}