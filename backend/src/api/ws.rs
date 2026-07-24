use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use serde::Deserialize;
use tracing::{info, warn};

use crate::services::auth_service::{AuthService, Claims};
use crate::utils::response::{AppError, AppState, ProgressMessage};

/// 构建 WebSocket 路由
pub fn ws_routes() -> Router<AppState> {
    Router::new().route("/", get(ws_handler))
}

#[derive(Debug, Deserialize)]
struct WsQuery {
    /// 可选：直接订阅指定 task_id
    task_id: Option<String>,
    /// JWT token（用于认证）
    token: Option<String>,
}

/// WebSocket 连接处理
async fn ws_handler(
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    // 验证 token
    let _claims: Claims = match &query.token {
        Some(token) => {
            AuthService::verify_token(&state.config, token)
                .map_err(|_| AppError::unauthorized("Token 无效或已过期"))?
        }
        None => {
            return Err(AppError::unauthorized("缺少认证 Token"));
        }
    };
    Ok(ws.on_upgrade(move |socket| handle_ws(socket, state, query.task_id)))
}

/// 处理 WebSocket 连接
async fn handle_ws(mut socket: WebSocket, state: AppState, initial_task_id: Option<String>) {
    info!("WebSocket 连接已建立, 初始订阅: {:?}", initial_task_id);

    // 订阅进度广播
    let mut rx = state.progress_tx.subscribe();

    // 如果有初始 task_id，发送订阅确认
    if let Some(task_id) = &initial_task_id {
        let msg = serde_json::json!({
            "type": "subscribed",
            "task_id": task_id,
        });
        if socket.send(Message::Text(msg.to_string())).await.is_err() {
            return;
        }
    }

    // 双向消息处理
    loop {
        tokio::select! {
            // 接收广播消息并转发
            msg = rx.recv() => {
                match msg {
                    Ok(progress) => {
                        let json = serde_json::to_string(&progress).unwrap_or_default();

                        // 如果有初始 task_id，只转发该 task 的消息
                        let should_send = match &initial_task_id {
                            Some(filter_id) => {
                                let task_id = match &progress {
                                    ProgressMessage::TaskStarted { task_id, .. } => task_id,
                                    ProgressMessage::UrlTesting { task_id, .. } => task_id,
                                    ProgressMessage::UrlCompleted { task_id, .. } => task_id,
                                    ProgressMessage::ProgressUpdate { task_id, .. } => task_id,
                                    ProgressMessage::Log { task_id, .. } => task_id,
                                    ProgressMessage::TaskCompleted { task_id, .. } => task_id,
                                    ProgressMessage::TaskFailed { task_id, .. } => task_id,
                                };
                                task_id == filter_id
                            }
                            None => true,
                        };

                        if should_send {
                            if socket.send(Message::Text(json)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        info!("广播通道已关闭");
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("WebSocket 消息滞后: {} 条", n);
                    }
                }
            }
            // 接收客户端消息
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // 客户端可以发送订阅消息
                        info!("WebSocket 收到消息: {}", text);
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket 连接关闭");
                        break;
                    }
                    Some(Ok(_)) => {} // 忽略其他类型
                    Some(Err(e)) => {
                        warn!("WebSocket 错误: {}", e);
                        break;
                    }
                    None => {
                        info!("WebSocket 连接断开");
                        break;
                    }
                }
            }
        }
    }

    info!("WebSocket 处理结束");
}
