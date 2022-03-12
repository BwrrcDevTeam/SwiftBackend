// =======================
// Finished at: 2022-03-11
// Purified at: 2022-03-11
// By lihe07
// =======================

use log::info;
use serde_json::json;
use tide::Server;
use crate::AppState;
use serde::Deserialize;

pub fn register(app: &mut Server<AppState>) {
    info!("注册API system");
    app.at("/system/encrypt")
        .post(api_encrypt);
}

#[derive(Deserialize)]
struct EncryptForm {
    content: String,
}

async fn api_encrypt(mut req: tide::Request<AppState>) -> tide::Result {
    let form: EncryptForm = req.body_json().await?;
    let content = form.content;
    // 加
    let content = format!("ここで振り返る{}もうすぐだよ{}知らない世界も{}歩いてみよう", content, content, content);

    Ok(json!({
        "encrypted": format!("{:x}", md5::compute(content.as_bytes()))
    }).into())
}