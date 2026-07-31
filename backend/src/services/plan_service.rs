use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::plan::{
    CreatePlanRequest, PlanItemInput, PlanRunWithTasks, PlanWithItems, RunPlanResponse, TaskPlan,
    TaskPlanItem, TaskPlanRun, UpdatePlanRequest,
};
use crate::utils::response::{ProgressMessage, TaskJob};

/// 计划业务逻辑服务
pub struct PlanService;

impl PlanService {
    const MAX_PAGE_SIZE: u32 = 100;
    const MAX_RUN_LIMIT: u32 = 100;

    /// 创建计划（含 items）
    pub async fn create_plan(
        db: &SqlitePool,
        task_tx: &tokio::sync::mpsc::Sender<TaskJob>,
        user_id: &str,
        req: CreatePlanRequest,
    ) -> anyhow::Result<PlanWithItems> {
        validate_plan_request(&req.name, req.cron_expression.as_deref(), &req.items)?;
        let plan_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let enabled = if req.enabled { 1 } else { 0 };

        // 计算下次执行时间
        let next_run_at = req
            .cron_expression
            .as_ref()
            .and_then(|expr| compute_next_run(expr, &now));

        let mut tx = db.begin().await?;
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
        .execute(&mut *tx)
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
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

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
        validate_plan_request(&req.name, req.cron_expression.as_deref(), &req.items)?;
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

        let mut tx = db.begin().await?;
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
        .execute(&mut *tx)
        .await?;

        // 删除旧 items
        sqlx::query("DELETE FROM task_plan_items WHERE plan_id = ?")
            .bind(plan_id)
            .execute(&mut *tx)
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
                "INSERT INTO task_plan_items (id, plan_id, task_type, urls, options, repeat_count, engine, order_index, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&item_id)
            .bind(plan_id)
            .bind(&item.task_type)
            .bind(&urls_json)
            .bind(&options_json)
            .bind(item.repeat_count)
            .bind(&item.engine)
            .bind(idx as i32)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

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
        let size = size.clamp(1, Self::MAX_PAGE_SIZE);
        let offset = (page.max(1) - 1).saturating_mul(size);
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
        let mut tx = db.begin().await?;
        let active: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM task_plan_runs WHERE plan_id = ? AND status IN ('pending', 'running')",
        )
        .bind(plan_id)
        .fetch_one(&mut *tx)
        .await?;
        if active > 0 {
            anyhow::bail!("计划已有运行中的任务");
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
        .execute(&mut *tx)
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
            // 合并 repeat_count 到 options
            let (urls, opts) = prepare_plan_item(item)?;

            let config = serde_json::json!({
                "plan_id": plan_id,
                "plan_run_id": plan_run_id,
                "urls": &urls,
                "options": &opts,
            });

            sqlx::query(
                "INSERT INTO test_task (id, user_id, task_type, status, config, progress, created_at) VALUES (?, ?, ?, 'pending', ?, 0, ?)",
            )
            .bind(&task_id)
            .bind(user_id)
            .bind(&item.task_type)
            .bind(config.to_string())
            .bind(&now)
                .execute(&mut *tx)
                .await?;

            task_ids.push(task_id);
        }

        // 一次性写入所有 task_ids JSON 数组
        let task_ids_json = serde_json::to_string(&task_ids)?;
        sqlx::query("UPDATE task_plan_runs SET task_ids = ? WHERE id = ?")
            .bind(&task_ids_json)
            .bind(&plan_run_id)
            .execute(&mut *tx)
            .await?;

