use wither::Model;
use serde::{Serialize, Deserialize};
use wither::bson::{DateTime, doc};
use wither::bson::oid::ObjectId;
use wither::mongodb::Database;

#[derive(Debug, Model, Serialize, Deserialize, Clone)]
#[model(collection_name = "invitations")]
pub struct Invitation {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub code: String,
    pub expire_at: DateTime,
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<String>>,
    pub permission: i8,
}

impl Invitation {
    pub async fn by_code(db: &Database, code: String) -> Option<Invitation> {
        if let Ok(res) = Invitation::find_one(db, Some(doc! {"code": code}), None).await {
            res
        } else {
            None
        }
    }
}