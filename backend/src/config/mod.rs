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
    pub video_browser: VideoBrowserConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VideoPlatformConfig {
    pub name: String,
    #[serde(default)]
    pub url_keywords: Vec<String>,
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
        self.url_keywords
            .iter()
            .any(|kw| lower.contains(&kw.to_lowercase()))
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
        .or_else(|| builtin_platform(url))
        .unwrap_or_else(|| VideoPlatformConfig {
            name: "html5".to_string(),
            url_keywords: vec![],
            detect_only: None,
        })
}

fn builtin_platform(url: &str) -> Option<VideoPlatformConfig> {
    let lower = url.to_lowercase();
    let (name, keywords) = if lower.contains("youtube.com") || lower.contains("youtu.be") {
        ("youtube", vec!["youtube.com", "youtu.be"])
    } else if lower.contains("bilibili.com") || lower.contains("b23.tv") {
        ("bilibili", vec!["bilibili.com", "b23.tv"])
    } else if lower.contains("sohu.com") {
        ("sohu", vec!["sohu.com"])
    } else if lower.contains("youku.com") || lower.contains("tudou.com") {
        ("youku", vec!["youku.com", "tudou.com"])
    } else if lower.contains("mgtv.com") || lower.contains("hunantv.com") {
        ("mgtv", vec!["mgtv.com", "hunantv.com"])
    } else if lower.contains("pptv.com") || lower.contains("pps.tv") {
        ("pptv", vec!["pptv.com", "pps.tv"])
    } else if lower.contains("v.qq.com") || lower.contains("video.qq.com") {
        ("qq", vec!["v.qq.com", "video.qq.com"])
    } else {
        return None;
    };

    Some(VideoPlatformConfig {
        name: name.to_string(),
        url_keywords: keywords.into_iter().map(str::to_string).collect(),
        detect_only: None,
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    /// Proxies whose forwarded client headers may be trusted.
    #[serde(default)]
    pub trusted_proxies: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_dir: String,
    #[serde(default = "default_log_format")]
    pub format: String, // "console" | "json"
    #[serde(default = "default_true")]
    pub console: bool,
    #[serde(default = "default_true")]
    pub file: bool,
}
fn default_log_format() -> String {
    "console".to_string()
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub struct BrowserConfig {
    pub path: String,
    pub headless: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VideoBrowserConfig {
    pub path: String,
    pub headless: bool,
    #[serde(default)]
    pub user_data_dir: Option<String>,
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
    #[serde(default = "default_secure_dir")]
    pub secure_dir: String,
}

fn default_secure_dir() -> String {
    "./storage".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: i64,
}

impl AppConfig {
    /// 加载配置文件（优先 config/config.toml，兜底 config.toml）
    pub fn load() -> anyhow::Result<Self> {
        let config_path = if std::path::Path::new("config/config.toml").exists() {
            "config/config.toml"
        } else {
            "config.toml"
        };
        let config = Config::builder()
            .add_source(File::with_name(config_path).required(false))
            .add_source(config::Environment::with_prefix("NETPULSE").separator("__"))
            .build()?;

        let app_config: AppConfig = config.try_deserialize()?;
        app_config.validate()?;
        Ok(app_config)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        const PLACEHOLDER: &str = "netpulse-jwt-secret-change-in-production";
        if self.jwt.secret == PLACEHOLDER {
            anyhow::bail!("JWT secret must be changed from the default placeholder");
        }
        if self.jwt.secret.as_bytes().len() < 32 {
            anyhow::bail!("JWT secret must be at least 32 bytes");
        }
        for proxy in &self.server.trusted_proxies {
            let valid = proxy.parse::<std::net::IpAddr>().is_ok()
                || proxy
                    .split_once('/')
                    .and_then(|(network, prefix)| {
                        let network = network.parse::<std::net::IpAddr>().ok()?;
                        let prefix = prefix.parse::<u8>().ok()?;
                        match network {
                            std::net::IpAddr::V4(_) if prefix <= 32 => Some(()),
                            std::net::IpAddr::V6(_) if prefix <= 128 => Some(()),
                            _ => None,
                        }
                    })
                    .is_some();
            if !valid {
                anyhow::bail!("invalid trusted proxy address or CIDR: {proxy}");
            }
        }
        Ok(())
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
