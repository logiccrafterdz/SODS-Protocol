# ============================================================
# SODS Protocol — Multi-stage Docker Build
# ============================================================
# Stage 1: Build the Rust workspace (sods-cli binary)
# Stage 2: Minimal runtime image with health check
# ============================================================

# --- Builder Stage ---
FROM rustlang/rust:nightly-bookworm-slim AS builder

LABEL maintainer="LogicCrafter <logiccrafterdz@gmail.com>"
LABEL org.opencontainers.image.source="https://github.com/logiccrafterdz/SODS-Protocol"
LABEL org.opencontainers.image.description="SODS Protocol — Trustless Behavioral Verification"

# Install build dependencies
RUN apt-get update && apt-get install -y \
    musl-tools \
    pkg-config \
    libssl-dev \
    libdbus-1-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Ensure we are using nightly and check version
RUN rustup default nightly && rustc --version && cargo --version

# Copy the entire workspace
COPY . .

# Build the binary in release mode
# Disabling ZK feature for stability (risc0 requires a separate heavy toolchain).
RUN cargo +nightly build --release -p sods-cli --bin sods --no-default-features

# --- Runtime Stage ---
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary
COPY --from=builder /app/target/release/sods /usr/local/bin/sods

# Expose the SODS API and Prometheus metrics ports
EXPOSE 3000
EXPOSE 9090

# Health check: ping the daemon's health endpoint every 30 seconds
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/usr/local/bin/sods", "chains"]

# Default entrypoint
ENTRYPOINT ["/usr/local/bin/sods"]
CMD ["daemon", "--chain", "sepolia"]
