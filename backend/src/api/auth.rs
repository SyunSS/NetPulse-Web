use axum::http::HeaderMap;
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

fn client_ip(headers: &HeaderMap) -> String {
    headers
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim())
        .or_else(|| {
            headers
                .get("X-Real-IP")
                .and_then(|v| v.to_str().ok())
        })
        .unwrap_or("unknown")
        .to_string()
}

/// 用户注册
async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<crate::utils::response::ApiResponse<crate::models::user::UserInfo>>, AppError> {
    let ip = client_ip(&headers);
    if !state.rate_limiter.check(&format!("register:{}", ip)) {
        return Err(AppError::bad_request("请求过于频繁，请稍后再试"));
    }
    let user = AuthService::register(&state.db, &req)
        .await
        .map_err(|e| AppError::bad_request(&e.to_string()))?;
    Ok(Json(ok_with_msg("注册成功", user)))
}

/// 用户登录
async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> Result<Json<crate::utils::response::ApiResponse<crate::models::user::LoginResponse>>, AppError> {
    let ip = client_ip(&headers);
    if !state.rate_limiter.check(&format!("login:{}", ip)) {
        return Err(AppError::bad_request("请求过于频繁，请稍后再试"));
    }
    let resp = AuthService::login(&state.db, &state.config, &req.username, &req.password)
        .await
        .map_err(|e| AppError::bad_request(&e.to_string()))?;
    Ok(Json(ok(resp)))
}
