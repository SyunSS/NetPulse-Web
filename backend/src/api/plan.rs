use axum::extract::{Extension, Path, Query, State};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use serde::Deserialize;

use crate::models::plan::{
    CreatePlanRequest, PlanListResponse, PlanWithItems, PlanRunWithTasks, RunPlanResponse,
    UpdatePlanRequest,
};
use crate::services::auth_service::Claims;
use crate::services::plan_service::PlanService;
use crate::utils::response::{ok, ok_with_msg, AppError, AppState};

/// 构建计划路由
pub fn plan_routes() -> Router<AppState> {
    Router::new()
        .route("/list", get(list_plans))
        .route("/create", post(create_plan))
        .route("/:id", get(get_plan))
        .route("/:id/update", post(update_plan))
        .route("/:id/delete", post(delete_plan))
        .route("/:id/run", post(run_plan))
        .route("/:id/runs", get(list_plan_runs))
        .route("/:id/run/:run_id/export", get(export_plan_run))
}

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

/// 列出我的计划
async fn list_plans(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ListQuery>,
) -> Result<Json<crate::utils::response::ApiResponse<PlanListResponse>>, AppError> {
    let (plans, total) = PlanService::list_plans(&state.db, &claims.sub, q.page, q.size)
        .await
        .map_err(|e| AppError::internal(&e.to_string()))?;
    Ok(Json(ok(PlanListResponse {
        plans,
        total,
        page: q.page,
        size: q.size,
    })))
}

/// 创建计划
async fn create_plan(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreatePlanRequest>,
) -> Result<Json<crate::utils::response::ApiResponse<PlanWithItems>>, AppError> {
    if req.name.trim().is_empty() {
        return Err(AppError::bad_request("计划名不能为空"));
    }
    if req.items.is_empty() {
        return Err(AppError::bad_request("计划至少需要包含一个测试项"));
    }
    let plan = PlanService::create_plan(&state.db, &state.task_tx, &claims.sub, req)
        .await
        .map_err(|e| AppError::bad_request(&e.to_string()))?;
    Ok(Json(ok_with_msg("计划创建成功", plan)))
}

/// 获取计划详情
async fn get_plan(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path(plan_id): Path<String>,
) -> Result<Json<crate::utils::response::ApiResponse<PlanWithItems>>, AppError> {
    let plan = PlanService::get_plan(&state.db, &plan_id)
        .await
        .map_err(|e| AppError::not_found(&e.to_string()))?;
    Ok(Json(ok(plan)))
}

/// 更新计划
async fn update_plan(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(plan_id): Path<String>,
    Json(req): Json<UpdatePlanRequest>,
) -> Result<Json<crate::utils::response::ApiResponse<PlanWithItems>>, AppError> {
    if req.name.trim().is_empty() {
        return Err(AppError::bad_request("计划名不能为空"));
    }
    if req.items.is_empty() {
        return Err(AppError::bad_request("计划至少需要包含一个测试项"));
    }
    let plan = PlanService::update_plan(&state.db, &claims.sub, &plan_id, req)
        .await
        .map_err(|e| AppError::bad_request(&e.to_string()))?;
    Ok(Json(ok_with_msg("计划更新成功", plan)))
}

/// 删除计划
async fn delete_plan(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(plan_id): Path<String>,
) -> Result<Json<crate::utils::response::ApiResponse<()>>, AppError> {
    PlanService::delete_plan(&state.db, &claims.sub, &plan_id)
        .await
        .map_err(|e| AppError::bad_request(&e.to_string()))?;
    Ok(Json(ok_with_msg("计划已删除", ())))
}

/// 立即运行计划
async fn run_plan(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(plan_id): Path<String>,
) -> Result<Json<crate::utils::response::ApiResponse<RunPlanResponse>>, AppError> {
    let resp = PlanService::run_now(
        &state.db,
        &state.task_tx,
        &state.progress_tx,
        &claims.sub,
        &plan_id,
    )
    .await
    .map_err(|e| AppError::bad_request(&e.to_string()))?;
    Ok(Json(ok_with_msg("计划已派发", resp)))
}

/// 列出计划运行历史
async fn list_plan_runs(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path(plan_id): Path<String>,
    Query(q): Query<RunListQuery>,
) -> Result<Json<crate::utils::response::ApiResponse<Vec<PlanRunWithTasks>>>, AppError> {
    let runs = PlanService::list_plan_runs(&state.db, &plan_id, q.limit.unwrap_or(20))
        .await
        .map_err(|e| AppError::internal(&e.to_string()))?;
    Ok(Json(ok(runs)))
}

#[derive(Debug, Deserialize)]
struct RunListQuery {
    limit: Option<u32>,
}

