# Multi-stage build for CRUST server
# Stage 1: Builder (Debian-based for reliable OpenSSL linking on arm64/amd64)
FROM rust:1.75-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libzstd-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY gitcore ./gitcore
COPY crust-server ./crust-server
COPY crust-cli ./crust-cli

# Copy sqlx offline query cache (generated via `cargo sqlx prepare --workspace`)
COPY .sqlx ./.sqlx

# Build crust-server in release mode (SQLX_OFFLINE avoids needing a live DB at build time)
ENV SQLX_OFFLINE=true
RUN cargo build --release -p crust-server --bin crust-server

# Stage 2: Runtime (slim Debian for small footprint)
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies only
RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 libzstd1 wget \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/crust-server /app/crust-server

# Create data directory for object storage
RUN mkdir -p /data/repos && chmod 755 /data/repos

# Copy migration files
COPY crust-server/migrations /app/migrations

# Set environment variables with defaults
ENV PORT=8080 \
    LOG_LEVEL=info \
    REPO_BASE_PATH=/data/repos \
    RUST_LOG=crust_server=info,tower_http=debug

# Health check
HEALTHCHECK --interval=10s --timeout=5s --start-period=30s --retries=3 \
    CMD wget -q -O- http://localhost:8080/health | grep -q '"status":"ok"' || exit 1

# Expose port
EXPOSE 8080

# Start the server
CMD ["./crust-server"]
