use log::info;
use serde_json::json;
use tide::{Request, Server};
use wither::bson::doc;
use crate::apis::{json_response, require_perm};
use crate::AppState;
use crate::forms::records::{NewRecordForm, RecordsQuery, UpdateDraftForm, UpdateRecordForm};
use crate::models::records::Record;
use crate::models::{SearchById, Session};
use crate::models::drafts::RecordDraft;
use crate::models::groups::Group;
use crate::models::projects::Project;
use wither::Model;
use futures::StreamExt;
use crate::models::positions::Position;

pub fn register(app: &mut Server<AppState>) {
    info!("注册API records");
    app.at("/records/count")
        .get(api_get_records_count);
    app.at("/records/:id")
        .get(api_get_record_by_id)
        .patch(api_update_record_by_id)
        .delete(api_delete_record_by_id);
    app.at("/records/user/:id")
        .get(api_get_records_by_user_id);
    app.at("/records")
        .post(api_create_record)
        .get(api_get_records);
    app.at("/drafts/record")
        .get(api_get_record_draft)
        .patch(api_update_record_draft)
        .delete(api_delete_record_draft);
}

async fn api_get_records_count(mut req: tide::Request<AppState>) -> tide::Result {
    require_perm(&mut req, vec![1, 2, 3]).await?;
    // 获取全站记录数
    let db = req.state().db.to_owned();
    if let Ok(count) = db.collection("records")
        .count_documents(None, None)
        .await {
        Ok(json!(count).into())
    } else {
        Ok(json!(0).into())
    }
}

async fn api_get_record_by_id(req: tide::Request<AppState>) -> tide::Result {
    require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = state.db.to_owned();
    let id = req.param("id")?.to_owned();
    if let Some(record) = Record::by_id(&db, &id).await {
        Ok(record.to_response().into())
    } else {
        Ok(json_response(404, json!({
            "code": 4,
            "message": {
                "cn": "记录不存在",
                "en": "Record not found"
            }
        })))
    }
}


async fn api_get_records(req: tide::Request<AppState>) -> tide::Result {
    require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = state.db.to_owned();
    let query: RecordsQuery = req.query()?;
    let mut result = Vec::new();
    let records: Vec<_> = Record::find(&db, Some(query.to_filter()), None)
        .await
        .unwrap()
        .collect()
        .await;
    for record in records {
        if let Ok(record) = record {
            result.push(record.to_response());
        }
    }
    Ok(json!(result).into())
}


async fn api_create_record(mut req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = state.db.to_owned();
    let form: NewRecordForm = req.body_json().await?;
    let session: &Session = req.ext().unwrap();
    form.validate(&db).await?;
    let project = Project::get_running_project(&db).await;
    if project.is_none() {
        return Ok(json_response(400, json!({
            "code": 1,
            "message": {
                "cn": "当前没有运行中的项目",
                "en": "No running project"
            }
        })));
    }
    let position = Position::by_id(&db, &form.position).await.unwrap();
    let project = project.unwrap();
    let mut record = Record {
        id: None,
        num: form.num as i16,
        position: form.position,
        time: chrono::DateTime::from_utc(
            chrono::NaiveDateTime::from_timestamp(form.time, 0),
            chrono::Utc,
        ).into(),
        collaborators: form.collaborators.unwrap_or(vec![]),
        description: form.description.unwrap_or("".to_string()),
        weather: form.weather,
        group: position.belongs_to,
        user: session.user.to_owned().unwrap(),
        project: project.id.unwrap().to_hex(),
        attachments: form.attachments.unwrap_or(vec![]),
        num_of_nests: None,
        return_time: None,
        return_direction: form.return_direction,
        nest_height: form.nest_height,
        nest_area: form.nest_area,
        nest_material: form.nest_material
    };
    record.save(&db, None).await?;
    Ok(record.to_response().into())
}

