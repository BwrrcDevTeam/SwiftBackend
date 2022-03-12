pub mod detections;
pub mod users;
pub mod storage;
pub mod invitations;
pub mod inactive_users;
pub mod drafts;
pub mod positions;
pub mod groups;
pub mod records;
pub mod projects;

use wither::bson::{DateTime, doc, oid::ObjectId};
use serde::{Serialize, Deserialize};
use wither::Model;
use wither::mongodb::Database;
use wither::mongodb::options::{FindOneAndUpdateOptions, ReturnDocument};
use crate::models::users::User;


#[derive(Debug, Model, Serialize, Deserialize, Clone)]
#[model(collection_name = "sessions")]
pub struct Session {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub fingerprint: String,
    // 指纹
    pub login: bool,
    // 是否登录
    pub permission: i8,
    // 权限
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    // string格式的uid
    pub expire_at: DateTime,
    // 过期时间
    pub ip: String, // ip地址
}

// 用于转换为response的结构体
#[derive(Debug, Serialize, Clone)]
pub struct SessionResponse {
    pub fingerprint: String,
    pub login: bool,
    pub permission: i8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<serde_json::Value>,
    pub expire_at: i64,
    pub ip: String,
}

impl Session {
    pub async fn find_by_fingerprint(db: &Database, fingerprint: &str) -> Option<Self> {
        Session::find_one(db, Some(doc! {"fingerprint": fingerprint}), None).await.unwrap_or(None)
    }
    pub async fn update_timeout(self, db: &Database, timeout: u64) -> wither::Result<Self> {
        let expire_at = chrono::prelude::Utc::now() + chrono::Duration::seconds(timeout as i64);
        let update = doc! {
            "$set": doc!{
                "expire_at": expire_at,
            }
        };
        let mut opts = FindOneAndUpdateOptions::default();
        opts.return_document = Some(ReturnDocument::After);
        self.update(db, None, update, Some(opts)).await
    }
    pub async fn update_ip(self, db: &Database, ip: &str) -> wither::Result<Self> {
        let update = doc! {
            "$set": doc!{
                "ip": ip,
            }
        };
        let mut opts = FindOneAndUpdateOptions::default();
        opts.return_document = Some(ReturnDocument::After);
        self.update(db, None, update, Some(opts)).await
    }
    pub async fn to_response(&self, db: &Database) -> SessionResponse {
        if let Some(user_id) = &self.user {
            SessionResponse {
                fingerprint: self.fingerprint.clone(),
                login: self.login,
                permission: self.permission,
                user: Some(User::by_id(db, user_id).await.unwrap().to_response()),
                expire_at: self.expire_at.timestamp(),
                ip: self.ip.clone(),
            }
        } else {
            SessionResponse {
                fingerprint: self.fingerprint.clone(),
                login: self.login,
                permission: self.permission,
                user: None,
                expire_at: self.expire_at.timestamp(),
                ip: self.ip.clone(),
            }
        }
    }
}

#[async_trait::async_trait]
pub trait SearchById {
    async fn by_id(db: &Database, id: &String) -> Option<Self>
        where Self: Model + Send + Sync + 'static {
        let oid = ObjectId::with_string(id);
        if oid.is_err() {
            return None;
        }
        let oid = oid.unwrap();
        let filter = doc! {"_id": oid};
        let content = Self::find_one(db, Some(filter), None).await;
        if content.is_err() {
            return None;
        }
        content.unwrap()
    }
}