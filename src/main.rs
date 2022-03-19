mod config;
mod apis;
mod models;
mod session;
mod errors;
mod forms;

use log::{error, info};
use tide::http::headers::HeaderValue;

use tide::utils::After;
use wither::mongodb::{Client, Database};

// 全局共用的State
#[derive(Clone)]
pub struct AppState {
    config: config::Config,
    db: Database,
}


#[async_std::main]
async fn main() {
    // 初始化配置
    badlog::init_from_env("LOG_LEVEL");
    info!("加载配置文件 ./config.toml");
    let config = config::load_config();
    let origins = config.server.get_origins();
    info!("配置文件加载完成");
    info!("连接数据库 {}", &config.database.path);
    let db = Client::with_uri_str(&config.database.path).await;
    if db.is_err() {
        error!("数据库连接失败");
        return;
    }
    let db = db.unwrap();
    let db = db.database("swiftnext");
    info!("数据库连接成功");
    info!("创建服务器实例");
    let address = format!("{}:{}", &config.server.host, &config.server.port);
    let mut app = tide::with_state(AppState {
        config,
        db,
    });
    // app.with(tide::log::LogMiddleware::new());

    app.with(session::SessionMiddleware {});
    app.with(tide::security::CorsMiddleware::new()
        .allow_methods("GET, POST, PUT, DELETE, OPTIONS, PATCH".parse::<HeaderValue>().unwrap())
        .allow_origin(origins)
        .allow_headers("Content-Type, Authorization, Accept, Origin, X-Requested-With, X-CSRF-Token,X-Forwarded-For".parse::<HeaderValue>().unwrap())
        .allow_credentials(true));
    app.with(After(errors::handle));

    // 注册路由
    info!("注册路由");
    apis::register(&mut app);
    info!("启动服务器");
    if let Err(err) = app.listen(address).await {
        error!("启动服务器失败 {}", err);
    }
}

