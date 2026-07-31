# NetPulse Web Agent Guide

## Commands

- Backend commands run from `backend/`: `cargo build --release`, `cargo test`, and focused tests such as `cargo test engines::dns` or `cargo test engines::ping`.
- Frontend uses pnpm, not npm. From `frontend/`, run `pnpm install` and `pnpm build`; the build runs `vue-tsc -b` before `vite build`.
- Run the frontend dev server with `pnpm dev` from `frontend/`. It listens on `:5173` and proxies `/api` to `http://127.0.0.1:3000`.
- `docker compose up -d` uses the published `ghcr.io/syunss/netpulse-web:latest` image, not a local build. The Dockerfile builds backend and frontend separately and embeds `frontend-dist/` in the runtime image.

## Structure

- This is a Rust backend plus Vue 3 frontend, with no workspace-level manifest. `backend/src/main.rs` initializes config, SQLite, worker, scheduler, routes, and optional static frontend serving.
- Backend API handlers are in `backend/src/api/`; business logic is in `backend/src/services/`; test engines are in `backend/src/engines/`; task dispatch is in `backend/src/worker/`; cron plans are in `backend/src/scheduler/`.
- Frontend pages are in `frontend/src/views/`, API clients in `frontend/src/api/`, shared state in `frontend/src/stores/`, and routing in `frontend/src/router/`. Vite maps `@/*` to `frontend/src/*`.
- Website and video tests use Chromium through the shared `chromiumoxide` dependency. The configured executable paths are `[browser].path` and `[video_browser].path`.

## Runtime Constraints

- Start the backend from the repository root or ensure its working directory contains the intended config/data paths. Startup creates `config/config.toml` if absent, loading it before the fallback `backend/config.toml`; `NETPULSE__...` environment variables override file values.
- Chromium is required for website/video tests. Browser launch uses `--no-sandbox`, so do not assume a normal sandboxed local browser setup.
- SQLite schema creation and incremental changes are inline in `backend/src/database/mod.rs`; there is no sqlx migration directory. Add new columns through the existing `add_column_if_missing` path.
- The backend serves `frontend-dist/` only when that directory exists; otherwise it is API-only. The frontend dev proxy therefore requires a separately running backend.
- Auth protects `/api/task`, `/api/plan`, `/api/admin`, `/api/metrics`, and `/api/dashboard/stats`. Health, auth, and WebSocket routes are public.

## Verification

- Before considering frontend changes complete, run `pnpm build` in `frontend/` rather than relying on `pnpm dev`, since dev does not run the TypeScript project build.
- Backend tests are unit-focused and run with Cargo; browser, network, and Docker behavior require their external prerequisites and are not covered by an integration harness.
