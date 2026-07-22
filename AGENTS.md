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
| `backend/src/worker/` | `TaskWorker` — mpsc-receiver, spawns per-type test |
| `backend/src/scheduler/` | `PlanScheduler` — cron-based, checks every 60s |
| `backend/src/database/` | SQLite init + inline migrations (14 tables) |
| `frontend/src/views/` | Vue pages (Dashboard, CreateTask, Plans, TaskDetail, etc.) |
| `frontend/src/router/` | Auth guard, history mode |

## Key Details

- **Auth**: JWT middleware on `/api/task`, `/api/plan`, `/api/admin`, `/api/dashboard`. Public: `/api/health`, `/api/auth/*`, `/api/ws`.
- **Config**: `config.toml` + env overrides with `NETPULSE__` prefix (e.g. `NETPULSE__DATABASE__PATH`).
- **DB**: SQLite with WAL, schema created inline (no sqlx migrations). `add_column_if_missing` for incremental schema changes.
- **Browser (Website)**: `headless_chrome` crate used directly, no trait abstraction. `ChromePage` + `launch_browser()`/`new_page()` in `browser/provider.rs`.
- **Browser (Video)**: `chromiumoxide` crate (async) — dedicated `[video_browser]` config section with separate chromium path.
- **Video**: New architecture — `ChromiumoxideBrowser` → CDP collectors (Media/Network/Performance/Runtime/Page) + JS Hook manager (HTMLMediaElement/fetch/XHR/MediaSource/MutationObserver) + PlayerDetector (Generic/Bilibili/YouTube via `PlayerAdapter` trait) + `MetricCollector` with unified `VideoEvent` pipeline. Player detection moved into `players/registry.rs`, no longer in config.
- **VideoConfig**: `video_selector` and `wait_seconds` removed from `[[video_platforms]]`. Detection logic handled by `PlayerAdapter::detect()`. `detect_only` flag retained for Netflix.
- **Chrome**: Requires `chromium` binary at the configured path. Sandbox disabled (`--no-sandbox`).
- **Ping**: ICMP primary, TCP SYN fallback.
- **Tests**: Minimal — only `dns/mod.rs` and `ping/mod.rs` have `#[cfg(test)]` blocks. No integration test harness.
- **Frontend deps**: pnpm. Vite `@` alias → `src/`. Layout system with dark theme + zhCN i18n (Naive UI).
- **Cron**: Scheduler polls every 60s, cron parsing via `cron` crate, `compute_next_run` in `plan_service`.

## Common Mistakes

- Not running `vue-tsc -b` before commit (it's part of `pnpm build` but not `pnpm dev`).
- Using npm instead of pnpm for frontend. The lockfile is `pnpm-lock.yaml`.
- Forgetting Chromium is required for website/video tests (binary at `config.browser.path` for website, `config.video_browser.path` for video).
- Modifying DB schema in one place but missing the `add_column_if_missing` pattern.
- Assuming `sqlx::query!` macros — this repo uses raw `sqlx::query()` strings.
