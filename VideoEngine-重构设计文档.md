# VideoEngine 重构设计文档（第一阶段：方案设计）

---

## 1. Chromium/CDP 技术选型方案

### 1.1 当前依赖分析

项目已使用 `headless_chrome = "1.0"`（实际 1.0.22），依赖链：

```
headless_chrome
  └── auto_generate_cdp (v0.4.6)  ← 从 Chrome CDP JSON 自动生成 Rust 绑定
        └── 49 个 CDP 域全部自动生成（含 Media / Network / Performance）
  └── tungstenite (v0.29)           ← WebSocket 连接
```

### 1.2 方案对比

| 维度 | headless_chrome (当前) | chromiumoxide | 直接 CDP Client |
|------|----------------------|---------------|-----------------|
| CDP 域覆盖 | **49 域全部** | 部分 | 需手动实现 |
| Media 域 | ✅ 自动生成 5 个事件 | ❌ 未实现 | 需手写 |
| Network 域 | ✅ 43 个事件 | 部分 | 需手写 |
| Performance 域 | ✅ 完整 | 部分 | 需手写 |
| 事件订阅 | ✅ `add_event_listener()` | ✅ | 需手写 |
| 原始 CDP 调用 | ✅ `tab.call_method()` | ✅ | ✅ |
| Rust 生态成熟度 | **1.0 稳定版** | 0.42 beta | - |
| 社区活跃度 | **1.8k stars** | 0.7k stars | - |
| 项目已有集成 | ✅ BrowserProvider trait 已实现 | ❌ 需从零实现 Provider | ❌ 需从零实现 |
| 迁移成本 | **零**（只扩展 trait） | 高（需新 Provider） | 极高 |

### 1.3 选择结论：**继续使用 headless_chrome，扩展 BrowserProvider trait**

**理由：**

1. `headless_chrome 1.0.22` 通过 `auto_generate_cdp` 自动生成了 Chrome 全部 49 个 CDP 域，包括 Media、Network、Performance、WebAudio、PerformanceTimeline 等
2. `Tab::call_method()` 可直接调用任何 CDP 方法（`Media::Enable`、`Network::Enable`、`Performance::Enable`）
3. `Tab::add_event_listener()` 可订阅任何 CDP 事件，包括 `Media.playerCreated`、`Network.dataReceived` 等
4. 项目已有 `HeadlessChromeProvider` 实现，只需扩展 `BrowserPage` trait 添加 CDP 方法，**无需替换底层**
5. `chromiumoxide` 不支持 Media 域，直接淘汰

### 1.4 BrowserPage trait 扩展方案

当前 trait（4 个方法）→ 扩展为：

```rust
#[async_trait]
pub trait BrowserPage: Send {
    // --- 已有 ---
    async fn navigate_to(&self, url: &str) -> anyhow::Result<()>;
    async fn wait_for_load(&self) -> anyhow::Result<()>;
    fn evaluate_sync(&self, js: &str) -> anyhow::Result<serde_json::Value>;
    fn screenshot(&self) -> anyhow::Result<Vec<u8>>;

    // --- 新增 CDP 域控制 ---
    fn enable_domain(&self, domain: CdpDomain) -> anyhow::Result<()>;

    // --- 新增 CDP 事件订阅 ---
    fn subscribe_events(&self, handler: Box<dyn CdpEventHandler>) -> anyhow::Result<()>;
}
```

`HeadlessChromePage` 内部通过 `tab.call_method(Media::Enable(None))` 和 `tab.add_event_listener()` 实现。

---

## 2. VideoEngine 类图设计

