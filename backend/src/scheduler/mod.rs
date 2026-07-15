use std::str::FromStr;
use std::time::Duration;

use chrono::{DateTime, Utc};
use cron::Schedule;
use sqlx::SqlitePool;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::models::plan::TaskPlan;
use crate::models::plan::TaskPlanItem;
use crate::utils::response::{ProgressMessage, TaskJob};

/// 计划调度器 — 后台守护进程，每分钟检查到期计划
pub struct PlanScheduler {
    db: SqlitePool,
    task_tx: mpsc::Sender<TaskJob>,
    progress_tx: broadcast::Sender<ProgressMessage>,
}

impl PlanScheduler {
    pub fn new(
        db: SqlitePool,
        task_tx: mpsc::Sender<TaskJob>,
        progress_tx: broadcast::Sender<ProgressMessage>,
    ) -> Self {
        Self { db, task_tx, progress_tx }
    }

    /// 启动调度器后台任务
    pub fn start(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            info!("PlanScheduler 启动，每 60 秒检查一次");
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            // 第一次 tick 立即触发
            interval.tick().await;

            loop {
                interval.tick().await;
                if let Err(e) = self.tick().await {
                    error!("调度 tick 失败: {}", e);
                }
            }
        })
    }

    /// 单次调度检查
    async fn tick(&self) -> anyhow::Result<()> {
        let plans = sqlx::query_as::<_, TaskPlan>(
            "SELECT * FROM task_plans WHERE enabled = 1 AND cron_expression IS NOT NULL",
        )
        .fetch_all(&self.db)
        .await?;

        let now = Utc::now();

        for plan in plans {
            let next_run_at = match &plan.next_run_at {
                Some(s) => s.parse::<DateTime<Utc>>().ok(),
                None => None,
            };

            let should_run = match next_run_at {
                Some(next) => now >= next,
                None => true, // 没有 next_run_at 表示从未跑过
            };

            if should_run {
                if let Err(e) = self.run_plan(&plan, "cron").await {
                    error!("定时执行计划 {} 失败: {}", plan.name, e);
                }
            } else {
                // 更新 next_run_at（防止漂移）
                if let Some(cron_expr) = &plan.cron_expression {
                    if let Some(new_next) = compute_next_run(cron_expr, &now.to_rfc3339()) {
                        let _ = sqlx::query("UPDATE task_plans SET next_run_at = ? WHERE id = ?")
                            .bind(&new_next)
                            .bind(&plan.id)
                            .execute(&self.db)
                            .await;
                    }
                }
            }
        }

        Ok(())
    }

    /// 执行一个计划（调度触发或手动触发共用此方法）
    pub async fn run_plan(&self, plan: &TaskPlan, triggered_by: &str) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();
        let plan_run_id = Uuid::new_v4().to_string();

        // 创建 plan_run
        sqlx::query(
            "INSERT INTO task_plan_runs (id, plan_id, triggered_by, started_at, status, created_at) VALUES (?, ?, ?, ?, 'running', ?)",
        )
        .bind(&plan_run_id)
        .bind(&plan.id)
        .bind(triggered_by)
        .bind(&now)
        .bind(&now)
        .execute(&self.db)
        .await?;

        // 推送计划开始
        let _ = self.progress_tx.send(ProgressMessage::Log {
            task_id: plan_run_id.clone(),
            level: "info".to_string(),
            message: format!("[cron] 执行计划: {}", plan.name),
        });

        // 加载 items
        let items = sqlx::query_as::<_, TaskPlanItem>(
            "SELECT * FROM task_plan_items WHERE plan_id = ? ORDER BY order_index ASC",
        )
        .bind(&plan.id)
        .fetch_all(&self.db)
        .await?;

        if items.is_empty() {
            warn!("计划 {} 无测试项，跳过", plan.name);
            let _ = sqlx::query("UPDATE task_plan_runs SET status = 'failed', finished_at = ? WHERE id = ?")
                .bind(&now)
                .bind(&plan_run_id)
                .execute(&self.db)
                .await;
            return Ok(());
        }

        // 为每个 item 创建 task 并派发
        let mut all_task_ids: Vec<String> = Vec::new();
        for item in &items {
            let task_id = Uuid::new_v4().to_string();
            let config = serde_json::json!({
                "plan_id": plan.id,
                "plan_run_id": plan_run_id,
                "options": serde_json::from_str::<serde_json::Value>(
                    item.options.as_deref().unwrap_or("null")
                ).unwrap_or(serde_json::Value::Null),
            });

            sqlx::query(
                "INSERT INTO test_task (id, user_id, task_type, status, config, progress, created_at) VALUES (?, ?, ?, 'pending', ?, 0, ?)",
            )
            .bind(&task_id).bind(&plan.user_id).bind(&item.task_type)
            .bind(config.to_string()).bind(&now)
            .execute(&self.db).await?;

            let urls: Vec<String> = serde_json::from_str(&item.urls).unwrap_or_default();
            let job = TaskJob {
                task_id: task_id.clone(), user_id: plan.user_id.clone(),
                task_type: item.task_type.clone(), urls,
                options: serde_json::from_str(item.options.as_deref().unwrap_or("null")).unwrap_or(serde_json::Value::Null),
            };
            if let Err(e) = self.task_tx.send(job).await {
                error!("派发失败: {}", e);
            }
            all_task_ids.push(task_id);
        }

        // 一次性写入所有 task_ids JSON
        let task_ids_json = serde_json::to_string(&all_task_ids).unwrap_or_default();
        let _ = sqlx::query("UPDATE task_plan_runs SET task_ids = ? WHERE id = ?")
            .bind(&task_ids_json).bind(&plan_run_id)
            .execute(&self.db).await;

        // 更新 last_run_at
        let _ = sqlx::query("UPDATE test_task SET status = 'pending' WHERE id IN (SELECT task_id FROM task_plan_runs WHERE id = ?)")
            .bind(&plan_run_id)
            .execute(&self.db)
            .await;

        // 更新 last_run_at
        let _ = sqlx::query("UPDATE task_plans SET last_run_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&plan.id)
            .execute(&self.db)
            .await;

        // 更新下次执行时间
        if let Some(cron_expr) = &plan.cron_expression {
            if let Some(new_next) = compute_next_run(cron_expr, &now) {
                let _ = sqlx::query("UPDATE task_plans SET next_run_at = ? WHERE id = ?")
                    .bind(&new_next)
                    .bind(&plan.id)
                    .execute(&self.db)
                    .await;
            }
        }

        info!("定时执行计划成功: {} ({} 个测试项)", plan.name, items.len());
        Ok(())
    }
}

/// 计算 cron 下次执行时间
pub fn compute_next_run(cron_expr: &str, from: &str) -> Option<String> {
    let normalized = if cron_expr.split_whitespace().count() == 5 {
        format!("0 {}", cron_expr)
    } else {
        cron_expr.to_string()
    };
    let schedule = Schedule::from_str(&normalized).ok()?;
    let now: DateTime<Utc> = from.parse().ok()?;
    let next = schedule.after(&now).next()?;
    Some(next.to_rfc3339())
}
