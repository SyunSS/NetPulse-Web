use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 计划主表
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TaskPlan {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub cron_expression: Option<String>,
    pub enabled: i32,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 计划项
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TaskPlanItem {
    pub id: String,
    pub plan_id: String,
    pub task_type: String,
    pub urls: String,
    pub options: Option<String>,
    pub repeat_count: i32,
    pub engine: Option<String>,
    pub order_index: i32,
    pub created_at: String,
}

/// 计划运行记录
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TaskPlanRun {
    pub id: String,
    pub plan_id: String,
    pub task_ids: String,       // JSON 数组: ["task1_id", "task2_id"]
    pub triggered_by: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: String,
    pub created_at: String,
}

/// 计划运行的扩展信息（返回给前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRunWithTasks {
    #[serde(flatten)]
    pub run: TaskPlanRun,
    pub task_count: usize,
    pub completed_count: usize,
}

/// 计划项 DTO（前端传入）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanItemInput {
    pub task_type: String,
    pub urls: Vec<String>,
    #[serde(default)]
    pub options: serde_json::Value,
    #[serde(default = "default_repeat")]
    pub repeat_count: i32,
    #[serde(default = "default_engine")]
    pub engine: String,
}

fn default_repeat() -> i32 { 1 }
fn default_engine() -> String { "headless_chrome".to_string() }

/// 创建计划请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePlanRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub cron_expression: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub items: Vec<PlanItemInput>,
}

fn default_enabled() -> bool {
    true
}

/// 更新计划请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePlanRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub cron_expression: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub items: Vec<PlanItemInput>,
}

/// 计划 + 项（API 返回完整数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanWithItems {
    #[serde(flatten)]
    pub plan: TaskPlan,
    pub items: Vec<TaskPlanItem>,
}

/// 计划运行响应（用于"立即运行"）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunPlanResponse {
    pub plan_run_id: String,
    pub task_ids: Vec<String>,
}

/// 计划列表分页响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanListResponse {
    pub plans: Vec<PlanWithItems>,
    pub total: u32,
    pub page: u32,
    pub size: u32,
}
