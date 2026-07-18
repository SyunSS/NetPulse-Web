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
            test_count INTEGER DEFAULT 1,
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
            test_count INTEGER DEFAULT 1,
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
            test_count INTEGER DEFAULT 1,
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
            test_count INTEGER DEFAULT 1,
            created_at TEXT NOT NULL,
            FOREIGN KEY (task_id) REFERENCES test_task(id)
        );
        "#,
        // metric_definition 表
        r#"
        CREATE TABLE IF NOT EXISTS metric_definition (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            display_name TEXT NOT NULL,
            category TEXT NOT NULL,
            collector TEXT NOT NULL,
            description TEXT,
            cost_level TEXT DEFAULT 'low',
            default_enable INTEGER DEFAULT 1
        );
        "#,
        // metric_profile 表
        r#"
        CREATE TABLE IF NOT EXISTS metric_profile (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            metric_ids TEXT NOT NULL DEFAULT '[]',
            user_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id)
        );
        "#,
        // task_metric_config 表
        r#"
        CREATE TABLE IF NOT EXISTS task_metric_config (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            metric_ids TEXT NOT NULL DEFAULT '[]',
            profile_id TEXT,
            FOREIGN KEY (task_id) REFERENCES test_task(id)
        );
        "#,
    ];

    for sql in migrations {
        sqlx::query(sql).execute(pool).await?;
    }

    // 增量迁移：给已有数据库补 test_count 列
    add_column_if_missing(pool, "website_result", "test_count", "INTEGER DEFAULT 1").await?;
    add_column_if_missing(pool, "website_result", "html_size", "INTEGER").await?;
    add_column_if_missing(pool, "website_result", "css_size", "INTEGER").await?;
    add_column_if_missing(pool, "website_result", "js_size", "INTEGER").await?;
    add_column_if_missing(pool, "website_result", "image_size", "INTEGER").await?;
    add_column_if_missing(pool, "website_result", "font_size", "INTEGER").await?;
    add_column_if_missing(pool, "website_result", "total_requests", "INTEGER").await?;
    add_column_if_missing(pool, "website_result", "failed_requests", "INTEGER").await?;
    add_column_if_missing(pool, "website_result", "lcp_ms", "REAL").await?;
    add_column_if_missing(pool, "website_result", "cls", "REAL").await?;
    add_column_if_missing(pool, "website_result", "tti_ms", "REAL").await?;
    add_column_if_missing(pool, "download_result", "test_count", "INTEGER DEFAULT 1").await?;
    add_column_if_missing(pool, "video_result", "test_count", "INTEGER DEFAULT 1").await?;
    add_column_if_missing(pool, "ping_result", "test_count", "INTEGER DEFAULT 1").await?;

    // 兜底：确保计划相关表一定存在（老数据库可能缺少）
    create_table_if_missing(pool, "task_plans",
        "CREATE TABLE IF NOT EXISTS task_plans (\
            id TEXT PRIMARY KEY, user_id TEXT NOT NULL, name TEXT NOT NULL, description TEXT,\
            cron_expression TEXT, enabled INTEGER DEFAULT 1, last_run_at TEXT, next_run_at TEXT,\
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,\
            FOREIGN KEY (user_id) REFERENCES users(id))"
    ).await?;
    create_table_if_missing(pool, "task_plan_items",
        "CREATE TABLE IF NOT EXISTS task_plan_items (\
            id TEXT PRIMARY KEY, plan_id TEXT NOT NULL, task_type TEXT NOT NULL,\
            urls TEXT NOT NULL, options TEXT, repeat_count INTEGER DEFAULT 1,\
            engine TEXT DEFAULT 'headless_chrome', order_index INTEGER DEFAULT 0,\
            created_at TEXT NOT NULL,\
            FOREIGN KEY (plan_id) REFERENCES task_plans(id) ON DELETE CASCADE)"
    ).await?;
    create_table_if_missing(pool, "task_plan_runs",
        "CREATE TABLE IF NOT EXISTS task_plan_runs (\
            id TEXT PRIMARY KEY, plan_id TEXT NOT NULL,\
            task_ids TEXT NOT NULL DEFAULT '[]', triggered_by TEXT NOT NULL,\
            started_at TEXT NOT NULL, finished_at TEXT, status TEXT NOT NULL,\
            created_at TEXT NOT NULL,\
            FOREIGN KEY (plan_id) REFERENCES task_plans(id) ON DELETE CASCADE)"
    ).await?;

    // 兜底：确保 metric 表也存在
    create_table_if_missing(pool, "metric_definition",
        "CREATE TABLE IF NOT EXISTS metric_definition (\
            id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, display_name TEXT NOT NULL,\
            category TEXT NOT NULL, collector TEXT NOT NULL, description TEXT,\
            cost_level TEXT DEFAULT 'low', default_enable INTEGER DEFAULT 1)"
    ).await?;
    create_table_if_missing(pool, "metric_profile",
        "CREATE TABLE IF NOT EXISTS metric_profile (\
            id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT,\
            metric_ids TEXT NOT NULL DEFAULT '[]', user_id TEXT NOT NULL,\
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,\
            FOREIGN KEY (user_id) REFERENCES users(id))"
    ).await?;
    create_table_if_missing(pool, "task_metric_config",
        "CREATE TABLE IF NOT EXISTS task_metric_config (\
            id TEXT PRIMARY KEY, task_id TEXT NOT NULL, metric_ids TEXT NOT NULL DEFAULT '[]',\
            profile_id TEXT, FOREIGN KEY (task_id) REFERENCES test_task(id))"
    ).await?;

    // 种子数据：默认指标定义
    seed_metric_definitions(pool).await?;

    info!("数据库迁移完成（14 张表）",);
    Ok(())
}

