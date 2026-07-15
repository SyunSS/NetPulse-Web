# ====== 阶段1: 编译后端 ======
FROM rust:1.93-slim AS backend-builder

WORKDIR /build

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY backend/Cargo.toml backend/Cargo.lock ./
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs && \
    mkdir -p src/api src/config src/database src/models src/services src/engines/browser \
    src/engines/dns src/engines/http src/engines/download src/engines/video \
    src/report/excel src/storage src/utils src/worker src/scheduler && \
    touch src/api/mod.rs src/config/mod.rs src/database/mod.rs src/models/mod.rs && \
    cargo build --release 2>/dev/null || true

COPY backend/src ./src
COPY backend/config.toml ./
RUN cargo build --release

# ====== 阶段2: 构建前端 ======
FROM node:22-slim AS frontend-builder

WORKDIR /build

RUN npm install -g pnpm

COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN pnpm install --no-frozen-lockfile

COPY frontend/ ./
RUN pnpm build

# ====== 阶段3: 运行镜像 ======
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 chromium fonts-noto-cjk fonts-liberation \
    && rm -rf /var/lib/apt/lists/*

ENV CHROME_PATH=/usr/bin/chromium

WORKDIR /app

COPY --from=backend-builder /build/target/release/netpulse-web /app/
COPY --from=backend-builder /build/config.toml /app/
COPY --from=frontend-builder /build/dist /app/frontend-dist

RUN mkdir -p /app/data /app/logs /app/data/screenshots /app/data/excel

EXPOSE 3000

CMD ["/app/netpulse-web"]
