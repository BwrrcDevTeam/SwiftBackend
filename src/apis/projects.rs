use tide::{Request, Server};
use crate::apis::{json_response, require_perm};
use crate::AppState;
use crate::models::projects::Project;
use futures::StreamExt;
use log::info;
use serde_json::json;
use wither::bson::doc;
use wither::mongodb::Database;
use crate::forms::projects::{NewProjectForm, UpdateProjectForm};
use crate::models::SearchById;
use wither::Model;


pub fn register(app: &mut Server<AppState>) {
    info!("注册API projects");
    app.at("/projects").get(api_get_projects)
        .post(api_create_project);
    app.at("/projects/running").get(api_get_running_project);
    app.at("/projects/:id").get(api_get_project)
        .delete(api_delete_project)
        .patch(api_update_project);
    // app.at("/projects/:id/set_running").post(api_set_running_project);
}

async fn api_get_projects(req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = state.db.clone();
    let projects: Vec<_> = Project::find(&db, None, None)
        .await
        .unwrap()
        .collect()
        .await;
    let mut result = Vec::new();
    for project in projects {
        if let Ok(project) = project {
            result.push(project);
        }
    }
    Ok(json!(result).into())
}

async fn api_get_running_project(req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = state.db.clone();
    if let Ok(project) = Project::find_one(&db, Some(doc! {
        "running": true
    }), None).await {
        Ok(json!(project).into())
    } else {
        Ok(json_response(200, json!({})))
    }
}


// 一次只允许一个项目处于运行状态
async fn set_running(db: &Database, project: &mut Project) -> tide::Result<()> {
    // 先将所有项目的running设置为false
    let collection = db.collection("projects");
    let _ = collection.update_many(doc! {}, doc! {
        "$set": {
            "running": false
        }
    }, None).await?;
    // 再将当前项目的running设置为true
    project.running = true;
    project.save(&db, None).await?;
    Ok(())
}

async fn api_create_project(mut req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![3]).await?;
    let form: NewProjectForm = req.body_json().await?;
    let state = req.state();
    let db = state.db.clone();
    form.validate(&db).await?;
    let mut project = Project {
        id: None,
        title: form.title.clone(),
        start_time: chrono::DateTime::from_utc(
            chrono::NaiveDateTime::from_timestamp(form.start_time as i64, 0),
            chrono::Utc,
        ).into(),
        duration: form.duration,
        running: form.running.clone(),
    };
    if form.running {
        set_running(&db, &mut project).await?;
    }
    project.save(&db, None).await?;
    Ok(json!(project).into())
}


// async fn api_get_running_projects(req: Request<AppState>) -> tide::Result {
//     require_perm(&req, vec![1, 2, 3]).await?;
//     let state = req.state();
//     let db = state.db.clone();
//     let projects: Vec<_> = Project::find(&db, Some(doc! {
//         "running": true
//     }), None)
//         .await
//         .unwrap()
//         .collect()
//         .await;
//     let mut result = Vec::new();
//     for project in projects {
//         if let Ok(project) = project {
//             result.push(project);
//         }
//     }
//     Ok(json!(result).into())
// }

async fn api_get_project(req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![1, 2, 3]).await?;
    let id = req.param("id").unwrap();
    let state = req.state();
    let db = state.db.clone();
    if let Some(project) = Project::by_id(&db, &id.to_string()).await {
        Ok(json!(project).into())
    } else {
        Ok(json_response(404, json!({
            "code": 4,
            "message": {
                "cn": "项目不存在",
                "en": "Project not found"
            }
        })))
    }
}

async fn api_delete_project(req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![3]).await?;
    let id = req.param("id").unwrap();
    let state = req.state();
    let db = state.db.clone();
    if let Some(project) = Project::by_id(&db, &id.to_string()).await {
        project.delete(&db).await?;
        Ok(json_response(200, json!({})))
    } else {
        Ok(json_response(404, json!({
            "code": 4,
            "message": {
                "cn": "项目不存在",
                "en": "Project not found"
            }
        })))
    }
}


async fn api_update_project(mut req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![3]).await?;
    let id = req.param("id").unwrap().to_owned();
    let state = req.state();
    let db = state.db.clone();
    let form: UpdateProjectForm = req.body_json().await?;
    form.validate(&db).await?;
    let project = Project::by_id(&db, &id).await;
    if project.is_none() {
        return Ok(json_response(404, json!({
            "code": 4,
            "message": {
                "cn": "项目不存在",
                "en": "Project not found"
            }
        })));
    }
    let mut project = project.unwrap();
    if let Some(running) = form.running {
        if running {
            set_running(&db, &mut project).await?;
        } else {
            project.running = false;
        }
    }
    if let Some(title) = form.title {
        project.title = title;
    }

    if let Some(start_time) = form.start_time {
        project.start_time = chrono::DateTime::from_utc(
            chrono::NaiveDateTime::from_timestamp(start_time as i64, 0),
            chrono::Utc,
        ).into();
    }
    if let Some(duration) = form.duration {
        project.duration = duration;
    }
    project.save(&db, None).await?;
    Ok(json!(project).into())
}