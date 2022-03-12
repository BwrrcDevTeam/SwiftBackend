use serde_json::json;
use tide::{Request, Server};
use wither::bson::doc;
use crate::apis::require_perm;
use crate::AppState;
use crate::models::groups::Group;
use crate::models::invitations::Invitation;
use crate::models::Session;

pub fn register(&mut app: Server<AppState>) {
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
        {"managers": {"$elemMatch": {"$eq": session.user.unwrap()}}}
    }), None)
        .await?
        .collect()
        .await;
    let mut result = Vec::new();
    for group in groups {
        if let Ok(group) = group {
            result.push(group.to_owned());
        }
    }
    Ok(json!(result).into())
}

async fn api_create_group(req: Request<AppState>) -> tide::Result {

}