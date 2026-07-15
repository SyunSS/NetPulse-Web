use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use sqlx::SqlitePool;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::config::{match_platform, AppConfig};
use crate::engines::browser::BrowserEngine;
use crate::engines::dns::DnsEngine;
use crate::engines::download::DownloadEngine;
use crate::engines::http::HttpEngine;
use crate::engines::ping::PingEngine;
use crate::engines::video::VideoEngine;
use crate::models::task::{DownloadResult, PingResult, VideoResult, WebsiteResult};
use crate::storage::StorageManager;
use crate::utils::response::{ProgressMessage, TaskJob};

/// 任务执行器
pub struct TaskWorker {
    db: SqlitePool,
    config: Arc<AppConfig>,
    task_rx: mpsc::Receiver<TaskJob>,
    progress_tx: broadcast::Sender<ProgressMessage>,
}

impl TaskWorker {
    pub fn new(
        db: SqlitePool,
        config: Arc<AppConfig>,
        task_rx: mpsc::Receiver<TaskJob>,
        progress_tx: broadcast::Sender<ProgressMessage>,
    ) -> Self {
        Self {
            db,
            config,
            task_rx,
            progress_tx,
        }
    }

    /// 启动 Worker，返回 JoinHandle
    pub fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            info!("TaskWorker 启动，等待任务...");

            while let Some(job) = self.task_rx.recv().await {
                info!("收到任务: {} (类型: {})", job.task_id, job.task_type);

                if job.task_type == "website" {
                    let db = self.db.clone();
                    let config = self.config.clone();
                    let progress_tx = self.progress_tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = run_website_task(db, config, progress_tx, job).await {
                            error!("任务执行异常: {}", e);
                        }
                    });
                } else if job.task_type == "video" {
                    let db = self.db.clone();
                    let config = self.config.clone();
                    let progress_tx = self.progress_tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = run_video_task(db, config, progress_tx, job).await {
                            error!("视频任务执行异常: {}", e);
                        }
                    });
                } else if job.task_type == "download" {
                    let db = self.db.clone();
                    let config = self.config.clone();
                    let progress_tx = self.progress_tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = run_download_task(db, config, progress_tx, job).await {
                            error!("下载任务执行异常: {}", e);
                        }
                    });
                } else if job.task_type == "ping" {
                    let db = self.db.clone();
                    let config = self.config.clone();
                    let progress_tx = self.progress_tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = run_ping_task(db, config, progress_tx, job).await {
                            error!("Ping 任务执行异常: {}", e);
                        }
                    });
                } else {
                    warn!("不支持的任务类型: {}", job.task_type);
                }
            }

            info!("TaskWorker 已停止");
        })
    }
}

