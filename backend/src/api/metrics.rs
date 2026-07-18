use axum::extract::{Extension, Path, State};
use axum::routing::get;
use axum::Json;
use axum::Router;

use crate::models::metrics::MetricDefinition;
use crate::services::auth_service::Claims;
use crate::utils::response::{ok, AppError, AppState};

pub fn metrics_routes() -> Router<AppState> {
    Router::new()
        .route("/metrics", get(list_metrics))
}

/// 获取所有可用指标定义
async fn list_metrics(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<crate::utils::response::ApiResponse<Vec<MetricDefinition>>>, AppError> {
    let metrics = sqlx::query_as::<_, MetricDefinition>(
        "SELECT * FROM metric_definition ORDER BY category, name"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::internal(&e.to_string()))?;
    Ok(Json(ok(metrics)))
}
