# =============================================================================
# Stage 1: Build environment
# Ubuntu 24.04 (glibc 2.39) required — fastembed ONNX Runtime binary needs
# glibc 2.38+; Debian Bookworm (2.36) fails with __isoc23_strtoll undefined.
# =============================================================================
FROM ubuntu:24.04 AS builder

ENV DEBIAN_FRONTEND=noninteractive

# build-essential pulls in g++ and libstdc++-13-dev, satisfying -lstdc++ link.
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    build-essential \
    libpq-dev \
    pkg-config \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain via rustup (no official Ubuntu 24.04 Rust image exists).
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --no-modify-path --default-toolchain 1.93.1 --profile minimal

WORKDIR /app

# ------------------------------------------------------------------
# Cache dependencies separately so Docker layer cache is reused when
# only source files change (but not Cargo.toml / Cargo.lock).
# ------------------------------------------------------------------
COPY Cargo.toml Cargo.lock ./

# Create dummy source files so cargo can resolve the dependency graph.
RUN mkdir -p src benches && \
    echo "fn main() {}" > src/main.rs && \
    echo "" > src/lib.rs && \
    echo "fn main() {}" > benches/core_functions.rs

# Pre-fetch and compile all dependencies (cached layer).
RUN cargo build --release && rm -rf src benches

# ------------------------------------------------------------------
# Now copy the real source and build the actual binary.
# ------------------------------------------------------------------
COPY src ./src
COPY benches ./benches
COPY migrations ./migrations

# Touch both lib.rs and main.rs to invalidate cached artifacts from dummy build.
RUN touch src/lib.rs src/main.rs && cargo build --release

# =============================================================================
# Stage 2: Runtime image
# Ubuntu 24.04 required for glibc 2.38+ symbols from ONNX Runtime.
# =============================================================================
FROM ubuntu:24.04 AS runtime

ENV DEBIAN_FRONTEND=noninteractive

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