/// 执行网站测试任务
async fn run_website_task(
    db: SqlitePool,
    config: Arc<AppConfig>,
    progress_tx: broadcast::Sender<ProgressMessage>,
    job: TaskJob,
) -> anyhow::Result<()> {
    let task_id = &job.task_id;
    let total = job.urls.len();
    let timeout = Duration::from_secs(config.task.timeout_seconds);

    // 更新任务状态为 running
    update_task_status(&db, task_id, "running", None).await?;

    // 推送任务开始
    let _ = progress_tx.send(ProgressMessage::TaskStarted {
        task_id: task_id.clone(),
        total_urls: total,
    });

    log_progress(&progress_tx, task_id, "info", &format!("开始测试 {} 个URL", total));

    let mut success_count = 0usize;
    let mut fail_count = 0usize;

    for (i, url) in job.urls.iter().enumerate() {
        // 推送当前 URL 测试中
        let _ = progress_tx.send(ProgressMessage::UrlTesting {
            task_id: task_id.clone(),
            url: url.clone(),
            current: i + 1,
            total,
        });

        log_progress(&progress_tx, task_id, "info", &format!("正在测试: {}", url));

        // 执行单个 URL 测试
        match test_single_url(&db, &config, task_id, url, timeout).await {
            Ok(result) => {
                success_count += 1;
                let _ = progress_tx.send(ProgressMessage::UrlCompleted {
                    task_id: task_id.clone(),
                    url: url.clone(),
                    result,
                });
            }
            Err(e) => {
                fail_count += 1;
                log_progress(&progress_tx, task_id, "error", &format!("测试失败 {}: {}", url, e));

                // 即使失败也写一条记录
                let failed_result = WebsiteResult {
                    id: Uuid::new_v4().to_string(),
                    task_id: task_id.clone(),
                    url: url.clone(),
                    dns_time_ms: None,
                    dns_success: None,
                    tcp_time_ms: None,
                    tls_time_ms: None,
                    http_status: None,
                    ttfb_ms: None,
                    fp_ms: None,
                    fcp_ms: None,
                    dom_content_loaded_ms: None,
                    load_event_ms: None,
                    page_open_time_ms: None,
                    first_paint_ms: None,
                    resource_count: None,
                    resource_total_size: None,
                    final_url: None,
                    page_title: None,
                    screenshot_path: None,
                    error_msg: Some(e.to_string()),
                    created_at: Utc::now().to_rfc3339(),
                    test_count: None,
                };
                save_website_result(&db, &failed_result).await.ok();
            }
        }

        // 更新进度
        let progress = ((i + 1) as f64 / total as f64) * 100.0;
        let _ = progress_tx.send(ProgressMessage::ProgressUpdate {
            task_id: task_id.clone(),
            progress,
        });

        // 更新数据库中的进度
        update_task_progress(&db, task_id, progress).await.ok();
    }

    // 任务完成
    let _ = progress_tx.send(ProgressMessage::TaskCompleted {
        task_id: task_id.clone(),
        success_count,
        fail_count,
    });

    log_progress(
        &progress_tx,
        task_id,
        "info",
        &format!("任务完成: 成功 {}, 失败 {}", success_count, fail_count),
    );

    // 更新任务状态
    update_task_status(&db, task_id, "completed", None).await?;

    Ok(())
}

/// 测试单个 URL
async fn test_single_url(
    db: &SqlitePool,
    config: &AppConfig,
    task_id: &str,
    url: &str,
    timeout: Duration,
) -> anyhow::Result<WebsiteResult> {
    let now = Utc::now().to_rfc3339();
    let result_id = Uuid::new_v4().to_string();

    // 1. DNS 解析
    let dns_result = DnsEngine::resolve(url).await?;

    // 2. HTTP 探测
    let http_result = HttpEngine::probe(url, timeout).await;

    // 3. 浏览器测试
    let browser_engine = BrowserEngine::new(
        &config.chrome.path,
        config.chrome.headless,
        timeout,
    );
    let browser_result = browser_engine.test_page(url).await;

    // 4. 保存截图
    let screenshot_path = if let Some(data) = &browser_result.screenshot {
        match StorageManager::save_screenshot(
            &config.storage.screenshot_dir,
            task_id,
            url,
            data,
        ) {
            Ok(path) => Some(path),
            Err(e) => {
                warn!("截图保存失败: {}", e);
                None
            }
        }
    } else {
        None
    };

    // 5. 组装结果
    let result = WebsiteResult {
        id: result_id,
        task_id: task_id.to_string(),
        url: url.to_string(),
        dns_time_ms: Some(dns_result.dns_time_ms),
        dns_success: Some(if dns_result.dns_success { 1 } else { 0 }),
        tcp_time_ms: Some(http_result.tcp_time_ms),
        tls_time_ms: Some(http_result.tls_time_ms),
        http_status: http_result.http_status,
        ttfb_ms: Some(http_result.ttfb_ms),
        fp_ms: browser_result.fp_ms,
        fcp_ms: browser_result.fcp_ms,
        dom_content_loaded_ms: browser_result.dom_content_loaded_ms,
        load_event_ms: browser_result.load_event_ms,
        page_open_time_ms: browser_result.page_open_time_ms,
        first_paint_ms: browser_result.first_paint_ms,
        resource_count: browser_result.resource_count,
        resource_total_size: browser_result.resource_total_size,
        final_url: browser_result.final_url.or(Some(http_result.final_url)),
        page_title: browser_result.page_title,
        screenshot_path,
        error_msg: browser_result.error,
        created_at: now,
        test_count: None,
    };

    // 6. 写入数据库
    save_website_result(db, &result).await?;

    Ok(result)
}