```
VideoEngine
│
├── BrowserProvider (已有 trait, 扩展)
│   └── HeadlessChromeProvider::launch() → BrowserHandle → BrowserPage
│       ├── enable_domain(Media)
│       ├── enable_domain(Network)
│       ├── enable_domain(Performance)
│       └── subscribe_events(handler)
│
├── MediaCollector (新建)
│   ├── 监听: playerCreated, playerPropertiesChanged, playerEventsAdded
│   ├── 采集: player_type, mime_type, resolution, first_play_time
│   ├── 采集: dropped_frames, decoded_frames
│   └── 输出: MediaMetrics
│
├── NetworkCollector (新建)
│   ├── 监听: requestWillBeSent, responseReceived, dataReceived, loadingFinished
│   ├── 采集: video_host, audio_host, cdn, segment_count, segment_size
│   ├── 计算: download_speed (实时/平均/峰值)
│   └── 输出: NetworkMetrics
│
├── PerformanceCollector (新建)
│   ├── 监听: PerformanceMetrics (FPS, CPU 等)
│   ├── 辅助: JS evaluate (document.title, video.currentTime)
│   └── 输出: PerfMetrics
│
├── DnsEngine (已有)
│   └── resolve(url) → dns_time, dns_success
│
├── HttpEngine (已有)
│   └── probe(url) → tcp_time, tls_time, ttfb_ms
│
└── VideoTestResult (已有, 扩展字段)
    ├── basic: url, title, platform
    ├── network: dns_time, tcp_time, tls_time
    ├── player: player_type, mime_type, codec, resolution, fps
    ├── quality: first_play_time, play_success, buffer_count, buffer_time
    ├── traffic: video_host, audio_host, cdn, segment_count, download_speed
    └── error: error_msg, screenshot
```

### 模块职责与调用关系

```
VideoEngine::test_page(url, platform_cfg)
│
├── 1. DnsEngine::resolve(url)              → dns_time, dns_success
├── 2. HttpEngine::probe(url)               → tcp_time, tls_time, ttfb_ms
├── 3. BrowserProvider::launch()            → BrowserPage
│       ├── page.enable_domain(Media)        → 注册 Media.playerCreated 等事件
│       ├── page.enable_domain(Network)      → 注册 Network.dataReceived 等事件
│       └── page.enable_domain(Performance)  → 注册 Performance.PreviousMetrics
├── 4. page.navigate_to(url)                → 加载视频页面
├── 5. MediaCollector::collect(page)        → 采集 player 创建/属性/事件
├── 6. NetworkCollector::collect(page)      → 采集请求/响应/数据量/速度
├── 7. PerformanceCollector::collect(page)  → 采集 FPS, 页面标题等
├── 8. 等待 play_duration 秒                → 给 Media Player 时间初始化
├── 9. MediaCollector::finalize()           → 计算最终指标
├──10. page.screenshot()                    → 截图
└──11. 组装 VideoTestResult                  → 返回
```

### 数据流向

```
CDP Events (Media/Network/Performance)
    │
    ▼
CdpEventHandler trait
    │
    ├──→ MediaCollector  ──→ MediaMetrics
    ├──→ NetworkCollector ──→ NetworkMetrics
    └──→ PerfCollector    ──→ PerfMetrics
                                    │
                                    ▼
                              VideoTestResult
```

---

## 3. 数据结构设计

### 3.1 VideoTestResult（已有，扩展字段）

```rust
pub struct VideoTestResult {
    // basic
    pub url: String,
    pub platform: String,
    pub page_title: Option<String>,

    // network (已有)
    pub dns_time_ms: Option<f64>,
    pub dns_success: bool,
    pub tcp_time_ms: Option<f64>,
    pub http_response_ms: Option<f64>,

    // player (新增)
    pub player_type: Option<String>,        // "DashPlayer" / "HlsPlayer" / "HTML5Video"
    pub mime_type: Option<String>,          // "video/mp4; codecs=\"av01\""
    pub codec: Option<String>,              // 解析后的编码名
    pub resolution: Option<String>,         // "3840x2160@60"
    pub video_bitrate_kbps: Option<f64>,    // Media 域获取
    pub audio_bitrate_kbps: Option<f64>,

    // quality
    pub first_play_time_ms: Option<f64>,    // Media.playerEventsAdded 中 playing 事件
    pub play_success: bool,
    pub buffer_count: i32,
    pub buffer_time_ms: f64,
    pub dropped_frames: i32,                // Media 域获取
    pub decoded_frames: i32,
    pub drop_rate: Option<f64>,             // dropped / decoded

    // traffic (新增)
    pub video_host: Option<String>,
    pub audio_host: Option<String>,
    pub cdn_node: Option<String>,
    pub segment_count: Option<i32>,         // DASH/HLS
    pub download_speed: Option<f64>,        // KB/s, 从 Network.dataReceived 计算
    pub avg_speed: Option<f64>,
    pub peak_speed: Option<f64>,

    // legacy
    pub screenshot: Option<Vec<u8>>,
    pub error: Option<String>,
}
```