/// 导出计划运行的全部 task 结果（合并到 1 个文件）
async fn export_plan_run(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path((plan_id, run_id)): Path<(String, String)>,
    Query(q): Query<ExportQuery>,
) -> Result<axum::response::Response, AppError> {
    // 获取 plan_run 及其 task_ids
    let run = sqlx::query_as::<_, crate::models::plan::TaskPlanRun>(
        "SELECT * FROM task_plan_runs WHERE id = ? AND plan_id = ?",
    )
    .bind(&run_id)
    .bind(&plan_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::internal(&e.to_string()))?
    .ok_or_else(|| AppError::not_found("运行记录不存在"))?;

    let task_ids: Vec<String> = serde_json::from_str(&run.task_ids).unwrap_or_default();

    // 收集所有 task 的类型 + 结果
    let mut website_data = Vec::new();
    let mut video_data = Vec::new();
    let mut download_data = Vec::new();
    let mut task_summaries = Vec::new();

    for tid in &task_ids {
        // 查 task 类型
        let task_row: Option<(String, String)> = sqlx::query_as(
            "SELECT task_type, status FROM test_task WHERE id = ?",
        )
        .bind(tid)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| AppError::internal(&e.to_string()))?;

        if let Some((task_type, status)) = task_row {
            // 查每种类型的 url（用测试结果的 url 字段）
            let url: String = match task_type.as_str() {
                "website" => sqlx::query_scalar::<_, String>(
                    "SELECT url FROM website_result WHERE task_id = ? LIMIT 1",
                )
                .bind(tid)
                .fetch_optional(&state.db)
                .await
                .map_err(|e| AppError::internal(&e.to_string()))?
                .unwrap_or_default(),
                "video" => sqlx::query_scalar::<_, String>(
                    "SELECT url FROM video_result WHERE task_id = ? LIMIT 1",
                )
                .bind(tid)
                .fetch_optional(&state.db)
                .await
                .map_err(|e| AppError::internal(&e.to_string()))?
                .unwrap_or_default(),
                "download" => sqlx::query_scalar::<_, String>(
                    "SELECT url FROM download_result WHERE task_id = ? LIMIT 1",
                )
                .bind(tid)
                .fetch_optional(&state.db)
                .await
                .map_err(|e| AppError::internal(&e.to_string()))?
                .unwrap_or_default(),
                _ => String::new(),
            };

            task_summaries.push(crate::report::excel::TaskSummary {
                task_id: tid.clone(),
                task_type: task_type.clone(),
                status: status.clone(),
                url,
            });

            // 查结果
            match task_type.as_str() {
                "website" => {
                    let results: Vec<crate::models::task::WebsiteResult> = sqlx::query_as(
                        "SELECT * FROM website_result WHERE task_id = ?",
                    )
                    .bind(tid)
                    .fetch_all(&state.db)
                    .await
                    .map_err(|e| AppError::internal(&e.to_string()))?;
                    website_data.extend(results);
                }
                "video" => {
                    let results: Vec<crate::models::task::VideoResult> = sqlx::query_as(
                        "SELECT * FROM video_result WHERE task_id = ?",
                    )
                    .bind(tid)
                    .fetch_all(&state.db)
                    .await
                    .map_err(|e| AppError::internal(&e.to_string()))?;
                    video_data.extend(results);
                }
                "download" => {
                    let results: Vec<crate::models::task::DownloadResult> = sqlx::query_as(
                        "SELECT * FROM download_result WHERE task_id = ?",
                    )
                    .bind(tid)
                    .fetch_all(&state.db)
                    .await
                    .map_err(|e| AppError::internal(&e.to_string()))?;
                    download_data.extend(results);
                }
                _ => {}
            }
        }
    }

    // 拼装响应
    let body = match q.format.as_str() {
        "xlsx" => {
            use crate::report::excel;
            let dir = &state.config.storage.excel_dir;
            let path = excel::export_plan_run_xlsx(
                &task_summaries, &website_data, &video_data, &download_data,
                &plan_id, &run_id, dir,
            )
            .map_err(|e| AppError::internal(&e.to_string()))?;
            std::fs::read(&path).map_err(|e| AppError::internal(&e.to_string()))?
        }
        "csv" => {
            let mut buf = Vec::new();
            {
                let mut wtr = csv::Writer::from_writer(&mut buf);
                wtr.write_record(&["task_type", "task_id", "status", "url", "extra"])
                    .map_err(|e| AppError::internal(&e.to_string()))?;
                for s in &task_summaries {
                    wtr.write_record(&[&s.task_type, &s.task_id, &s.status, &s.url, ""])
                        .map_err(|e| AppError::internal(&e.to_string()))?;
                }
                wtr.flush().map_err(|e| AppError::internal(&e.to_string()))?;
            }
            buf
        }
        _ => {
            // 默认 JSON
            serde_json::to_vec_pretty(&serde_json::json!({
                "plan_id": plan_id,
                "run_id": run_id,
                "tasks": task_summaries,
                "website_results": website_data,
                "video_results": video_data,
                "download_results": download_data,
            }))
            .map_err(|e| AppError::internal(&e.to_string()))?
        }
    };

    let ext = match q.format.as_str() {
        "xlsx" => "xlsx",
        "csv" => "csv",
        _ => "json",
    };
    let mime = match q.format.as_str() {
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "csv" => "text/csv",
        _ => "application/json",
    };
    let run_id_short: String = run_id.chars().take(8).collect();
    let filename = format!("plan_run_{}.{}", run_id_short, ext);

    Ok(axum::response::Response::builder()
        .header(axum::http::header::CONTENT_TYPE, mime)
        .header(axum::http::header::CONTENT_DISPOSITION, format!("attachment; filename={}", filename))
        .body(axum::body::Body::from(body))
        .unwrap())
}

#[derive(Debug, Deserialize)]
struct ExportQuery {
    #[serde(default = "default_export_format")]
    format: String,
}

fn default_export_format() -> String {
    "json".to_string()
}
