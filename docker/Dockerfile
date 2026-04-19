# ─────────────────────────────────────────────────────────────────────────────
# Stage 1 – base image with cargo-chef already installed
# Using the official cargo-chef image avoids compiling cargo-chef from scratch
# on a slim base that lacks pkg-config / libssl-dev / gcc.
# ─────────────────────────────────────────────────────────────────────────────
FROM lukemathwalker/cargo-chef:latest-rust-1-slim AS chef
WORKDIR /app

# ─────────────────────────────────────────────────────────────────────────────
# Stage 2 – compute the dependency "recipe" from Cargo.toml + Cargo.lock only.
# This layer is invalidated only when those two files change.
# ─────────────────────────────────────────────────────────────────────────────
FROM chef AS planner
COPY backend/Cargo.toml backend/Cargo.lock ./
# src/ needs to exist for `cargo chef prepare` to resolve workspace members
COPY backend/src ./src
RUN cargo chef prepare --recipe-path recipe.json

# ─────────────────────────────────────────────────────────────────────────────
# Stage 3 – build dependencies (cached layer) then the real binary.
# System libs are installed here, not in the chef base, so the chef image
# stays small and reusable across platforms.
# ─────────────────────────────────────────────────────────────────────────────
FROM chef AS builder

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 1. Cook (compile) all dependencies — only reruns when Cargo.lock changes.
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# 2. Copy full source and build the application binary.
COPY backend/ .
RUN cargo build --release --bin miransas-db

# ─────────────────────────────────────────────────────────────────────────────
# Stage 4 – minimal runtime image (~100 MB total).
# ─────────────────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/miransas-db /usr/local/bin/miransas-db
COPY --from=builder /app/migrations                 ./migrations

ENV APP_HOST=0.0.0.0 \
    APP_PORT=3001

EXPOSE 3001

HEALTHCHECK --interval=15s --timeout=5s --retries=3 \
    CMD curl -fs http://localhost:3001/health || exit 1

CMD ["miransas-db"]
