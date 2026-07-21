use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 测试任务模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TestTask {
    pub id: String,
    pub user_id: String,
    pub task_type: String,
    pub status: String,
    pub config: String,
    pub progress: Option<f64>,
    pub result: Option<String>,
    pub error_msg: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

/// 任务类型枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    Website,
    Download,
    Video,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::Website => "website",
            TaskType::Download => "download",
            TaskType::Video => "video",
        }
    }
}

/// 任务状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
        }
    }
}

/// 创建任务请求
#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub task_type: String,
    pub urls: Vec<String>,
    #[serde(default)]
    pub options: serde_json::Value,
}

/// 创建任务响应
#[derive(Debug, Serialize)]
pub struct CreateTaskResponse {
    pub task_id: String,
    pub status: String,
}

/// 测试配置（从 options 解析，含重复次数）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    #[serde(default = "default_repeat_count")]
    pub repeat_count: usize,
}
fn default_repeat_count() -> usize { 1 }
impl Default for TestConfig {
    fn default() -> Self { Self { repeat_count: 1 } }
}

/// 网站测试结果模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WebsiteResult {
    pub id: String,
    pub task_id: String,
    pub url: String,
    pub dns_time_ms: Option<f64>,
    pub dns_success: Option<i32>,
    pub tcp_time_ms: Option<f64>,
    pub tls_time_ms: Option<f64>,
    pub http_status: Option<i32>,
    pub ttfb_ms: Option<f64>,
    pub fp_ms: Option<f64>,
    pub fcp_ms: Option<f64>,
    pub dom_content_loaded_ms: Option<f64>,
    pub load_event_ms: Option<f64>,
    pub page_open_time_ms: Option<f64>,
    pub first_paint_ms: Option<f64>,
    pub resource_count: Option<i32>,
    pub resource_total_size: Option<i32>,
    pub html_size: Option<i32>,
    pub css_size: Option<i32>,
    pub js_size: Option<i32>,
    pub image_size: Option<i32>,
    pub font_size: Option<i32>,
    pub total_requests: Option<i32>,
    pub failed_requests: Option<i32>,
    pub lcp_ms: Option<f64>,
    pub cls: Option<f64>,
    pub tti_ms: Option<f64>,
    pub site_size_kb: Option<f64>,
    pub avg_speed_kbps: Option<f64>,
    pub total_speed_kbps: Option<f64>,
    pub first_screen_ratio: Option<f64>,
    pub final_url: Option<String>,
    pub page_title: Option<String>,
    pub screenshot_path: Option<String>,
    pub error_msg: Option<String>,
    pub test_count: Option<i32>,
    pub created_at: String,
}

/// 下载测试结果模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DownloadResult {
    pub id: String,
    pub task_id: String,
    pub url: String,
    pub dns_time_ms: Option<f64>,
    pub dns_success: Option<i32>,
    pub tcp_time_ms: Option<f64>,
    pub download_speed: Option<f64>,
    pub avg_speed: Option<f64>,
    pub peak_speed: Option<f64>,
    pub download_time_ms: Option<f64>,
    pub file_size: Option<i32>,
    pub success: Option<i32>,
    pub error_msg: Option<String>,
    pub test_count: Option<i32>,
    pub created_at: String,
}

/// 视频测试结果模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VideoResult {
    pub id: String,
    pub task_id: String,
    pub url: String,
    pub platform: Option<String>,
    pub dns_time_ms: Option<f64>,
    pub dns_success: Option<i32>,
    pub tcp_time_ms: Option<f64>,
    pub http_response_ms: Option<f64>,
    pub first_play_time_ms: Option<f64>,
    pub buffer_count: Option<i32>,
    pub total_buffer_time_ms: Option<f64>,
    pub buffer_rate: Option<f64>,
    pub play_success: Option<i32>,
    pub video_download_speed: Option<f64>,
    pub video_size: Option<i32>,
    pub video_duration_ms: Option<f64>,
    pub dropped_frames: Option<i32>,
    pub decoded_frames: Option<i32>,
    pub screenshot_path: Option<String>,
    pub page_title: Option<String>,
    pub error_msg: Option<String>,
    pub test_count: Option<i32>,
    pub trigger_method: Option<String>,
    pub stutter_count: Option<i32>,
    pub stutter_duration_ms: Option<f64>,
    pub play_duration_sec: Option<f64>,
    pub stutter_ratio: Option<f64>,
    pub video_width: Option<i32>,
    pub video_height: Option<i32>,
    pub video_duration_sec: Option<f64>,
    pub created_at: String,
}

/// 任务日志模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TaskLog {
    pub id: String,
    pub task_id: String,
    pub level: String,
    pub message: String,
    pub created_at: String,
}

/// Ping 测试结果模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PingResult {
    pub id: String,
    pub task_id: String,
    pub host: String,
    pub avg_latency_ms: Option<f64>,
    pub packet_loss_rate: Option<f64>,
    pub jitter_ms: Option<f64>,
    pub success: Option<i32>,
    pub error_msg: Option<String>,
    pub method: Option<String>,
    pub test_count: Option<i32>,
    pub created_at: String,
}
