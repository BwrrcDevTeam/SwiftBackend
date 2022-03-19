// =======================
// Finished at: 2022-03-11
// Purified at: 2022-03-11
// By lihe07
// =======================

use log::{info, warn};
use tide::{Request, Response, Server};
use tide::prelude::*;
use wither::bson::doc;
use swift_det_lib::{BBox, detect, DetectConfig, make_env};
use crate::apis::{json_response, require_perm};
use crate::AppState;
use crate::models::detections::Detection;
use crate::models::Session;
use wither::Model;
use crate::models::SearchById;
use futures::StreamExt;
use wither::bson::oid::ObjectId;
use wither::mongodb::Database;

pub fn register(app: &mut Server<AppState>) {
    info!("注册检测器API");
    app.at("/detector").post(api_create_task);
    app.at("/detector/:task_id/status").get(api_get_task_status);
    app.at("/detector/:task_id").get(api_get_task_info)
        .put(api_update_task)
        .delete(api_delete_task);
    app.at("/detector/:task_id/draw").get(api_draw);
    app.at("/detector/:task_id/count").get(api_compute_number);
    app.at("/detector/mine").get(api_get_user_detections);
}

#[derive(Deserialize)]
struct CreateTaskForm {
    attachment: String,
    model_name: String,
    overlap: u8,
    window_size: usize,
    tile_max_num: u16,
}

async fn do_task(db: Database, task_id: ObjectId, task_config: DetectConfig) {
    let env = make_env();
    let env = env.unwrap();
    info!("开始检测任务");
    let mut task = Detection::by_id(&db, &task_id.to_hex()).await.unwrap();
    let attachment = task.get_attachment(&db).await.unwrap();
    let result = detect(attachment.local_path.as_str(), task_config, env, |current, total| {
        let task = task.to_owned();
        if let Ok(..) = async_std::task::block_on(
            task.update(&db, None, doc! {
                    "$set": {
                        "status": "processing",
                        "current": current.to_owned() as i32,
                        "total": total.to_owned() as i32,
                    }
                }, None)
        ) {
            info!("任务 {} 更新成功 进度 {}/{}", &task_id, current, total);
        } else {
            warn!("任务 {} 更新失败 进度 {}/{}", &task_id, current, total);
        }
    }, false);
    if result.is_err() {
        if let Ok(..) = task.update(&db, None, doc! {
                "$set": {
                    "status": "failed",
                },
                "$unset": {
                    "result": "",
                    "current": "",
                    "total": "",
                }
            }, None).await {
            warn!("任务 {} 失败", &task_id);
        } else {
            warn!("任务 {} 失败 + 更新失败", &task_id);
        }
        return;
    }
    info!("任务 {} 执行完毕", &task_id);
    task.status = "finished".to_owned();
    task.result = Some(result.unwrap());
    task.save(&db, None).await.unwrap();
}

async fn api_create_task(mut req: Request<AppState>) -> tide::Result<tide::Response> {
    let form: CreateTaskForm = req.body_json().await?;

    let session: &Session = req.ext().unwrap();
    let state = req.state();


    let model_path = state.config.ai.get_model_path(&form.model_name);
    // 不存在这个model_name
    if model_path.is_none() {
        let mut resp = tide::Response::new(tide::StatusCode::BadRequest);
        resp.set_body(json!({
                "code": 4,
                "message": {
                    "cn": "模型不存在",
                    "en": "Model not found",
                },
                "description": {
                    "model_name": form.model_name,
                },
            }));
        return Ok(resp);
    }
    let model_path = model_path.unwrap();

    let mut task = Detection {
        id: None,
        creator: if let Some(user) = session.user.as_ref() { user.to_owned() } else { "anonymous".to_owned() },
        created_at: chrono::Utc::now().into(),
        status: "pending".to_string(),
        attachment: form.attachment,
        window_size: form.window_size as isize,
        overlap: form.overlap as i8,
        tile_max_num: form.tile_max_num as i16,
        model_name: form.model_name,
        result: None,
        current: None,
        total: None,
        threshold: None,
    };


    // 将任务插入数据库
    task.save(&state.db, None).await?;
    let task_id = task.id.as_ref().unwrap().to_owned();
    let task_config = task.get_config(model_path);
    let attachment = task.get_attachment(&state.db).await;
    if attachment.is_none() {
        let mut resp = tide::Response::new(tide::StatusCode::BadRequest);
        resp.set_body(json!({
                "code": 4,
                "message": {
                    "cn": "附件不存在",
                    "en": "Attachment not found",
                },
                "description": {
                    "task_id": task_id,
                },
            }));
        return Ok(resp);
    }

    // 启动任务
    // 这两个数据是要送给closure的
    let db = state.db.clone();
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024 * 1024)
        .spawn(move || {
            async_std::task::block_on(do_task(db, task_id, task_config));
        })
        .unwrap();
    Ok(json!({
        "task_id": task.id.unwrap().to_hex()
    }).into())
}

