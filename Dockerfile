# ====== 阶段1: 编译后端 ======
FROM rust:1.93-slim AS backend-builder

WORKDIR /build

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 先拷贝依赖清单，缓存依赖下载
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs && cargo build --release && rm -rf src/

# 拷贝真实源码并强制重新编译
COPY backend/src ./src
COPY backend/config.toml ./
RUN touch src/main.rs && cargo build --release

# ====== 阶段2: 构建前端 ======
FROM node:22-slim AS frontend-builder

WORKDIR /build

RUN npm install -g pnpm@10

COPY frontend/ ./
RUN pnpm install --no-frozen-lockfile && pnpm build

# ====== 阶段3: 运行镜像 ======
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 chromium fonts-noto-cjk fonts-liberation wget \
    && rm -rf /var/lib/apt/lists/*

ENV CHROME_PATH=/usr/bin/chromium

WORKDIR /app

COPY --from=backend-builder /build/target/release/netpulse-web /app/
COPY --from=backend-builder /build/config.toml /app/
COPY --from=frontend-builder /build/dist /app/frontend-dist

RUN mkdir -p /app/data /app/logs /app/data/screenshots /app/data/excel

EXPOSE 3000

CMD ["/app/netpulse-web"]