async fn api_update_record_by_id(mut req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = state.db.to_owned();
    let id = req.param("id")?.to_owned();
    let form: UpdateRecordForm = req.body_json().await?;
    let session: &Session = req.ext().unwrap();
    form.validate(&db).await?;
    let record = Record::by_id(&db, &id).await;
    if record.is_none() {
        return Ok(json_response(404, json!({
            "code": 4,
            "message": {
                "cn": "记录不存在",
                "en": "Record not found"
            }
        })));
    }
    let mut record = record.unwrap();
    // 如果非管理员 并且是自己的记录，则允许修改
    if session.user.as_ref().unwrap() != &record.user {
        if session.permission == 2 {
            // 检查是否为小组长
            let group = Group::by_id(&db, &record.group).await.unwrap();
            if !group.managers.contains(session.user.as_ref().unwrap()) {
                return Ok(json_response(403, json!({
                    "code": 1,
                    "message": {
                        "cn": "您没有权限修改该记录",
                        "en": "You have no permission to update this record"
                    }
                })));
            }
        } else if session.permission == 1 {
            return Ok(json_response(403, json!({
                "code": 1,
                "message": {
                    "cn": "您没有权限修改该记录",
                    "en": "You have no permission to update this record"
                }
            })));
        }
    }
    // 执行修改
    if let Some(num) = form.num {
        record.num = num as i16;
    }

    if let Some(position) = form.position {
        let position_ = Position::by_id(&db, &position).await.unwrap();
        record.position = position;
        // 这俩要改一起改
        record.group = position_.belongs_to;
    }
    if let Some(time) = form.time {
        record.time = chrono::DateTime::from_utc(
            chrono::NaiveDateTime::from_timestamp(time, 0),
            chrono::Utc,
        ).into();
    }
    if let Some(collaborators) = form.collaborators {
        record.collaborators = collaborators;
    }
    if let Some(description) = form.description {
        record.description = description;
    }
    if let Some(attachments) = form.attachments {
        record.attachments = attachments;
    }
    if let Some(weather) = form.weather {
        record.weather = weather;
    }
    if let Some(return_direction) = form.return_direction {
        record.return_direction = Some(return_direction);
    }
    if let Some(nest_height) = form.nest_height {
        record.nest_height = Some(nest_height);
    }
    if let Some(nest_area) = form.nest_area {
        record.nest_area = Some(nest_area);
    }
    if let Some(nest_material) = form.nest_material {
        record.nest_material = Some(nest_material);
    }
    if let Some(return_time) = form.return_time {
        record.return_time = Some(return_time);
    }


    record.save(&db, None).await?;

    Ok(record.to_response().into())
}

async fn api_delete_record_by_id(req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = state.db.to_owned();
    let id = req.param("id")?.to_owned();
    let session: &Session = req.ext().unwrap();
    let record = Record::by_id(&db, &id).await;
    if record.is_none() {
        return Ok(json_response(404, json!({
            "code": 4,
            "message": {
                "cn": "记录不存在",
                "en": "Record not found"
            }
        })));
    }
    let record = record.unwrap();
    // 如果非管理员 并且是自己的记录，则允许修改
    if session.user.as_ref().unwrap() != &record.user {
        if session.permission == 2 {
            // 检查是否为小组长
            let group = Group::by_id(&db, &record.group).await.unwrap();
            if !group.managers.contains(session.user.as_ref().unwrap()) {
                return Ok(json_response(403, json!({
                    "code": 1,
                    "message": {
                        "cn": "您没有权限删除该记录",
                        "en": "You have no permission to delete this record"
                    }
                })));
            }
        } else if session.permission == 1 {
            return Ok(json_response(403, json!({
                "code": 1,
                "message": {
                    "cn": "您没有权限删除该记录",
                    "en": "You have no permission to delete this record"
                }
            })));
        }
    }
    // 执行删除
    record.delete(&db).await?;

    Ok(json!({}).into())
}

async fn api_get_records_by_user_id(req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = state.db.to_owned();
    let user_id = req.param("id")?.to_owned();
    let mut result = Vec::new();
    let records: Vec<_> = Record::find(&db, Some(doc! {
        "user": user_id
    }), None)
        .await?
        .collect()
        .await;
    for record in records {
        if let Ok(record) = record {
            result.push(record.to_response());
        }
    }
    Ok(json!(result).into())
}

async fn api_get_record_draft(req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = state.db.to_owned();
    let session: &Session = req.ext().unwrap();
    if let Some(draft) = RecordDraft::by_user(&db, session.user.as_ref().unwrap()).await {
        Ok(draft.to_response().into())
    } else {
        Ok(json!({}).into())
    }
}

async fn api_update_record_draft(mut req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = state.db.to_owned();
    let session: &Session = req.ext().unwrap();
    let session = session.to_owned();
    let form: UpdateDraftForm = req.body_json().await?;
    let mut draft = RecordDraft::by_user(&db, session.user.as_ref().unwrap()).await
        .unwrap_or(RecordDraft {
            id: None,
            position: None,
            collaborators: None,
            num: None,
            time: None,
            description: None,
            user: session.user.unwrap(),
            num_of_nests: None,
            return_time: None,
            return_direction: None,
            nest_height: None,
            nest_area: None,
            nest_material: None,
            weather: None
        });
    draft.position = form.position;
    draft.collaborators = form.collaborators;
    draft.num = form.num;
    draft.time = form.time;
    draft.description = form.description;
    draft.num_of_nests = form.num_of_nests;
    draft.return_time = form.return_time;
    draft.return_direction = form.return_direction;
    draft.nest_height = form.nest_height;
    draft.nest_area = form.nest_area;
    draft.nest_material = form.nest_material;
    draft.weather = form.weather;
    draft.save(&db, None).await?;
    Ok(json!({}).into())
}

async fn api_delete_record_draft(req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = state.db.to_owned();
    let session: &Session = req.ext().unwrap();
    let draft = RecordDraft::by_user(&db, session.user.as_ref().unwrap()).await;
    if draft.is_some() {
        draft.unwrap().delete(&db).await?;
    }
    Ok(json!({}).into())
}

