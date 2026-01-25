# Builder stage
# Using nightly to support Rust 2024 edition requirements in dependencies
FROM rust:nightly-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    musl-tools \
    pkg-config \
    libssl-dev \
    libdbus-1-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Debug: Print toolchain versions
RUN rustc --version && cargo --version

# Copy the entire workspace
COPY . .

# Build the binary in release mode
# Disabling ZK for stability. Explicitly targeting sods-cli.
RUN cargo build --release -p sods-cli --bin sods --no-default-features

# Runtime stage
FROM gcr.io/distroless/cc-debian12

# Copy the binary
COPY --from=builder /app/target/release/sods /usr/local/bin/sods

ENTRYPOINT ["/usr/local/bin/sods"]
