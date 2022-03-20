use serde_json::json;
use tide::Response;
use wither::bson::doc;
use wither::mongodb::Database;
use serde::Deserialize;
use crate::errors::AppErrors;
use crate::models::SearchById;
use crate::models::users::User;
use crate::models::groups::Group;

// 管理员直接创建新用户时的Form
#[derive(Deserialize)]
pub struct NewUserForm {
    pub name: String,
    pub password: String,
    pub email: String,
    pub permission: i8,
    pub groups: Option<Vec<String>>, // 允许多个小组ID
}

// 校验邮箱格式
fn check_email(email: String) -> bool {
    let re = regex::Regex::new(r"^[a-zA-Z0-9_-]+@[a-zA-Z0-9_-]+(\.[a-zA-Z0-9_-]+)+$").unwrap();
    re.is_match(&email)
}


async fn register_check_name(name: String, db: &Database) -> Result<(), AppErrors> {
    if name.len() > 32 || name.len() < 2 {
        return Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "用户名称长度必须在2-32个字符之间",
                    "en": "Username length must be between 2-32 characters"
                },
                "description": {
                    "max": 32,
                    "min": 2,
                    "got": name.len()
                }
            })));
    }
    // 这部分没有必要使用数据模型
    let users = db.collection("users");
    let user = users.find_one(Some(doc! {
            "name": name.clone()
        }), None).await.unwrap_or(None);
    if user.is_some() {
        return Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "这个用户名已经存在",
                    "en": "Username already exists"
                },
                "description": {
                    "name": name.clone()
                }
            })));
    }
    Ok(())
}

fn check_password(password: String) -> Result<(), AppErrors> {
    let mut error_resp = Response::new(400);
    error_resp.set_content_type("application/json");
    // 这里的密码已经被加密了 必须为32位
    if password.len() != 32 {
        return Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "密码长度必须为32位",
                    "en": "Password length must be 32"
                },
                "description": {
                    "max": 32,
                    "min": 32,
                    "got": password.len()
                }
            })));
    }

    Ok(())
}

async fn register_check_email(email: String, db: &Database) -> Result<(), AppErrors> {
    // 检查邮箱格式 并检查是否已经存在
    if !check_email(email.clone()) {
        return Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "邮箱格式不正确",
                    "en": "Email format is not correct"
                },
                "description": {
                    "got": email.clone()
                }
            })));
    }
    // 检查邮箱是否已经被注册
    let users = db.collection("users");

    let user = users.find_one(Some(doc! {
            "email": email.clone()
        }), None).await.unwrap_or(None);
    if user.is_some() {
        return Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "这个邮箱已经被注册",
                    "en": "Email already exists"
                },
                "description": {
                    "email": email.clone()
                }
            })));
    }
    Ok(())
}

async fn check_groups(group_ids: &Vec<String>, db: &Database) -> Result<(), AppErrors> {
    for group_id in group_ids.iter() {
        // 尝试将其转换为ObjectId
        if Group::by_id(&db, group_id).await.is_none() {
            return Err(AppErrors::ValidationError(json!({
                    "code": 4,
                    "message": {
                        "cn": "这个小组不存在",
                        "en": "Group does not exist"
                    },
                    "description": {
                        "id": group_id
                    }
                })));
        }
    }
    Ok(())
}

impl NewUserForm {
    pub async fn validate(&self, db: &Database) -> Result<(), AppErrors> {
        // 首先执行非联网验证
        register_check_name(self.name.clone(), db).await?;
        check_password(self.password.clone())?;
        register_check_email(self.email.clone(), db).await?;

        if self.permission < 0 || self.permission > 3 {
            return Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "权限值必须在0-3之间",
                    "en": "Permission value must be between 0-3"
                },
                "description": {
                    "max": 3,
                    "min": 0,
                    "got": self.permission
                }
            })));
        }

        // 检查小组是否存在
        check_groups(self.groups.as_ref().unwrap_or(&vec![]), db).await?;
        // let groups = db.collection("groups");
        // for group_id in self.groups.as_ref().unwrap_or(&vec![]) {
        //     // 尝试将其转换为ObjectId
        //     let oid = try_into_object_id(group_id.to_owned())?;
        //     let group = groups.find_one(Some(doc! {
        //         "id": oid
        //     }), None).await.unwrap_or(None);
        //     if group.is_none() {
        //         return Err(AppErrors::ValidationError(json!({
        //             "code": 4,
        //             "message": {
        //                 "cn": "这个小组不存在",
        //                 "en": "Group does not exist"
        //             },
        //             "description": {
        //                 "id": group_id
        //             }
        //         })));
        //     }
        // }
        Ok(())
    }
    pub fn to_user(self) -> User {
        User {
            id: None,
            name: self.name,
            password: self.password,
            email: self.email,
            permission: self.permission as f64,
            groups: self.groups,
            created_at: chrono::Utc::now().into(),
            avatar: None,
        }
    }
}

