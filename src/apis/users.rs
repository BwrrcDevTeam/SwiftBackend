// =====================
// Finished at 2022-3-10
// Purified at 2022-3-10
// By lihe07
// =====================


use log::info;
use rand::Rng;
use serde_json::{json, Value};
use tide::{Request, Response, Server, StatusCode};

use crate::apis::{json_response, require_perm};
use crate::AppState;
use crate::errors::AppErrors;
use crate::forms::users::{CreateInactiveUserForm, LoginForm, NewInvitationForm, NewUserForm, NewUserFromInactive, UpdateUserForm};
use crate::models::inactive_users::InactiveUser;
use crate::models::{SearchById, Session};

use wither::Model;
use crate::models::invitations::Invitation;
use crate::models::users::User;
use futures::StreamExt;
use crate::models::groups::Group;

pub fn register(app: &mut Server<AppState>) {
    info!("注册用户API");
    app.at("/users")
        .get(api_get_users)
        .post(api_create_user);
    // .get(app_get_users);
    app.at("/users/check_email")
        .post(api_check_email);
    app.at("/users/check_name")
        .post(api_check_name);
    app.at("/users/login")
        .post(api_login);
    app.at("/users/logout")
        .get(api_logout);
    app.at("/users/register_invitations")
        .post(api_new_register_invitation);
    app.at("/users/register_invitations/:code")
        .get(api_get_register_invitation);
    app.at("/users/:id")
        .get(api_get_user)
        .patch(api_update_user);
    app.at("/users/inactive")
        .post(api_create_inactive_user);
}


async fn api_create_user(mut req: Request<AppState>) -> tide::Result<Response> {
    require_perm(&req, vec![0, 3]).await?;
    let state = req.state();
    let db = state.db.clone();
    let session: &Session = req.ext().unwrap();
    if session.permission == 0 {
        // 普通用户
        let form: NewUserFromInactive = req.body_json().await?;
        form.validate(&db).await?;

        if let Some(inactive_user) = InactiveUser::by_code(&db, form.code.clone()).await {
            let mut user = inactive_user.to_user();
            // 将其保存到数据库

            info!("新注册用户: {}", user.name);
            user.save(&db, None).await?;
            for group_id in user.groups.as_ref().unwrap_or(&vec![]) {
                // 如果是管理员 则将其加入管理员组
                if user.permission == 2.0 || user.permission == 3.0 {
                    let mut group = Group::by_id(&db, &group_id).await.unwrap();
                    group.managers.push(user.id.as_ref().unwrap().to_hex());
                    group.save(&db, None).await?;
                }
            }
            InactiveUser::by_code(&db, form.code).await.unwrap().delete(&db).await?;
            Ok(user.to_response().into())
        } else {
            Err(AppErrors::ValidationError(json!({
                "code": 4,
                "message": {
                    "cn": "无效的验证码",
                    "en": "Invalid verification code"
                }
            })).into())
        }
    } else if session.permission == 3 {
        // 管理员
        let form: NewUserForm = req.body_json().await?;
        form.validate(&db).await?;
        let mut user = form.to_user();
        // 将其保存到数据库
        user.save(&db, None).await?;
        Ok(user.to_response().into())
    } else {
        Err(AppErrors::CrossPermissionError(session.permission, vec![0, 3]).into())
    }
}

// 获取相同email的用户列表
async fn api_check_email(mut req: Request<AppState>) -> tide::Result<Response> {
    let state = req.state();
    let db = state.db.clone();
    let email: Value = req.body_json().await?;
    let email = email.get("email").ok_or(AppErrors::ValidationError(json!({
        "code": 4,
        "message": {
            "cn": "邮箱不能为空",
            "en": "Email cannot be empty"
        }
    })))?.as_str().ok_or(AppErrors::ValidationError(json!({
        "code": 4,
        "message": {
            "cn": "邮箱格式不正确",
            "en": "Email format is incorrect"
        }
    })))?;
    let users = User::by_email(&db, &email.to_string()).await;
    let mut resp = Vec::new();
    for user in users {
        resp.push(user.to_response());
    }
    Ok(json!(resp).into())
}

