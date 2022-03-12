use log::info;
use serde_json::json;
use tide::Server;
use crate::apis::require_perm;
use crate::AppState;

pub fn register(app: &mut Server<AppState>) {
    info!("注册API records");
    app.at("/records/count")
        .get(api_get_records_count);
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