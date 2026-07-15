use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::plan::{
    CreatePlanRequest, PlanItemInput, PlanWithItems, PlanRunWithTasks, RunPlanResponse, TaskPlan, TaskPlanItem,
    TaskPlanRun, UpdatePlanRequest,
};
use crate::utils::response::{ProgressMessage, TaskJob};

/// 计划业务逻辑服务
pub struct PlanService;

impl PlanService {
    /// 创建计划（含 items）
    pub async fn create_plan(
        db: &SqlitePool,
        task_tx: &tokio::sync::mpsc::Sender<TaskJob>,
        user_id: &str,
        req: CreatePlanRequest,
    ) -> anyhow::Result<PlanWithItems> {
        if req.items.is_empty() {
            anyhow::bail!("计划至少需要包含一个测试项");
        }
        let plan_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let enabled = if req.enabled { 1 } else { 0 };

        // 计算下次执行时间
        let next_run_at = req
            .cron_expression
            .as_ref()
            .and_then(|expr| compute_next_run(expr, &now));

        // 插入计划
        sqlx::query(
            "INSERT INTO task_plans (id, user_id, name, description, cron_expression, enabled, next_run_at, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&plan_id)
        .bind(user_id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.cron_expression)
        .bind(enabled)
        .bind(&next_run_at)
        .bind(&now)
        .bind(&now)
        .execute(db)
        .await?;

        // 插入 items
        for (idx, item) in req.items.iter().enumerate() {
            let item_id = Uuid::new_v4().to_string();
            let urls_json = serde_json::to_string(&item.urls)?;
            let options_json = if item.options.is_null() {
                None
            } else {
                Some(serde_json::to_string(&item.options)?)
            };

            sqlx::query(
                "INSERT INTO task_plan_items (id, plan_id, task_type, urls, options, repeat_count, engine, order_index, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&item_id)
            .bind(&plan_id)
            .bind(&item.task_type)
            .bind(&urls_json)
            .bind(&options_json)
            .bind(item.repeat_count)
            .bind(&item.engine)
            .bind(idx as i32)
            .bind(&now)
            .execute(db)
            .await?;
        }

        let _ = task_tx; // 暂时不用，避免警告

        Self::get_plan(db, &plan_id).await
    }

    /// 更新计划
    pub async fn update_plan(
        db: &SqlitePool,
        user_id: &str,
        plan_id: &str,
        req: UpdatePlanRequest,
    ) -> anyhow::Result<PlanWithItems> {
        // 校验权限
        let existing = sqlx::query_as::<_, TaskPlan>("SELECT * FROM task_plans WHERE id = ?")
            .bind(plan_id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("计划不存在"))?;

        if existing.user_id != user_id {
            anyhow::bail!("无权修改此计划");
        }

        let now = Utc::now().to_rfc3339();
        let enabled = if req.enabled { 1 } else { 0 };
        let next_run_at = req
            .cron_expression
            .as_ref()
            .and_then(|expr| compute_next_run(expr, &now));

        sqlx::query(
            "UPDATE task_plans SET name = ?, description = ?, cron_expression = ?, enabled = ?, next_run_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.cron_expression)
        .bind(enabled)
        .bind(&next_run_at)
        .bind(&now)
        .bind(plan_id)
        .execute(db)
        .await?;

        // 删除旧 items
        sqlx::query("DELETE FROM task_plan_items WHERE plan_id = ?")
            .bind(plan_id)
            .execute(db)
            .await?;

        // 重新插入
        for (idx, item) in req.items.iter().enumerate() {
            let item_id = Uuid::new_v4().to_string();
            let urls_json = serde_json::to_string(&item.urls)?;
            let options_json = if item.options.is_null() {
                None
            } else {
                Some(serde_json::to_string(&item.options)?)
            };

            sqlx::query(
                "INSERT INTO task_plan_items (id, plan_id, task_type, urls, options, order_index, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&item_id)
            .bind(plan_id)
            .bind(&item.task_type)
            .bind(&urls_json)
            .bind(&options_json)
            .bind(idx as i32)
            .bind(&now)
            .execute(db)
            .await?;
        }

        Self::get_plan(db, plan_id).await
    }

    /// 删除计划
    pub async fn delete_plan(db: &SqlitePool, user_id: &str, plan_id: &str) -> anyhow::Result<()> {
        let existing = sqlx::query_as::<_, TaskPlan>("SELECT * FROM task_plans WHERE id = ?")
            .bind(plan_id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("计划不存在"))?;

        if existing.user_id != user_id {
            anyhow::bail!("无权删除此计划");
        }

        sqlx::query("DELETE FROM task_plans WHERE id = ?")
            .bind(plan_id)
            .execute(db)
            .await?;
        Ok(())
    }

    /// 列出我的计划
    pub async fn list_plans(
        db: &SqlitePool,
        user_id: &str,
        page: u32,
        size: u32,
    ) -> anyhow::Result<(Vec<PlanWithItems>, u32)> {
        let offset = (page - 1) * size;
        let total: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM task_plans WHERE user_id = ?")
            .bind(user_id)
            .fetch_one(db)
            .await?;

        let plans = sqlx::query_as::<_, TaskPlan>(
            "SELECT * FROM task_plans WHERE user_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(user_id)
        .bind(size)
        .bind(offset)
        .fetch_all(db)
        .await?;

        let mut results = Vec::new();
        for p in plans {
            let items = sqlx::query_as::<_, TaskPlanItem>(
                "SELECT * FROM task_plan_items WHERE plan_id = ? ORDER BY order_index ASC",
            )
            .bind(&p.id)
            .fetch_all(db)
            .await?;
            results.push(PlanWithItems { plan: p, items });
        }

        Ok((results, total as u32))
    }

    /// 列出所有启用的计划（用于调度器）
    pub async fn list_enabled_plans(db: &SqlitePool) -> anyhow::Result<Vec<TaskPlan>> {
        let plans = sqlx::query_as::<_, TaskPlan>(
            "SELECT * FROM task_plans WHERE enabled = 1 AND cron_expression IS NOT NULL",
        )
        .fetch_all(db)
        .await?;
        Ok(plans)
    }

    /// 获取计划详情
    pub async fn get_plan(db: &SqlitePool, plan_id: &str) -> anyhow::Result<PlanWithItems> {
        let plan = sqlx::query_as::<_, TaskPlan>("SELECT * FROM task_plans WHERE id = ?")
            .bind(plan_id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("计划不存在"))?;

        let items = sqlx::query_as::<_, TaskPlanItem>(
            "SELECT * FROM task_plan_items WHERE plan_id = ? ORDER BY order_index ASC",
        )
        .bind(plan_id)
        .fetch_all(db)
        .await?;

        Ok(PlanWithItems { plan, items })
    }

    /// 立即运行计划
    pub async fn run_now(
        db: &SqlitePool,
        task_tx: &tokio::sync::mpsc::Sender<TaskJob>,
        progress_tx: &tokio::sync::broadcast::Sender<ProgressMessage>,
        user_id: &str,
        plan_id: &str,
    ) -> anyhow::Result<RunPlanResponse> {
        let plan_data = Self::get_plan(db, plan_id).await?;
        if plan_data.plan.user_id != user_id {
            anyhow::bail!("无权运行此计划");
        }
        if plan_data.items.is_empty() {
            anyhow::bail!("计划无测试项");
        }

        let now = Utc::now().to_rfc3339();
        let plan_run_id = Uuid::new_v4().to_string();

        // 创建 plan_run 记录
        sqlx::query(
            "INSERT INTO task_plan_runs (id, plan_id, triggered_by, started_at, status, created_at) VALUES (?, ?, 'manual', ?, 'running', ?)",
        )
        .bind(&plan_run_id)
        .bind(plan_id)
        .bind(&now)
        .bind(&now)
        .execute(db)
        .await?;

        // 广播计划开始
        let _ = progress_tx.send(ProgressMessage::Log {
            task_id: plan_run_id.clone(),
            level: "info".to_string(),
            message: format!("手动执行计划: {}", plan_data.plan.name),
        });

        // 为每个 item 创建 task 并派发
        let mut task_ids = Vec::new();
        for item in &plan_data.items {
            let task_id = Uuid::new_v4().to_string();
            let config = serde_json::json!({
                "plan_id": plan_id,
                "plan_run_id": plan_run_id,
                "options": serde_json::from_str::<serde_json::Value>(item.options.as_deref().unwrap_or("null")).unwrap_or(serde_json::Value::Null),
            });

            sqlx::query(
                "INSERT INTO test_task (id, user_id, task_type, status, config, progress, created_at) VALUES (?, ?, ?, 'pending', ?, 0, ?)",
            )
            .bind(&task_id)
            .bind(user_id)
            .bind(&item.task_type)
            .bind(config.to_string())
            .bind(&now)
            .execute(db)
            .await?;

            // 解析 urls
            let urls: Vec<String> = serde_json::from_str(&item.urls).unwrap_or_default();

            // 派发到 Worker
            let job = TaskJob {
                task_id: task_id.clone(),
                user_id: user_id.to_string(),
                task_type: item.task_type.clone(),
                urls,
                options: serde_json::from_str(&item.options.clone().unwrap_or_else(|| "null".to_string()))
                    .unwrap_or(serde_json::Value::Null),
            };
            task_tx.send(job).await?;

            task_ids.push(task_id);
        }

        // 一次性写入所有 task_ids JSON 数组
        let task_ids_json = serde_json::to_string(&task_ids)?;
        sqlx::query("UPDATE task_plan_runs SET task_ids = ? WHERE id = ?")
            .bind(&task_ids_json)
            .bind(&plan_run_id)
            .execute(db)
            .await?;

        // 更新 last_run_at
        sqlx::query("UPDATE task_plans SET last_run_at = ? WHERE id = ?")
            .bind(&now)
            .bind(plan_id)
            .execute(db)
            .await?;

        Ok(RunPlanResponse {
            plan_run_id,
            task_ids,
        })
    }

    /// 列出计划运行历史（含已完成任务数）
    pub async fn list_plan_runs(
        db: &SqlitePool,
        plan_id: &str,
        limit: u32,
    ) -> anyhow::Result<Vec<PlanRunWithTasks>> {
        let runs = sqlx::query_as::<_, TaskPlanRun>(
            "SELECT * FROM task_plan_runs WHERE plan_id = ? ORDER BY started_at DESC LIMIT ?",
        )
        .bind(plan_id)
        .bind(limit)
        .fetch_all(db)
        .await?;

        let mut results = Vec::new();
        for run in runs {
            let task_ids: Vec<String> = serde_json::from_str(&run.task_ids).unwrap_or_default();
            let task_count = task_ids.len();

            // 查询已完成的 task 数量
            let completed_count = if task_count > 0 {
                let placeholders: Vec<String> = task_ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
                let query = format!(
                    "SELECT COUNT(*) FROM test_task WHERE id IN ({}) AND status IN ('completed', 'failed', 'cancelled')",
                    placeholders.join(",")
                );
                let mut q = sqlx::query_scalar(&query);
                for tid in &task_ids {
                    q = q.bind(tid);
                }
                q.fetch_one(db).await.unwrap_or(0)
            } else {
                0
            } as usize;

            // 自动更新 plan_run 状态
            if completed_count == task_count && task_count > 0 && run.status == "running" {
                let _ = Self::complete_plan_run(db, &run.id, "completed").await;
            }

            results.push(PlanRunWithTasks {
                run,
                task_count,
                completed_count: completed_count as usize,
            });
        }

        Ok(results)
    }

    /// 标记 plan_run 完成
    pub async fn complete_plan_run(
        db: &SqlitePool,
        plan_run_id: &str,
        status: &str,
    ) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE task_plan_runs SET status = ?, finished_at = ? WHERE id = ?",
        )
        .bind(status)
        .bind(&now)
        .bind(plan_run_id)
        .execute(db)
        .await?;
        Ok(())
    }

    /// 删除 plan_run（不删除关联的 task，task 仍可查看）
    pub async fn delete_plan_run(
        db: &SqlitePool,
        user_id: &str,
        plan_id: &str,
        run_id: &str,
    ) -> anyhow::Result<()> {
        // 校验权限
        let plan = sqlx::query_as::<_, TaskPlan>("SELECT * FROM task_plans WHERE id = ?")
            .bind(plan_id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("计划不存在"))?;
        if plan.user_id != user_id {
            anyhow::bail!("无权删除");
        }
        sqlx::query("DELETE FROM task_plan_runs WHERE id = ? AND plan_id = ?")
            .bind(run_id)
            .bind(plan_id)
            .execute(db)
            .await?;
        Ok(())
    }

    /// 按时间范围筛选 plan_run
    pub async fn list_plan_runs_filtered(
        db: &SqlitePool,
        plan_id: &str,
        start_time: Option<&str>,
        end_time: Option<&str>,
        limit: u32,
    ) -> anyhow::Result<Vec<PlanRunWithTasks>> {
        let mut sql = String::from(
            "SELECT * FROM task_plan_runs WHERE plan_id = ?"
        );
        if start_time.is_some() { sql.push_str(" AND started_at >= ?"); }
        if end_time.is_some() { sql.push_str(" AND started_at <= ?"); }
        sql.push_str(" ORDER BY started_at DESC LIMIT ?");

        let mut q = sqlx::query_as::<_, TaskPlanRun>(&sql).bind(plan_id);
        if let Some(s) = start_time { q = q.bind(s); }
        if let Some(e) = end_time { q = q.bind(e); }
        q = q.bind(limit);

        let runs = q.fetch_all(db).await?;
        let mut results = Vec::new();
        for run in runs {
            let task_ids: Vec<String> = serde_json::from_str(&run.task_ids).unwrap_or_default();
            let task_count = task_ids.len();
            let completed_count = if task_count > 0 {
                let placeholders: Vec<String> = task_ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
                let query = format!(
                    "SELECT COUNT(*) FROM test_task WHERE id IN ({}) AND status IN ('completed', 'failed', 'cancelled')",
                    placeholders.join(",")
                );
                let mut q = sqlx::query_scalar(&query);
                for tid in &task_ids { q = q.bind(tid); }
                q.fetch_one(db).await.unwrap_or(0)
            } else {
                0
            } as usize;

            if completed_count == task_count && task_count > 0 && run.status == "running" {
                let _ = Self::complete_plan_run(db, &run.id, "completed").await;
            }
            results.push(PlanRunWithTasks { run, task_count, completed_count });
        }
        Ok(results)
    }

    /// 更新下次执行时间
    pub async fn update_next_run(
        db: &SqlitePool,
        plan_id: &str,
        next_run_at: Option<&str>,
    ) -> anyhow::Result<()> {
        sqlx::query("UPDATE task_plans SET next_run_at = ? WHERE id = ?")
            .bind(next_run_at)
            .bind(plan_id)
            .execute(db)
            .await?;
        Ok(())
    }
}

/// 计算 cron 表达式的下次执行时间
fn compute_next_run(cron_expr: &str, from: &str) -> Option<String> {
    use chrono::DateTime;
    use cron::Schedule;
    use std::str::FromStr;

    // cron crate v0.12 要求至少 6 字段（带秒），自动给 5 字段表达式补 "0 "
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