/// 保存网站测试结果到数据库
async fn save_website_result(db: &SqlitePool, result: &WebsiteResult) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT INTO website_result (
            id, task_id, url, dns_time_ms, dns_success, tcp_time_ms, tls_time_ms,
            http_status, ttfb_ms, fp_ms, fcp_ms, dom_content_loaded_ms, load_event_ms,
            page_open_time_ms, first_paint_ms, resource_count, resource_total_size,
            final_url, page_title, screenshot_path, error_msg, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&result.id)
    .bind(&result.task_id)
    .bind(&result.url)
    .bind(result.dns_time_ms)
    .bind(result.dns_success)
    .bind(result.tcp_time_ms)
    .bind(result.tls_time_ms)
    .bind(result.http_status)
    .bind(result.ttfb_ms)
    .bind(result.fp_ms)
    .bind(result.fcp_ms)
    .bind(result.dom_content_loaded_ms)
    .bind(result.load_event_ms)
    .bind(result.page_open_time_ms)
    .bind(result.first_paint_ms)
    .bind(result.resource_count)
    .bind(result.resource_total_size)
    .bind(&result.final_url)
    .bind(&result.page_title)
    .bind(&result.screenshot_path)
    .bind(&result.error_msg)
    .bind(&result.created_at)
    .execute(db)
    .await?;

    Ok(())
}

/// 更新任务状态
async fn update_task_status(
    db: &SqlitePool,
    task_id: &str,
    status: &str,
    error_msg: Option<&str>,
) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    match status {
        "running" => {
            sqlx::query("UPDATE test_task SET status = ?, started_at = ? WHERE id = ?")
                .bind(status)
                .bind(&now)
                .bind(task_id)
                .execute(db)
                .await?;
        }
        "completed" | "failed" | "cancelled" => {
            sqlx::query("UPDATE test_task SET status = ?, finished_at = ?, error_msg = ? WHERE id = ?")
                .bind(status)
                .bind(&now)
                .bind(error_msg)
                .bind(task_id)
                .execute(db)
                .await?;
        }
        _ => {
            sqlx::query("UPDATE test_task SET status = ? WHERE id = ?")
                .bind(status)
                .bind(task_id)
                .execute(db)
                .await?;
        }
    }
    Ok(())
}

/// 更新任务进度
async fn update_task_progress(
    db: &SqlitePool,
    task_id: &str,
    progress: f64,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE test_task SET progress = ? WHERE id = ?")
        .bind(progress)
        .bind(task_id)
        .execute(db)
        .await?;
    Ok(())
}

/// 写入任务日志并推送进度
fn log_progress(
    tx: &broadcast::Sender<ProgressMessage>,
    task_id: &str,
    level: &str,
    message: &str,
) {
    // 写入数据库日志
    let level_str = level.to_string();
    let msg = message.to_string();
    let tid = task_id.to_string();

    // 也推送 WebSocket 消息
    let _ = tx.send(ProgressMessage::Log {
        task_id: task_id.to_string(),
        level: level.to_string(),
        message: message.to_string(),
    });

    // 记录到 tracing
    match level {
        "error" => error!("[{}] {}", tid, msg),
        "warn" => warn!("[{}] {}", tid, msg),
        _ => info!("[{}] {}", tid, msg),
    }
}

// ===== 视频任务 =====

