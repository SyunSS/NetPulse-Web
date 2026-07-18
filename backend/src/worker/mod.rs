use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use sqlx::SqlitePool;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::config::{match_platform, AppConfig};
use crate::engines::browser::provider::BrowserProvider;
use crate::engines::browser::BrowserEngine;
use crate::engines::dns::DnsEngine;
use crate::engines::download::DownloadEngine;
use crate::engines::http::HttpEngine;
use crate::engines::ping::PingEngine;
use crate::engines::video::VideoEngine;
use crate::models::task::{DownloadResult, PingResult, TestConfig, VideoResult, WebsiteResult};
use crate::storage::StorageManager;
use crate::utils::response::{ProgressMessage, TaskJob};

/// 任务执行器
pub struct TaskWorker {
    db: SqlitePool,
    config: Arc<AppConfig>,
    task_rx: mpsc::Receiver<TaskJob>,
    progress_tx: broadcast::Sender<ProgressMessage>,
    browser_provider: Arc<Box<dyn BrowserProvider>>,
}

impl TaskWorker {
    pub fn new(
        db: SqlitePool,
        config: Arc<AppConfig>,
        task_rx: mpsc::Receiver<TaskJob>,
        progress_tx: broadcast::Sender<ProgressMessage>,
        browser_provider: Arc<Box<dyn BrowserProvider>>,
    ) -> Self {
        Self { db, config, task_rx, progress_tx, browser_provider }
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
                    let bp = self.browser_provider.clone();
                    tokio::spawn(async move {
                        if let Err(e) = run_website_task(db, config, progress_tx, bp, job).await {
                            error!("任务执行异常: {}", e);
                        }
                    });
                } else if job.task_type == "video" {
                    let db = self.db.clone();
                    let config = self.config.clone();
                    let progress_tx = self.progress_tx.clone();
                    let bp = self.browser_provider.clone();
                    tokio::spawn(async move {
                        if let Err(e) = run_video_task(db, config, progress_tx, bp, job).await {
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
    browser_provider: Arc<Box<dyn BrowserProvider>>,
    job: TaskJob,
) -> anyhow::Result<()> {
    let task_id = &job.task_id;
    let total = job.urls.len();
    let timeout = Duration::from_secs(config.task.timeout_seconds);
    let repeat_count = parse_repeat_count(&job.options);

    update_task_status(&db, task_id, "running", None).await?;

    let _ = progress_tx.send(ProgressMessage::TaskStarted { task_id: task_id.clone(), total_urls: total });
    log_progress(&progress_tx, task_id, "info", &format!("开始测试 {} 个URL (重复 {} 次)", total, repeat_count));

    let mut success_count = 0usize;
    let mut fail_count = 0usize;

    for (i, url) in job.urls.iter().enumerate() {
        let _ = progress_tx.send(ProgressMessage::UrlTesting { task_id: task_id.clone(), url: url.clone(), current: i + 1, total });
        log_progress(&progress_tx, task_id, "info", &format!("正在测试: {}", url));

        match test_website_url(&db, &config, &browser_provider, task_id, url, timeout, repeat_count).await {
            Ok(result) => {
                success_count += 1;
                let _ = progress_tx.send(ProgressMessage::UrlCompleted { task_id: task_id.clone(), url: url.clone(), result: result.clone() });
            }
            Err(e) => {
                fail_count += 1;
                log_progress(&progress_tx, task_id, "error", &format!("测试失败 {}: {}", url, e));
                let failed_result = WebsiteResult {
                    id: Uuid::new_v4().to_string(), task_id: task_id.clone(), url: url.clone(),
                    dns_time_ms: None, dns_success: None, tcp_time_ms: None, tls_time_ms: None,
                    http_status: None, ttfb_ms: None, fp_ms: None, fcp_ms: None,
                    dom_content_loaded_ms: None, load_event_ms: None, page_open_time_ms: None,
                    first_paint_ms: None, resource_count: None, resource_total_size: None,
                    final_url: None, page_title: None, screenshot_path: None,
                    error_msg: Some(e.to_string()),
                    html_size: None, css_size: None, js_size: None, image_size: None, font_size: None,
                    total_requests: None, failed_requests: None,
                    lcp_ms: None, cls: None, tti_ms: None,
                    created_at: Utc::now().to_rfc3339(), test_count: Some(repeat_count as i32),
                };
                save_website_result(&db, &failed_result).await.ok();
            }
        }
        let progress = ((i + 1) as f64 / total as f64 * 100.0);
        let _ = progress_tx.send(ProgressMessage::ProgressUpdate { task_id: task_id.clone(), progress });
    }

    let _ = progress_tx.send(ProgressMessage::TaskCompleted { task_id: task_id.clone(), success_count, fail_count });
    update_task_status(&db, task_id, "completed", None).await?;
    Ok(())
}

/// 测试单个 URL
/// 从 job options 中提取 repeat_count，默认 1
fn parse_repeat_count(options: &serde_json::Value) -> usize {
    options.get("repeat_count")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(1)
        .max(1)
}

/// 从 job options 中提取启用的指标集合
fn parse_metrics(options: &serde_json::Value) -> Vec<String> {
    options.get("metrics")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_else(|| vec!["basic".into(), "page".into(), "resource".into()])
}

/// 检查是否启用了某类指标
fn metric_enabled(metrics: &[String], category: &str) -> bool {
    metrics.iter().any(|m| m == category || m == "all")
}

/// 对单个 URL 执行 N 次网站测试并取平均
async fn test_website_url(
    db: &SqlitePool,
    config: &AppConfig,
    browser_provider: &Arc<Box<dyn BrowserProvider>>,
    task_id: &str,
    url: &str,
    timeout: Duration,
    repeat_count: usize,
) -> anyhow::Result<WebsiteResult> {
    let mut dns_times = Vec::with_capacity(repeat_count);
    let mut dns_ok = 0usize;
    let mut tcp_times = Vec::with_capacity(repeat_count);
    let mut tls_times = Vec::with_capacity(repeat_count);
    let mut http_statuses = Vec::with_capacity(repeat_count);
    let mut ttfb_times = Vec::with_capacity(repeat_count);
    let mut fp_times = Vec::with_capacity(repeat_count);
    let mut fcp_times = Vec::with_capacity(repeat_count);
    let mut dcl_times = Vec::with_capacity(repeat_count);
    let mut load_times = Vec::with_capacity(repeat_count);
    let mut page_open_times = Vec::with_capacity(repeat_count);
    let mut resource_counts = Vec::with_capacity(repeat_count);
    let mut resource_sizes = Vec::with_capacity(repeat_count);
    let mut html_sizes = Vec::with_capacity(repeat_count);
    let mut css_sizes = Vec::with_capacity(repeat_count);
    let mut js_sizes = Vec::with_capacity(repeat_count);
    let mut image_sizes = Vec::with_capacity(repeat_count);
    let mut font_sizes = Vec::with_capacity(repeat_count);
    let mut total_reqs = Vec::with_capacity(repeat_count);
    let mut failed_reqs = Vec::with_capacity(repeat_count);
    let mut final_url: Option<String> = None;
    let mut page_title: Option<String> = None;
    let mut screenshot_path: Option<String> = None;
    let mut last_error: Option<String> = None;

    for _ in 0..repeat_count {
        let dns = DnsEngine::resolve(url).await?;
        dns_times.push(dns.dns_time_ms);
        if dns.dns_success { dns_ok += 1; }

        let http = HttpEngine::probe(url, timeout).await;
        tcp_times.push(http.tcp_time_ms);
        tls_times.push(http.tls_time_ms);
        ttfb_times.push(http.ttfb_ms);
        if let Some(s) = http.http_status { http_statuses.push(s); }
        if final_url.is_none() { final_url = Some(http.final_url.clone()); }

        let browser = BrowserEngine::new(browser_provider.clone(), &config.browser.path, config.browser.headless, timeout).test_page(url).await;
        if let Some(v) = browser.fp_ms { fp_times.push(v); }
        if let Some(v) = browser.fcp_ms { fcp_times.push(v); }
        if let Some(v) = browser.dom_content_loaded_ms { dcl_times.push(v); }
        if let Some(v) = browser.load_event_ms { load_times.push(v); }
        if let Some(v) = browser.page_open_time_ms { page_open_times.push(v); }
        if let Some(v) = browser.resource_count { resource_counts.push(v); }
        if let Some(v) = browser.resource_total_size { resource_sizes.push(v); }
        if let Some(v) = browser.html_size { html_sizes.push(v); }
        if let Some(v) = browser.css_size { css_sizes.push(v); }
        if let Some(v) = browser.js_size { js_sizes.push(v); }
        if let Some(v) = browser.image_size { image_sizes.push(v); }
        if let Some(v) = browser.font_size { font_sizes.push(v); }
        if let Some(v) = browser.total_requests { total_reqs.push(v); }
        if let Some(v) = browser.failed_requests { failed_reqs.push(v); }
        if page_title.is_none() { page_title = browser.page_title.clone(); }

        if screenshot_path.is_none() {
            if let Some(ref data) = browser.screenshot {
                screenshot_path = StorageManager::save_screenshot(&config.storage.screenshot_dir, task_id, url, data).ok();
            }
        }

        if let Some(ref e) = browser.error { last_error = Some(e.clone()); }
    }

    let result = WebsiteResult {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_string(),
        url: url.to_string(),
        dns_time_ms: avg(&dns_times),
        dns_success: Some(if repeat_count > 0 { (dns_ok * 100 / repeat_count) as i32 } else { 0 }),
        tcp_time_ms: avg(&tcp_times),
        tls_time_ms: avg(&tls_times),
        http_status: http_statuses.last().copied(),
        ttfb_ms: avg(&ttfb_times),
        fp_ms: avg(&fp_times),
        fcp_ms: avg(&fcp_times),
        dom_content_loaded_ms: avg(&dcl_times),
        load_event_ms: avg(&load_times),
        page_open_time_ms: avg(&page_open_times),
        first_paint_ms: avg(&fp_times),
        resource_count: avg_i32(&resource_counts),
        resource_total_size: avg_i32(&resource_sizes),
        html_size: avg_i32(&html_sizes),
        css_size: avg_i32(&css_sizes),
        js_size: avg_i32(&js_sizes),
        image_size: avg_i32(&image_sizes),
        font_size: avg_i32(&font_sizes),
        total_requests: avg_i32(&total_reqs),
        failed_requests: avg_i32(&failed_reqs),
        lcp_ms: None,
        cls: None,
        tti_ms: None,
        final_url,
        page_title,
        screenshot_path,
        error_msg: last_error,
        created_at: Utc::now().to_rfc3339(),
        test_count: Some(repeat_count as i32),
    };

    save_website_result(db, &result).await?;
    Ok(result)
}

/// 向量取平均
fn avg(v: &[f64]) -> Option<f64> {
    if v.is_empty() { None } else { Some(v.iter().sum::<f64>() / v.len() as f64) }
}

fn avg_i32(v: &[i32]) -> Option<i32> {
    if v.is_empty() { None } else { Some((v.iter().map(|&x| x as f64).sum::<f64>() / v.len() as f64).round() as i32) }
}

/// 保存网站测试结果到数据库
async fn save_website_result(db: &SqlitePool, result: &WebsiteResult) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT INTO website_result (
            id, task_id, url, dns_time_ms, dns_success, tcp_time_ms, tls_time_ms,
            http_status, ttfb_ms, fp_ms, fcp_ms, dom_content_loaded_ms, load_event_ms,
            page_open_time_ms, first_paint_ms, resource_count, resource_total_size,
            html_size, css_size, js_size, image_size, font_size, total_requests, failed_requests,
            final_url, page_title, screenshot_path, error_msg, test_count, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
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
    .bind(result.html_size)
    .bind(result.css_size)
    .bind(result.js_size)
    .bind(result.image_size)
    .bind(result.font_size)
    .bind(result.total_requests)
    .bind(result.failed_requests)
    .bind(&result.final_url)
    .bind(&result.page_title)
    .bind(&result.screenshot_path)
    .bind(&result.error_msg)
    .bind(result.test_count)
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
    browser_provider: Arc<Box<dyn BrowserProvider>>,
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

        match test_single_video(&db, &config, &browser_provider, task_id, url, timeout).await {
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
                        html_size: None, css_size: None, js_size: None, image_size: None, font_size: None,
                        total_requests: None, failed_requests: None,
                        lcp_ms: None, cls: None, tti_ms: None,
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
    browser_provider: &Arc<Box<dyn BrowserProvider>>,
    task_id: &str,
    url: &str,
    timeout: Duration,
) -> anyhow::Result<VideoResult> {
    let now = Utc::now().to_rfc3339();
    let result_id = Uuid::new_v4().to_string();

    // 视频引擎测试 — 配置驱动平台匹配
    let platform_cfg = match_platform(&config.video_platforms, url);
    let video_engine = VideoEngine::new(browser_provider.clone(), &config.browser.path, config.browser.headless, timeout);
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
        dns_success: Some(if video_result.dns_success { 100 } else { 0 }),
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
            dropped_frames, decoded_frames, screenshot_path, page_title, error_msg, test_count, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
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
    .bind(result.test_count)
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
            dns_time_ms: result.dns_time_ms,
            dns_success: result.dns_success,
            tcp_time_ms: result.tcp_time_ms,
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
        "INSERT INTO download_result (id, task_id, url, dns_time_ms, dns_success, tcp_time_ms, download_speed, avg_speed, peak_speed, download_time_ms, file_size, success, error_msg, test_count, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&result.id).bind(&result.task_id).bind(&result.url)
    .bind(result.dns_time_ms).bind(result.dns_success).bind(result.tcp_time_ms)
    .bind(result.download_speed).bind(result.avg_speed).bind(result.peak_speed)
    .bind(result.download_time_ms).bind(result.file_size).bind(result.success)
    .bind(&result.error_msg).bind(result.test_count).bind(&result.created_at)
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
        "INSERT INTO ping_result (id, task_id, host, avg_latency_ms, packet_loss_rate, jitter_ms, success, error_msg, test_count, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&result.id).bind(&result.task_id).bind(&result.host)
    .bind(result.avg_latency_ms).bind(result.packet_loss_rate).bind(result.jitter_ms)
    .bind(result.success).bind(&result.error_msg).bind(result.test_count).bind(&result.created_at)
    .execute(db).await?;
    Ok(())
}
