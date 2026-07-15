use axum::{extract::State, routing::post, Json, Router};

use crate::models::user::{LoginRequest, RegisterRequest};
use crate::services::auth_service::AuthService;
use crate::utils::response::{ok, ok_with_msg, AppError, AppState};

/// 构建认证路由
pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
}

/// 用户注册
async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<crate::utils::response::ApiResponse<crate::models::user::UserInfo>>, AppError> {
    let user = AuthService::register(&state.db, &req)
        .await
        .map_err(|e| AppError::bad_request(&e.to_string()))?;
    Ok(Json(ok_with_msg("注册成功", user)))
}

/// 用户登录
async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<crate::utils::response::ApiResponse<crate::models::user::LoginResponse>>, AppError> {
    let resp = AuthService::login(&state.db, &state.config, &req.username, &req.password)
        .await
        .map_err(|e| AppError::bad_request(&e.to_string()))?;
    Ok(Json(ok(resp)))
}
