use wither::bson::{DateTime, doc};
use wither::bson::oid::ObjectId;
use wither::mongodb::Database;
use wither::Model;
use serde::{Serialize, Deserialize};
use serde_json::json;
use crate::models::SearchById;
use futures::StreamExt;
use crate::models::groups::Group;

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
    pub async fn by_email(db: &Database, email: &String) -> Vec<Self> {
        let filter = doc! {"email": email};
        let users_: Vec<_> = User::find(db, Some(filter), None)
            .await
            .expect("Failed to find users")
            .collect()
            .await;
        let mut users = Vec::new();
        for user in users_ {
            if let Ok(user) = user {
                users.push(user);
            }
        }
        users
    }
    pub async fn by_name(db: &Database, name: &String) -> Option<Self> {
        let filter = doc! {"name": name};
        if let Ok(user) = User::find_one(db, Some(filter), None).await {
            user
        } else {
            None
        }
    }
    pub async fn to_response(&self, db: &Database) -> serde_json::Value {
        let mut groups = Vec::new();
        for group_id in self.groups.as_ref().unwrap_or(&Vec::new()) {
            groups.push(Group::by_id(&db, group_id).await.unwrap().to_response())
        }
        json!(UserResponse {
            id: self.id.clone().unwrap().to_string(),
            name: self.name.clone(),
            email: self.email.clone(),
            permission: self.permission as i8,
            created_at: self.created_at.timestamp(),
            groups: groups,
            avatar: self.avatar.clone(),
        })
    }
    pub async fn by_group(db: &Database, group: &String) -> Vec<Self> {
        let filter = doc! {"groups": {"$elemMatch": {
            "$eq": group
        }}};
        let mut result = Vec::new();
        let users: Vec<_> = User::find(db, Some(filter), None)
            .await
            .unwrap()
            .collect()
            .await;
        for user in users {
            if let Ok(user) = user {
                result.push(user);
            }
        }
        result
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
    pub groups: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}