// 激活一个InactiveUser
#[derive(Debug, Deserialize)]
pub struct NewUserFromInactive {
    // pub name: String,
    // pub password: String,
    pub code: String, // 验证码
}

impl NewUserFromInactive {
    pub async fn validate(&self, db: &Database) -> Result<(), AppErrors> {
        // // 首先执行非联网验证
        // register_check_name(self.name.clone(), db).await?;
        // check_password(self.password.clone())?;

        // 检查InactiveUser是否存在
        let codes = db.collection("inactive_users");
        let code = codes.find_one(Some(doc! {
            "code": self.code.clone()
        }), None).await.unwrap_or(None);
        if code.is_none() {
            return Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "验证码不存在",
                    "en": "Verification code does not exist"
                },
                "description": {
                    "code": self.code.clone()
                }
            })));
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub id: String,
    pub password: String,
}

impl LoginForm {
    pub async fn validate(&self, db: &Database) -> Result<(), AppErrors> {
        // 首先执行非联网验证
        if User::by_id(&db, &self.id).await.is_none() {
            // 登录逻辑改为了用uid登录
            return Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "用户不存在",
                    "en": "User does not exist"
                },
                "description": {
                    "id": self.id
                }
            })));
        }
        check_password(self.password.clone())?;
        Ok(())
    }
}


#[derive(Debug, Deserialize)]
pub struct NewInvitationForm {
    pub groups: Option<Vec<String>>,
    pub expire_at: i64,
    pub permission: i8,
}

impl NewInvitationForm {
    pub async fn validate(&self, db: &Database) -> Result<(), AppErrors> {
        // 检查小组是否存在
        check_groups(self.groups.as_ref().unwrap_or(&vec![]), db).await?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserForm {
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub permission: Option<i8>,
}

impl UpdateUserForm {
    pub async fn validate(&self, db: &Database, user_id: &String) -> Result<(), AppErrors> {
        // 检查用户是否存在
        let users = db.collection("users");
        let user = users.find_one(Some(doc! {
            "id": user_id
        }), None).await.unwrap_or(None);
        if user.is_none() {
            return Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "这个用户不存在",
                    "en": "User does not exist"
                },
                "description": {
                    "id": user_id
                }
            })));
        }
        // 检查是否重名
        if let Some(name) = self.name.to_owned() {
            register_check_name(name.to_owned(), db).await?;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateInactiveUserForm {
    pub invitation: String,
    // 邀请码
    pub email: String,
    pub password: String,
    pub name: String,
    pub lang: String, // 语言
}

impl CreateInactiveUserForm {
    pub async fn validate(&self, db: &Database) -> Result<(), AppErrors> {
        // 这里不用检查邀请码是否存在，可以节省逻辑
        // 离线验证
        // 决定允许重复邮箱 但不允许重复用户名
        // register_check_email(self.email.clone(), db).await?;
        // // 检查邮箱是否存在
        // let users = db.collection("users");
        // let user = users.find_one(Some(doc! {
        //     "email": self.email.clone()
        // }), None).await.unwrap_or(None);
        // if user.is_some() {
        //     return Err(AppErrors::ValidationError(json!({
        //         "code": 4,
        //         "message": {
        //             "cn": "这个邮箱已经被注册",
        //             "en": "This email has been registered"
        //         },
        //         "description": {
        //             "email": self.email.clone()
        //         }
        //     })));
        // }

        // 检查用户名是否存在
        let users = db.collection("users");
        let user = users.find_one(Some(doc! {
            "name": self.name.clone()
        }), None).await.unwrap_or(None);
        if user.is_some() {
            return Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "这个用户名已经被注册",
                    "en": "This username has been registered"
                },
                "description": {
                    "name": self.name.clone()
                }
            })));
        }
        Ok(())
    }
}