async fn api_check_name(mut req: Request<AppState>) -> tide::Result<Response> {
    let state = req.state();
    let db = state.db.clone();
    let name: Value = req.body_json().await?;
    let name = name.get("name").ok_or(AppErrors::ValidationError(json!({
        "code": 4,
        "message": {
            "cn": "用户名不能为空",
            "en": "User name cannot be empty"
        }
    })))?.as_str().ok_or(AppErrors::ValidationError(json!({
        "code": 4,
        "message": {
            "cn": "用户名格式不正确",
            "en": "User name format is incorrect"
        }
    })))?;
    if let Some(user) = User::by_name(&db, &name.to_string()).await {
        Ok(json!(user.to_response()).into())
    } else {
        Ok(json_response(404, json!({
            "code": 1001,
            "message": {
                "cn": "用户不存在",
                "en": "User does not exist"
            }
        })))
    }
}

async fn api_login(mut req: Request<AppState>) -> tide::Result<Response> {
    let state = req.state();
    let db = state.db.clone();
    let session: &Session = req.ext().unwrap();
    let mut session = session.to_owned();

    let form: LoginForm = req.body_json().await?;
    form.validate(&db).await?;
    // 已经验证过了，用户一定存在
    let user = User::by_id(&db, &form.id).await.unwrap();
    if user.password == form.password {
        // 修改Session
        session.login = true;
        session.permission = user.permission as i8;
        session.user = Some(user.id.as_ref().unwrap().to_hex());
        session.save(&db, None).await?;
        let mut resp = Response::new(200);
        resp.set_body(user.to_response());
        Ok(resp)
    } else {
        Err(AppErrors::ValidationError(json!({
            "code": 4,
            "message": {
                "cn": "邮箱或密码错误",
                "en": "Email or password is incorrect"
            }
        })).into())
    }
}

async fn api_logout(req: Request<AppState>) -> tide::Result<Response> {
    require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = state.db.clone();
    let session: &Session = req.ext().unwrap();
    let mut session = session.to_owned();
    session.login = false;
    session.permission = 0;
    session.user = None;
    session.save(&db, None).await?;
    let mut resp = Response::new(200);
    resp.set_body(json!({
        "code": 0,
        "message": {
            "cn": "注销成功",
            "en": "Logout successfully"
        }
    }));
    Ok(resp)
}


fn random_string(len: usize) -> String {
    let mut rng = rand::thread_rng();
    let mut s = String::new();
    for _ in 0..len {
        s.push(rng.gen_range(b'a'..b'z') as char);
    }
    s
}

// 新注册邀请
async fn api_new_register_invitation(mut req: Request<AppState>) -> tide::Result<Response> {
    require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = state.db.clone();
    let session: &Session = req.ext().unwrap();
    // 取得session的所有权
    let session = session.to_owned();

    let form: NewInvitationForm = req.body_json().await?;
    if form.permission > session.permission {
        return Err(AppErrors::ValidationError(json!({
            "code": 4,
            "message": {
                "cn": "权限不足",
                "en": "Permission denied"
            }
        })).into());
    }
    form.validate(&db).await?;
    let expire_at = chrono::DateTime::from_utc(
        chrono::NaiveDateTime::from_timestamp(form.expire_at, 0),
        chrono::Utc,
    );
    let mut invitation = Invitation {
        id: None,
        code: random_string(10),
        expire_at: expire_at.into(),
        groups: form.groups,
        permission: form.permission,
    };
    invitation.save(&db, None).await?;
    Ok(json!({
        "code": invitation.code
    }).into())
}

async fn api_get_register_invitation(req: Request<AppState>) -> tide::Result<Response> {
    let state = req.state();
    let code = req.param("code").unwrap();
    if let Some(invitation) = Invitation::by_code(&state.db, code.to_string()).await {
        Ok(invitation.to_response().into())
    } else {
        Err(AppErrors::ValidationError(json!({
            "code": 4,
            "message": {
                "cn": "邀请码不存在",
                "en": "Invitation code does not exist"
            }
        })).into())
    }
}

