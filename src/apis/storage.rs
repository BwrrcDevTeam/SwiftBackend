use log::info;
use serde_json::json;
use tide::{Body, Request, Response, Server};
use crate::apis::{json_response, require_perm};
use crate::AppState;
use crate::models::{SearchById, Session};
use crate::models::storage::Storage;
use wither::Model;

pub fn register(app: &mut Server<AppState>) {
    info!("注册API storage");
    app.at("/storage").post(api_upload);
    app.at("/storage/inline/:id").get(api_download_inline);
    app.at("/storage/download/:id").get(api_download_attachment);
    app.at("/storage/inline/:id/w/:width/h/:height").get(api_download_inline_resized);
    app.at("/storage/:id").delete(api_delete)
        .get(api_get_info);
}

// POST /storage

use async_std::io::{self, Read};
use async_std::stream::Stream;
use async_std::task::{Context, Poll, ready};

use std::pin::Pin;
use std::str::FromStr;
use futures::AsyncWriteExt;
use tide::http::Mime;

#[derive(Debug)]
pub struct BufferedBytesStream<T> {
    inner: T,
}

impl<T: Read + Unpin> Stream for BufferedBytesStream<T> {
    type Item = async_std::io::Result<Vec<u8>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut buf = [0u8; 2048];

        let rd = Pin::new(&mut self.inner);

        match ready!(rd.poll_read(cx, &mut buf)) {
            Ok(0) => Poll::Ready(None),
            Ok(n) => Poll::Ready(Some(Ok(buf[..n].to_vec()))),
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => Poll::Pending,
            Err(e) => Poll::Ready(Some(Err(e))),
        }
    }
}

fn random_filename(origin_ext: String) -> String {
    use rand::Rng;
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    let now = SystemTime::now();
    let since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
    let timestamp = since_epoch.as_secs();

    let mut rng = rand::thread_rng();
    let random_number = rng.gen::<u64>();

    format!("{}-{}.{}", timestamp, random_number, origin_ext)
}

pub async fn api_upload(mut req: Request<AppState>) -> tide::Result {
    require_perm(&mut req, vec![0, 1, 2, 3]).await?;
    let state = req.state().to_owned();
    let session: &Session = req.ext().unwrap();
    let session = session.to_owned();
    let mime = req.content_type().unwrap();
    if mime.essence().to_string() == "multipart/form-data" {
        let boundary = mime.param("boundary").unwrap().to_string();
        let mut body = BufferedBytesStream { inner: req };
        let mut multipart = multer::Multipart::new(&mut body, boundary);
        let mut storage = None;
        while let Some(mut field) = multipart.next_field().await? {
            if field.name() != Some("file") {
                continue;
            }
            let file_name = field.file_name().unwrap().to_string();
            let ext = file_name.split(".").last().unwrap();
            let local_path = state.config.storage.get_path(random_filename(ext.to_string()));
            let mut output = async_std::fs::File::create(&local_path).await?;
            while let Some(chunk) = field.chunk().await? {
                output.write_all(&chunk).await?;
            }
            output.flush().await?;
            storage = Some(Storage {
                id: None,
                filename: file_name,
                local_path,
                mime_type: field.content_type().unwrap().to_string(),
                created_at: chrono::Utc::now().into(),
                owner: session.user.to_owned().unwrap_or("Anonymous".to_string()),
            });
        }
        if let Some(mut storage) = storage {
            storage.save(&state.db, None).await?;
            Ok(storage.to_response().into())
        } else {
            Ok(json_response(400, json!({ "code": 400, "message": {
                "cn": "请指定文件",
                "en": "Please specify a file"
            } })))
        }
    } else {
        Ok(json_response(400, json!({
            "code": 400,
            "message": {
                "cn": "无法识别的请求类型",
                "en": "Unrecognized request type"
            },
            "description": {
                "cn": "请求类型应该为 multipart/form-data",
                "en": "Request type should be multipart/form-data"
            }
        })))
    }
}

async fn api_download_inline(req: Request<AppState>) -> tide::Result {
    let state = req.state();
    // let session: &Session = req.ext().unwrap();
    let db = state.db.to_owned();
    let id = req.param("id").unwrap().to_owned();
    if let Some(storage) = Storage::by_id(&db, &id).await {
        if let Ok(body) = Body::from_file(storage.local_path).await {
            let mut resp = Response::new(200);
            // resp.set_content_type(Mime::from_str(&*storage.mime_type.to_owned()).unwrap());
            resp.insert_header("Content-Disposition", "inline");
            resp.insert_header("Cache-Control", "max-age=86400");
            resp.set_body(body);
            Ok(resp)
        } else {
            Ok(json_response(500, json!({
                "code": 500,
                "message": {
                    "cn": "服务器无法读取文件",
                    "en": "Server can't read file"
                }
            })))
        }
    } else {
        Ok(json_response(404, json!({
            "code": 404,
            "message": {
                "cn": "附件不存在",
                "en": "Attachment not found"
            }
        })))
    }
}

