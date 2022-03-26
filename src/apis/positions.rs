// =======================
// Finished at: 2022-03-11
// Purified at: 2022-03-11
// By lihe07
// =======================
use log::info;
use serde_json::json;
use tide::{Request, Server};
use crate::apis::{json_response, require_perm};
use crate::AppState;
use crate::forms::positions::{NewPositionForm, UpdatePositionForm};
use crate::models::positions::Position;
use crate::models::{SearchById, Session};
use crate::models::groups::Group;
use crate::models::users::User;
use wither::Model;

pub fn register(app: &mut Server<AppState>) {
    info!("注册API positions");
    app.at("/positions/available").get(api_get_available_positions);
    app.at("/positions/:id")
        .get(api_get_position)
        .put(api_put_position);
    app.at("/positions").post(api_new_position);
    app.at("/positions/by_group/:id").put(api_replace_by_group)
        .get(api_get_by_group);
}

// rust版本特有的API 获取一个用户可用的调查点
async fn api_get_available_positions(mut req: Request<AppState>) -> tide::Result {
    require_perm(&mut req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = state.db.clone();
    let session: &Session = req.ext().unwrap();
    let session = session.to_owned();
    // 先获取用户 这里不会出现获取不到的情况
    let user = User::by_id(&db, &session.user.unwrap()).await.unwrap();
    let mut positions = Vec::new();
    for group in user.groups.unwrap_or(vec![]).iter() {
        let group_positions = Position::by_group(&db, &group).await.unwrap();
        positions.extend(group_positions);
    }
    let mut result = Vec::new();
    for position in positions {
        result.push(position.to_response())
    }
    Ok(json!(result).into())
}

async fn api_get_position(mut req: Request<AppState>) -> tide::Result {
    require_perm(&mut req, vec![1, 2, 3]).await?;
    let id = req.param("id")?;
    let state = req.state();
    let db = state.db.to_owned();
    if let Some(position) = Position::by_id(&db, &id.to_string()).await {
        Ok(position.to_response().into())
    } else {
        Ok(json_response(404, json!({
            "code": 4,
            "message": {
                "cn": "调查点不存在",
                "en": "Position not found"
            }
        })))
    }
}


async fn api_put_position(mut req: Request<AppState>) -> tide::Result {
    require_perm(&mut req, vec![2, 3]).await?;
    let form: UpdatePositionForm = req.body_json().await?;
    let id = req.param("id")?;
    let state = req.state();
    let db = state.db.to_owned();

    if let Some(mut position) = Position::by_id(&db, &id.to_string()).await {
        position.name = form.name;
        position.longitude = form.longitude;
        position.latitude = form.latitude;
        position.save(&db, None).await?;
        Ok(position.to_response().into())
    } else {
        Ok(json_response(404, json!({
            "code": 4,
            "message": {
                "cn": "调查点不存在",
                "en": "Position not found"
            }
        })))
    }
}


async fn api_new_position(mut req: Request<AppState>) -> tide::Result {
    require_perm(&mut req, vec![2, 3]).await?;
    let state = req.state();
    let db = state.db.to_owned();
    let form: NewPositionForm = req.body_json().await?;
    form.validate(&db).await?;
    let mut position = Position {
        id: None,
        name: form.name,
        belongs_to: form.group_id,
        longitude: form.longitude,
        latitude: form.latitude,
    };
    position.save(&db, None).await?;
    Ok(position.to_response().into())
}

async fn api_replace_by_group(mut req: Request<AppState>) -> tide::Result {
    require_perm(&mut req, vec![2, 3]).await?;
    let group_id = req.param("id").unwrap().to_owned();
    let state = req.state();
    let db = state.db.to_owned();

    if let None = Group::by_id(&db, &group_id.to_string()).await {
        return Ok(json_response(404, json!({
            "code": 4,
            "message": {
                "cn": "调查小组不存在",
                "en": "Group not found"
            }
        })));
    }
    let positions_form: Vec<UpdatePositionForm> = req.body_json().await?;
    // 先删除所有的调查点
    // for position in Position::by_group(&db, &group_id).await.unwrap() {
    //     position.delete(&db).await?;
    // }

    let mut previous_positions = Position::by_group(&db, &group_id).await.unwrap();
    // 添加新的调查点 如果longitude和latitude已经存在 则修改旧点name
    let mut positions = Vec::new();
    for position in positions_form {
        let mut need_add = true;
        for index in 0..previous_positions.len() {
            // 取出一个旧的调查点
            let mut previous_position = previous_positions.remove(index);

            if previous_position.longitude == position.longitude && previous_position.latitude == position.latitude {
                // 修改name
                previous_position.name = position.name.to_owned();
                previous_position.save(&db, None).await?;
                positions.push(previous_position.to_response());
                need_add = false;
                break;
            } else {
                // 放回去
                previous_positions.insert(index, previous_position);
            }
        }
        if need_add {
            let mut position = Position {
                id: None,
                name: position.name,
                belongs_to: group_id.to_string(),
                longitude: position.longitude,
                latitude: position.latitude,
            };
            position.save(&db, None).await?;
            positions.push(position.to_response());
        }
    }
    // 删除多余的调查点
    for position in previous_positions {
        position.delete(&db).await?;
    }
    Ok(json!(positions).into())
}

async fn api_get_by_group(mut req: Request<AppState>) -> tide::Result {
    require_perm(&mut req, vec![1, 2, 3]).await?;
    let group_id = req.param("id").unwrap().to_owned();
    let state = req.state();
    let db = state.db.to_owned();

    if let None = Group::by_id(&db, &group_id.to_string()).await {
        return Ok(json_response(404, json!({
            "code": 4,
            "message": {
                "cn": "调查小组不存在",
                "en": "Group not found"
            }
        })));
    }
    let positions = Position::by_group(&db, &group_id).await.unwrap();
    let mut result = Vec::new();
    for position in positions {
        result.push(position.to_response());
    }
    Ok(json!(result).into())
}

