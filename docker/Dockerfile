# ─────────────────────────────────────────────────────────────────────────────
# Stage 1 – cargo-chef dependency cache planner
# ─────────────────────────────────────────────────────────────────────────────
FROM rust:1.78-slim AS chef
WORKDIR /app
RUN cargo install cargo-chef --locked

# ─────────────────────────────────────────────────────────────────────────────
# Stage 2 – create the "recipe" (dependency manifest only)
# ─────────────────────────────────────────────────────────────────────────────
FROM chef AS planner
COPY backend/ .
RUN cargo chef prepare --recipe-path recipe.json

# ─────────────────────────────────────────────────────────────────────────────
# Stage 3 – build dependencies first (layer is cached unless Cargo.lock changes)
# ─────────────────────────────────────────────────────────────────────────────
FROM chef AS builder
# System libs needed by sqlx / ring / openssl
RUN apt-get update && \
    apt-get install -y --no-install-recommends pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Now copy the real source and compile the binary
COPY backend/ .
RUN cargo build --release --bin miransas-db

# ─────────────────────────────────────────────────────────────────────────────
# Stage 4 – minimal runtime image
# ─────────────────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/miransas-db /usr/local/bin/miransas-db
COPY --from=builder /app/migrations               ./migrations

ENV APP_HOST=0.0.0.0 \
    APP_PORT=3001

EXPOSE 3001

HEALTHCHECK --interval=15s --timeout=5s --retries=3 \
    CMD curl -fs http://localhost:3001/health || exit 1

CMD ["miransas-db"]
