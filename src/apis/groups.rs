//
use serde_json::json;
use tide::{Request, Server};
use wither::bson::doc;
use crate::apis::{json_response, require_perm};
use crate::AppState;
use crate::forms::groups::{CreateGroupForm, JoinInvitationForm, UpdateGroupForm};
use crate::models::groups::Group;
use crate::models::{SearchById, Session};
use crate::models::users::User;
use wither::Model;
use futures::StreamExt;
use crate::apis::users::random_string;
use crate::models::invitations::Invitation;

pub fn register(app: &mut Server<AppState>) {
    // 未来实现
    // app.at("/groups/invitations/:invitation_code").get(api_check_invitation);
    // app.at("/groups/invitations").post(api_create_invitation);
    // app.at("/groups/invitations/:invitation_code/apply").get(api_apply_invitation);
    app.at("/groups/manageable").get(api_get_manageable_groups);
    app.at("/groups").post(api_create_group)
        .get(api_get_groups);
    app.at("/groups/:group_id").get(api_get_group)
        .patch(api_update_group);
    app.at("/groups/:group_id/members").get(api_get_group_members);
    app.at("/groups/:group_id/members/:user_id").delete(api_delete_group_member);
    app.at("/groups/invitations").post(api_create_invitation);
    app.at("/groups/invitations/:code").get(api_check_invitation);
    app.at("/groups/invitations/:code/apply").get(api_apply_invitation);
}

// async fn api_check_invitation(req: Request<AppState>) -> tide::Result {
//     // 检查邀请码是否有效
//     let state = req.state();
//     let db = state.db.to_owned();
//     let invitation_code = req.param("invitation_code").unwrap().to_owned();
//     if let Some(invitation) = Invitation::by_code(&db, invitation_code).await {
//     }
// }

async fn api_get_manageable_groups(req: Request<AppState>) -> tide::Result {
    // 获取可管理的群组
    require_perm(&req, vec![2, 3]).await?;
    let state = req.state();
    let db = state.db.to_owned();
    let session: &Session = req.ext().unwrap();
    // 获取group.managers含有session.user的group
    let groups: Vec<_> = Group::find(&db, Some(doc! {
            "managers":{
                "$elemMatch": {"$eq": session.user.as_ref().unwrap()}
            }
        }
    ), None)
        .await?
        .collect()
        .await;
    let mut result = Vec::new();
    for group in groups {
        if let Ok(group) = group {
            result.push(group.to_response());
        }
    }
    Ok(json!(result).into())
}

async fn api_create_group(mut req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![2, 3]).await?;
    let state = req.state();
    let db = state.db.to_owned();
    let session: &Session = req.ext().unwrap();
    let session = session.to_owned();
    let form: CreateGroupForm = req.body_json().await?;
    form.validate(&db).await?;
    let mut group = Group {
        id: None,
        name: form.name,
        created_at: chrono::Utc::now().into(),
        managers: vec![session.user.to_owned().unwrap()],
        cover: None,
    };
    group.save(&db, None).await?;
    let mut user = User::by_id(&db, &session.user.unwrap()).await.unwrap();
    // 将用户加入群组
    if let Some(ref mut groups) = user.groups {
        groups.push(group.id.as_ref().unwrap().to_hex());
    } else {
        user.groups = Some(vec![group.id.as_ref().unwrap().to_hex()]);
    }
    user.save(&db, None).await?;
    Ok(json!(group).into())
}

async fn api_get_group(req: Request<AppState>) -> tide::Result {
    // 获取群组信息
    // 允许所有人访问
    let state = req.state();
    let db = state.db.to_owned();
    let group_id = req.param("group_id").unwrap().to_owned();
    if let Some(group) = Group::by_id(&db, &group_id).await {
        Ok(group.to_response().into())
    } else {
        Ok(json_response(404, json!({
            "code": 4,
            "message": {
                "cn": "调查小组不存在",
                "en": "Group not found"
            }
        })))
    }
}

async fn api_get_group_members(req: Request<AppState>) -> tide::Result {
    // 获取所有groups包含group_id的user
    require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = state.db.to_owned();
    let group_id = req.param("group_id").unwrap().to_owned();
    let users = User::by_group(&db, &group_id).await;
    let mut result = Vec::new();
    for user in users {
        result.push(user.to_response());
    }
    Ok(json!(result).into())
}

async fn api_get_groups(req: Request<AppState>) -> tide::Result {
    // 获取所有groups
    // require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = state.db.to_owned();
    let groups: Vec<_> = Group::find(&db, None, None)
        .await
        .unwrap()
        .collect()
        .await;
    let mut result = Vec::new();
    for group in groups {
        if let Ok(group) = group {
            result.push(group.to_response());
        }
    }
    Ok(json!(result).into())
}

