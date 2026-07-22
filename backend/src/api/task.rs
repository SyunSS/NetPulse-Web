use axum::extract::{Extension, Path, Query, State};
use axum::http::header;
use axum::response::Response;
use axum::routing::{delete, get, post};
use axum::Json;
use axum::Router;
use serde::{Deserialize, Serialize};

use crate::models::task::{CreateTaskRequest, CreateTaskResponse, DownloadResult, PingResult, TestConfig, TestTask, VideoResult, WebsiteResult};
use crate::services::auth_service::Claims;
use crate::services::task_service::TaskService;
use crate::storage::StorageManager;
use crate::utils::response::{ok, ok_with_msg, AppError, AppState};
/// 构建任务路由
pub fn task_routes() -> Router<AppState> {
    Router::new()
        .route("/create", post(create_task))
        .route("/list", get(list_tasks))
        .route("/:id", get(get_task))
        .route("/:id/logs", get(get_task_logs))
        .route("/:id/result", get(get_task_results))
        .route("/:id/video-result", get(get_video_results))
        .route("/:id/download-result", get(get_download_results))
        .route("/:id/ping-result", get(get_ping_results))
        .route("/:id/export", get(export_result))
        .route("/:id/cancel", post(cancel_task))
        .route("/:id/retry", post(retry_task))
        .route("/:id", delete(delete_task))
        .route("/batch-delete", post(batch_delete_tasks))
        .route("/import", post(import_tasks))
        .route("/template", get(download_template))
}

