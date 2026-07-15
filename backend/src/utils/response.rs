use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};

use crate::config::AppConfig;
use crate::models::task::WebsiteResult;

/// 统一 API 响应格式
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub code: i32,
    pub msg: String,
    pub data: Option<T>,
}

/// 成功响应
pub fn ok<T: Serialize>(data: T) -> ApiResponse<T> {
    ApiResponse {
        code: 0,
        msg: "ok".to_string(),
        data: Some(data),
    }
}

/// 成功响应（无数据）
pub fn ok_empty() -> ApiResponse<()> {
    ApiResponse {
        code: 0,
        msg: "ok".to_string(),
        data: None,
    }
}

/// 成功响应（带自定义消息）
pub fn ok_with_msg<T: Serialize>(msg: &str, data: T) -> ApiResponse<T> {
    ApiResponse {
        code: 0,
        msg: msg.to_string(),
        data: Some(data),
    }
}

/// 错误响应
pub fn api_error(code: i32, msg: &str) -> ApiResponse<()> {
    ApiResponse {
        code,
        msg: msg.to_string(),
        data: None,
    }
}

/// 应用错误类型
#[derive(Debug)]
pub struct AppError {
    pub code: i32,
    pub msg: String,
    pub status: StatusCode,
}

impl AppError {
    pub fn new(code: i32, msg: &str) -> Self {
        Self {
            code,
            msg: msg.to_string(),
            status: StatusCode::OK,
        }
    }

    pub fn with_status(code: i32, msg: &str, status: StatusCode) -> Self {
        Self {
            code,
            msg: msg.to_string(),
            status,
        }
    }

    pub fn bad_request(msg: &str) -> Self {
        Self::new(1000, msg)
    }

    pub fn unauthorized(msg: &str) -> Self {
        Self::with_status(1001, msg, StatusCode::UNAUTHORIZED)
    }

    pub fn not_found(msg: &str) -> Self {
        Self::with_status(1002, msg, StatusCode::NOT_FOUND)
    }

    pub fn internal(msg: &str) -> Self {
        Self::with_status(5000, msg, StatusCode::INTERNAL_SERVER_ERROR)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.msg)
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let body = Json(api_error(self.code, &self.msg));
        (self.status, body).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::internal(&e.to_string())
    }
}

/// 将 ApiResponse 直接转为 HTTP 响应
impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}

// ===== 消息系统 =====

/// 应用状态（共享状态）
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub config: AppConfig,
    pub task_tx: mpsc::Sender<TaskJob>,
    pub progress_tx: broadcast::Sender<ProgressMessage>,
}

/// 任务作业 — 通过 channel 发送给 Worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskJob {
    pub task_id: String,
    pub user_id: String,
    pub task_type: String,
    pub urls: Vec<String>,
    pub options: serde_json::Value,
}

/// 进度消息 — 通过 broadcast 推送给 WebSocket 客户端
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProgressMessage {
    TaskStarted {
        task_id: String,
        total_urls: usize,
    },
    UrlTesting {
        task_id: String,
        url: String,
        current: usize,
        total: usize,
    },
    UrlCompleted {
        task_id: String,
        url: String,
        result: WebsiteResult,
    },
    ProgressUpdate {
        task_id: String,
        progress: f64,
    },
    Log {
        task_id: String,
        level: String,
        message: String,
    },
    TaskCompleted {
        task_id: String,
        success_count: usize,
        fail_count: usize,
    },
    TaskFailed {
        task_id: String,
        error: String,
    },
}