async fn api_get_task_status(req: Request<AppState>) -> tide::Result<tide::Response> {
    let task_id = req.param("task_id").unwrap();
    let state = req.state();
    if let Some(task) = Detection::by_id(&state.db, &task_id.to_string()).await {
        Ok(json!(task.to_status().await).into())
    } else {
        let mut resp = tide::Response::new(tide::StatusCode::BadRequest);
        resp.set_body(json!({
                "code": 4,
                "message": {
                    "cn": "任务不存在",
                    "en": "Task not found",
                },
                "description": {
                    "task_id": task_id,
                },
            }));
        Ok(resp)
    }
}

async fn api_get_task_info(req: Request<AppState>) -> tide::Result<tide::Response> {
    let task_id = req.param("task_id").unwrap();
    let state = req.state();
    if let Some(task) = Detection::by_id(&state.db, &task_id.to_string()).await {
        let mut resp = tide::Response::new(tide::StatusCode::Ok);
        resp.set_body(json!(task.to_info().await));
        Ok(resp)
    } else {
        let mut resp = tide::Response::new(tide::StatusCode::BadRequest);
        resp.set_body(json!({
                "code": 4,
                "message": {
                    "cn": "任务不存在",
                    "en": "Task not found",
                },
                "description": {
                    "task_id": task_id,
                },
            }));
        Ok(resp)
    }
}


fn draw_box(img: &mut image::RgbImage, bbox: &mut BBox, color: image::Rgb<u8>, thickness: i32) {
    for x in bbox.x_min..bbox.x_min + thickness {
        for y in bbox.y_min..bbox.y_max {
            if let Some(pixel) = img.get_pixel_mut_checked(x as u32, y as u32) {
                *pixel = color;
            }
        }
    }
    for x in bbox.x_max..bbox.x_max + thickness {
        for y in bbox.y_min..bbox.y_max {
            if let Some(pixel) = img.get_pixel_mut_checked(x as u32, y as u32) {
                *pixel = color;
            }
        }
    }
    for y in bbox.y_min..bbox.y_min + thickness {
        for x in bbox.x_min..bbox.x_max {
            if let Some(pixel) = img.get_pixel_mut_checked(x as u32, y as u32) {
                *pixel = color;
            }
        }
    }
    for y in bbox.y_max..bbox.y_max + thickness {
        for x in bbox.x_min..bbox.x_max {
            if let Some(pixel) = img.get_pixel_mut_checked(x as u32, y as u32) {
                *pixel = color;
            }
        }
    }
}

fn draw_boxes(image_path: String, boxes: Vec<BBox>, threshold: f32) -> Option<image::RgbImage> {
    if let Ok(img) = image::open(image_path) {
        let mut img = img.to_rgb8();
        for mut bbox in boxes {
            if bbox.score > threshold {
                draw_box(&mut img, &mut bbox, image::Rgb([0, 255, 0]), 1);
            }
        }
        Some(img)
    } else {
        None
    }
}

#[derive(Deserialize)]
struct DrawQuery {
    threshold: f32,
}

