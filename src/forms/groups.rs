use serde_json::json;
use wither::bson::doc;
use wither::mongodb::Database;
use crate::errors::AppErrors;
use crate::forms::positions::UpdatePositionForm;
use crate::models::SearchById;
use crate::models::users::User;
use serde::Deserialize;
use crate::models::groups::Group;

#[derive(Deserialize)]
pub struct CreateGroupForm {
    pub name: String,
    pub points: Vec<UpdatePositionForm>,
}

impl CreateGroupForm {
    pub async fn validate(&self, db: &Database) -> Result<(), AppErrors> {
        if self.name.is_empty() {
            return Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "组名不能为空",
                    "en": "Group name cannot be empty",
                }
            })));
        }
        // 检查组名是否重复
        let groups = db.collection("groups");
        let count = groups.count_documents(Some(doc! {
            "name": self.name.clone()
        }), None).await;
        if count.is_err() {
            return Err(AppErrors::ValidationError(json!({
                "code": 3,
                "message": {
                    "cn": "数据库查询失败",
                    "en": "Database query failed",
                }
            })));
        }
        if count.unwrap() > 0 {
            return Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "组名已存在",
                    "en": "Group name already exists",
                }
            })));
        }
        Ok(())
    }
}

#[derive(Deserialize, Debug)]
pub struct UpdateGroupForm {
    pub name: Option<String>,
    pub cover: Option<String>,
    pub managers: Option<Vec<String>>,
}

impl UpdateGroupForm {
    pub async fn validate(&self, db: &Database) -> Result<(), AppErrors> {
        // 检查组名是否重复
        if let Some(name) = &self.name {
            let groups = db.collection("groups");
            let count = groups.count_documents(Some(doc! {
                "name": name
            }), None).await;
            if count.is_err() {
                return Err(AppErrors::ValidationError(json!({
                    "code": 3,
                    "message": {
                        "cn": "数据库查询失败",
                        "en": "Database query failed",
                    }
                })));
            }
            if count.unwrap() > 0 {
                return Err(AppErrors::ValidationError(json!({
                    "code": 4,
                    "message": {
                        "cn": "组名已存在",
                        "en": "Group name already exists",
                    },
                    "description": {
                        "name": name
                    }
                })));
            }
        }

        // 检查小组管理员是否存在
        if let Some(managers) = &self.managers {
            for manager in managers {
                if let None = User::by_id(&db, manager).await {
                    return Err(AppErrors::ValidationError(json!({
                        "code": 4,
                        "message": {
                            "cn": "小组管理员不存在",
                            "en": "Group manager does not exist",
                        },
                        "description": {
                            "id": manager
                        }
                    })));
                }
            }
        }
        Ok(())
    }
}

#[derive(Deserialize, Debug)]
pub struct JoinInvitationForm {
    pub expire_at: i64,
    pub groups: Vec<String>,
    pub permission: i8,
}

impl JoinInvitationForm {
    pub async fn validate(&self, db: &Database) -> Result<(), AppErrors> {
        // 检查小组是否存在
        for group in &self.groups {
            if let None = Group::by_id(&db, group).await {
                return Err(AppErrors::ValidationError(json!({
                    "code": 4,
                    "message": {
                        "cn": "小组不存在",
                        "en": "Group does not exist",
                    },
                    "description": {
                        "id": group
                    }
                })));
            }
        }
        // 检查权限是否合理
        if self.permission != 1 && self.permission != 2 {
            return Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "权限不合法",
                    "en": "Permission is not valid",
                }
            })));
        }
        Ok(())
    }
}