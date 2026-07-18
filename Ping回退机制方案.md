# Ping 回退机制方案

## 当前状态

`engines/ping/mod.rs` — 仅 ICMP ping（调用系统 `ping` 命令），失败则直接标记 100% 丢包。

## 目标

```
ICMP ping
  ↓ 失败(100%丢包)
TCP connect :80 (超时 3s)
  ↓ 失败(连接拒绝/超时)
TCP connect :443 (超时 3s)
  ↓ 失败
返回失败结果
```

## 实现方案

纯 Rust 实现，**不依赖外部 tcping 工具**。

### PingResult 新增字段

```rust
pub struct PingTestResult {
    pub host: String,
    pub avg_latency_ms: f64,
    pub packet_loss_rate: f64,
    pub jitter_ms: f64,
    pub success: bool,
    pub error: Option<String>,
    pub method: Option<String>,  // "icmp" | "tcp80" | "tcp443" | null
}
```

### 核心逻辑

```rust
pub async fn test_ping(&self, host: &str) -> PingTestResult {
    // 1. ICMP
    let icmp_result = run_icmp_ping(host);
    if icmp_result.success {
        return icmp_result;  // method = "icmp"
    }

    // 2. TCP 80
    let tcp80_result = run_tcp_ping(host, 80);
    if tcp80_result.success {
        return tcp80_result; // method = "tcp80"
    }

    // 3. TCP 443
    let tcp443_result = run_tcp_ping(host, 443);
    if tcp443_result.success {
        return tcp443_result; // method = "tcp443"
    }

    // 全部失败
    return fail_result(host);
}
```

### TCP Ping 实现

```rust
async fn run_tcp_ping(host: &str, port: u16) -> PingTestResult {
    let addr = format!("{}:{}", host, port);
    // 连 5 次，取平均
    let mut times = Vec::new();
    for _ in 0..5 {
        let start = Instant::now();
        match tokio::time::timeout(Duration::from_secs(3),
            TcpStream::connect(addr.clone())
        ).await {
            Ok(Ok(_)) => times.push(start.elapsed().as_secs_f64() * 1000.0),
            _ => {}
        }
    }
    if times.is_empty() {
        return PingTestResult { success: false, method: format!("tcp{}", port), ... }
    }
    let avg = times.iter().sum::<f64>() / times.len() as f64;
    PingTestResult { avg_latency_ms: avg, packet_loss_rate: (5-times.len())*20.0, success: true, method: format!("tcp{}", port), ... }
}
```

## 修改文件

| 文件 | 修改 |
|------|------|
| `engines/ping/mod.rs` | 新增 `run_tcp_ping()` + `run_ping_with_fallback()`，`PingTestResult` 加 `method` 字段 |
| `worker/mod.rs` | `PingResult` 模型加 `method` 字段，DB migration |
| `models/task.rs` | `PingResult` 加 `method: Option<String>` |

## 前端显示

Ping 结果增加一列「检测方法」: `ICMP` / `TCP:80` / `TCP:443` / `失败`

## 优点

- 纯 Rust，无需外部工具（tcping.exe）
- Docker 容器自带 TCP 能力，不需要额外安装
- 和参考项目逻辑一致：ICMP → TCP:80 → TCP:443 三级回退
