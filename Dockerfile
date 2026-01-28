# ============================================
# Stage 1: Build
# ============================================
FROM rust:1.85-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for better caching
COPY Cargo.toml Cargo.lock ./

# Create dummy src to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    mkdir -p src/bin && \
    echo "fn main() {}" > src/bin/server.rs && \
    echo "fn main() {}" > src/bin/smooth_csv.rs && \
    echo "fn main() {}" > src/bin/test_smooth.rs

# Downgrade home crate to version compatible with Rust 1.85
# home 0.5.12 requires Rust 1.88 (not yet released)
RUN cargo update home --precise 0.5.11

# Build dependencies only (will be cached)
RUN cargo build --release --bin server && \
    rm -rf src

# Copy actual source code
COPY src ./src

# Touch files to ensure rebuild
RUN touch src/main.rs src/lib.rs src/bin/server.rs

# Build the actual application
RUN cargo build --release --bin server

# ============================================
# Stage 2: Runtime
# ============================================
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 mortar

# Copy binary from builder
COPY --from=builder /app/target/release/server /app/server

# Copy data files and web assets
COPY data ./data
COPY src/web ./src/web

# Set ownership
RUN chown -R mortar:mortar /app

# Switch to non-root user
USER mortar

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/api/health || exit 1

# Run the server
CMD ["./server"]
