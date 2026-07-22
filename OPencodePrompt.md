# NetPulse Web — 完整项目上下文提示词

## 项目简介

NetPulse Web 是一个**网络质量测试平台**，支持对多个目标进行批量化的 Ping / Website / Video / Download 测试，通过 Chromium CDP 采集详细性能指标，并提供 Web 管理界面、Excel/CSV/JSON 导出、定时计划任务等功能。

---

## 技术栈

| 层 | 技术 | 版本 |
|----|------|------|
| 后端主框架 | Rust + Cargo | edition 2021 |
| Web 框架 | axum 0.7 | macros/ws 特性 |
| 数据库 | SQLite (sqlx 0.8) | 异步 |
| 浏览器 CDP | headless_chrome 1.0 | 自动生成 CDP 代码 |
| 序列化 | serde / serde_json | — |
| 日志 | tracing + tracing-subscriber + tracing-appender | — |
| 前端框架 | Vue 3.5 + TypeScript 6.0 | — |
| UI 库 | naive-ui 2.44 | — |
| 图表 | echarts 6.1 + vue-echarts | — |
| 状态管理 | pinia | — |
| 路由 | vue-router | — |
| 打包 | vite 8.1 + vue-tsc | — |
| HTTP | axios | — |
| Excel | rust_xlsxwriter (后端) | — |
| 密码 | bcrypt + JWT (jsonwebtoken) | — |
| 定时 | cron crate (后端) | — |

---

## 目录结构