/// 安全地创建表（如果表不存在）
async fn create_table_if_missing(pool: &SqlitePool, table: &str, create_sql: &str) -> anyhow::Result<()> {
    let exists: i32 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
    )
    .bind(table)
    .fetch_one(pool)
    .await?;
    if exists == 0 {
        sqlx::query(create_sql).execute(pool).await?;
        info!("增量迁移: 创建表 {}", table);
    }
    Ok(())
}

/// 安全地添加列（如果列不存在）
async fn add_column_if_missing(
    pool: &SqlitePool,
    table: &str,
    column: &str,
    col_def: &str,
) -> anyhow::Result<()> {
    let exists: i32 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info(?) WHERE name = ?",
    )
    .bind(table)
    .bind(column)
    .fetch_one(pool)
    .await?;
    if exists == 0 {
        let sql = format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, col_def);
        sqlx::query(&sql).execute(pool).await?;
        info!("增量迁移: 为 {}.{} 添加列", table, column);
    }
    Ok(())
}

/// 种子数据：插入默认指标定义（幂等）
async fn seed_metric_definitions(pool: &SqlitePool) -> anyhow::Result<()> {
    let count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM metric_definition")
        .fetch_one(pool).await?;
    if count > 0 { return Ok(()); }

    let defs = [
        ("dns_time","DNS解析时延","network","DnsEngine","DNS 解析耗时"),
        ("tcp_time","TCP连接时延","network","HttpEngine","TCP 握手耗时"),
        ("tls_time","TLS握手时延","network","HttpEngine","TLS 握手耗时"),
        ("ttfb","首包时延","network","HttpEngine","首字节响应时间"),
        ("http_status","HTTP状态码","network","HttpEngine","HTTP 状态码"),
        ("fcp","首屏时延(FCP)","page","PageCollector","First Contentful Paint"),
        ("dom_load","DOM加载时延","page","PageCollector","DOMContentLoaded"),
        ("load_time","首页时延","page","PageCollector","Load Event 时间"),
        ("lcp","最大内容绘制","performance","TraceCollector","Largest Contentful Paint"),
        ("cls","累计布局偏移","performance","TraceCollector","Cumulative Layout Shift"),
        ("total_size","页面总大小","resource","NetworkCollector","所有资源总大小"),
        ("html_size","HTML大小","resource","NetworkCollector","HTML 文档大小"),
        ("css_size","CSS大小","resource","NetworkCollector","样式表总大小"),
        ("js_size","JS大小","resource","NetworkCollector","脚本总大小"),
        ("img_size","图片大小","resource","NetworkCollector","图片总大小"),
        ("requests","请求数量","resource","NetworkCollector","总 HTTP 请求数"),
    ];

    for (name, display, cat, collector, desc) in &defs {
        let id = uuid::Uuid::new_v4().to_string();
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO metric_definition (id,name,display_name,category,collector,description) VALUES (?,?,?,?,?,?)"
        ).bind(&id).bind(name).bind(display).bind(cat).bind(collector).bind(desc).execute(pool).await;
    }
    info!("种子数据: 已插入 {} 条指标定义", defs.len());
    Ok(())
}