        // 更新 last_run_at
        sqlx::query("UPDATE task_plans SET last_run_at = ? WHERE id = ?")
            .bind(&now)
            .bind(plan_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        for (item, task_id) in plan_data.items.iter().zip(task_ids.iter()) {
            let (urls, opts) = prepare_plan_item(item)?;
            if let Err(error) = task_tx
                .send(TaskJob {
                    task_id: task_id.clone(),
                    user_id: user_id.to_string(),
                    task_type: item.task_type.clone(),
                    urls,
                    options: opts,
                })
                .await
            {
                sqlx::query("UPDATE test_task SET status='failed', finished_at=?, error_msg=? WHERE id=? AND status='pending'")
                    .bind(Utc::now().to_rfc3339())
                    .bind(format!("任务派发失败: {error}"))
                    .bind(task_id)
                    .execute(db)
                    .await?;
            }
        }

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
        .bind(limit.min(Self::MAX_RUN_LIMIT))
        .fetch_all(db)
        .await?;

        let mut results = Vec::new();
        for mut run in runs {
            let task_ids: Vec<String> = serde_json::from_str(&run.task_ids).unwrap_or_default();
            let task_count = task_ids.len();

            // 查询已完成的 task 数量
            let completed_count = if task_count > 0 {
                let placeholders: Vec<String> = task_ids
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("?{}", i + 1))
                    .collect();
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
                run.status = "completed".to_string();
                run.finished_at = Some(Utc::now().to_rfc3339());
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
        sqlx::query("UPDATE task_plan_runs SET status = ?, finished_at = ? WHERE id = ?")
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
        cancel_tx: &tokio::sync::broadcast::Sender<String>,
        user_id: &str,
        plan_id: &str,
        run_id: &str,
        force: bool,
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
        let run: crate::models::plan::TaskPlanRun =
            sqlx::query_as("SELECT * FROM task_plan_runs WHERE id = ? AND plan_id = ?")
                .bind(run_id)
                .bind(plan_id)
                .fetch_optional(db)
                .await?
                .ok_or_else(|| anyhow::anyhow!("运行记录不存在"))?;

        let task_ids: Vec<String> = serde_json::from_str(&run.task_ids).unwrap_or_default();
        if !force && matches!(run.status.as_str(), "pending" | "running") {
            anyhow::bail!("请等待任务完成后再删除，或使用强制删除");
        }
        let active_tasks: i64 = if task_ids.is_empty() {
            0
        } else {
            let placeholders = vec!["?"; task_ids.len()].join(",");
            let query_sql = format!(
                "SELECT COUNT(*) FROM test_task WHERE id IN ({}) AND status IN ('pending', 'running')",
                placeholders
            );
            let mut query = sqlx::query_scalar(&query_sql);
            for task_id in &task_ids {
                query = query.bind(task_id);
            }
            query.fetch_one(db).await?
        };
        if active_tasks > 0 {
            if force {
                let query_sql = format!(
                    "UPDATE test_task SET status='cancelled', finished_at=? WHERE id IN ({}) AND status IN ('pending','running')",
                    vec!["?"; task_ids.len()].join(",")
                );
                let mut query = sqlx::query(&query_sql).bind(Utc::now().to_rfc3339());
                for task_id in &task_ids {
                    let _ = cancel_tx.send(task_id.clone());
                    query = query.bind(task_id);
                }
                query.execute(db).await?;
            }
            anyhow::bail!("运行中的子任务已取消，请完成协调后再删除运行记录");
        }

        // 强制模式: 删除关联的所有 task 及其结果
        if force && !run.task_ids.is_empty() {
            {
                for tid in &task_ids {
                    // 先删结果表
                    let _ = sqlx::query("DELETE FROM website_result WHERE task_id=?")
                        .bind(tid)
                        .execute(db)
                        .await;
                    let _ = sqlx::query("DELETE FROM video_result WHERE task_id=?")
                        .bind(tid)
                        .execute(db)
                        .await;
                    let _ = sqlx::query("DELETE FROM download_result WHERE task_id=?")
                        .bind(tid)
                        .execute(db)
                        .await;
                    let _ = sqlx::query("DELETE FROM ping_result WHERE task_id=?")
                        .bind(tid)
                        .execute(db)
                        .await;
                    let _ = sqlx::query("DELETE FROM task_log WHERE task_id=?")
                        .bind(tid)
                        .execute(db)
                        .await;
                    let _ = sqlx::query("DELETE FROM task_metric_config WHERE task_id=?")
                        .bind(tid)
                        .execute(db)
                        .await;
                    // 再删任务
                    let _ = sqlx::query("UPDATE test_task SET status='cancelled', finished_at=? WHERE id=? AND status IN ('pending','running')")
                        .bind(&Utc::now().to_rfc3339())
                        .bind(tid)
                        .execute(db)
                        .await;
                    let _ = sqlx::query("DELETE FROM test_task WHERE id=? AND status IN ('completed','failed','cancelled')")
                        .bind(tid)
                        .execute(db)
                        .await;
                }
            }
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
        let mut sql = String::from("SELECT * FROM task_plan_runs WHERE plan_id = ?");
        if start_time.is_some() {
            sql.push_str(" AND started_at >= ?");
        }
        if end_time.is_some() {
            sql.push_str(" AND started_at <= ?");
        }
        sql.push_str(" ORDER BY started_at DESC LIMIT ?");

        let mut q = sqlx::query_as::<_, TaskPlanRun>(&sql).bind(plan_id);
        if let Some(s) = start_time {
            q = q.bind(s);
        }
        if let Some(e) = end_time {
            q = q.bind(e);
        }
        q = q.bind(limit.min(Self::MAX_RUN_LIMIT));

        let runs = q.fetch_all(db).await?;
        let mut results = Vec::new();
        for mut run in runs {
            let task_ids: Vec<String> = serde_json::from_str(&run.task_ids).unwrap_or_default();
            let task_count = task_ids.len();
            let completed_count = if task_count > 0 {
                let placeholders: Vec<String> = task_ids
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("?{}", i + 1))
                    .collect();
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

            if completed_count == task_count && task_count > 0 && run.status == "running" {
                let _ = Self::complete_plan_run(db, &run.id, "completed").await;
                run.status = "completed".to_string();
                run.finished_at = Some(Utc::now().to_rfc3339());
            }
            results.push(PlanRunWithTasks {
                run,
                task_count,
                completed_count,
            });
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
pub fn compute_next_run(cron_expr: &str, from: &str) -> Option<String> {
    use chrono::DateTime;
    use cron::Schedule;
    use std::str::FromStr;

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

pub(crate) fn prepare_plan_item(
    item: &TaskPlanItem,
) -> anyhow::Result<(Vec<String>, serde_json::Value)> {
    if !matches!(
        item.task_type.as_str(),
        "website" | "download" | "video" | "ping"
    ) {
        anyhow::bail!("任务类型无效: {}", item.task_type);
    }
    if !(1..=100).contains(&item.repeat_count) {
        anyhow::bail!("重复次数必须在 1 到 100 之间");
    }
    let urls: Vec<String> = serde_json::from_str(&item.urls)?;
    if urls.is_empty() {
        anyhow::bail!("计划项至少需要一个 URL");
    }
    for value in &urls {
        if value.trim() != value {
            anyhow::bail!("目标不能包含首尾空格");
        }
        let value = value.trim();
        if item.task_type == "ping" {
            crate::utils::url::validate_ping_target(value)
        } else {
            crate::utils::url::validate_url(value)
        }
        .map_err(|error| anyhow::anyhow!("无效目标: {}", error))?;
    }

    let mut opts: serde_json::Value =
        serde_json::from_str(item.options.as_deref().unwrap_or("null"))
            .unwrap_or(serde_json::Value::Null);
    if opts.is_null() {
        opts = serde_json::json!({});
    }
    opts["repeat_count"] = serde_json::json!(item.repeat_count);
    opts["engine"] = serde_json::json!(item.engine);
    Ok((urls, opts))
}

fn validate_plan_request(
    name: &str,
    cron_expression: Option<&str>,
    items: &[PlanItemInput],
) -> anyhow::Result<()> {
    if name.trim().is_empty() {
        anyhow::bail!("计划名不能为空");
    }
    if items.is_empty() {
        anyhow::bail!("计划至少需要包含一个测试项");
    }
    if let Some(expr) = cron_expression {
        if expr.trim().is_empty() || compute_next_run(expr, &Utc::now().to_rfc3339()).is_none() {
            anyhow::bail!("无效的 cron 表达式");
        }
    }
    for (index, item) in items.iter().enumerate() {
        if !matches!(
            item.task_type.as_str(),
            "website" | "download" | "video" | "ping"
        ) {
            anyhow::bail!("第 {} 项的任务类型无效", index + 1);
        }
        if !(1..=100).contains(&item.repeat_count) {
            anyhow::bail!("第 {} 项的重复次数必须在 1 到 100 之间", index + 1);
        }
        if item.urls.is_empty() {
            anyhow::bail!("第 {} 项至少需要一个 URL", index + 1);
        }
        for value in &item.urls {
            if value.trim() != value {
                anyhow::bail!("第 {} 项包含带空格的目标", index + 1);
            }
            let value = value.trim();
            let result = if item.task_type == "ping" {
                crate::utils::url::validate_ping_target(value)
            } else {
                crate::utils::url::validate_url(value)
            };
            if let Err(error) = result {
                anyhow::bail!("第 {} 项包含无效目标: {}", index + 1, error);
            }
        }
    }
    Ok(())
}