/// 执行视频测试任务
async fn run_video_task(
    db: SqlitePool,
    config: Arc<AppConfig>,
    progress_tx: broadcast::Sender<ProgressMessage>,
    job: TaskJob,
) -> anyhow::Result<()> {
    let task_id = &job.task_id;
    let total = job.urls.len();
    let timeout = Duration::from_secs(config.task.timeout_seconds);

    update_task_status(&db, task_id, "running", None).await?;

    let _ = progress_tx.send(ProgressMessage::TaskStarted {
        task_id: task_id.clone(),
        total_urls: total,
    });

    log_progress(&progress_tx, task_id, "info", &format!("开始视频测试 {} 个URL", total));

    let mut success_count = 0usize;
    let mut fail_count = 0usize;

    for (i, url) in job.urls.iter().enumerate() {
        let _ = progress_tx.send(ProgressMessage::UrlTesting {
            task_id: task_id.clone(),
            url: url.clone(),
            current: i + 1,
            total,
        });

        log_progress(&progress_tx, task_id, "info", &format!("视频测试: {}", url));

        match test_single_video(&db, &config, task_id, url, timeout).await {
            Ok(result) => {
                success_count += 1;
                let _ = progress_tx.send(ProgressMessage::UrlCompleted {
                    task_id: task_id.clone(),
                    url: url.clone(),
                    result: crate::models::task::WebsiteResult {
                        id: result.id.clone(),
                        task_id: result.task_id.clone(),
                        url: result.url.clone(),
                        dns_time_ms: None,
                        dns_success: None,
                        tcp_time_ms: None,
                        tls_time_ms: None,
                        http_status: None,
                        ttfb_ms: None,
                        fp_ms: None,
                        fcp_ms: None,
                        dom_content_loaded_ms: None,
                        load_event_ms: None,
                        page_open_time_ms: Some(result.first_play_time_ms.unwrap_or(0.0)),
                        first_paint_ms: None,
                        resource_count: None,
                        resource_total_size: result.video_size,
                        final_url: None,
                        page_title: result.page_title.clone(),
                        screenshot_path: result.screenshot_path.clone(),
                        error_msg: result.error_msg.clone(),
                        created_at: result.created_at.clone(),
                        test_count: None,
                    },
                });
            }
            Err(e) => {
                fail_count += 1;
                log_progress(&progress_tx, task_id, "error", &format!("视频测试失败 {}: {}", url, e));

                let failed_result = VideoResult {
                    id: Uuid::new_v4().to_string(),
                    task_id: task_id.clone(),
                    url: url.clone(),
                    platform: None,
                    dns_time_ms: None,
                    dns_success: None,
                    tcp_time_ms: None,
                    http_response_ms: None,
                    first_play_time_ms: None,
                    buffer_count: None,
                    total_buffer_time_ms: None,
                    buffer_rate: None,
                    play_success: Some(0),
                    video_download_speed: None,
                    video_size: None,
                    video_duration_ms: None,
                    dropped_frames: None,
                    decoded_frames: None,
                    screenshot_path: None,
                    page_title: None,
                    error_msg: Some(e.to_string()),
                    created_at: Utc::now().to_rfc3339(),
                    test_count: None,
                };
                save_video_result(&db, &failed_result).await.ok();
            }
        }

        let progress = ((i + 1) as f64 / total as f64) * 100.0;
        let _ = progress_tx.send(ProgressMessage::ProgressUpdate {
            task_id: task_id.clone(),
            progress,
        });
        update_task_progress(&db, task_id, progress).await.ok();
    }

    let _ = progress_tx.send(ProgressMessage::TaskCompleted {
        task_id: task_id.clone(),
        success_count,
        fail_count,
    });

    log_progress(
        &progress_tx,
        task_id,
        "info",
        &format!("视频任务完成: 成功 {}, 失败 {}", success_count, fail_count),
    );

    update_task_status(&db, task_id, "completed", None).await?;

    Ok(())
}

