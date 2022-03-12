use wither::Model;
use serde::{Deserialize, Serialize};
use wither::bson::DateTime;
use wither::bson::oid::ObjectId;
use crate::models::SearchById;

#[derive(Debug, Model, Serialize, Deserialize, Clone)]
#[model(collection_name = "projects")]
pub struct Project {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    // 项目名称
    pub title: String,
    pub start_time: DateTime,
    // 持续周数
    pub duration: i32,
    // 是否运行中
    pub running: bool,
}

impl SearchById for Project {}