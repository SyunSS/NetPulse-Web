pub mod auth;
pub mod task;
pub mod ws;

use axum::{
    extract::{Extension, State},
    middleware,
    routing::get,
    Router,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use tracing::info;

use crate::services::auth_service::Claims;
use crate::services::task_service::TaskService;
use crate::utils::response::{ok, AppError, AppState};

/// 构建所有 API 路由
pub fn build_router(state: AppState) -> Router {
    // 需要认证的路由
    let protected_routes = Router::new()
        .nest("/api/task", task::task_routes())
        .route("/api/dashboard/stats", get(dashboard_stats))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    // 公开路由（无需认证）
    let public_routes = Router::new()
        .route("/api/health", get(health_check))
        .nest("/api/auth", auth::auth_routes())
        .nest("/api/ws", ws::ws_routes());

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(state)
}

/// 健康检查
async fn health_check() -> axum::Json<crate::utils::response::ApiResponse<serde_json::Value>> {
    axum::Json(ok(serde_json::json!({
        "status": "ok",
        "service": "NetPulse Web",
        "version": env!("CARGO_PKG_VERSION"),
    })))
}

/// Dashboard 统计数据
async fn dashboard_stats(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<axum::Json<crate::utils::response::ApiResponse<crate::services::task_service::DashboardStats>>, AppError> {
    let stats = TaskService::get_dashboard_stats(&state.db, &claims.sub)
        .await
        .map_err(|e| AppError::internal(&e.to_string()))?;
    Ok(axum::Json(ok(stats)))
}

/// JWT 认证中间件
async fn auth_middleware(
    State(state): State<AppState>,
    mut request: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> Result<axum::response::Response, AppError> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match auth_header {
        Some(token) => {
            match decode::<Claims>(
                token,
                &DecodingKey::from_secret(state.config.jwt.secret.as_bytes()),
                &Validation::default(),
            ) {
                Ok(token_data) => {
                    request.extensions_mut().insert(token_data.claims);
                    Ok(next.run(request).await)
                }
                Err(e) => {
                    info!("JWT 验证失败: {}", e);
                    Err(AppError::unauthorized("Token 无效或已过期"))
                }
            }
        }
        None => Err(AppError::unauthorized("缺少认证 Token")),
    }
}
