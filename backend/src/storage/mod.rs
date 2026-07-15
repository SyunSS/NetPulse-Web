use std::path::Path;

use tracing::info;

/// 存储管理器 — 负责截图等文件的保存
pub struct StorageManager;

impl StorageManager {
    /// 确保目录存在
    pub fn ensure_dir(path: &str) -> anyhow::Result<()> {
        let p = Path::new(path);
        if !p.exists() {
            std::fs::create_dir_all(p)?;
            info!("创建目录: {}", path);
        }
        Ok(())
    }

    /// 保存截图
    ///
    /// 返回保存的相对路径（相对于 screenshot_dir）
    pub fn save_screenshot(
        screenshot_dir: &str,
        task_id: &str,
        url: &str,
        data: &[u8],
    ) -> anyhow::Result<String> {
        let task_dir = format!("{}/{}", screenshot_dir, task_id);
        Self::ensure_dir(&task_dir)?;

        let safe_name = sanitize_url_for_filename(url);
        let filename = format!("{}/{}.png", task_dir, safe_name);

        std::fs::write(&filename, data)?;
        info!("截图已保存: {} ({} bytes)", filename, data.len());

        // 返回相对路径
        Ok(format!("{}/{}.png", task_id, safe_name))
    }

    /// 获取截图完整路径
    pub fn get_screenshot_path(screenshot_dir: &str, relative_path: &str) -> String {
        format!("{}/{}", screenshot_dir, relative_path)
    }
}

/// 将 URL 转换为安全的文件名
fn sanitize_url_for_filename(url: &str) -> String {
    url.replace("://", "_")
        .replace("/", "_")
        .replace(":", "_")
        .replace("?", "_")
        .replace("&", "_")
        .replace("=", "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '.' || *c == '-')
        .collect()
}
