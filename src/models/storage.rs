use wither::bson::{DateTime, doc};
use wither::bson::oid::ObjectId;
use wither::Model;
use serde::{Serialize, Deserialize};
use serde_json::json;
use crate::models::SearchById;

#[derive(Debug, Model, Serialize, Deserialize, Clone)]
#[model(collection_name = "storage")]
pub struct Storage {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub filename: String,
    pub local_path: String,
    pub mime_type: String,
    pub created_at: DateTime,
    pub owner: String,
    // pub md5: String,
}

impl SearchById for Storage {}

impl Storage {
    pub fn to_response(self) -> serde_json::Value {
        json!({
            "id": self.id.unwrap().to_hex(),
            "filename": self.filename,
            "mime_type": self.mime_type,
            "created_at": self.created_at.timestamp(),
            "owner": self.owner,
        })
    }
}