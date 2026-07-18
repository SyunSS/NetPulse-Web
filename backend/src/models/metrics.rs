use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 指标定义
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MetricDefinition {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub category: String,
    pub collector: String,
    pub description: Option<String>,
    pub cost_level: String,
    pub default_enable: i32,
}

/// 指标配置（任务创建时传入）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricConfig {
    pub metric_ids: Vec<String>,
}

/// 指标配置简写
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsPayload {
    pub metrics: Option<Vec<String>>,
}