### 3.2 MediaMetrics

```rust
/// Media 域采集的原始数据
pub struct MediaMetrics {
    pub player_type: Option<String>,       // 来源: Media.playerPropertiesChanged (name="kPlaybackStateInfo")
    pub mime_type: Option<String>,         // 来源: Media.playerPropertiesChanged (name="kMimeType")
    pub width: Option<i32>,                // 来源: Media.playerPropertiesChanged (name="kResolution")
    pub height: Option<i32>,
    pub fps: Option<f64>,                  // 来源: Media.playerPropertiesChanged (name="kFps")
    pub video_bitrate: Option<f64>,
    pub audio_bitrate: Option<f64>,
    pub first_play_time_ms: Option<f64>,   // 来源: Media.playerEventsAdded (value="playing")
    pub play_success: bool,
    pub buffer_count: i32,                 // 来源: Media.playerEventsAdded (value="buffering")
    pub buffer_time_ms: f64,
    pub dropped_frames: i32,               // 来源: Media.playerPropertiesChanged (name="kDroppedFrames")
    pub decoded_frames: i32,               // 来源: Media.playerPropertiesChanged (name="kDecodedFrames")
    pub pipeline_status: Option<String>,   // 来源: Media.playerMessagesLogged
    pub player_id: Option<String>,         // 来源: Media.playerCreated
}
```

### 3.3 NetworkMetrics

```rust
/// Network 域采集的原始数据
pub struct NetworkMetrics {
    pub video_host: Option<String>,        // 来源: Network.requestWillBeSent → 过滤 mp4/m3u8
    pub audio_host: Option<String>,        // 同上，过滤音频资源
    pub cdn_node: Option<String>,          // 来源: Network.responseReceived → RemoteIPAddress
    pub segment_count: i32,                // 来源: Network.requestWillBeSent 计数
    pub total_bytes: u64,                  // 来源: Network.dataReceived + loadingFinished
    pub download_speed: Option<f64>,       // total_bytes / elapsed / 1024
    pub avg_speed: Option<f64>,
    pub peak_speed: Option<f64>,
    pub speed_samples: Vec<f64>,           // 每秒采样
}
```

### 3.4 PerfMetrics

```rust
/// Performance 域辅助指标
pub struct PerfMetrics {
    pub page_title: Option<String>,        // document.title (JS fallback)
    pub final_url: Option<String>,         // window.location.href
    pub fps: Option<f64>,                  // Performance.getMetrics
    pub cpu_usage: Option<f64>,            // Performance.getMetrics
}
```

### 3.5 CDP Event DTO

```rust
/// CDP 域枚举
pub enum CdpDomain {
    Media,
    Network,
    Performance,
    Page,
    Runtime,
}

/// 事件处理器 trait
pub trait CdpEventHandler: Send {
    fn on_media_created(&self, player_id: &str);
    fn on_media_property_changed(&self, player_id: &str, name: &str, value: &str);
    fn on_media_event(&self, player_id: &str, event: &str, timestamp: f64);
    fn on_network_request(&self, url: &str, resource_type: &str);
    fn on_network_data(&self, request_id: &str, data_len: u64);
    fn on_network_finished(&self, request_id: &str, total_bytes: u64);
}
```

---

## 4. 需要修改的文件列表

### 4.1 新建文件

