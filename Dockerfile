# ====== 阶段1: 编译后端 ======
FROM rust:1.93-slim AS backend-builder

WORKDIR /build

# 安装编译需要的系统依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 先拷贝依赖文件，利用 Docker 缓存层
COPY backend/Cargo.toml backend/Cargo.lock* ./

# 创建一个最小的 main.rs 用于预编译依赖
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs && \
    cargo build --release 2>/dev/null || true

# 拷贝真实源码
COPY backend/src ./src
COPY backend/config.toml ./

# 编译
RUN cargo build --release

# ====== 阶段2: 构建前端 ======
FROM node:22-slim AS frontend-builder

WORKDIR /build

COPY frontend/package.json frontend/pnpm-lock.yaml* ./
RUN corepack enable && pnpm install --frozen-lockfile 2>/dev/null || pnpm install

COPY frontend/ ./
RUN pnpm build

# ====== 阶段3: 最终运行镜像 ======
FROM debian:bookworm-slim AS runtime

# 安装运行时依赖（后续阶段需要 Chromium）
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    # 以下为后续浏览器引擎预留（第二阶段启用）
    chromium \
    fonts-noto-cjk \
    fonts-liberation \
    && rm -rf /var/lib/apt/lists/*

# 创建运行目录
WORKDIR /app

# 拷贝后端二进制
COPY --from=backend-builder /build/target/release/netpulse-web /app/netpulse-web
COPY --from=backend-builder /build/config.toml /app/config.toml

# 拷贝前端构建产物
COPY --from=frontend-builder /build/dist /app/frontend-dist

# 创建数据目录
RUN mkdir -p /app/data /app/logs

# 暴露端口
EXPOSE 3000

# 运行
CMD ["/app/netpulse-web"]
