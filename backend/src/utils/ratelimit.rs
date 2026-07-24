use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 简单的内存频率限制器
pub struct RateLimiter {
    attempts: Mutex<HashMap<String, Vec<Instant>>>,
    max_attempts: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_attempts: usize, window_secs: u64) -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
            max_attempts,
            window: Duration::from_secs(window_secs),
        }
    }

    /// 检查是否允许请求。返回 `false` 表示超出限制。
    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut map = self.attempts.lock().unwrap();

        let timestamps = map.entry(key.to_string()).or_insert_with(Vec::new);

        // 清理过期记录
        timestamps.retain(|t| now.duration_since(*t) < self.window);

        if timestamps.len() >= self.max_attempts {
            return false;
        }

        timestamps.push(now);
        true
    }
}