| 文件 | 说明 |
|------|------|
| `engines/video/collector.rs` | MediaCollector / NetworkCollector / PerformanceCollector |
| `engines/video/metrics.rs` | MediaMetrics / NetworkMetrics / PerfMetrics 数据结构 |
| `engines/video/cdp_handler.rs` | CdpEventHandler trait + HeadlessChrome 事件适配 |

### 4.2 修改文件

| 文件 | 修改内容 |
|------|----------|
| `engines/browser/provider.rs` | BrowserPage trait 新增 `enable_domain()` + `subscribe_events()` <br> HeadlessChromePage 实现上述方法 |
| `engines/video/mod.rs` | VideoEngine::test_page() 重构为新流水线<br> VideoTestResult 扩展字段 |
| `models/task.rs` | VideoResult 新增 player/quality/traffic 字段 |
| `worker/mod.rs` | 适配新的 VideoTestResult → VideoResult 转换 |
| `database/mod.rs` | video_result 表新增字段迁移 |
| `report/excel/mod.rs` | 视频测试 Excel 导出列更新 |

### 4.3 删除代码

| 文件 | 删除内容 |
|------|----------|
| `engines/video/mod.rs` | `build_inject_js_sync()` / `build_collect_js_sync()` — 旧的 JS 注入方案 |
| `engines/video/mod.rs` | `VideoJsData` struct |
| `engines/video/mod.rs` | `video_err()` 中重复的 VideoTestResult 构造 |

---

## 5. 迁移风险分析

### 5.1 可复用

| 组件 | 复用方式 |
|------|----------|
| `BrowserProvider` trait | ✅ 直接扩展，不加破坏性变更 |
| `DnsEngine::resolve()` | ✅ 不变 |
| `HttpEngine::probe()` | ✅ 不变 |
| `VideoPlatformConfig` + `match_platform` | ✅ 不变 |
| `VideoTestResult` | ✅ 扩展字段，向后兼容 |
| `VideoEngine::new()` 构造函数 | ✅ 保持接口兼容 |
| `test_page()` 函数签名 | ✅ 保持不变 |
| DNS/HTTP 预探测逻辑 | ✅ 不变 |
| 截图采集 | ✅ 通过 trait 的 `screenshot()` |
| Worker 调度 | ✅ 适配 VideoResult 新字段 |

### 5.2 必须删除

| 旧实现 | 原因 |
|--------|------|
| `build_inject_js_sync()` | JS 注入方案废弃 |
| `build_collect_js_sync()` | JS 注入方案废弃 |
| `VideoJsData` struct | 替换为 CDP 事件驱动 |
| 所有 `video.addEventListener()` 逻辑 | 替换为 Media CDP 域 |

### 5.3 接口调整

| 接口 | 变更 |
|------|------|
| `BrowserPage` trait | 新增 2 个方法（`enable_domain`, `subscribe_events`） |
| `VideoTestResult` | 新增 10+ 字段，旧的 `video_download_speed` 改为从 Network 域计算 |
| `VideoResult` (DB model) | 新增 player/quality/traffic 字段 |
| `video_result` 表 | 需要 migration 新增列 |
| Excel 导出 | 列顺序和数量变化 |

### 5.4 风险点

| 风险 | 缓解措施 |
|------|----------|
| Media 域实验性 API 可能变更 | 加 feature flag，失败降级到 JS fallback |
| 某些平台不暴露 CDP Media 事件 | detect_only 模式兜底 |
| Network 事件量大可能影响性能 | 限制采样率，只过滤视频资源 |
| DB migration 可能失败 | `add_column_if_missing` 已有兜底机制 |

---

## 阶段二实施计划

确认方案后，按以下顺序逐步实现：

1. **扩展 BrowserPage trait** — 新增 `enable_domain` + `subscribe_events`，编译通过
2. **实现 collector 模块** — MediaCollector / NetworkCollector，不连线 VideoEngine
3. **重构 VideoEngine** — 连线 collector，替换旧 JS 注入，编译通过
4. **扩展数据模型** — VideoTestResult / VideoResult / DB migration
5. **更新 Worker + 导出** — 适配新字段
6. **全量构建测试**
