use wither::Model;
use serde::{Deserialize, Serialize};
use wither::bson::DateTime;
use wither::bson::oid::ObjectId;
use crate::models::SearchById;

#[derive(Debug, Model, Serialize, Deserialize, Clone)]
#[model(collection_name = "records")]
pub struct Record {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub num: i16,
    pub position: String,
    pub time: DateTime,
    pub collaborators: Vec<String>,
    pub description: String,
    // 属于的小组
    pub group: String,
    // 创建这个记录的用户
    pub user: String,
    // 这个记录属于的项目
    pub project: String,
    // 附件
    pub attachments: Vec<String>,
}

impl SearchById for Record {}