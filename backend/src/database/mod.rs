use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use tracing::info;

/// 初始化数据库连接池并执行迁移
pub async fn init_db(path: &str) -> anyhow::Result<SqlitePool> {
    // 确保数据库目录存在
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&format!("sqlite:{}?mode=rwc", path))
        .await?;

    // 启用 WAL 模式和 foreign keys
    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA foreign_keys=ON;")
        .execute(&pool)
        .await?;

    info!("数据库连接已建立: {}", path);

    // 执行建表迁移
    run_migrations(&pool).await?;

    Ok(pool)
}

/// 执行数据库迁移（建表）
async fn run_migrations(pool: &SqlitePool) -> anyhow::Result<()> {
    let migrations = vec![
        // users 表
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
        // test_task 表
        r#"
        CREATE TABLE IF NOT EXISTS test_task (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            task_type TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            config TEXT NOT NULL,
            progress REAL DEFAULT 0,
            result TEXT,
            error_msg TEXT,
            created_at TEXT NOT NULL,
            started_at TEXT,
            finished_at TEXT,
            FOREIGN KEY (user_id) REFERENCES users(id)
        );
        "#,
        // website_result 表
        r#"
        CREATE TABLE IF NOT EXISTS website_result (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            url TEXT NOT NULL,
            dns_time_ms REAL,
            dns_success INTEGER DEFAULT 1,
            tcp_time_ms REAL,
            tls_time_ms REAL,
            http_status INTEGER,
            ttfb_ms REAL,
            fp_ms REAL,
            fcp_ms REAL,
            dom_content_loaded_ms REAL,
            load_event_ms REAL,
            page_open_time_ms REAL,
            first_paint_ms REAL,
            resource_count INTEGER,
            resource_total_size INTEGER,
            final_url TEXT,
            page_title TEXT,
            screenshot_path TEXT,
            error_msg TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (task_id) REFERENCES test_task(id)
        );
        "#,
        // download_result 表
        r#"
        CREATE TABLE IF NOT EXISTS download_result (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            url TEXT NOT NULL,
            dns_time_ms REAL,
            dns_success INTEGER,
            tcp_time_ms REAL,
            download_speed REAL,
            avg_speed REAL,
            peak_speed REAL,
            download_time_ms REAL,
            file_size INTEGER,
            success INTEGER DEFAULT 1,
            error_msg TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (task_id) REFERENCES test_task(id)
        );
        "#,
        // video_result 表
        r#"
        CREATE TABLE IF NOT EXISTS video_result (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            url TEXT NOT NULL,
            platform TEXT,
            dns_time_ms REAL,
            dns_success INTEGER,
            tcp_time_ms REAL,
            http_response_ms REAL,
            first_play_time_ms REAL,
            buffer_count INTEGER,
            total_buffer_time_ms REAL,
            buffer_rate REAL,
            play_success INTEGER DEFAULT 1,
            video_download_speed REAL,
            video_size INTEGER,
            video_duration_ms REAL,
            dropped_frames INTEGER,
            decoded_frames INTEGER,
            screenshot_path TEXT,
            page_title TEXT,
            error_msg TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (task_id) REFERENCES test_task(id)
        );
        "#,
        // task_log 表
        r#"
        CREATE TABLE IF NOT EXISTS task_log (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            level TEXT NOT NULL,
            message TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (task_id) REFERENCES test_task(id)
        );
        "#,
        // system_setting 表
        r#"
        CREATE TABLE IF NOT EXISTS system_setting (
            id TEXT PRIMARY KEY,
            key TEXT NOT NULL UNIQUE,
            value TEXT NOT NULL,
            description TEXT,
            updated_at TEXT NOT NULL
        );
        "#,
        // task_plans 表
        r#"
        CREATE TABLE IF NOT EXISTS task_plans (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            name TEXT NOT NULL,
            description TEXT,
            cron_expression TEXT,
            enabled INTEGER DEFAULT 1,
            last_run_at TEXT,
            next_run_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id)
        );
        "#,
        // task_plan_items 表
        r#"
        CREATE TABLE IF NOT EXISTS task_plan_items (
            id TEXT PRIMARY KEY,
            plan_id TEXT NOT NULL,
            task_type TEXT NOT NULL,
            urls TEXT NOT NULL,
            options TEXT,
            repeat_count INTEGER DEFAULT 1,
            engine TEXT DEFAULT 'headless_chrome',
            order_index INTEGER DEFAULT 0,
            created_at TEXT NOT NULL,
            FOREIGN KEY (plan_id) REFERENCES task_plans(id) ON DELETE CASCADE
        );
        "#,
        // task_plan_runs 表
        r#"
        CREATE TABLE IF NOT EXISTS task_plan_runs (
            id TEXT PRIMARY KEY,
            plan_id TEXT NOT NULL,
            task_ids TEXT NOT NULL DEFAULT '[]',
            triggered_by TEXT NOT NULL,
            started_at TEXT NOT NULL,
            finished_at TEXT,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (plan_id) REFERENCES task_plans(id) ON DELETE CASCADE
        );
        "#,
        // ping_result 表
        r#"
        CREATE TABLE IF NOT EXISTS ping_result (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            host TEXT NOT NULL,
            avg_latency_ms REAL,
            packet_loss_rate REAL,
            jitter_ms REAL,
            success INTEGER DEFAULT 1,
            error_msg TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (task_id) REFERENCES test_task(id)
        );
        "#,
    ];

    for sql in migrations {
        sqlx::query(sql).execute(pool).await?;
    }

    info!("数据库迁移完成（{} 张表）", 11);
    Ok(())
}
