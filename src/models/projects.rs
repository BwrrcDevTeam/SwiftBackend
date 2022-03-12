use wither::Model;
use serde::{Deserialize, Serialize};
use serde_json::json;
use wither::bson::{DateTime, doc};
use wither::bson::oid::ObjectId;
use wither::mongodb::Database;
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

impl Project {
    pub async fn get_running_project(db: &Database) -> Option<Project> {
        let filter = doc! {"running": true};
        if let Ok(project) = Project::find_one(db, filter, None).await {
            project
        } else {
            None
        }
    }

    pub fn to_response(self) -> serde_json::Value {
        json!({
            "id": self.id.unwrap().to_string(),
            "title": self.title,
            "start_time": self.start_time.timestamp(),
            "duration": self.duration,
            "running": self.running
        })
    }
}