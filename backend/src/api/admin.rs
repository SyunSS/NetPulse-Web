use axum::extract::{Extension, Path, State};
use axum::http::{HeaderMap, HeaderValue};
use axum::routing::{delete, get, post};
use axum::Json;
use axum::Router;
use serde::{Deserialize, Serialize};

use crate::models::user::User;
use crate::services::auth_service::Claims;
use crate::services::video_cookie_service::{VideoCookieBackup, VideoCookieService, VideoCookieStatus};
use crate::utils::response::{ok, ok_with_msg, AppError, AppState};

pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users))
        .route("/users/role", post(update_user_role))
        .route("/users/:id", delete(delete_user))
        .route("/video-cookies", get(list_video_cookies))
        .route("/video-cookies/import", post(import_video_cookies))
        .route("/video-cookies/import-backup", post(import_video_cookie_backup))
        .route("/video-cookies/:platform/export", get(export_video_cookie_backup))
        .route("/video-cookies/:platform", delete(delete_video_cookies))
}

fn ensure_admin(claims: &Claims) -> Result<(), AppError> {
    if claims.role != "admin" {
        return Err(AppError::unauthorized("无权限"));
    }
    Ok(())
}

/// 列出所有用户（仅 admin）
async fn list_users(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<crate::utils::response::ApiResponse<Vec<User>>>, AppError> {
    ensure_admin(&claims)?;
    let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at ASC")
        .fetch_all(&state.db)
        .await
        .map_err(|e| AppError::internal(&e.to_string()))?;

    let safe_users: Vec<User> = users
        .into_iter()
        .map(|u| User {
            password_hash: String::new(),
            ..u
        })
        .collect();
    Ok(Json(ok(safe_users)))
}

/// 修改用户权限（仅 admin）
#[derive(Debug, Deserialize)]
struct UpdateRoleRequest {
    user_id: String,
    role: String,
}

async fn update_user_role(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<UpdateRoleRequest>,
) -> Result<Json<crate::utils::response::ApiResponse<()>>, AppError> {
    ensure_admin(&claims)?;
    if !["admin", "user"].contains(&req.role.as_str()) {
        return Err(AppError::bad_request("无效角色"));
    }
    sqlx::query("UPDATE users SET role = ? WHERE id = ?")
        .bind(&req.role)
        .bind(&req.user_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::internal(&e.to_string()))?;

    Ok(Json(ok_with_msg("权限已更新", ())))
}

/// 删除用户（仅 admin，不能删自己，不能删最后一个 admin）
async fn delete_user(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(user_id): Path<String>,
) -> Result<Json<crate::utils::response::ApiResponse<()>>, AppError> {
    ensure_admin(&claims)?;
    // 不能删除自己
    if user_id == claims.sub {
        return Err(AppError::bad_request("不能删除自己"));
    }
    // 检查是否是最后一个 admin
    let admin_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'admin'")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::internal(&e.to_string()))?;
    let target_role: String = sqlx::query_scalar("SELECT role FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| AppError::internal(&e.to_string()))?
        .unwrap_or_default();
    if admin_count <= 1 && target_role == "admin" {
        return Err(AppError::bad_request("不能删除最后一个管理员"));
    }

    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(&user_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::internal(&e.to_string()))?;

    Ok(Json(ok_with_msg("用户已删除", ())))
}

#[derive(Debug, Serialize)]
struct VideoCookieListResponse {
    platforms: Vec<String>,
    cookies: Vec<VideoCookieStatus>,
}

async fn list_video_cookies(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<crate::utils::response::ApiResponse<VideoCookieListResponse>>, AppError> {
    ensure_admin(&claims)?;
    let cookies = VideoCookieService::list(&state.db)
        .await
        .map_err(|e| AppError::internal(&e.to_string()))?;
    Ok(Json(ok(VideoCookieListResponse {
        platforms: VideoCookieService::supported_platforms()
            .into_iter()
            .map(str::to_string)
            .collect(),
        cookies,
    })))
}

#[derive(Debug, Deserialize)]
struct ImportVideoCookiesRequest {
    platform: String,
    cookies: serde_json::Value,
}

async fn import_video_cookies(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<ImportVideoCookiesRequest>,
) -> Result<Json<crate::utils::response::ApiResponse<VideoCookieStatus>>, AppError> {
    ensure_admin(&claims)?;
    let status = VideoCookieService::import_json(
        &state.db,
        &state.config.storage.secure_dir,
        &req.platform,
        req.cookies,
        &claims.sub,
    )
    .await
    .map_err(|e| AppError::bad_request(&e.to_string()))?;
    Ok(Json(ok_with_msg("视频 Cookie 已导入", status)))
}

async fn export_video_cookie_backup(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(platform): Path<String>,
) -> Result<(HeaderMap, Json<crate::utils::response::ApiResponse<VideoCookieBackup>>), AppError> {
    ensure_admin(&claims)?;
    let backup = VideoCookieService::export_backup(&state.db, &platform)
        .await
        .map_err(|e| AppError::bad_request(&e.to_string()))?;
    let mut headers = HeaderMap::new();
    headers.insert("cache-control", HeaderValue::from_static("no-store"));
    Ok((headers, Json(ok(backup))))
}

async fn import_video_cookie_backup(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(backup): Json<VideoCookieBackup>,
) -> Result<Json<crate::utils::response::ApiResponse<VideoCookieStatus>>, AppError> {
    ensure_admin(&claims)?;
    let status = VideoCookieService::import_backup(
        &state.db,
        &state.config.storage.secure_dir,
        backup,
        &claims.sub,
    )
    .await
    .map_err(|e| AppError::bad_request(&e.to_string()))?;
    Ok(Json(ok_with_msg("视频 Cookie 备份已导入", status)))
}

async fn delete_video_cookies(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(platform): Path<String>,
) -> Result<Json<crate::utils::response::ApiResponse<()>>, AppError> {
    ensure_admin(&claims)?;
    let deleted = VideoCookieService::delete(&state.db, &platform)
        .await
        .map_err(|e| AppError::bad_request(&e.to_string()))?;
    if !deleted {
        return Err(AppError::not_found("平台尚未导入 Cookie"));
    }
    Ok(Json(ok_with_msg("视频 Cookie 已删除", ())))
}
