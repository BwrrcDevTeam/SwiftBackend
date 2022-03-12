use serde_json::json;
use wither::bson::{doc, Document};
use wither::mongodb::Database;
use crate::errors::AppErrors;
use crate::models::groups::Group;
use crate::models::positions::Position;
use crate::models::SearchById;
use crate::models::storage::Storage;
use crate::models::users::User;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct RecordsQuery {
    user: Option<String>,
    // 创建者
    project: Option<String>,
    // 项目id
    group: Option<String>,
    // 分组id
    from: Option<i64>,
    // 开始时间
    to: Option<i64>, // 结束时间
}

impl RecordsQuery {
    pub fn to_filter(self) -> Document {
        let mut filter = doc! {};
        if let Some(user) = self.user {
            filter.insert("user", user);
        }
        if let Some(project) = self.project {
            filter.insert("project", project);
        }
        if let Some(group) = self.group {
            filter.insert("group", group);
        }
        if let Some(from) = self.from {
            let from = chrono::DateTime::from_utc(
                chrono::NaiveDateTime::from_timestamp(from, 0),
                chrono::Utc,
            );


            if let Some(to) = self.to {
                let to = chrono::DateTime::from_utc(
                    chrono::NaiveDateTime::from_timestamp(to, 0),
                    chrono::Utc,
                );

                filter.insert("time", doc! { "$gte": from, "$lte": to });
            } else {
                filter.insert("time", doc! { "$gte": from });
            }
        }
        if let Some(to) = self.to {
            if let None = self.from {
                let to = chrono::DateTime::from_utc(
                    chrono::NaiveDateTime::from_timestamp(to, 0),
                    chrono::Utc,
                );
                filter.insert("time", doc! { "$lte": to });
            }
        }

        filter
    }
}

#[derive(Deserialize)]
pub struct NewRecordForm {
    pub group: String,
    pub time: i64,
    pub num: i64,
    pub position: String,
    pub collaborators: Option<Vec<String>>,
    pub attachments: Option<Vec<String>>,
    pub description: Option<String>,
}

impl NewRecordForm {
    pub async fn validate(&self, db: &Database) -> Result<(), AppErrors> {
        if let None = Group::by_id(&db, &self.group).await {
            return Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "调查小组不存在",
                    "en": "Group does not exist"
                }
            })));
        }
        if let Some(collaborators) = &self.collaborators {
            for collaborator in collaborators {
                if let None = User::by_id(&db, &collaborator).await {
                    return Err(AppErrors::ValidationError(json!({
                        "code": 4,
                        "message": {
                            "cn": "协作者不存在",
                            "en": "Collaborator does not exist"
                        },
                        "description": {
                            "collaborator": collaborator
                        }
                    })));
                }
            }
        }
        if let Some(attachments) = &self.attachments {
            for attachment in attachments {
                if let None = Storage::by_id(&db, &attachment).await {
                    return Err(AppErrors::ValidationError(json!({
                        "code": 4,
                        "message": {
                            "cn": "附件不存在",
                            "en": "Attachment does not exist"
                        },
                        "description": {
                            "attachment": attachment
                        }
                    })));
                }
            }
        }
        if let Some(position) = Position::by_id(&db, &self.position).await {
            if position.belongs_to != self.group {
                return Err(AppErrors::ValidationError(json!({
                    "code": 4,
                    "message": {
                        "cn": "调查小组与调查位置不匹配",
                        "en": "Group does not match position"
                    }
                })));
            }
        } else {
            return Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "调查位置不存在",
                    "en": "Position does not exist"
                }
            })));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
pub struct UpdateRecordForm {
    pub time: Option<i64>,
    pub num: Option<i64>,
    pub collaborators: Option<Vec<String>>,
    pub attachments: Option<Vec<String>>,
    pub description: Option<String>,
    pub position: Option<String>,
    pub group: Option<String>,
}

impl UpdateRecordForm {
    pub async fn validate(&self, db: &Database) -> Result<(), AppErrors> {
        // 检查协作者
        if let Some(collaborators) = &self.collaborators {
            for collaborator in collaborators {
                if let None = User::by_id(&db, &collaborator).await {
                    return Err(AppErrors::ValidationError(json!({
                        "code": 4,
                        "message": {
                            "cn": "协作者不存在",
                            "en": "Collaborator does not exist"
                        },
                        "description": {
                            "collaborator": collaborator
                        }
                    })));
                }
            }
        }
        // 检查附件
        if let Some(attachments) = &self.attachments {
            for attachment in attachments {
                if let None = Storage::by_id(&db, &attachment).await {
                    return Err(AppErrors::ValidationError(json!({
                        "code": 4,
                        "message": {
                            "cn": "附件不存在",
                            "en": "Attachment does not exist"
                        },
                        "description": {
                            "attachment": attachment
                        }
                    })));
                }
            }
        }
        // 检查调查位置
        if let Some(position) = &self.position {
            if let None = Position::by_id(&db, &position).await {
                return Err(AppErrors::ValidationError(json!({
                    "code": 4,
                    "message": {
                        "cn": "调查位置不存在",
                        "en": "Position does not exist"
                    }
                })));
            }
        }
        // 检查调查小组
        if let Some(group) = &self.group {
            if let None = Group::by_id(&db, &group).await {
                return Err(AppErrors::ValidationError(json!({
                    "code": 4,
                    "message": {
                        "cn": "调查小组不存在",
                        "en": "Group does not exist"
                    }
                })));
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
pub struct UpdateDraftForm {
    pub time: Option<i32>,
    pub num: Option<i32>,
    pub collaborators: Option<Vec<String>>,
    pub attachments: Option<Vec<String>>,
    pub description: Option<String>,
    pub position: Option<String>,
}