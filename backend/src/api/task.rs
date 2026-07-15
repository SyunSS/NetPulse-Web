use axum::extract::{Extension, Path, Query, State};
use axum::http::header;
use axum::response::Response;
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use serde::Deserialize;

use crate::models::task::{CreateTaskRequest, CreateTaskResponse, DownloadResult, PingResult, TestTask, VideoResult, WebsiteResult};
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
        .route("/:id/result", get(get_task_results))
        .route("/:id/video-result", get(get_video_results))
        .route("/:id/download-result", get(get_download_results))
        .route("/:id/ping-result", get(get_ping_results))
        .route("/:id/export", get(export_result))
        .route("/:id/cancel", post(cancel_task))
        .route("/:id/retry", post(retry_task))
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
    TaskService::cancel_task(&state.db, &task_id)
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
        _ => Err(AppError::bad_request("不支持的任务类型")),
    }
}

use serde::Serialize;

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

fn file_response(bytes: Vec<u8>, content_type: &str, filename: &str) -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_DISPOSITION, format!("attachment; filename={}", filename))
        .body(axum::body::Body::from(bytes))
        .unwrap()
}
