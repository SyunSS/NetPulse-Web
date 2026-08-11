# NetPulse Web Agent Guide

## Commands

- No root manifest/workspace exists; run backend commands in `backend/` and frontend commands in `frontend/`.
- Backend: `cargo check`, `cargo test`, `cargo build --release`; focused tests work, e.g. `cargo test engines::dns` or `cargo test engines::ping`.
- The repo is not fully rustfmt-clean; avoid `cargo fmt --all` unless broad churn is intended. For touched backend files, prefer `rustfmt --edition 2021 <files>`.
- Frontend uses pnpm, not npm. Use `pnpm install`, `pnpm build`, or `pnpm dev` from `frontend/`; `build` runs `vue-tsc -b && vite build`.
- Vite dev runs on `:5173` and proxies `/api` to `http://127.0.0.1:3000`; run the Rust backend separately for local frontend testing.
- `docker compose up -d` pulls `ghcr.io/syunss/netpulse-web:latest`; it does not build local changes. Use `Dockerfile` for local image builds.

## Structure

- Rust backend + Vue 3 frontend, with no workspace-level manifest.
- Backend entrypoint is `backend/src/main.rs`; it loads config, initializes SQLite, starts `TaskWorker`, starts `PlanScheduler`, builds API routes, and optionally serves `frontend-dist/`.
- Backend boundaries: handlers in `backend/src/api/`, business logic in `backend/src/services/`, test engines in `backend/src/engines/`, task dispatch in `backend/src/worker/`, cron scheduling in `backend/src/scheduler/`.
- Frontend boundaries: pages in `frontend/src/views/`, layout/components in `frontend/src/layouts/` and `frontend/src/components/`, API clients in `frontend/src/api/`, Pinia stores in `frontend/src/stores/`, routing in `frontend/src/router/`.
- Vite maps `@/*` to `frontend/src/*`; Naive UI theme overrides live in `frontend/src/main.ts`, and shared design tokens live in `frontend/src/assets/styles/main.css`.

## Runtime Gotchas

- Start the backend from the repository root, or intentionally choose a cwd containing the desired `config/`, `data/`, `logs/`, and `storage/` paths.
- On startup, missing `config/config.toml` is created relative to cwd from `backend/config.toml`, with a random JWT secret substituted.
- Config loading checks `config/config.toml` first, then `config.toml`; `NETPULSE__...` environment variables override file values.
- Existing configs using `netpulse-jwt-secret-change-in-production` fail validation; JWT secrets must also be at least 32 bytes.
- SQLite schema creation and incremental migrations are inline in `backend/src/database/mod.rs`; add columns through `add_column_if_missing`, not a separate migration directory.
- The backend serves static frontend files only when `frontend-dist/` exists; otherwise it is API-only.
- Auth protects `/api/task`, `/api/plan`, `/api/admin`, `/api/metrics`, and `/api/dashboard/stats`; health, auth, and WebSocket routes are public.

## Browser / Video Tests

- Website and Video tests require Chromium and use `chromiumoxide`; executable paths are configured separately in `[browser].path` and `[video_browser].path`.
- Browser launch uses `--no-sandbox`. Website sessions use temporary unique profiles; `video_browser.user_data_dir` is the persistent login-state profile when configured.
- Website/Video browser sessions must go through `backend/src/engines/chromium.rs`; it owns profile dirs, handler draining, close/wait/kill, and temp-dir cleanup.
- `task.concurrency` is task-level. Website and Video additionally share a fixed browser semaphore of `1`, so Ping/Download can still run while browser tests serialize.
- Video platform config deserializes only `name`, `url_keywords`, and optional `detect_only`; selectors and waits live in code, not `config.toml`.
- Video Cookie encryption keys and imported login state live under `storage.secure_dir` (`/app/storage` in Docker). Keep that directory persistent and untracked.

## Docker / CI

- `docker-compose.yml` currently hardcodes `/mnt/sata1-4/docker/Netpulse-web/storage:/app/storage`; change that host path on machines without it.
- Docker image CI is `.github/workflows/docker-build.yml`; it pushes GHCR images only on `master` pushes except docs-only changes (`**.md` ignored), or via `workflow_dispatch`.
- Feature-branch pushes do not automatically publish Docker images unless the workflow is manually dispatched.

## Verification

- Frontend changes are not complete until `pnpm build` passes in `frontend/`; `pnpm dev` does not run the TypeScript project build.
- Backend changes should run at least `cargo check` in `backend/`; use `cargo build --release` when validating Docker/release parity.
- Backend tests are unit-focused; browser, network, Chromium, Docker/GHCR, and scheduled Website/Video concurrency need external/runtime verification.
