use axum::extract::{Extension, Path, State};
use axum::routing::{delete, get, post};
use axum::Json;
use axum::Router;
use serde::Deserialize;

use crate::models::user::User;
use crate::services::auth_service::Claims;
use crate::utils::response::{ok, ok_with_msg, AppError, AppState};

pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users))
        .route("/users/role", post(update_user_role))
        .route("/users/:id", delete(delete_user))
}

/// 列出所有用户（仅 admin）
async fn list_users(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<crate::utils::response::ApiResponse<Vec<User>>>, AppError> {
    if claims.role != "admin" {
        return Err(AppError::unauthorized("无权限"));
    }
    let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at ASC")
        .fetch_all(&state.db)
        .await
        .map_err(|e| AppError::internal(&e.to_string()))?;

    let safe_users: Vec<User> = users.into_iter().map(|u| User { password_hash: String::new(), ..u }).collect();
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
    if claims.role != "admin" {
        return Err(AppError::unauthorized("无权限"));
    }
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
    if claims.role != "admin" {
        return Err(AppError::unauthorized("无权限"));
    }
    // 不能删除自己
    if user_id == claims.sub {
        return Err(AppError::bad_request("不能删除自己"));
    }
    // 检查是否是最后一个 admin
    let admin_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'admin'")
        .fetch_one(&state.db).await.map_err(|e| AppError::internal(&e.to_string()))?;
    let target_role: String = sqlx::query_scalar("SELECT role FROM users WHERE id = ?")
        .bind(&user_id).fetch_optional(&state.db).await
        .map_err(|e| AppError::internal(&e.to_string()))?
        .unwrap_or_default();
    if admin_count <= 1 && target_role == "admin" {
        return Err(AppError::bad_request("不能删除最后一个管理员"));
    }

    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(&user_id).execute(&state.db).await
        .map_err(|e| AppError::internal(&e.to_string()))?;

    Ok(Json(ok_with_msg("用户已删除", ())))
}