/// 测试单个视频 URL
async fn test_single_video(
    db: &SqlitePool,
    config: &AppConfig,
    task_id: &str,
    url: &str,
    timeout: Duration,
) -> anyhow::Result<VideoResult> {
    let now = Utc::now().to_rfc3339();
    let result_id = Uuid::new_v4().to_string();

    // 视频引擎测试 — 配置驱动平台匹配
    let platform_cfg = match_platform(&config.video_platforms, url);
    let video_engine = VideoEngine::new(&config.chrome.path, config.chrome.headless, timeout);
    let video_result = video_engine.test_page(url, &platform_cfg).await;

    // 保存截图
    let screenshot_path = if let Some(data) = &video_result.screenshot {
        match StorageManager::save_screenshot(
            &config.storage.screenshot_dir,
            task_id,
            url,
            data,
        ) {
            Ok(path) => Some(path),
            Err(e) => {
                warn!("视频截图保存失败: {}", e);
                None
            }
        }
    } else {
        None
    };

    let result = VideoResult {
        id: result_id,
        task_id: task_id.to_string(),
        url: url.to_string(),
        platform: Some(video_result.platform),
        dns_time_ms: video_result.dns_time_ms,
        dns_success: Some(if video_result.dns_success { 1 } else { 0 }),
        tcp_time_ms: video_result.tcp_time_ms,
        http_response_ms: video_result.http_response_ms,
        first_play_time_ms: video_result.first_play_time_ms,
        buffer_count: video_result.buffer_count,
        total_buffer_time_ms: video_result.total_buffer_time_ms,
        buffer_rate: video_result.buffer_rate,
        play_success: Some(if video_result.play_success { 1 } else { 0 }),
        video_download_speed: video_result.video_download_speed,
        video_size: video_result.video_size,
        video_duration_ms: video_result.video_duration_ms,
        dropped_frames: video_result.dropped_frames,
        decoded_frames: video_result.decoded_frames,
        screenshot_path: screenshot_path.clone(),
        page_title: video_result.page_title.clone(),
        error_msg: video_result.error,
        created_at: now,
        test_count: None,
    };

    save_video_result(db, &result).await?;

    Ok(result)
}

/// 保存视频测试结果到数据库
async fn save_video_result(db: &SqlitePool, result: &VideoResult) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT INTO video_result (
            id, task_id, url, platform, dns_time_ms, dns_success, tcp_time_ms, http_response_ms,
            first_play_time_ms, buffer_count, total_buffer_time_ms, buffer_rate,
            play_success, video_download_speed, video_size, video_duration_ms,
            dropped_frames, decoded_frames, screenshot_path, page_title, error_msg, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&result.id)
    .bind(&result.task_id)
    .bind(&result.url)
    .bind(&result.platform)
    .bind(result.dns_time_ms)
    .bind(result.dns_success)
    .bind(result.tcp_time_ms)
    .bind(result.http_response_ms)
    .bind(result.first_play_time_ms)
    .bind(result.buffer_count)
    .bind(result.total_buffer_time_ms)
    .bind(result.buffer_rate)
    .bind(result.play_success)
    .bind(result.video_download_speed)
    .bind(result.video_size)
    .bind(result.video_duration_ms)
    .bind(result.dropped_frames)
    .bind(result.decoded_frames)
    .bind(&result.screenshot_path)
    .bind(&result.page_title)
    .bind(&result.error_msg)
    .bind(&result.created_at)
    .execute(db)
    .await?;

    Ok(())
}

// ===== 下载任务 =====

async fn run_download_task(
    db: SqlitePool,
    config: Arc<AppConfig>,
    progress_tx: broadcast::Sender<ProgressMessage>,
    job: TaskJob,
) -> anyhow::Result<()> {
    let task_id = &job.task_id;
    let total = job.urls.len();
    let timeout = Duration::from_secs(config.task.timeout_seconds);

    update_task_status(&db, task_id, "running", None).await?;
    let _ = progress_tx.send(ProgressMessage::TaskStarted { task_id: task_id.clone(), total_urls: total });
    log_progress(&progress_tx, task_id, "info", &format!("开始下载测试 {} 个URL", total));

    let mut success_count = 0usize;
    let mut fail_count = 0usize;

    for (i, url) in job.urls.iter().enumerate() {
        let _ = progress_tx.send(ProgressMessage::UrlTesting {
            task_id: task_id.clone(), url: url.clone(), current: i + 1, total,
        });
        log_progress(&progress_tx, task_id, "info", &format!("下载测试: {}", url));

        let engine = DownloadEngine::new(timeout);
        let result = engine.test_download(url).await;

        let now = Utc::now().to_rfc3339();
        let dl = DownloadResult {
            id: Uuid::new_v4().to_string(),
            task_id: task_id.clone(),
            url: url.clone(),
            dns_time_ms: None,
            dns_success: None,
            tcp_time_ms: None,
            download_speed: Some(result.download_speed),
            avg_speed: Some(result.avg_speed),
            peak_speed: Some(result.peak_speed),
            download_time_ms: Some(result.download_time_ms),
            file_size: Some(result.file_size),
            success: Some(if result.success { 1 } else { 0 }),
            error_msg: result.error.clone(),
            created_at: now,
        test_count: None,
        };

        save_download_result(&db, &dl).await?;

        if result.success { success_count += 1; } else { fail_count += 1; }

        let progress = ((i + 1) as f64 / total as f64) * 100.0;
        let _ = progress_tx.send(ProgressMessage::ProgressUpdate { task_id: task_id.clone(), progress });
        update_task_progress(&db, task_id, progress).await.ok();
    }

    let _ = progress_tx.send(ProgressMessage::TaskCompleted {
        task_id: task_id.clone(), success_count, fail_count,
    });
    log_progress(&progress_tx, task_id, "info", &format!("下载任务完成: 成功{}, 失败{}", success_count, fail_count));
    update_task_status(&db, task_id, "completed", None).await?;

    Ok(())
}

