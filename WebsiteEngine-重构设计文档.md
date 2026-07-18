# WebsiteEngine 重构设计文档（第一阶段）

---

## 1. 当前 WebsiteEngine 架构

### 1.1 数据采集方式

```
BrowserEngine::test_page()
  │
  ├── DnsEngine::resolve()      → dns_time, dns_success  (独立探测)
  ├── HttpEngine::probe()       → tcp, tls, ttfb, status (独立探测)
  ├── BrowserProvider::launch() → 启动 Chrome
  ├── page.navigate_to()        → 加载页面
  ├── page.wait_for_load()      → 等导航完成
  ├── sleep(3s)                 → ⚠️ 硬编码等待
  ├── page.evaluate_sync(PERF_JS) → ⚠️ 内联 JS (performance.timing + paint + resource)
  ├── page.evaluate_sync("document.title")
  ├── page.evaluate_sync("window.location.href")
  └── page.screenshot()
```

### 1.2 当前 JS 注入

`PERF_JS` 通过 `JSON.stringify((function(){...})())` 采集：
- `performance.timing` (Navigation Timing Level 1，已废弃)
- `performance.getEntriesByType('paint')` (Paint Timing)
- `performance.getEntriesByType('resource')` (Resource Timing，有跨域限制)

### 1.3 当前 Result 结构

`BrowserResult` 14 个字段：fp_ms, fcp_ms, dom_content_loaded_ms, load_event_ms, page_open_time_ms, first_paint_ms, resource_count, resource_total_size, final_url, page_title, screenshot, error

### 1.4 当前 CDP 能力（已有但未用）

`BrowserPage` trait 已有 `send_cdp()` 和 `on_cdp_event()`，但 WebsiteEngine 完全没用。

---

## 2. 重构方案

### 2.1 保留

| 组件 | 原因 |
|------|------|
| `BrowserEngine` 结构体 | 持有 provider + chrome_path，接口不变 |
| `test_page()` 签名 | `async fn test_page(&self, url: &str) -> BrowserResult` |
| `DnsEngine::resolve()` | 独立探测，不变 |
| `HttpEngine::probe()` | 独立探测，不变 |
| `Worker::test_website_url()` | 编排循环不变，只扩展字段 |
| `BrowserResult` 现有字段 | 向后兼容 |

### 2.2 删除

| 旧代码 | 替代 |
|--------|------|
| `PERF_JS` 常量 + `PerfData` struct | CDP Performance + Page 域 |
| `sleep(3s)` 硬编码等待 | 事件驱动 (等 Page.loadEventFired 或 Network idle) |
| `evaluate_sync` 获取性能数据 | CDP 事件回调 |

### 2.3 新增模块

| 文件 | 说明 |
|------|------|
| `engines/browser/collectors.rs` | PageCollector, NetworkCollector, ResourceCollector |
| `engines/browser/metrics.rs` | CdpPageMetrics, CdpNetworkMetrics 等 |

### 2.4 修改文件

| 文件 | 修改 |
|------|------|
| `engines/browser/mod.rs` | BrowserResult 扩展字段, test_page() 重构为 CDP 驱动 |
| `engines/browser/provider.rs` | call_cdp 支持更多 CDP 命令, 事件桥接到 listeners |
| `models/task.rs` | WebsiteResult 新增 lcp_ms/cls/tti_ms/total_requests 等 |
| `database/mod.rs` | website_result 表新增列 migration |
| `worker/mod.rs` | test_website_url 传播新字段 |
| `report/excel/mod.rs` | 网站导出新增列 |
| `frontend/src/views/TaskDetail.vue` | 表格新增列 |
| `frontend/src/api/task.ts` | TypeScript 类型新增字段 |

---

## 3. WebsiteEngine 新架构设计

