# ─────────────────────────────────────────────────────────────────────────────
# Stage 1 – Base (Apple Silicon / ARM64 desteği için)
# ─────────────────────────────────────────────────────────────────────────────
FROM lukemathwalker/cargo-chef:latest-rust-1-slim AS chef
WORKDIR /app

# Sistem kütüphanelerini en başta kuralım ki chef image'ı her şeyi görsün
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    gcc \
    libc6-dev \
    && rm -rf /var/lib/apt/lists/*

# ─────────────────────────────────────────────────────────────────────────────
# Stage 2 – Planner
# ─────────────────────────────────────────────────────────────────────────────
FROM chef AS planner
# Monorepo yapısında olduğun için dosyaları doğru yoldan çekelim
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
RUN cargo chef prepare --recipe-path recipe.json

# ─────────────────────────────────────────────────────────────────────────────
# Stage 3 – Builder
# ─────────────────────────────────────────────────────────────────────────────
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# SQLx Offline mode uyarısı: Eğer query! makrosu kullanıyorsan bu kritik!
# ENV SQLX_OFFLINE=true 

# Bağımlılıkları pişirelim (Bu aşama en çok hata veren yerdir)
RUN cargo chef cook --release --recipe-path recipe.json

# Şimdi asıl kodu kopyalayıp derleyelim
COPY backend/ .
RUN cargo build --release --bin miransas-db

# ─────────────────────────────────────────────────────────────────────────────
# Stage 4 – Runtime
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

# Eğer migration dosyaların varsa onları da çekelim
# COPY --from=builder /app/migrations ./migrations

ENV APP_HOST=0.0.0.0 \
    APP_PORT=3001
EXPOSE 3001

CMD ["miransas-db"]