use wither::bson::{DateTime, doc};
use wither::bson::oid::ObjectId;
use wither::mongodb::Database;
use wither::Model;
use serde::{Serialize, Deserialize};
use serde_json::json;
use crate::models::SearchById;

#[derive(Debug, Model, Serialize, Deserialize, Clone)]
#[model(collection_name = "users")]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub password: String,
    pub email: String,
    pub name: String,
    pub created_at: DateTime,
    pub groups: Option<Vec<String>>,
    pub permission: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

impl User {
    pub async fn by_email(db: &Database, email: &String) -> Option<Self> {
        let filter = doc! {"email": email};
        let user = User::find_one(db, Some(filter), None).await;
        if user.is_err() {
            return None;
        }
        user.unwrap()
    }
    pub fn to_response(&self) -> serde_json::Value {
        json!(UserResponse {
            id: self.id.clone().unwrap().to_string(),
            name: self.name.clone(),
            email: self.email.clone(),
            permission: self.permission as i8,
            created_at: self.created_at.timestamp(),
            groups: self.groups.clone(),
            avatar: self.avatar.clone(),
        })
    }
}

impl SearchById for User {}

// 用于返回给前端的用户信息
#[derive(Debug, Serialize, Clone)]
pub struct UserResponse {
    pub id: String,
    pub name: String,
    pub email: String,
    pub permission: i8,
    pub created_at: i64,
    // 变成时间戳
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}