```
WebsiteEngine::test_page(url)
│
├── DnsEngine::resolve(url)           → dns_time, dns_success
├── HttpEngine::probe(url)            → tcp_time, tls_time, ttfb, http_status
├── BrowserProvider::launch()         → BrowserPage
│
├── CDP Domain Enable:
│   ├── page.send_cdp("Page.enable")
│   ├── page.send_cdp("Network.enable")
│   └── page.send_cdp("Performance.enable")
│
├── 注册 Collectors (via on_cdp_event):
│   ├── PageCollector    → domContentLoaded, load, fp, fcp
│   ├── NetworkCollector → requests, responses, data, timings
│   └── ResourceCollector → html/css/js/image sizes
│
├── page.navigate_to(url)             → 触发事件流
├── page.wait_for_load()              → 等导航完成
│
├── 等待收集完成:
│   └── await page 稳定 (Load fired + 2s 空闲)
│
├── page.evaluate_sync("document.title")     (仅获取标题, 不获取性能)
├── page.screenshot()
│
└── 汇总 Metric → BrowserResult
```

### 数据流

```
CDP Events (Page / Network / Performance)
  ↓
CdpEventListener::on_event()
  ├→ PageCollector     → CdpPageMetrics
  ├→ NetworkCollector  → CdpNetworkMetrics (DNS/TCP/TLS/HTTP)
  └→ ResourceCollector → CdpResourceMetrics
                              ↓
                    BrowserResult (汇总)
```

### Collector 职责

| Collector | 监听事件 | 输出 |
|-----------|---------|------|
| PageCollector | Page.domContentEventFired, Page.loadEventFired, Page.lifecycleEvent, Page.frameStartedLoading | dom_content_loaded, load_time, fp, fcp |
| NetworkCollector | Network.requestWillBeSent, Network.responseReceived, Network.loadingFinished | request_count, failed_count, timing |
| ResourceCollector | Network.responseReceived, Network.loadingFinished | html_size, css_size, js_size, image_size, total_size |

---

## 4. Metric Profile 设计方案（简化版）

考虑到实现复杂度和用户实际需求，设计为**简化版 Metric Profile**：

### 4.1 方案

不为每个指标创建独立定义表，而是在**创建任务时**通过 `options.metrics` 字段指定需要的指标类别：

```json
{
  "task_type": "website",
  "urls": ["https://example.com"],
  "options": {
    "repeat_count": 1,
    "metrics": ["basic", "network", "resource"]
  }
}
```

### 4.2 指标类别

| 类别 | 包含指标 | 默认 |
|------|---------|------|
| `basic` | DNS, TCP, TLS, HTTP status, TTFB, FP, FCP, DOM, Load | ✅ |
| `network` | TTFB from CDP timing, request_count, failed_requests | ✅ |
| `resource` | html/css/js/image sizes, total count/size | ✅ |
| `performance` | LCP, CLS, TTI (需要 Tracing 域) | ❌ |
| `security` | certificate_status, security_state | ❌ |

### 4.3 Collector 动态启用

```
options.metrics 包含 "performance"
  → 启动 TraceCollector (开启 Tracing 域)
  → 采集 LCP, CLS, TTI

options.metrics 不包含 "performance"
  → 不启动 Tracing 域 (省资源)
  → LCP/CLS 字段为 null
```

### 4.4 前端指标选择

CreateTask 页面的 Website 类型增加一个**快捷选择**：

```
☑ 基础指标 (DNS/TCP/TLS/TTFB/状态码)     ← 总是选中
☑ 页面性能 (FP/FCP/DOM/Load)             ← 默认选中
☐ 高级性能 (LCP/CLS/TTI, 较耗时)         ← 可选
☐ 安全信息 (证书状态)                    ← 可选
```

配置存入 `task.options.metrics` 数组，默认为 `["basic", "network", "resource"]`。

---

## 5. 实施计划

| 阶段 | 内容 | 文件 |
|------|------|------|
| 1 | 扩展 call_cdp 支持 Page/Performance/Runtime 域 + 事件桥接 | provider.rs |
| 2 | 实现 PageCollector / NetworkCollector / ResourceCollector | collectors.rs (新) |
| 3 | 重构 BrowserEngine::test_page() 为 CDP 驱动 | browser/mod.rs |
| 4 | 扩展 BrowserResult + WebsiteResult + DB migration | mod.rs, task.rs, database |
| 5 | 更新 Worker 字段传播 | worker/mod.rs |
| 6 | 更新前端表格 + 导出 Excel | TaskDetail.vue, excel/mod.rs |
| 7 | Metric Profile 选项 (options.metrics) | CreateTask.vue, worker |
| 8 | 构建验证 | cargo build + pnpm build |

每阶段独立编译验证，逐步推进。
