mod api;
mod config;
mod database;
mod engines;
mod models;
mod report;
mod scheduler;
mod services;
mod storage;
mod utils;
mod worker;

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::{broadcast, mpsc};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;

use crate::config::AppConfig;
use crate::database::init_db;
use crate::utils::response::AppState;
use crate::worker::TaskWorker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志系统
    init_logging();

    info!("NetPulse Web 启动中...");

    // 加载配置
    let config = AppConfig::load()?;
    info!("配置加载完成，监听地址: {}", config.server_addr());

    // 确保存储目录存在
    crate::storage::StorageManager::ensure_dir(&config.storage.screenshot_dir)?;
    crate::storage::StorageManager::ensure_dir(&config.storage.excel_dir)?;

    // 初始化数据库
    let db_pool = init_db(config.database_path()).await?;
    info!("数据库初始化完成");

    // 创建消息通道
    let (task_tx, task_rx) = mpsc::channel::<crate::utils::response::TaskJob>(32);
    let (progress_tx, _) = broadcast::channel::<crate::utils::response::ProgressMessage>(256);
    info!("消息通道已创建");

    // 启动 Worker
    let worker = TaskWorker::new(
        db_pool.clone(),
        Arc::new(config.clone()),
        task_rx,
        progress_tx.clone(),
    );
    worker.start();
    info!("TaskWorker 已启动");

    // 构建应用状态
    let state = AppState {
        db: db_pool,
        config: config.clone(),
        task_tx,
        progress_tx,
    };

    // 构建应用路由
    let app = api::build_router(state)
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any));

    // 生产模式：服务前端静态文件
    let app = if std::path::Path::new("frontend-dist").exists() {
        info!("前端静态文件模式已启用 (frontend-dist/)");
        app.fallback_service(
            ServeDir::new("frontend-dist").fallback(ServeFile::new("frontend-dist/index.html")),
        )
    } else {
        app
    };

    // 启动服务器
    let addr: SocketAddr = config.server_addr().parse()?;
    info!("服务器启动: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// 初始化日志系统
fn init_logging() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,netpulse_web=debug"));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true))
        .with(filter)
        .init();
}