async fn api_download_attachment(req: Request<AppState>) -> tide::Result {
    let state = req.state();
    let db = state.db.to_owned();
    let id = req.param("id").unwrap().to_owned();
    if let Some(storage) = Storage::by_id(&db, &id).await {
        if let Ok(body) = Body::from_file(storage.local_path).await {
            let mut resp = Response::new(200);
            // resp.set_content_type(Mime::from_str(&*storage.mime_type.to_owned()).unwrap());
            resp.insert_header("Content-Disposition", "attachment; filename=\"".to_string() + &*urlencoding::encode(&*storage.filename).to_string() + "\"");
            resp.insert_header("Cache-Control", "max-age=86400");
            resp.set_body(body);
            Ok(resp)
        } else {
            Ok(json_response(500, json!({
                "code": 500,
                "message": {
                    "cn": "服务器无法读取文件",
                    "en": "Server can't read file"
                }
            })))
        }
    } else {
        Ok(json_response(404, json!({
            "code": 404,
            "message": {
                "cn": "附件不存在",
                "en": "Attachment not found"
            }
        })))
    }
}

async fn api_delete(req: Request<AppState>) -> tide::Result {
    let state = req.state();
    let session: &Session = req.ext().unwrap();
    let db = state.db.to_owned();
    let id = req.param("id").unwrap().to_owned();
    if let Some(storage) = Storage::by_id(&db, &id).await {
        if &storage.owner == session.user.as_ref().unwrap_or(&"Anonymous".to_string()) {
            if let Ok(()) = async_std::fs::remove_file(&storage.local_path).await {
                if let Ok(..) = storage.delete(&db).await {
                    Ok(json!({ "code": 200, "message": {
                        "cn": "删除成功",
                        "en": "Delete success"
                    } }).into())
                } else {
                    Ok(json_response(500, json!({
                        "code": 500,
                        "message": {
                            "cn": "服务器无法删除文件",
                            "en": "Server can't delete file"
                        }
                    })))
                }
            } else {
                Ok(json_response(500, json!({
                    "code": 500,
                    "message": {
                        "cn": "服务器无法删除文件",
                        "en": "Server can't delete file"
                    }
                })))
            }
        } else {
            Ok(json_response(403, json!({
                "code": 403,
                "message": {
                    "cn": "您无权删除该附件",
                    "en": "You don't have permission to delete this attachment"
                }
            })))
        }
    } else {
        Ok(json_response(404, json!({
            "code": 404,
            "message": {
                "cn": "附件不存在",
                "en": "Attachment not found"
            }
        })))
    }
}

async fn api_download_inline_resized(req: Request<AppState>) -> tide::Result {
    let state = req.state();
    // let session: &Session = req.ext().unwrap();
    let db = state.db.to_owned();
    let id = req.param("id").unwrap().to_owned();
    let width = req.param("width").unwrap().to_owned().parse::<u32>().unwrap_or(0);
    let height = req.param("height").unwrap().to_owned().parse::<u32>().unwrap_or(0);
    if let Some(storage) = Storage::by_id(&db, &id).await {
        if let Ok(image) = image::open(storage.local_path) {
            let image = image.resize_to_fill(width, height, image::imageops::FilterType::Triangle);
            let buffer = Vec::new();
            let mut buffer = std::io::Cursor::new(buffer);
            image.write_to(&mut buffer, image::ImageOutputFormat::Png).unwrap();
            let body = Body::from_bytes(buffer.into_inner());
            let mut resp = Response::new(200);
            resp.set_content_type(Mime::from_str(&*storage.mime_type.to_owned()).unwrap());
            resp.insert_header("Content-Disposition", "inline");
            resp.insert_header("Cache-Control", "max-age=86400");
            resp.set_body(body);
            Ok(resp)
        } else {
            Ok(json_response(500, json!({
                "code": 500,
                "message": {
                    "cn": "无法读取图片文件",
                    "en": "Can't read image file"
                }
            })))
        }
    } else {
        Ok(json_response(404, json!({
            "code": 404,
            "message": {
                "cn": "附件不存在",
                "en": "Attachment not found"
            }
        })))
    }
}

async fn api_get_info(req: Request<AppState>) -> tide::Result {
    require_perm(&req, vec![1, 2, 3]).await?;
    let state = req.state();
    let db = state.db.to_owned();
    let id = req.param("id").unwrap().to_owned();
    if let Some(storage) = Storage::by_id(&db, &id).await {
        Ok(storage.to_response().into())
    } else {
        Ok(json_response(404, json!({
            "code": 404,
            "message": {
                "cn": "附件不存在",
                "en": "Attachment not found"
            }
        })))
    }
}
