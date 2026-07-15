use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::task::{
    CreateTaskRequest, CreateTaskResponse, DownloadResult, TestTask, VideoResult, WebsiteResult,
};
use crate::utils::response::{ProgressMessage, TaskJob};

/// 任务业务逻辑服务
pub struct TaskService;

impl TaskService {
    /// 创建测试任务
    pub async fn create_task(
        db: &SqlitePool,
        task_tx: &tokio::sync::mpsc::Sender<TaskJob>,
        user_id: &str,
        req: &CreateTaskRequest,
    ) -> anyhow::Result<CreateTaskResponse> {
        let task_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let config = serde_json::json!({
            "urls": req.urls,
            "options": req.options,
        });

        // 写入数据库
        sqlx::query(
            r#"INSERT INTO test_task (id, user_id, task_type, status, config, progress, created_at)
               VALUES (?, ?, ?, 'pending', ?, 0, ?)"#,
        )
        .bind(&task_id)
        .bind(user_id)
        .bind(&req.task_type)
        .bind(config.to_string())
        .bind(&now)
        .execute(db)
        .await?;

        // 派发给 Worker
        let job = TaskJob {
            task_id: task_id.clone(),
            user_id: user_id.to_string(),
            task_type: req.task_type.clone(),
            urls: req.urls.clone(),
            options: req.options.clone(),
        };

        task_tx.send(job).await?;

        Ok(CreateTaskResponse {
            task_id,
            status: "pending".to_string(),
        })
    }

    /// 获取任务详情
    pub async fn get_task(db: &SqlitePool, task_id: &str) -> anyhow::Result<TestTask> {
        let task = sqlx::query_as::<_, TestTask>("SELECT * FROM test_task WHERE id = ?")
            .bind(task_id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("任务不存在"))?;
        Ok(task)
    }

    /// 分页查询任务列表
    pub async fn list_tasks(
        db: &SqlitePool,
        user_id: &str,
        page: u32,
        size: u32,
    ) -> anyhow::Result<(Vec<TestTask>, u32)> {
        let offset = (page - 1) * size;

        let total: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM test_task WHERE user_id = ?")
            .bind(user_id)
            .fetch_one(db)
            .await?;

        let tasks = sqlx::query_as::<_, TestTask>(
            "SELECT * FROM test_task WHERE user_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(user_id)
        .bind(size)
        .bind(offset)
        .fetch_all(db)
        .await?;

        Ok((tasks, total as u32))
    }

    /// 获取网站测试结果
    pub async fn get_task_results(
        db: &SqlitePool,
        task_id: &str,
    ) -> anyhow::Result<Vec<WebsiteResult>> {
        let results = sqlx::query_as::<_, WebsiteResult>(
            "SELECT * FROM website_result WHERE task_id = ? ORDER BY created_at ASC",
        )
        .bind(task_id)
        .fetch_all(db)
        .await?;
        Ok(results)
    }

    /// 获取视频测试结果
    pub async fn get_video_results(
        db: &SqlitePool,
        task_id: &str,
    ) -> anyhow::Result<Vec<VideoResult>> {
        let results = sqlx::query_as::<_, VideoResult>(
            "SELECT * FROM video_result WHERE task_id = ? ORDER BY created_at ASC",
        )
        .bind(task_id)
        .fetch_all(db)
        .await?;
        Ok(results)
    }

    /// 获取下载测试结果
    pub async fn get_download_results(
        db: &SqlitePool,
        task_id: &str,
    ) -> anyhow::Result<Vec<DownloadResult>> {
        let results = sqlx::query_as::<_, DownloadResult>(
            "SELECT * FROM download_result WHERE task_id = ? ORDER BY created_at ASC",
        )
        .bind(task_id)
        .fetch_all(db)
        .await?;
        Ok(results)
    }

    /// 取消任务
    pub async fn cancel_task(db: &SqlitePool, task_id: &str) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE test_task SET status = 'cancelled', finished_at = ? WHERE id = ? AND status IN ('pending', 'running')")
            .bind(&now)
            .bind(task_id)
            .execute(db)
            .await?;
        Ok(())
    }

