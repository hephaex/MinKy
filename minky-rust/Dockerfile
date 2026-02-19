# =============================================================================
# Stage 1: Build environment
# Uses the official Rust image pinned to a stable toolchain.
# =============================================================================
FROM rust:1.82-slim-bookworm AS builder

# Install build-time OS dependencies (libpq for sqlx, pkg-config for linking).
RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# ------------------------------------------------------------------
# Cache dependencies separately so Docker layer cache is reused when
# only source files change (but not Cargo.toml / Cargo.lock).
# ------------------------------------------------------------------
COPY Cargo.toml Cargo.lock ./

# Create a dummy lib.rs so cargo can resolve the dependency graph.
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs

# Pre-fetch and compile all dependencies (cached layer).
RUN cargo build --release && rm -rf src

# ------------------------------------------------------------------
# Now copy the real source and build the actual binary.
# ------------------------------------------------------------------
COPY src ./src
COPY migrations ./migrations

# Touch main.rs so cargo detects the change and recompiles the crate.
RUN touch src/main.rs && cargo build --release

# =============================================================================
# Stage 2: Runtime image
# Only copy the compiled binary and required runtime libs.
# Result is a minimal, non-root image.
# =============================================================================
FROM debian:bookworm-slim AS runtime

# Install runtime OS dependencies only.
RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq5 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create a dedicated non-root user for security.
RUN useradd --create-home --shell /bin/bash --uid 1001 minky

WORKDIR /app

# Copy the compiled binary from the builder stage.
COPY --from=builder /app/target/release/minky ./minky

# Copy migrations so the server can run sqlx migrate on startup if needed.
COPY --from=builder /app/migrations ./migrations

# Ensure the binary is executable and owned by the app user.
RUN chown -R minky:minky /app

USER minky

# Port the Axum server listens on.
EXPOSE 8000

# Health check: call the /api/health endpoint.
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8000/api/health || exit 1

# Run the binary directly (no shell wrapper, so signals are forwarded).
ENTRYPOINT ["./minky"]
