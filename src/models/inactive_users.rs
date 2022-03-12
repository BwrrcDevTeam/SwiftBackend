use wither::Model;
use serde::{Serialize, Deserialize};
// use serde_json::json;
use wither::bson::{DateTime, doc};
use wither::bson::oid::ObjectId;
use wither::mongodb::Database;
use crate::models::users::User;

#[derive(Debug, Model, Serialize, Deserialize, Clone)]
#[model(collection_name = "inactive_users")]
pub struct InactiveUser {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<String>>,
    pub email: String,
    pub password: String,
    pub permission: i8,
    pub name: String,
    // 验证码
    pub code: String,
    pub expire_at: DateTime, // 过期时间
}

impl InactiveUser {
    pub async fn by_code(db: &Database, code: String) -> Option<InactiveUser> {
        if let Ok(res) = InactiveUser::find_one(db, Some(doc! {"code": code}), None).await {
            res
        } else {
            None
        }
    }
    pub fn to_user(self) -> User {
        // 这个方法消耗自身
        User {
            id: None,
            groups: self.groups,
            email: self.email,
            password: self.password,
            permission: self.permission as f64,
            name: self.name,
            created_at: chrono::Utc::now().into(),
            avatar: None,
        }
    }
    // pub fn to_response(self) -> serde_json::Value {
    //     json!({
    //         "id": self.id.unwrap().to_hex(),
    //         "email": self.email,
    //         "name": self.name,
    //         "permission": self.permission,
    //         "expire_at": self.expire_at.timestamp(),
    //         "groups": self.groups,
    //         "code": self.code,
    //     })
    // }
}