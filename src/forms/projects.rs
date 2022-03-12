use serde_json::json;
use wither::bson::doc;
use wither::mongodb::Database;
use crate::errors::AppErrors;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct NewProjectForm {
    pub title: String,
    pub start_time: i32,
    pub duration: i32,
    pub running: bool,
}

impl NewProjectForm {
    pub async fn validate(&self, db: &Database) -> Result<(), AppErrors> {
        // 检查有无重名项目
        let projects = db.collection("projects");
        let count = projects.count_documents(Some(doc! { "title": self.title.clone() }), None).await.unwrap_or(0);
        if count > 0 {
            return Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "这个项目标题已存在",
                    "en": "This project title already exists"
                }
            })));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
pub struct UpdateProjectForm {
    pub title: Option<String>,
    pub start_time: Option<i32>,
    pub duration: Option<i32>,
    pub running: Option<bool>,
}


impl UpdateProjectForm {
    pub async fn validate(&self, db: &Database) -> Result<(), AppErrors> {
        // 检查有无重名项目
        if let Some(title) = &self.title {
            let projects = db.collection("projects");
            let count = projects.count_documents(Some(doc! { "title": title }), None).await.unwrap_or(0);
            if count > 0 {
                return Err(AppErrors::ValidationError(json!({
                    "code": 4,
                    "message": {
                        "cn": "这个项目标题已存在",
                        "en": "This project title already exists"
                    }
                })));
            }
        }

        Ok(())
    }
}