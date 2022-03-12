mod detector;
mod users;
mod system;
mod records;
mod positions;
mod groups;
mod projects;
mod storage;
// mod data;

use log::info;
use tide::{Request, Server};
use tide::prelude::json;

use crate::AppState;
use crate::errors::AppErrors;


pub fn register(app: &mut Server<AppState>) {
    info!("注册API index");
    app.at("/").get(|req: Request<AppState>| async move {
        let session: &crate::models::Session = req.ext().unwrap();
        Ok(json!({
            "message": "Hello!!",
            "session": session.to_response()
        }))
    });
    detector::register(app);
    users::register(app);
    system::register(app);
    records::register(app);
    positions::register(app);
    groups::register(app);
    storage::register(app);
}


pub async fn require_perm(req: &tide::Request<AppState>, allowed: Vec<i8>) -> Result<(), AppErrors> {
    let session: &crate::models::Session = req.ext().unwrap();
    if allowed.contains(&session.permission) {
        Ok(())
    } else {
        Err(AppErrors::CrossPermissionError(session.permission, allowed))
    }
}

pub fn json_response(status: u16, data: serde_json::Value) -> tide::Response  {
    let mut resp = tide::Response::new(status);
    resp.set_content_type("application/json");
    resp.set_body(data);
    resp
}