```
netpulse-web/
├── backend/
│   ├── Cargo.toml
│   ├── config.toml          # 主配置: 服务器/数据库/日志/浏览器/视频平台
│   └── src/
│       ├── main.rs          # 入口: 日志初始化 + 路由注册 + WebSocket + Scheduler
│       ├── config/
│       │   └── mod.rs       # AppConfig 结构体 + 配置加载
│       ├── database/
│       │   └── mod.rs       # 数据库连接 + migration (自动建表 + 增量加列)
│       ├── models/
│       │   ├── mod.rs
│       │   ├── user.rs      # User 模型
│       │   ├── task.rs      # TestTask / WebsiteResult / VideoResult / DownloadResult / PingResult / TaskLog
│       │   ├── setting.rs   # UserSetting
│       │   ├── plan.rs      # TaskPlan / TaskPlanRun / PlanItemInput
│       │   └── metrics.rs   # MetricDefinition / MetricConfig
│       ├── services/
│       │   ├── mod.rs
│       │   ├── auth_service.rs   # JWT 认证
│       │   ├── task_service.rs   # 任务 CRUD + 取消
│       │   └── plan_service.rs   # 计划 CRUD + 执行 + 删除
│       ├── api/
│       │   ├── mod.rs       # 路由聚合 (public + protected)
│       │   ├── auth.rs      # /api/auth/login /register /profile
│       │   ├── task.rs      # /api/task/* (创建/列表/详情/结果/导出/取消/重试/删除/批量删除/logs)
│       │   ├── plan.rs      # /api/plan/* (CRUD + 运行 + 导入/导出 + 历史)
│       │   ├── admin.rs     # /api/admin/* (配置/代理)
│       │   ├── metrics.rs   # GET /api/metrics (指标定义列表)
│       │   └── ws.rs        # WebSocket 推送 (进度/日志)
│       ├── worker/
│       │   └── mod.rs       # 任务执行引擎:
│       │       ├── run_website_task()   # 网站测试 (BrowserEngine)
│       │       ├── run_video_task()     # 视频测试 (VideoEngine)
│       │       ├── run_download_task()  # 下载测试 (DownloadEngine)
│       │       ├── run_ping_task()      # Ping 测试 (PingEngine 含 ICMP→TCP回退)
│       │       ├── save_*_result()      # 各类型结果存 DB
│       │       └── log_progress()       # 日志写入 DB + WS 推送 + tracing
│       ├── scheduler/
│       │   └── mod.rs       # 定时计划调度器 (cron 解析 + 任务提交)
│       ├── engines/
│       │   ├── mod.rs
│       │   ├── dns/         # DnsEngine (DNS 解析 + 时延)
│       │   ├── http/        # HttpEngine (HTTP 探测 + TCP/TLS 时延)
│       │   ├── download/    # DownloadEngine (文件下载测速 + TCP 探测)
│       │   ├── ping/        # PingEngine (ICMP ping + TCP:80/443 回退 + 抖动)
│       │   ├── browser/
│       │   │   ├── mod.rs        # BrowserEngine: test_page()
│       │   │   │   → 注入 LCP Observer
│       │   │   │   → Navigation Timing API 采集 25+ 指标
│       │   │   │   → NetworkCollector 资源统计
│       │   │   ├── collectors.rs  # PageCollector + NetworkCollector
│       │   │   │   → collect_js(): 25+ 性能指标 JS
│       │   │   │   → lcp_inject_js(): LCP PerformanceObserver
│       │   │   └── provider.rs    # BrowserProvider / BrowserPage trait
│       │   └── video/
│       │       ├── mod.rs             # VideoEngine: test_page()
│       │       │   → autoplay-policy + mute 启动参数
│       │       │   → PagePreprocessor 弹窗处理
│       │       │   → PlaybackController 多级触发
│       │       │   → 轮询监控 首帧/卡顿/分辨率
│       │       ├── cdp_handler.rs    # MediaCollector + NetworkCollector (CDP 事件)
│       │       ├── collector.rs      # MediaSnapshot / NetworkSnapshot 结构
│       │       ├── monitor.rs        # 视频轮询监控 JS (首帧/卡顿/播放时长)
│       │       └── playback.rs       # PlaybackController + VideoDiagnostics
│       ├── report/
│       │   └── excel/mod.rs  # 各类型 Excel/CSV/JSON 导出
│       ├── storage/
│       │   └── mod.rs        # 文件存储 (截图/Excel)
│       └── utils/
│           ├── mod.rs
│           └── response.rs   # ApiResponse / AppError / AppState / ok() 等
│
└── frontend/
    ├── package.json
    ├── vite.config.ts
    ├── tsconfig.json
    └── src/
        ├── main.ts           # 入口
        ├── App.vue           # 根组件 + 路由
        ├── router/
        │   └── index.ts      # 路由定义
        ├── stores/
        │   ├── auth.ts       # 认证状态
        │   └── plan.ts       # 计划状态
        ├── api/
        │   ├── index.ts      # axios 实例 + 拦截器
        │   ├── auth.ts       # 认证 API
        │   ├── task.ts       # 任务 API + 类型定义
        │   ├── plan.ts       # 计划 API + 类型定义
        │   └── ws.ts         # WebSocket 客户端
        ├── views/
        │   ├── Login.vue     # 登录
        │   ├── Register.vue  # 注册
        │   ├── Dashboard.vue # 仪表盘
        │   ├── CreateTask.vue# 创建单次任务 (含 MetricSelector)
        │   ├── TaskDetail.vue# 任务详情 (结果表格 + 日志面板 + 导出)
        │   ├── History.vue   # 历史记录 (多选/批量删除/强制删除)
        │   ├── PlanEdit.vue  # 计划编辑 (导入/手动添加 + MetricSelector)
        │   ├── PlanRuns.vue  # 计划运行历史 (停止/强制删除)
        │   └── Profile.vue   # 用户设置
        ├── components/
        │   └── MetricSelector.vue  # 指标多选组件 (4 组 19 项)
        └── utils/
            └── index.ts      # 工具函数
```

---

## 数据库表 (SQLite, 自动迁移)

```
users             — 用户
user_settings     — 设置
test_task         — 任务 (task_type: ping/website/video/download)
task_log          — 日志 (level/message/created_at)
website_result    — 网站结果 (35 列)
video_result      — 视频结果 (31 列)
download_result   — 下载结果 (15 列)
ping_result       — Ping 结果 (11 列)
task_plans        — 计划
task_plan_items   — 计划项 (每个项一个类型+ URL 列表 + options JSON)
task_plan_runs    — 运行记录 (含 task_ids JSON)
metric_definition — 指标定义 (16 条种子数据)
metric_profile    — 指标模板
task_metric_config— 任务-指标关联
```

`database/mod.rs::init_db()` 用 `sqlx::query("CREATE TABLE IF NOT EXISTS ...")` 创建 + `add_column_if_missing()` 增量加列，不依赖 ORM。

---

## 关键功能

### 1. 网站测试 (BrowserEngine)
- 启动 Chromium headless → 注入 LCP Observer → 导航 → 等待5s → JS 采集 25+ 指标
- Navigation Timing API 取 DNS/TCP/TTFB/FP/FCP/DCL/LCP/资源分类大小
- 首屏比例 / 平均下载速率 / 总大小 衍生指标
- HTML/CSS/JS/图片/字体分类统计 (transferSize)
- 可配置指标选择 (MetricSelector: 19 项指标, 4 组)

