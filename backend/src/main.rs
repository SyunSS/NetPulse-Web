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
use crate::scheduler::PlanScheduler;
use crate::utils::response::AppState;
use crate::worker::TaskWorker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 加载配置
    let config = AppConfig::load()?;

    // 初始化日志系统
    init_logging(&config.logging);

    info!("NetPulse Web 启动中...");

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

    // 创建浏览器提供者
    let browser_provider: Arc<Box<dyn crate::engines::browser::provider::BrowserProvider>> =
        Arc::new(crate::engines::browser::provider::create_browser_provider(&config.browser)?);
    info!("浏览器提供者已创建");

    // 启动 Worker
    let worker = TaskWorker::new(
        db_pool.clone(),
        Arc::new(config.clone()),
        task_rx,
        progress_tx.clone(),
        browser_provider,
    );
    worker.start();
    info!("TaskWorker 已启动");

    // 启动调度器
    let scheduler = PlanScheduler::new(db_pool.clone(), task_tx.clone(), progress_tx.clone());
    scheduler.start();
    info!("PlanScheduler 已启动");

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
fn init_logging(log_cfg: &crate::config::LoggingConfig) {
    use tracing_appender::rolling;
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&log_cfg.level));

    let registry = tracing_subscriber::registry();

    // 控制台输出
    if log_cfg.console {
        let console_layer = match log_cfg.format.as_str() {
            "json" => Box::new(fmt::layer().json().with_target(false).with_writer(std::io::stdout))
                as Box<dyn tracing_subscriber::Layer<_> + Send + Sync>,
            _ => Box::new(fmt::layer().compact().with_target(false).with_writer(std::io::stdout)),
        };
        registry.with(console_layer).with(filter.clone()).init();
        return;
    }

    // 文件滚动输出
    let file_layer = match log_cfg.format.as_str() {
        "json" => {
            let file = rolling::daily(&log_cfg.file_dir, "netpulse.log");
            Box::new(fmt::layer().json().with_target(false).with_writer(file))
                as Box<dyn tracing_subscriber::Layer<_> + Send + Sync>
        }
        _ => {
            let file = rolling::daily(&log_cfg.file_dir, "netpulse.log");
            Box::new(fmt::layer().with_target(false).with_ansi(false).with_writer(file))
                as Box<dyn tracing_subscriber::Layer<_> + Send + Sync>
        }
    };
    registry.with(file_layer).with(filter).init();
}
