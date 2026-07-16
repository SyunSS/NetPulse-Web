# NetPulse Web

> 网络质量测试平台 — DNS / TCP / HTTP / 浏览器渲染 / 下载 / 视频 一站式探测

## 功能

| 测试类型 | 采集指标 |
|----------|----------|
| **Ping** (连通性) | DNS 解析时延、成功率、TCP 连接时延、IP 地址 |
| **Website** (网站) | DNS / TCP / TLS / TTFB / DOMContentLoaded(首屏) / Load(首页) / 截图 |
| **Download** (下载) | DNS / TCP / 下载速率(KB/s→Mbps) / 峰值速度 / 成功率 |
| **Video** (视频) | DNS / TCP / HTTP 响应 / 首次播放时延 / 缓冲次数 / 丢解码帧 / 截图 |

## 架构

```
                  ┌──────────────┐
                  │   Browser    │
  ┌─────────┐     │  Provider    │     ┌─────────────────┐
  │ Website │────▶│   (trait)    │────▶│ HeadlessChrome  │
  │  Engine │     │              │     │   Provider      │
  └─────────┘     │ - launch     │     └─────────────────┘
                  │ - new_page   │
  ┌─────────┐     │ - navigate   │     ┌─────────────────┐
  │  Video  │────▶│ - eval_sync  │     │ Chromiumoxide   │
  │  Engine │     │ - screenshot │     │   (桩)          │
  └─────────┘     └──────────────┘     └─────────────────┘
```

- **引擎层**（Website / Video / Download / Ping / DNS / HTTP）不依赖任何具体浏览器 crate
- **BrowserProvider trait** 抽象所有 CDP 操作，支持切换后端（headless_chrome / chromiumoxide / Playwright）
- **Worker** 通过 mpsc channel 接收任务，broadcast 推送 WebSocket 实时进度
- **PlanScheduler** 支持 cron 定时计划，自动执行 + 合并导出

## 快速开始

### Docker

```bash
# 拉取并启动
docker compose up -d

# 查看日志
docker compose logs -f

# 停止
docker compose down
```

浏览器打开 `http://localhost:3000`。

### 从源码构建

```bash
# 后端
cd backend
cargo build --release

# 前端
cd frontend
pnpm install && pnpm build

# 启动 (需安装 Chromium)
./target/release/netpulse-web
```

## 配置

```toml
# config.toml

[server]
host = "0.0.0.0"
port = 3000

[browser]
provider = "headless_chrome"   # headless_chrome | chromiumoxide
path = "/usr/bin/chromium"
headless = true

[task]
concurrency = 5
timeout_seconds = 120

[database]
path = "./data/netpulse.db"

[jwt]
secret = "change-me-in-production"
expiration_hours = 24

# 视频平台（可选，未匹配的走 html5 兜底）
[[video_platforms]]
name = "bilibili"
url_keywords = ["bilibili.com", "b23.tv"]
video_selector = "video.bpx-player-video"
wait_seconds = 5

[[video_platforms]]
name = "netflix"
url_keywords = ["netflix.com"]
detect_only = true
```

## 导入文件格式

支持在「创建任务」页面或「测试计划」页面导入批量 URL。

### JSON 格式 (POST /api/task/import)

```json
{
  "tasks": [
    {
      "task_type": "ping",
      "urls": ["lobby-prod-b.df.qq.com", "1.1.1.1:443"],
      "options": { "repeat_count": 3 }
    },
    {
      "task_type": "website",
      "urls": ["https://www.baidu.com"],
      "options": { "repeat_count": 2 }
    },
    {
      "task_type": "download",
      "urls": ["http://speedtest.tele2.net/1MB.zip"],
      "options": { "repeat_count": 1 }
    }
  ]
}
```

### 纯文本格式（计划导入）

```
[website]
https://www.baidu.com
https://github.com

[ping]
lobby-prod-b.df.qq.com
receiver.tdm.qq.com
1.1.1.1:443

[download]
http://speedtest.tele2.net/1MB.zip

[video]
https://www.bilibili.com/video/BV1GJ411x7h7
```

页面提供 **📥 下载模板** 按钮，一键获取示例文件。

## API

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/auth/register` | 注册 |
| POST | `/api/auth/login` | 登录 |
| GET | `/api/health` | 健康检查 |
| POST | `/api/task/create` | 创建任务 |
| GET | `/api/task/list` | 任务列表 |
| GET | `/api/task/:id` | 任务详情 |
| GET | `/api/task/:id/result` | 网站结果 |
| GET | `/api/task/:id/export` | 导出 (xlsx/csv/json) |
| POST | `/api/task/import` | 批量导入 |
| GET | `/api/task/template` | 模板下载 |
| POST | `/api/task/:id/cancel` | 取消 |
| POST | `/api/task/:id/retry` | 重试 |
| CRUD | `/api/plan` | 定时计划管理 |
| GET | `/api/plan/:id/runs/:rid/export` | 计划运行合并导出 |

## 技术栈

| 层 | 技术 |
|----|------|
| 后端 | Rust / Axum / Tokio / SQLx (SQLite) |
| 前端 | Vue 3 / Naive UI / ECharts / TypeScript |
| 浏览器 | headless_chrome (CDP) / Chromium |
| 容器 | Docker / docker-compose |
| CI | GitHub Actions → ghcr.io |

## 浏览器后端切换

```toml
# 默认 (无需配置)
[browser]
provider = "headless_chrome"

# 未来可切换
[browser]
provider = "chromiumoxide"
```

引擎层零改动，只需改一行配置。新增后端只需实现 `BrowserProvider` trait。

## License

MIT
