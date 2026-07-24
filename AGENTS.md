# NetPulse Web — Agent Guide

## Build & Dev

```sh
# backend
cd backend && cargo build --release

# frontend (pnpm, not npm)
cd frontend && pnpm install && pnpm build
pnpm dev          # :5173, proxies /api → :3000

# docker
docker compose up -d   # image: ghcr.io/syunss/netpulse-web:latest
```

Frontend build runs `vue-tsc -b && vite build` (typecheck first).

## Architecture

Backend Rust (Axum + Tokio + SQLx/SQLite) + Frontend Vue 3 (Naive UI, ECharts, Pinia)

Single binary: backend serves `frontend-dist/` if present, else API-only. Nginx config included for split deployment.

| Dir | Purpose |
|-----|---------|
| `backend/src/api/` | Axum route handlers (auth, task, plan, admin, ws, metrics) |
| `backend/src/engines/` | Test engines: ping, dns, http, download, website, video |
| `backend/src/engines/browser/` | Website testing via `headless_chrome` directly |
| `backend/src/engines/video/` | Video engine: CDP collectors, JS hooks, player adapters |
| `backend/src/worker/` | `TaskWorker` — mpsc-receiver, spawns per-type test |
| `backend/src/scheduler/` | `PlanScheduler` — cron-based, checks every 60s |
| `backend/src/database/` | SQLite init + inline migrations (14 tables) |
| `backend/src/services/` | Business logic (auth, task, plan services) |
| `backend/src/models/` | Data structs for DB rows and API payloads |
| `frontend/src/views/` | Vue pages (Dashboard, CreateTask, Plans, TaskDetail, etc.) |
| `frontend/src/router/` | Auth guard, history mode |
| `frontend/src/stores/` | Pinia stores |

## Key Details

- **Auth**: JWT middleware on `/api/task`, `/api/plan`, `/api/admin`, `/api/dashboard`. Public: `/api/health`, `/api/auth/*`, `/api/ws`.
- **Config**: loads `config/config.toml` first, falls back to `config.toml` (auto-creates default if neither exists). Env overrides with `NETPULSE__` prefix (e.g. `NETPULSE__DATABASE__PATH`).
- **DB**: SQLite with WAL, schema created inline (no sqlx migrations). `add_column_if_missing` for incremental schema changes.
- **Browser (Website)**: `headless_chrome` crate used directly, no trait abstraction. `ChromePage` + `launch_browser()`/`new_page()` in `browser/provider.rs`.
- **Browser (Video)**: `chromiumoxide` crate (async) — dedicated `[video_browser]` config section with separate chromium path.
- **Video**: `ChromiumoxideBrowser` → CDP collectors + JS hooks + `PlayerAdapter` trait (`players/registry.rs` dispatches to Bilibili/YouTube/Generic). Platform detection is code-driven by URL keyword matching in `config.toml:[[video_platforms]]`. `detect_only` flag for Netflix skips play.
- **Metrics system**: `metric_definition` table seeded with 16 built-in metrics (dns_time, tcp_time, lcp, cls, etc.). `metric_profile` groups metrics per user. `task_metric_config` binds selected metrics to a task. API at `/api/metrics/*`.
- **Chrome**: Requires `chromium` binary at configured path. Sandbox disabled (`--no-sandbox`). Website and video engines each launch a separate Chromium process — two concurrent Chromium instances is normal. Docker sets `CHROME_PATH` as convention only; the app reads `browser.path` from config (or `NETPULSE__BROWSER__PATH`).
- **Ping**: ICMP via system `ping` command (`spawn_blocking`), falls back to TCP connect on port 80 → 443. On Linux, ICMP may need `CAP_NET_RAW` or root (macOS works without special perms).
- **Tests**: Minimal — only `dns/mod.rs` and `ping/mod.rs` have `#[cfg(test)]` blocks. No integration test harness.
- **Frontend deps**: pnpm. Vite `@` alias → `src/`. Layout system with dark theme + zhCN i18n (Naive UI). TypeScript is lenient: `noUnusedLocals: false`, `noUnusedParameters: false`.
- **Cron**: Scheduler polls every 60s, cron parsing via `cron` crate, `compute_next_run` in `plan_service`.

## Common Mistakes

- Not running `vue-tsc -b` before commit (it's part of `pnpm build` but not `pnpm dev`).
- Using npm instead of pnpm for frontend. The lockfile is `pnpm-lock.yaml`.
- Forgetting Chromium is required for website/video tests (binary at `config.browser.path` for website, `config.video_browser.path` for video).
- Modifying DB schema in one place but missing the `add_column_if_missing` pattern.
- Assuming `sqlx::query!` macros — this repo uses raw `sqlx::query()` strings.