async fn api_update_group(mut req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![2, 3]).await?;
    let state = req.state();
    let db = state.db.to_owned();
    let group_id = req.param("group_id").unwrap().to_owned();
    let form: UpdateGroupForm = req.body_json().await?;
    form.validate(&db).await?;
    let group = Group::by_id(&db, &group_id).await;
    if let Some(mut group) = group {
        if let Some(name) = form.name {
            group.name = name;
        }
        if let Some(cover) = form.cover {
            group.cover = Some(cover);
        }
        if let Some(managers) = form.managers {
            group.managers = managers;
        }
        group.save(&db, None).await?;
        Ok(group.to_response().into())
    } else {
        Ok(json_response(404, json!({
            "code": 4,
            "message": {
                "cn": "调查小组不存在",
                "en": "Group not found"
            }
        })))
    }
}

pub async fn api_delete_group_member(req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![2, 3]).await?;
    let state = req.state();
    let db = state.db.to_owned();
    let group_id = req.param("group_id").unwrap().to_owned();
    let user_id = req.param("user_id").unwrap().to_owned();
    let group = Group::by_id(&db, &group_id).await;
    let session: &Session = req.ext().unwrap();
    if let Some(group) = group {
        // 如果目标是小组管理员 则不能删除
        if group.managers.contains(&user_id) {
            return Ok(json_response(400, json!({
                "code": 1,
                "message": {
                    "cn": "不能删除小组管理员",
                    "en": "Can't delete group manager"
                }
            })));
        }
        // 非小组管理员 只能删除自己
        if session.permission == 1 && &user_id != session.user.as_ref().unwrap() {
            return Ok(json_response(403, json!({
                "code": 4,
                "message": {
                    "cn": "非小组管理员不能删除其他成员",
                    "en": "Can't delete other user"
                }
            })));
        }
        // 执行删除 将小组从user的groups中删除
        let user = User::by_id(&db, &user_id).await;
        if let Some(mut user) = user {
            if let Some(mut groups) = user.groups.to_owned() {
                groups.retain(|g| g != &group_id);
                user.groups = Some(groups);
                user.save(&db, None).await?;
            } else {
                return Ok(json_response(400, json!({
                    "code": 4,
                    "message": {
                        "cn": "用户不属于任何小组",
                        "en": "User not in any group"
                    }
                })));
            }
        } else {
            return Ok(json_response(404, json!({
                "code": 4,
                "message": {
                    "cn": "用户不存在",
                    "en": "User not found"
                }
            })));
        }
        Ok(group.to_response().into())
    } else {
        Ok(json_response(404, json!({
            "code": 4,
            "message": {
                "cn": "调查小组不存在",
                "en": "Group not found"
            }
        })))
    }
}


async fn api_create_invitation(mut req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![2, 3]).await?;
    let state = req.state();
    let db = state.db.to_owned();
    let form: JoinInvitationForm = req.body_json().await?;
    form.validate(&db).await?;
    let code = random_string(10);
    let expire_at = chrono::DateTime::from_utc(
        chrono::NaiveDateTime::from_timestamp(form.expire_at, 0),
        chrono::Utc,
    );
    let mut invitation = Invitation {
        id: None,
        code: code.to_owned(),
        expire_at: expire_at.into(),
        groups: Some(form.groups),
        permission: form.permission,
    };
    invitation.save(&db, None).await?;
    return Ok(json!({
        "code": code
    }).into());
}

async fn api_check_invitation(req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = state.db.to_owned();
    let code = req.param("code").unwrap().to_owned();
    let invitation = Invitation::by_code(&db, code.clone()).await;
    if let Some(invitation) = invitation {
        Ok(invitation.to_response().into())
    } else {
        Ok(json_response(404, json!({
            "code": 1001,
            "message": {
                "cn": "邀请不存在",
                "en": "Invitation not found"
            }
        })))
    }
}

async fn api_apply_invitation(req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = state.db.to_owned();
    let code = req.param("code").unwrap().to_owned();
    let invitation = Invitation::by_code(&db, code.clone()).await;
    return if let Some(invitation) = invitation {
        let session: &Session = req.ext().unwrap();
        let user = User::by_id(&db, session.user.as_ref().unwrap()).await;
        if let Some(mut user) = user {
            if user.groups.is_some() {
                return Ok(json_response(400, json!({
                    "code": 1003,
                    "message": {
                        "cn": "用户已经加入小组",
                        "en": "User already in group"
                    }
                })));
            }
            let mut groups = user.groups.to_owned().unwrap_or(vec![]);
            groups.extend(invitation.groups.clone().unwrap());
            user.groups = Some(groups);
            user.save(&db, None).await?;
            if invitation.permission == 2 {
                user.permission = 2.0;
                // 将用户加入小组的管理员列表中
                for group_id in invitation.groups.as_ref().unwrap_or(&vec![]) {
                    let mut group = Group::by_id(&db, group_id).await.unwrap();
                    group.managers.push(user.id.to_owned().unwrap().to_hex());
                    group.save(&db, None).await?;
                }
                user.save(&db, None).await?;
            }
            invitation.delete(&db).await?;
            Ok(tide::Response::new(204))
        } else {
            Ok(json_response(404, json!({
                "code": 1004,
                "message": {
                    "cn": "用户不存在",
                    "en": "User not found"
                }
            })))
        }
    } else {
        Ok(json_response(404, json!({
            "code": 1001,
            "message": {
                "cn": "邀请不存在",
                "en": "Invitation not found"
            }
        })))
    };
}