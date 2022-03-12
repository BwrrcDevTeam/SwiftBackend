use rand::Rng;
use tide::{Middleware, Next, Request};
use crate::AppState;
use crate::models::Session;
use log::{error, info, warn};
use tide::http::Cookie;
use wither::bson::doc;
use wither::Model;


pub(crate) struct SessionMiddleware {}

fn generate_fingerprint() -> String {
    // 生成一个随机的字符串
    let mut rng = rand::thread_rng();
    let mut s = String::new();
    for _ in 0..32 {
        s.push(rng.gen_range(b'a'..b'z') as char);
    }
    s
}

fn default_session(timeout: u64, ip: String) -> Session {
    let utc_now = chrono::prelude::Utc::now();
    let expire_at = utc_now + chrono::Duration::seconds(timeout as i64);
    Session {
        id: None,
        fingerprint: generate_fingerprint(),
        login: false,
        permission: 0,
        user: None,
        expire_at: expire_at.into(),
        ip,
    }
}

#[async_trait::async_trait]
impl Middleware<AppState> for SessionMiddleware {
    async fn handle(&self, mut request: Request<AppState>, next: Next<'_, AppState>) -> tide::Result {
        let state = request.state();
        if let Some(fingerprint) = request.cookie("fingerprint") {
            // 如果 cookie 中有 fingerprint，尝试从数据库中查找
            if let Some(mut session) = Session::find_by_fingerprint(&state.db, fingerprint.value()).await {
                if session.ip != request.remote().unwrap_or("-").to_string() {
                    // ip发生了变化
                    if state.config.session.logout_on_ip_change {
                        // 删除这个 session
                        warn!("IP发生了变化 删除session {}", session.fingerprint);
                        if let Err(e) = session.delete(&state.db).await {
                            error!("无法删除 session: {}", e);
                            return Err(tide::Error::from(e));
                        }
                    } else {
                        // 更新 ip
                        let _session = session.update_ip(&state.db, request.remote().unwrap_or("-")).await;
                        if let Err(e) = _session {
                            error!("无法更新 session ip: {:?}", e);
                            return Err(tide::Error::from(e));
                        }
                        session = _session.unwrap();
                        info!("更新 session ip {}", session.ip);
                    }
                }
                // 为这个 session 刷新过期时间
                if let Ok(session) = session.update_timeout(&state.db, state.config.session.timeout).await {
                    // 将 session 放入请求中
                    request.set_ext(session);
                    // 获得响应
                    return Ok(next.run(request).await);
                } else {
                    error!("无法更新 session 过期时间, 将删除这个 session");

                    if let Err(e) = Session::find_one_and_delete(&state.db, doc! {
                        "fingerprint": fingerprint.value()
                    }, None).await {
                        error!("无法删除 session: {}", e);
                        return Err(tide::Error::from(e));
                    }
                }
            }
        }
        // 客户端没有 cookie，则生成一个新的 fingerprint
        let ip = request.remote().unwrap_or("-");
        let mut session = default_session(state.config.session.timeout, ip.to_string());
        if let Err(e) = session.save(&state.db, None).await {
            error!("无法保存 session: {}", e);
            return Err(tide::Error::from(e));
        }
        info!("创建了新的session: {}", session.fingerprint);
        // 将 session 放入请求中
        let fingerprint = session.fingerprint.clone();
        request.set_ext(session);
        // 获得响应
        let mut resp = next.run(request).await;
        // 将新的 fingerprint 放入 cookie
        resp.insert_cookie(Cookie::new("fingerprint", fingerprint));
        Ok(resp)
    }

    fn name(&self) -> &str {
        "SessionMiddleware"
    }
}