### 2. 视频测试 (VideoEngine)
- autoplay-policy + mute 启动参数 → CDP Media.enable → PagePreprocess 弹窗
- PlaybackController 三级触发: Auto→Click→Keyboard (最长 60s)
- 轮询监控: 1s 间隔检查 currentTime → 首帧时间 / 卡顿计数 / 分辨率
- Media Collector: 编解码 / 码率 / 丢帧 / 播放器类型
- Network Collector: 媒体分片 / 下载速率 / CDN 节点

### 3. Ping 测试 (PingEngine)
- ICMP ping (-c N, 可配置) → TCP:80 → TCP:443 三级回退
- 丢包率/抖动/时延 采集
- method 字段标记检测方式 (icmp/tcp80/tcp443)

### 4. 下载测试 (DownloadEngine)
- DNS 解析 + TCP 连接 + 文件下载 (tokio::net::TcpStream)
- 实时速率 / 平均 / 峰值

### 5. 计划任务 (Scheduler)
- 基于 cron 表达式的定时调度
- 计划导入/导出 (TXT/Excel)
- 计划运行历史管理 (强制停止删除子任务)

### 6. 指标选择系统
- MetricSelector 组件: 基础网络/页面性能/资源统计/高级性能
- 指标定义存储在 metric_definition 表
- 任务/计划均支持指标选择

### 7. 日志系统
- tracing + tracing-subscriber + tracing-appender 每日滚动
- config.toml 控制: level/format/console/file
- task_log 表存结构化日志
- GET /api/task/:id/logs 端点
- TaskDetail 面板显示

### 8. 导出
- 单任务: Excel / CSV / JSON
- 计划运行: Excel (多sheet) / CSV (多section) / JSON

---

## 沙盒环境安装说明

**所有依赖必须在容器沙盒内安装，不得安装到宿主机系统。**

### 后端 (Rust)

```bash
# 沙盒内只需编译（所有 crate 自动下载到 ~/.cargo）
cd backend
cargo build --release

# 运行
cargo run --release
# 或直接运行二进制
target/release/netpulse-web
```

```toml
# Cargo.toml 依赖 (已完整列出)
[dependencies]
# 见上方技术栈表。全部在 ~/.cargo 内，不侵入系统。
```

### 前端 (Node.js + Vite)

```bash
cd frontend

# 安装依赖 (沙盒内)
pnpm install

# 开发
pnpm dev

# 构建
pnpm build
```

### Chromium 依赖

后端依赖 `headless_chrome` crate，需要 Chromium 二进制。**安装命令必须在沙盒内执行**：

```bash
# 沙盒内安装 Chromium (不暴露到宿主机)
apt-get update && apt-get install -y chromium

# 或使用浏览器自动化器的内置 chromedriver
```

Chromium 路径在 `config.toml` 的 `[browser].path` 配置，默认 `/usr/bin/chromium`。

### 数据库

SQLite，运行后在 `[database].path` 路径自动创建。无需单独安装。

---

## 配置说明

`backend/config.toml`:

```toml
[server]
host = "0.0.0.0"
port = 3000

[database]
path = "./data/netpulse.db"

[logging]
level = "info"          # trace | debug | info | warn | error
format = "console"      # console | json
console = true
file = true
file_dir = "./logs"

[browser]
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

[[video_platforms]]
name = "youtube"
url_keywords = ["youtube.com", "youtu.be"]

[[video_platforms]]
name = "bilibili"
url_keywords = ["bilibili.com", "b23.tv"]

[[video_platforms]]
name = "html5"
url_keywords = []
```

---

## 端口说明

- 后端 HTTP Server: `3000` (默认)
- 后端同时提供静态文件服务（`/` 路由，代理前端构建产物）
- 前端开发服务器: `5173` (Vite 默认)

---

## 项目状态

- 完成: 数据库迁移、基础 CRUD API、四种测试引擎、WebSocket 实时进度、多级 Ping 回退、LCP/FP/FCP 采集、命名 Timing 采集、首屏比例/下载速率计算、PlaybackController、视频轮询监控、卡顿/首帧检测、定时计划任务、指标选择系统、Excel/CSV/JSON 导出、用户认证、任务日志系统、批量删除/强制删除、文件滚动日志
