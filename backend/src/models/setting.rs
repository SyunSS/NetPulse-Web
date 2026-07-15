use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 系统设置模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SystemSetting {
    pub id: String,
    pub key: String,
    pub value: String,
    pub description: Option<String>,
    pub updated_at: String,
}