async fn api_get_user(req: Request<AppState>) -> tide::Result<Response> {
    // require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let id = req.param("id").unwrap();
    if let Some(user) = User::by_id(&state.db, &id.to_string()).await {
        Ok(user.to_response().into())
    } else {
        Err(AppErrors::ValidationError(json!({
            "code": 4,
            "message": {
                "cn": "用户不存在",
                "en": "User does not exist"
            }
        })).into())
    }
}

async fn api_update_user(mut req: Request<AppState>) -> tide::Result<Response> {
    require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = state.db.clone();
    let session: &Session = req.ext().unwrap();
    let session = session.to_owned();
    let id = req.param("id").unwrap().to_owned();
    let form: UpdateUserForm = req.body_json().await?;
    if session.permission != 3 && session.user.unwrap() != id {
        // 不是管理员，不能修改其他用户
        return Err(AppErrors::ValidationError(json!({
            "code": 4,
            "message": {
                "cn": "权限不足",
                "en": "Permission denied"
            }
        })).into());
    }
    form.validate(&db, &id).await?;
    if let Some(mut user) = User::by_id(&db, &id).await {
        if let Some(name) = form.name {
            user.name = name;
        }
        if let Some(avatar) = form.avatar {
            user.avatar = Some(avatar);
        }
        // 只有管理员可以修改用户权限
        if let Some(permission) = form.permission {
            if session.permission == 3 {
                user.permission = permission as f64;
            } else {
                return Err(AppErrors::ValidationError(json!({
                    "code": 4,
                    "message": {
                        "cn": "权限不足",
                        "en": "Permission denied"
                    }
                })).into());
            }
        }

        // 保存更改
        user.save(&db, None).await?;
        Ok(user.to_response().into())
    } else {
        Err(AppErrors::ValidationError(json!({
            "code": 4,
            "message": {
                "cn": "用户不存在",
                "en": "User does not exist"
            }
        })).into())
    }
}

// 随机验证码
fn random_code(length: usize) -> String {
    let mut rng = rand::thread_rng();
    let mut code = String::new();
    for _ in 0..length {
        code += (rng.gen_range(0..10) as u8).to_string().as_str();
    }
    code
}

async fn api_create_inactive_user(mut req: Request<AppState>) -> tide::Result<Response> {
    let state = req.state().to_owned();
    let db = state.db.clone();
    // let session: &Session = req.ext().unwrap();
    let form: CreateInactiveUserForm = req.body_json().await?;
    form.validate(&db).await?;
    // 尝试获取邀请内容
    if let Some(invitation) = Invitation::by_code(&db, form.invitation).await {
        // 生成验证码
        let code = random_code(4);
        let mut user = InactiveUser {
            id: None,
            groups: invitation.groups,
            email: form.email.to_owned(),
            password: form.password,
            permission: invitation.permission,
            name: form.name.to_owned(),
            code: code.to_owned(),
            // 验证码过期时间
            expire_at: (chrono::Utc::now() + chrono::Duration::hours(1)).into(),
        };
        // 保存到数据库
        user.save(&db, None).await?;
        // 发送邮件
        let mail = state.config.email.code_letter(&state.config.server,
                                                  code, // 把code消费掉
                                                  form.name,
                                                  form.lang,
                                                  form.email);
        // 发送邮件
        if let Ok(..) = state.config.email.send(mail) {
            Ok(Response::new(StatusCode::NoContent))
        } else {
            // 发送失败
            // 删除用户
            user.delete(&db).await?;
            Ok(json_response(500, json!({
                "code": 1001,
                "message": {
                    "cn": "邮件发送失败",
                    "en": "Failed to send email"
                }
            })))
        }
    } else {
        Err(AppErrors::ValidationError(json!({
            "code": 4,
            "message": {
                "cn": "邀请码不存在",
                "en": "Invitation code does not exist"
            }
        })).into())
    }
}

async fn api_get_users(req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state().to_owned();
    let db = state.db.clone();
    let users: Vec<_> = User::find(&db, None, None)
        .await?
        .collect()
        .await;
    let mut result = Vec::new();
    for user in users {
        if let Ok(user) = user {
            result.push(user.to_response());
        }
    }
    Ok(json!(result).into())
}