use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use chromiumoxide::{Browser, BrowserConfig as ChromeBrowserConfig};
use futures::StreamExt;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};
use uuid::Uuid;

pub enum ChromiumArg {
    Key(&'static str),
    Value(&'static str, &'static str),
}

pub struct ChromiumSession {
    browser: Browser,
    handler: JoinHandle<()>,
    temporary_user_data_dir: Option<PathBuf>,
    kind: &'static str,
}

impl ChromiumSession {
    pub async fn launch(
        kind: &'static str,
        path: &str,
        headless: bool,
        user_data_dir: Option<PathBuf>,
        extra_args: &[ChromiumArg],
    ) -> anyhow::Result<Self> {
        let temporary = user_data_dir.is_none();
        let profile_dir = user_data_dir.unwrap_or_else(|| {
            std::env::temp_dir().join(format!("netpulse-chromium-{kind}-{}", Uuid::new_v4()))
        });
        std::fs::create_dir_all(&profile_dir).with_context(|| {
            format!("创建 Chromium profile 目录失败: {}", profile_dir.display())
        })?;

        info!(
            "使用 Chromium profile: type={}, temporary={}, path={}",
            kind,
            temporary,
            profile_dir.display()
        );

        let mut builder = ChromeBrowserConfig::builder()
            .no_sandbox()
            .window_size(1920, 1080)
            .chrome_executable(path)
            .user_data_dir(&profile_dir)
            .arg(("log-level", "0"));

        if !headless {
            builder = builder.with_head();
        }

        for arg in extra_args {
            builder = match arg {
                ChromiumArg::Key(key) => builder.arg(*key),
                ChromiumArg::Value(key, value) => builder.arg((*key, *value)),
            };
        }

        let launch_config = match builder.build() {
            Ok(config) => config,
            Err(error) => {
                if temporary {
                    let _ = std::fs::remove_dir_all(&profile_dir);
                }
                return Err(anyhow::anyhow!("构建 BrowserConfig 失败: {}", error));
            }
        };

        let (browser, mut handler) = match Browser::launch(launch_config)
            .await
            .with_context(|| format!("{kind} Chromiumoxide 启动失败"))
        {
            Ok(result) => result,
            Err(error) => {
                if temporary {
                    let _ = std::fs::remove_dir_all(&profile_dir);
                }
                return Err(error);
            }
        };

        let handler_kind = kind;
        let handler_task = tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                if let Err(error) = event {
                    warn!(
                        "Chromium handler 异常: type={}, error={}",
                        handler_kind, error
                    );
                    break;
                }
            }
        });

        info!("Chromium 启动成功: type={}", kind);
        Ok(Self {
            browser,
            handler: handler_task,
            temporary_user_data_dir: temporary.then_some(profile_dir),
            kind,
        })
    }

    pub fn browser(&self) -> &Browser {
        &self.browser
    }

    pub async fn shutdown(self) {
        let Self {
            mut browser,
            mut handler,
            temporary_user_data_dir,
            kind,
        } = self;

        match tokio::time::timeout(Duration::from_secs(10), browser.close()).await {
            Ok(Ok(_)) => debug!("Chromium close 请求完成: type={}", kind),
            Ok(Err(error)) => warn!("Chromium close 请求失败: type={}, error={:#}", kind, error),
            Err(_) => warn!("Chromium close 请求超时: type={}", kind),
        }

        let exited = match tokio::time::timeout(Duration::from_secs(10), browser.wait()).await {
            Ok(Ok(Some(status))) => {
                debug!("Chromium 进程退出: type={}, status={}", kind, status);
                true
            }
            Ok(Ok(None)) => true,
            Ok(Err(error)) => {
                warn!("等待 Chromium 进程退出失败: type={}, error={}", kind, error);
                false
            }
            Err(_) => {
                warn!("等待 Chromium 进程退出超时: type={}", kind);
                false
            }
        };

        if !exited {
            match browser.kill().await {
                Some(Ok(())) => debug!("Chromium 进程已强制结束: type={}", kind),
                Some(Err(error)) => warn!("强制结束 Chromium 失败: type={}, error={}", kind, error),
                None => {}
            }
        }

        match tokio::time::timeout(Duration::from_secs(2), &mut handler).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => warn!("Chromium handler join 失败: type={}, error={}", kind, error),
            Err(_) => {
                warn!("Chromium handler 未及时退出，已中止: type={}", kind);
                handler.abort();
                let _ = handler.await;
            }
        }

        if let Some(dir) = temporary_user_data_dir {
            match std::fs::remove_dir_all(&dir) {
                Ok(()) => debug!("已删除临时 Chromium profile: path={}", dir.display()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => warn!(
                    "删除临时 Chromium profile 失败: path={}, error={}",
                    dir.display(),
                    error
                ),
            }
        }

        info!("Chromium 关闭完成: type={}", kind);
    }
}