/// 创建测试任务
async fn create_task(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<Json<crate::utils::response::ApiResponse<CreateTaskResponse>>, AppError> {
    if req.urls.is_empty() {
        return Err(AppError::bad_request("测试URL列表不能为空"));
    }

    let valid_types = ["website", "download", "video"];
    if !valid_types.contains(&req.task_type.as_str()) {
        return Err(AppError::bad_request("无效的任务类型"));
    }

    let resp = TaskService::create_task(&state.db, &state.task_tx, &claims.sub, &req)
        .await
        .map_err(|e| AppError::internal(&e.to_string()))?;

    Ok(Json(ok_with_msg("任务创建成功", resp)))
}

/// 分页查询任务列表
#[derive(Debug, Deserialize)]
struct ListQuery {
    #[serde(default = "default_page")]
    page: u32,
    #[serde(default = "default_size")]
    size: u32,
}

fn default_page() -> u32 {
    1
}
fn default_size() -> u32 {
    20
}

async fn list_tasks(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<ListQuery>,
) -> Result<Json<crate::utils::response::ApiResponse<serde_json::Value>>, AppError> {
    let (tasks, total) = TaskService::list_tasks(&state.db, &claims.sub, query.page, query.size)
        .await
        .map_err(|e| AppError::internal(&e.to_string()))?;

    Ok(Json(ok(serde_json::json!({
        "tasks": tasks,
        "total": total,
        "page": query.page,
        "size": query.size,
    }))))
}

/// 获取任务详情
async fn get_task(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path(task_id): Path<String>,
) -> Result<Json<crate::utils::response::ApiResponse<TestTask>>, AppError> {
    let task = TaskService::get_task(&state.db, &task_id)
        .await
        .map_err(|e| AppError::not_found(&e.to_string()))?;
    Ok(Json(ok(task)))
}

/// 获取网站测试结果
async fn get_task_results(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path(task_id): Path<String>,
) -> Result<Json<crate::utils::response::ApiResponse<Vec<WebsiteResult>>>, AppError> {
    let results = TaskService::get_task_results(&state.db, &task_id)
        .await
        .map_err(|e| AppError::internal(&e.to_string()))?;
    Ok(Json(ok(results)))
}

/// 获取视频测试结果
async fn get_video_results(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path(task_id): Path<String>,
) -> Result<Json<crate::utils::response::ApiResponse<Vec<VideoResult>>>, AppError> {
    let results = TaskService::get_video_results(&state.db, &task_id)
        .await
        .map_err(|e| AppError::internal(&e.to_string()))?;
    Ok(Json(ok(results)))
}

/// 获取下载测试结果
async fn get_download_results(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path(task_id): Path<String>,
) -> Result<Json<crate::utils::response::ApiResponse<Vec<DownloadResult>>>, AppError> {
    let results = TaskService::get_download_results(&state.db, &task_id)
        .await
        .map_err(|e| AppError::internal(&e.to_string()))?;
    Ok(Json(ok(results)))
}

/// 获取 Ping 测试结果
async fn get_ping_results(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path(task_id): Path<String>,
) -> Result<Json<crate::utils::response::ApiResponse<Vec<PingResult>>>, AppError> {
    let results = TaskService::get_ping_results(&state.db, &task_id)
        .await
        .map_err(|e| AppError::internal(&e.to_string()))?;
    Ok(Json(ok(results)))
}

/// 取消任务
async fn cancel_task(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path(task_id): Path<String>,
) -> Result<Json<crate::utils::response::ApiResponse<()>>, AppError> {
    TaskService::cancel_task(&state.db, &state.cancel_tx, &task_id)
        .await
        .map_err(|e| AppError::internal(&e.to_string()))?;
    Ok(Json(ok_with_msg("任务已取消", ())))
}

/// 重试任务
async fn retry_task(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path(task_id): Path<String>,
) -> Result<Json<crate::utils::response::ApiResponse<CreateTaskResponse>>, AppError> {
    let resp = TaskService::retry_task(&state.db, &state.task_tx, &task_id)
        .await
        .map_err(|e| AppError::internal(&e.to_string()))?;
    Ok(Json(ok_with_msg("任务已重新创建", resp)))
}

/// 导出查询参数
#[derive(Debug, Deserialize)]
struct ExportQuery {
    #[serde(default = "default_format")]
    format: String,
}

fn default_format() -> String {
    "xlsx".to_string()
}

/// 导出测试结果
async fn export_result(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path(task_id): Path<String>,
    Query(query): Query<ExportQuery>,
) -> Result<Response, AppError> {
    let task = TaskService::get_task(&state.db, &task_id)
        .await
        .map_err(|e| AppError::not_found(&e.to_string()))?;

    StorageManager::ensure_dir(&state.config.storage.excel_dir)
        .map_err(|e| AppError::internal(&e.to_string()))?;

    let dir = &state.config.storage.excel_dir;

    if query.format == "xlsx" {
        return match task.task_type.as_str() {
            "website" => {
                let data = TaskService::get_task_results(&state.db, &task_id).await
                    .map_err(|e| AppError::internal(&e.to_string()))?;
                let path = crate::report::excel::export_website_xlsx(&data, &task_id, dir)
                    .map_err(|e| AppError::internal(&e.to_string()))?;
                let bytes = std::fs::read(&path).map_err(|e| AppError::internal(&e.to_string()))?;
                Ok(file_response(bytes, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", &format!("website_{}.xlsx", task_id)))
            }
            "video" => {
                let data = TaskService::get_video_results(&state.db, &task_id).await
                    .map_err(|e| AppError::internal(&e.to_string()))?;
                let path = crate::report::excel::export_video_xlsx(&data, &task_id, dir)
                    .map_err(|e| AppError::internal(&e.to_string()))?;
                let bytes = std::fs::read(&path).map_err(|e| AppError::internal(&e.to_string()))?;
                Ok(file_response(bytes, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", &format!("video_{}.xlsx", task_id)))
            }
            "download" => {
                let data = TaskService::get_download_results(&state.db, &task_id).await
                    .map_err(|e| AppError::internal(&e.to_string()))?;
                let path = crate::report::excel::export_download_xlsx(&data, &task_id, dir)
                    .map_err(|e| AppError::internal(&e.to_string()))?;
                let bytes = std::fs::read(&path).map_err(|e| AppError::internal(&e.to_string()))?;
                Ok(file_response(bytes, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", &format!("download_{}.xlsx", task_id)))
            }
            "ping" => {
                let data = TaskService::get_ping_results(&state.db, &task_id).await
                    .map_err(|e| AppError::internal(&e.to_string()))?;
                let path = crate::report::excel::export_ping_xlsx(&data, &task_id, dir)
                    .map_err(|e| AppError::internal(&e.to_string()))?;
                let bytes = std::fs::read(&path).map_err(|e| AppError::internal(&e.to_string()))?;
                Ok(file_response(bytes, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", &format!("ping_{}.xlsx", task_id)))
            }
            _ => Err(AppError::bad_request("不支持的任务类型")),
        };
    }

    match task.task_type.as_str() {
        "website" => {
            let data = TaskService::get_task_results(&state.db, &task_id)
                .await
                .map_err(|e| AppError::internal(&e.to_string()))?;
            export_typed(&data, &task_id, dir, &query.format, "website")
        }
        "video" => {
            let data = TaskService::get_video_results(&state.db, &task_id)
                .await
                .map_err(|e| AppError::internal(&e.to_string()))?;
            export_typed(&data, &task_id, dir, &query.format, "video")
        }
        "download" => {
            let data = TaskService::get_download_results(&state.db, &task_id)
                .await
                .map_err(|e| AppError::internal(&e.to_string()))?;
            export_typed(&data, &task_id, dir, &query.format, "download")
        }
        "ping" => {
            let data = TaskService::get_ping_results(&state.db, &task_id)
                .await
                .map_err(|e| AppError::internal(&e.to_string()))?;
            export_typed(&data, &task_id, dir, &query.format, "ping")
        }
        _ => Err(AppError::bad_request("不支持的任务类型")),
    }
}

fn export_typed<T: Serialize>(
    data: &[T],
    task_id: &str,
    _output_dir: &str,
    format: &str,
    prefix: &str,
) -> Result<Response, AppError> {
    match format {
        "csv" => {
            let mut wtr = csv::Writer::from_writer(Vec::new());
            for row in data {
                wtr.serialize(row).map_err(|e| AppError::internal(&e.to_string()))?;
            }
            let bytes = wtr.into_inner().map_err(|e| AppError::internal(&e.to_string()))?;
            Ok(file_response(bytes, "text/csv", &format!("{}_{}.csv", prefix, task_id)))
        }
        _ => {
            // json 作为默认（也支持 xlsx 但需要具体类型）
            let bytes = serde_json::to_vec_pretty(data)
                .map_err(|e| AppError::internal(&e.to_string()))?;
            Ok(file_response(bytes, "application/json", &format!("{}_{}.json", prefix, task_id)))
        }
    }
}

// ─── 批量导入 ────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ImportRequest { tasks: Vec<serde_json::Value> }

#[derive(Debug, Serialize)]
struct ImportResponse { created: usize, failed: usize, task_ids: Vec<String>, message: String }

async fn import_tasks(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<ImportRequest>,
) -> Result<Json<crate::utils::response::ApiResponse<ImportResponse>>, AppError> {
    let valid = ["ping", "website", "download", "video"];
    if body.tasks.is_empty() { return Err(AppError::bad_request("任务列表不能为空")); }

    let mut created = 0; let mut failed = 0; let mut ids = Vec::new();

    for item in &body.tasks {
        let tt = item.get("task_type").and_then(|v| v.as_str()).unwrap_or("website")
            .trim().trim_matches(|c: char| c == '[' || c == ']' || c.is_whitespace()).to_lowercase();
        if !valid.contains(&tt.as_str()) { failed += 1; continue; }

        let urls: Vec<String> = item.get("urls").and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|u| u.as_str().map(String::from)).collect())
            .unwrap_or_default();
        if urls.is_empty() { failed += 1; continue; }

        let rc = item.get("options").and_then(|o| o.get("repeat_count"))
            .and_then(|v| v.as_u64()).unwrap_or(1) as usize;

        let tid = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let cfg = serde_json::json!({ "urls": &urls, "options": { "repeat_count": rc } });

        if sqlx::query("INSERT INTO test_task (id, user_id, task_type, status, config, progress, created_at) VALUES (?,?,?,'pending',?,0,?)")
            .bind(&tid).bind(&claims.sub).bind(&tt).bind(&cfg.to_string()).bind(&now)
            .execute(&state.db).await.is_err() { failed += 1; continue; }

        ids.push(tid.clone());
        created += 1;

        let _ = state.task_tx.send(crate::utils::response::TaskJob {
            task_id: tid, user_id: claims.sub.clone(), task_type: tt,
            urls, options: serde_json::json!({ "repeat_count": rc }),
        }).await;
    }

    Ok(Json(ok(ImportResponse { created, failed, task_ids: ids,
        message: format!("导入完成: {} 成功, {} 失败", created, failed) })))
}

// ─── 模板下载 ────────────────────────────────────────

#[derive(Debug, Serialize)]
struct Tmpl { version: String, description: String, supported_types: Vec<String>, examples: Vec<TmplEx>, batch_import_format: TmplFmt }
#[derive(Debug, Serialize)]
struct TmplEx { name: String, task_type: String, urls: Vec<String>, #[serde(skip_serializing_if="Option::is_none")] options: Option<serde_json::Value> }
#[derive(Debug, Serialize)]
struct TmplFmt { description: String, json_body: serde_json::Value }

async fn download_template() -> Result<Json<crate::utils::response::ApiResponse<Tmpl>>, AppError> {
    Ok(Json(ok(Tmpl {
        version: "1.0".into(),
        description: "NetPulse 批量测试模板。4 种类型, 可直接导入或参照填写。".into(),
        supported_types: vec![
            "ping    — DNS+TCP 连通性探测".into(),
            "website — 完整网站测速 (DNS→HTTP→浏览器)".into(),
            "download — 下载速度测试".into(),
            "video   — 视频播放测速".into(),
        ],
        examples: vec![
            TmplEx { name: "Ping".into(), task_type: "ping".into(),
                urls: vec!["https://www.baidu.com".into(), "1.1.1.1:443".into()],
                options: Some(serde_json::json!({"repeat_count":3,"_comment":">1 取平均值"})) },
            TmplEx { name: "网站".into(), task_type: "website".into(),
                urls: vec!["https://www.baidu.com".into()],
                options: Some(serde_json::json!({"repeat_count":2})) },
            TmplEx { name: "下载".into(), task_type: "download".into(),
                urls: vec!["http://speedtest.tele2.net/1MB.zip".into()],
                options: Some(serde_json::json!({"repeat_count":2})) },
            TmplEx { name: "视频".into(), task_type: "video".into(),
                urls: vec!["https://www.bilibili.com/video/BV1GJ411x7h7".into()],
                options: Some(serde_json::json!({"repeat_count":1})) },
        ],
        batch_import_format: TmplFmt {
            description: "POST /api/task/import 格式".into(),
            json_body: serde_json::json!({"tasks":[
                {"task_type":"ping","urls":["https://www.baidu.com"],"options":{"repeat_count":1}},
                {"task_type":"website","urls":["https://github.com"],"options":{"repeat_count":3}}
            ]}),
        },
    })))
}

/// 获取任务运行日志
async fn get_task_logs(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path(task_id): Path<String>,
) -> Result<Json<crate::utils::response::ApiResponse<Vec<TaskLogEntry>>>, AppError> {
    use crate::models::task::TaskLog;
    let logs = sqlx::query_as::<_, TaskLog>(
        "SELECT * FROM task_log WHERE task_id = ? ORDER BY created_at ASC"
    ).bind(&task_id).fetch_all(&state.db).await
        .map_err(|e| AppError::internal(&e.to_string()))?;
    let entries: Vec<TaskLogEntry> = logs.iter().map(|l| TaskLogEntry {
        level: l.level.clone(), message: l.message.clone(), created_at: l.created_at.clone(),
    }).collect();
    Ok(Json(ok(entries)))
}

#[derive(Debug, Serialize)]
struct TaskLogEntry { level: String, message: String, created_at: String }

fn file_response(bytes: Vec<u8>, content_type: &str, filename: &str) -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_DISPOSITION, format!("attachment; filename={}", filename))
        .body(axum::body::Body::from(bytes))
        .unwrap()
}

/// 硬删除任务。?force=true 可跳过状态检查
async fn delete_task(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(task_id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<crate::utils::response::ApiResponse<()>>, AppError> {
    let force = params.get("force").map(|s| s.as_str()) == Some("true");
    let task = TaskService::get_task(&state.db, &task_id).await
        .map_err(|_| AppError::not_found("任务不存在"))?;
    if task.user_id != claims.sub {
        return Err(AppError::unauthorized("无权删除"));
    }
    if !force && (task.status == "pending" || task.status == "running") {
        return Err(AppError::bad_request("请先取消运行中的任务再删除"));
    }
    delete_task_data(&state.db, &task_id).await
        .map_err(|e| AppError::internal(&e.to_string()))?;
    Ok(Json(ok_with_msg("任务已删除", ())))
}

async fn delete_task_data(db: &sqlx::SqlitePool, task_id: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM website_result WHERE task_id = ?").bind(task_id).execute(db).await?;
    sqlx::query("DELETE FROM video_result WHERE task_id = ?").bind(task_id).execute(db).await?;
    sqlx::query("DELETE FROM download_result WHERE task_id = ?").bind(task_id).execute(db).await?;
    sqlx::query("DELETE FROM ping_result WHERE task_id = ?").bind(task_id).execute(db).await?;
    sqlx::query("DELETE FROM task_log WHERE task_id = ?").bind(task_id).execute(db).await?;
    sqlx::query("DELETE FROM task_metric_config WHERE task_id = ?").bind(task_id).execute(db).await?;
    sqlx::query("DELETE FROM test_task WHERE id = ?").bind(task_id).execute(db).await?;
    Ok(())
}

/// 批量删除任务
#[derive(Debug, Deserialize)]
struct BatchDeleteRequest { task_ids: Vec<String> }

async fn batch_delete_tasks(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<BatchDeleteRequest>,
) -> Result<Json<crate::utils::response::ApiResponse<serde_json::Value>>, AppError> {
    let mut deleted = 0usize;
    for tid in &body.task_ids {
        let task = match TaskService::get_task(&state.db, tid).await {
            Ok(t) => t,
            Err(_) => continue,
        };
        if task.user_id != claims.sub { continue; }
        if task.status == "pending" || task.status == "running" { continue; }
        delete_task_data(&state.db, tid).await.ok();
        deleted += 1;
    }
    Ok(Json(ok(serde_json::json!({"deleted": deleted, "total": body.task_ids.len()}))))
}
