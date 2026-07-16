use config::{Config, File};
use serde::Deserialize;

/// 应用配置
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub logging: LoggingConfig,
    pub browser: BrowserConfig,
    pub task: TaskConfig,
    pub storage: StorageConfig,
    pub jwt: JwtConfig,
    pub video_platforms: Vec<VideoPlatformConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VideoPlatformConfig {
    pub name: String,
    #[serde(default)]
    pub url_keywords: Vec<String>,
    #[serde(default)]
    pub video_selector: Option<String>,
    #[serde(default)]
    pub wait_seconds: Option<u64>,
    #[serde(default)]
    pub detect_only: Option<bool>,
}

impl VideoPlatformConfig {
    /// 判断 URL 是否匹配该平台
    pub fn matches_url(&self, url: &str) -> bool {
        if self.url_keywords.is_empty() {
            return false;
        }
        let lower = url.to_lowercase();
        self.url_keywords.iter().any(|kw| lower.contains(&kw.to_lowercase()))
    }

    /// 是否需要仅检测可访问性
    pub fn is_detect_only(&self) -> bool {
        self.detect_only.unwrap_or(false)
    }
}

/// 根据 URL 匹配平台配置
pub fn match_platform(platforms: &[VideoPlatformConfig], url: &str) -> VideoPlatformConfig {
    platforms
        .iter()
        .find(|p| p.matches_url(url))
        .cloned()
        .unwrap_or_else(|| {
            platforms
                .iter()
                .find(|p| p.name == "html5")
                .cloned()
                .unwrap_or_else(|| VideoPlatformConfig {
                    name: "html5".to_string(),
                    url_keywords: vec![],
                    video_selector: Some("video".to_string()),
                    wait_seconds: Some(1),
                    detect_only: None,
                })
        })
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BrowserConfig {
    pub path: String,
    pub headless: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TaskConfig {
    pub concurrency: usize,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    pub screenshot_dir: String,
    pub excel_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: i64,
}

impl AppConfig {
    /// 加载配置文件
    pub fn load() -> anyhow::Result<Self> {
        let config = Config::builder()
            .add_source(File::with_name("config.toml").required(false))
            .add_source(config::Environment::with_prefix("NETPULSE").separator("__"))
            .build()?;

        let app_config: AppConfig = config.try_deserialize()?;
        Ok(app_config)
    }

    /// 获取服务器监听地址
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    /// 数据库路径
    pub fn database_path(&self) -> &str {
        &self.database.path
    }
}