    /// 重试任务
    pub async fn retry_task(
        db: &SqlitePool,
        task_tx: &tokio::sync::mpsc::Sender<TaskJob>,
        task_id: &str,
    ) -> anyhow::Result<CreateTaskResponse> {
        // 获取原任务
        let original = Self::get_task(db, task_id).await?;

        // 创建新任务
        let new_task_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let config: serde_json::Value = serde_json::from_str(&original.config)?;

        let urls: Vec<String> = config["urls"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        sqlx::query(
            r#"INSERT INTO test_task (id, user_id, task_type, status, config, progress, created_at)
               VALUES (?, ?, ?, 'pending', ?, 0, ?)"#,
        )
        .bind(&new_task_id)
        .bind(&original.user_id)
        .bind(&original.task_type)
        .bind(&original.config)
        .bind(&now)
        .execute(db)
        .await?;

        // 派发任务
        let job = TaskJob {
            task_id: new_task_id.clone(),
            user_id: original.user_id,
            task_type: original.task_type,
            urls,
            options: config["options"].clone(),
        };

        task_tx.send(job).await?;

        Ok(CreateTaskResponse {
            task_id: new_task_id,
            status: "pending".to_string(),
        })
    }

    /// 获取 Dashboard 统计数据
    pub async fn get_dashboard_stats(
        db: &SqlitePool,
        user_id: &str,
    ) -> anyhow::Result<DashboardStats> {
        // 今日测试次数
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let today_count: i32 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM test_task WHERE user_id = ? AND created_at LIKE ?",
        )
        .bind(user_id)
        .bind(format!("{}%", today))
        .fetch_one(db)
        .await?;

        // 成功率
        let total_count: i32 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM test_task WHERE user_id = ? AND status = 'completed'",
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;

        let failed_count: i32 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM website_result WHERE error_msg IS NOT NULL AND task_id IN (SELECT id FROM test_task WHERE user_id = ?)",
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;

        let success_rate = if total_count > 0 {
            let success_count: i32 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM website_result WHERE error_msg IS NULL AND task_id IN (SELECT id FROM test_task WHERE user_id = ?)",
            )
            .bind(user_id)
            .fetch_one(db)
            .await?;
            (success_count as f64 / (success_count + failed_count).max(1) as f64) * 100.0
        } else {
            0.0
        };

        // 平均 DNS 时间
        let avg_dns: Option<f64> = sqlx::query_scalar(
            "SELECT AVG(dns_time_ms) FROM website_result WHERE dns_time_ms IS NOT NULL AND task_id IN (SELECT id FROM test_task WHERE user_id = ?)",
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;

        // 平均 TTFB
        let avg_ttfb: Option<f64> = sqlx::query_scalar(
            "SELECT AVG(ttfb_ms) FROM website_result WHERE ttfb_ms IS NOT NULL AND ttfb_ms > 0 AND task_id IN (SELECT id FROM test_task WHERE user_id = ?)",
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;

        // 平均页面打开时间
        let avg_page: Option<f64> = sqlx::query_scalar(
            "SELECT AVG(page_open_time_ms) FROM website_result WHERE page_open_time_ms IS NOT NULL AND task_id IN (SELECT id FROM test_task WHERE user_id = ?)",
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;

        // 最近任务
        let recent_tasks = sqlx::query_as::<_, TestTask>(
            "SELECT * FROM test_task WHERE user_id = ? ORDER BY created_at DESC LIMIT 10",
        )
        .bind(user_id)
        .fetch_all(db)
        .await?;

        Ok(DashboardStats {
            today_tests: today_count,
            success_rate,
            avg_dns: avg_dns.unwrap_or(0.0),
            avg_ttfb: avg_ttfb.unwrap_or(0.0),
            avg_page_time: avg_page.unwrap_or(0.0),
            recent_tasks,
            trend_data: Self::get_trend_data(db, user_id).await.unwrap_or_default(),
        })
    }

    /// 获取趋势数据（最近 10 条网站测试结果）
    async fn get_trend_data(
        db: &SqlitePool,
        user_id: &str,
    ) -> anyhow::Result<Vec<TrendPoint>> {
        let results = sqlx::query_as::<_, TrendRow>(
            "SELECT created_at, dns_time_ms, ttfb_ms, page_open_time_ms FROM website_result
             WHERE dns_time_ms IS NOT NULL AND task_id IN (SELECT id FROM test_task WHERE user_id = ?)
             ORDER BY created_at DESC LIMIT 10",
        )
        .bind(user_id)
        .fetch_all(db)
        .await?;

        Ok(results.into_iter().rev().map(|r| TrendPoint {
            time: r.created_at,
            dns_ms: r.dns_time_ms.unwrap_or(0.0),
            ttfb_ms: r.ttfb_ms.unwrap_or(0.0),
            page_ms: r.page_open_time_ms.unwrap_or(0.0),
        }).collect())
    }
}

/// Dashboard 统计数据
#[derive(Debug, serde::Serialize)]
pub struct DashboardStats {
    pub today_tests: i32,
    pub success_rate: f64,
    pub avg_dns: f64,
    pub avg_ttfb: f64,
    pub avg_page_time: f64,
    pub recent_tasks: Vec<TestTask>,
    pub trend_data: Vec<TrendPoint>,
}

/// 趋势图数据点
#[derive(Debug, serde::Serialize)]
pub struct TrendPoint {
    pub time: String,
    pub dns_ms: f64,
    pub ttfb_ms: f64,
    pub page_ms: f64,
}

/// 趋势查询行
#[derive(Debug, sqlx::FromRow)]
struct TrendRow {
    created_at: String,
    dns_time_ms: Option<f64>,
    ttfb_ms: Option<f64>,
    page_open_time_ms: Option<f64>,
}
