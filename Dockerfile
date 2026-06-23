# ── Build stage ────────────────────────────────────────
FROM rust:1.82-slim AS builder

WORKDIR /app

# PostgreSQL client libs + SSL for compilation
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Cache dependency builds
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release 2>/dev/null || true
RUN rm -rf src

# Copy actual source and build
COPY src ./src
COPY Cargo.toml Cargo.lock* ./
RUN touch src/main.rs && cargo build --release

# ── Runtime stage ──────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    git \
    curl \
    docker.io \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/vercel-clone /usr/local/bin/vercel-clone

# Create directories
RUN mkdir -p /data/artifacts /app/caddy/configs

EXPOSE 3000

CMD ["vercel-clone"]
