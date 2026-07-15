use axum::extract::{Extension, Path, Query, State};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use serde::Deserialize;

use crate::models::plan::{
    CreatePlanRequest, PlanListResponse, PlanWithItems, RunPlanResponse, TaskPlanRun,
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
) -> Result<Json<crate::utils::response::ApiResponse<Vec<TaskPlanRun>>>, AppError> {
    let runs = PlanService::list_plan_runs(&state.db, &plan_id, q.limit.unwrap_or(20))
        .await
        .map_err(|e| AppError::internal(&e.to_string()))?;
    Ok(Json(ok(runs)))
}

#[derive(Debug, Deserialize)]
struct RunListQuery {
    limit: Option<u32>,
}