async fn save_download_result(db: &SqlitePool, result: &DownloadResult) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO download_result (id, task_id, url, dns_time_ms, dns_success, tcp_time_ms, download_speed, avg_speed, peak_speed, download_time_ms, file_size, success, error_msg, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&result.id).bind(&result.task_id).bind(&result.url)
    .bind(result.dns_time_ms).bind(result.dns_success).bind(result.tcp_time_ms)
    .bind(result.download_speed).bind(result.avg_speed).bind(result.peak_speed)
    .bind(result.download_time_ms).bind(result.file_size).bind(result.success)
    .bind(&result.error_msg).bind(&result.created_at)
    .execute(db).await?;
    Ok(())
}

// ===== Ping 任务 =====

async fn run_ping_task(
    db: SqlitePool,
    config: Arc<AppConfig>,
    progress_tx: broadcast::Sender<ProgressMessage>,
    job: TaskJob,
) -> anyhow::Result<()> {
    let task_id = &job.task_id;
    let total = job.urls.len();
    let timeout = Duration::from_secs(config.task.timeout_seconds);

    update_task_status(&db, task_id, "running", None).await?;
    let _ = progress_tx.send(ProgressMessage::TaskStarted { task_id: task_id.clone(), total_urls: total });
    log_progress(&progress_tx, task_id, "info", &format!("开始 Ping 测试 {} 个目标", total));

    let mut success_count = 0usize;
    let mut fail_count = 0usize;

    for (i, host) in job.urls.iter().enumerate() {
        let _ = progress_tx.send(ProgressMessage::UrlTesting {
            task_id: task_id.clone(), url: host.clone(), current: i + 1, total,
        });

        let engine = PingEngine::new(timeout);
        let result = engine.test_ping(host).await;

        let now = Utc::now().to_rfc3339();
        let pr = PingResult {
            id: Uuid::new_v4().to_string(),
            task_id: task_id.clone(),
            host: result.host,
            avg_latency_ms: Some(result.avg_latency_ms),
            packet_loss_rate: Some(result.packet_loss_rate),
            jitter_ms: Some(result.jitter_ms),
            success: Some(if result.success { 1 } else { 0 }),
            error_msg: result.error,
            created_at: now,
        test_count: None,
        };

        save_ping_result(&db, &pr).await?;

        if result.success { success_count += 1; } else { fail_count += 1; }

        let progress = ((i + 1) as f64 / total as f64) * 100.0;
        let _ = progress_tx.send(ProgressMessage::ProgressUpdate { task_id: task_id.clone(), progress });
        update_task_progress(&db, task_id, progress).await.ok();
    }

    let _ = progress_tx.send(ProgressMessage::TaskCompleted {
        task_id: task_id.clone(), success_count, fail_count,
    });
    update_task_status(&db, task_id, "completed", None).await?;
    Ok(())
}

async fn save_ping_result(db: &SqlitePool, result: &PingResult) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO ping_result (id, task_id, host, avg_latency_ms, packet_loss_rate, jitter_ms, success, error_msg, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&result.id).bind(&result.task_id).bind(&result.host)
    .bind(result.avg_latency_ms).bind(result.packet_loss_rate).bind(result.jitter_ms)
    .bind(result.success).bind(&result.error_msg).bind(&result.created_at)
    .execute(db).await?;
    Ok(())
}