async fn api_draw(req: Request<AppState>) -> tide::Result<Response> {
    let task_id = req.param("task_id").unwrap();
    let state = req.state();
    let query = req.query::<DrawQuery>().unwrap_or(DrawQuery { threshold: 0.5 });
    let threshold = query.threshold;
    if let Some(task) = Detection::by_id(&state.db, &task_id.to_string()).await {
        if let Some(attachment) = task.get_attachment(&state.db).await {
            let mut resp = tide::Response::new(tide::StatusCode::Ok);
            let img = draw_boxes(attachment.local_path.clone(), task.result.unwrap(), threshold);
            if let Some(img) = img {
                let buffer = Vec::new();
                let mut buffer = std::io::Cursor::new(buffer);
                img.write_to(&mut buffer, image::ImageOutputFormat::Png).unwrap();
                resp.set_body(buffer.into_inner());
                resp.set_content_type("image/png");
                resp.insert_header("Cache-Control", "max-age=114514");
            } else {
                resp.set_status(tide::StatusCode::NotFound);
                resp.set_body(json!({
                    "code": 4,
                    "message": {
                        "cn": "图片不存在",
                        "en": "Image not found",
                    },
                    "description": {
                        "task_id": task_id,
                    },
                }));
            }
            Ok(resp)
        } else {
            let mut resp = tide::Response::new(tide::StatusCode::BadRequest);
            resp.set_body(json!({
                    "code": 4,
                    "message": {
                        "cn": "附件不存在",
                        "en": "Attachment not found",
                    },
                    "description": {
                        "task_id": task_id,
                    },
                }));
            Ok(resp)
        }
    } else {
        let mut resp = tide::Response::new(tide::StatusCode::BadRequest);
        resp.set_body(json!({
                "code": 4,
                "message": {
                    "cn": "任务不存在",
                    "en": "Task not found",
                },
                "description": {
                    "task_id": task_id,
                },
            }));
        Ok(resp)
    }
}

#[derive(Deserialize)]
struct UpdateTaskForm {
    threshold: f64,
}


async fn api_update_task(mut req: Request<AppState>) -> tide::Result<tide::Response> {
    require_perm(&req, vec![1, 2, 3]).await?;
    let form: UpdateTaskForm = req.body_json().await?;
    let task_id = req.param("task_id").unwrap().to_owned();
    let state = req.state();
    let db = &state.db.to_owned();
    if let Some(mut task) = Detection::by_id(db, &task_id).await {
        task.threshold = Some(form.threshold);
        task.save(&db, None).await?;
        Ok(json!(task.to_info().await).into())
    } else {
        Ok(json_response(404, json!( {
            "code": 4,
            "message": {
                "cn": "任务不存在",
                "en": "Task not found",
            },
        })))
    }
}

async fn api_delete_task(req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![1, 2, 3]).await?;
    let task_id = req.param("task_id").unwrap().to_owned();
    let state = req.state();
    let db = &state.db.to_owned();
    if let Some(task) = Detection::by_id(db, &task_id).await {
        task.delete(&db).await?;
        Ok(Response::new(204))
    } else {
        Ok(json_response(404, json!( {
            "code": 4,
            "message": {
                "cn": "任务不存在",
                "en": "Task not found",
            },
        })))
    }
}

async fn api_compute_number(req: Request<AppState>) -> tide::Result {
    let state = req.state();
    let db = &state.db.to_owned();
    let task_id = req.param("task_id").unwrap().to_owned();

    if let Some(task) = Detection::by_id(db, &task_id).await {
        if let Some(boxes) = task.result {
            let threshold = if let Some(threshold) = task.threshold {
                threshold
            } else {
                0.5
            } as f32;
            let mut num = 0;
            for box_ in boxes {
                if box_.score >= threshold {
                    num += 1;
                }
            }
            Ok(json!(num).into())
        } else {
            Ok(json_response(404, json!( {
                "code": 1001,
                "message": {
                    "cn": "任务尚未完成",
                    "en": "Task not finished",
                },
            })))
        }
    } else {
        Ok(json_response(404, json!( {
            "code": 4,
            "message": {
                "cn": "任务不存在",
                "en": "Task not found",
            },
        })))
    }
}


async fn api_get_user_detections(req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = &state.db.to_owned();
    let session: &Session = req.ext().unwrap();
    // 查询用户的任务
    let mut result = Vec::new();
    let detections: Vec<_> = Detection::find(&db, Some(doc! {
        "creator": session.user.as_ref().unwrap()
    }), None).await?.collect().await;
    for detection in detections {
        if let Ok(detection) = detection {
            result.push(detection.to_info().await);
        }
    }
    Ok(json!(result).into())
}