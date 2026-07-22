# NetPulse Web

> 网络质量测试平台 — DNS / TCP / HTTP / 浏览器渲染 / 下载 / 视频 一站式探测

## 功能

| 测试类型 | 采集指标 |
|----------|----------|
| **Ping** (连通性) | DNS 解析时延、成功率、TCP 连接时延、IP 地址 |
| **Website** (网站) | DNS / TCP / TLS / TTFB / FP / FCP / LCP / CLS / TTI / DOMContentLoaded / Load / 资源分类型统计(html/css/js/img/font) / 页面大小 / 页面速度 / 首屏占比 / 截图 |
| **Download** (下载) | DNS / TCP / 下载速率(KB/s→Mbps) / 峰值速度 / 成功率 |
| **Video** (视频) | DNS / TCP / HTTP 响应 / 首次播放时延 / 缓冲次数 / 卡顿次数 / 丢解码帧 / 解码帧数 / 分辨率 / 码率 / 编码格式 / 截图 |

## 架构

```
 ┌──────────────┐
 │  Chromium    │
 │  (CDP)       │
 └──────┬───────┘
        │
 ┌──────┴───────┐
 │  Website     │
 │  Engine      │
 │ (chromiumox.)│
 └──────────────┘

 ┌──────────────┐
 │  Video       │
 │  Engine      │
 │ (chromiumox.)│
 └──────────────┘
```

Web 引擎和视频引擎共用同一套 chromiumoxide CDP 库，只需一个 Chromium 进程。

- **引擎层**（Website / Video / Download / Ping / DNS / HTTP）可独立并行运行
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
# config.toml (示例, 完整参见 backend/config.toml)

[server]
host = "0.0.0.0"
port = 3000

[database]
path = "./data/netpulse.db"

[logging]
level = "info"
file_dir = "./logs"
format = "console"
console = true
file = true

[browser]
path = "/usr/bin/chromium"
headless = true

[video_browser]
path = "/usr/bin/chromium"
headless = true

[task]
concurrency = 5
timeout_seconds = 120

[storage]
screenshot_dir = "./data/screenshots"
excel_dir = "./data/excel"

[jwt]
secret = "netpulse-jwt-secret-change-in-production"
expiration_hours = 24

# 视频平台配置（平台检测走代码逻辑，不依赖 video_selector / wait_seconds）
[[video_platforms]]
name = "youtube"
url_keywords = ["youtube.com", "youtu.be"]

[[video_platforms]]
name = "bilibili"
url_keywords = ["bilibili.com", "b23.tv"]

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
| GET | `/api/health` | 健康检查 |
| POST | `/api/auth/register` | 注册 |
| POST | `/api/auth/login` | 登录 |
| GET | `/api/dashboard/stats` | 仪表盘统计 |
| POST | `/api/task/create` | 创建任务 |
| GET | `/api/task/list` | 任务列表 |
| GET | `/api/task/:id` | 任务详情 |
| DELETE | `/api/task/:id` | 删除任务 |
| POST | `/api/task/batch-delete` | 批量删除 |
| GET | `/api/task/:id/logs` | 任务日志 |
| GET | `/api/task/:id/result` | 网站测试结果 |
| GET | `/api/task/:id/video-result` | 视频测试结果 |
| GET | `/api/task/:id/download-result` | 下载测试结果 |
| GET | `/api/task/:id/ping-result` | Ping 测试结果 |
| GET | `/api/task/:id/export` | 导出 (xlsx/csv/json) |
| POST | `/api/task/import` | 批量导入 |
| GET | `/api/task/template` | 模板下载 |
| POST | `/api/task/:id/cancel` | 取消 |
| POST | `/api/task/:id/retry` | 重试 |
| GET | `/api/metrics` | 指标定义列表 |
| POST | `/api/plan/create` | 创建计划 |
| GET | `/api/plan/list` | 计划列表 |
| GET | `/api/plan/:id` | 计划详情 |
| POST | `/api/plan/:id/update` | 更新计划 |
| POST | `/api/plan/:id/delete` | 删除计划 |
| POST | `/api/plan/:id/run` | 手动执行计划 |
| GET | `/api/plan/:id/runs` | 计划运行历史 |
| POST | `/api/plan/:id/run/:run_id/delete` | 删除运行记录 |
| GET | `/api/plan/:id/run/:run_id/export` | 计划运行合并导出 |
| GET | `/api/admin/users` | 用户列表 |
| POST | `/api/admin/users/role` | 修改用户角色 |
| DELETE | `/api/admin/users/:id` | 删除用户 |
| WS | `/api/ws` | WebSocket 实时推送 |

## 技术栈

| 层 | 技术 |
|----|------|
| 后端 | Rust / Axum / Tokio / SQLx (SQLite) |
| 前端 | Vue 3 / Naive UI / ECharts / TypeScript |
| 浏览器 | chromiumoxide (CDP) / Chromium |
| 容器 | Docker / docker-compose |
| CI | GitHub Actions → ghcr.io |

## License

MIT
