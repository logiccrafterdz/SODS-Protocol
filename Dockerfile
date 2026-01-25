# Builder stage
# Using rustlang/rust:nightly-slim for guaranteed nightly toolchain access
FROM rustlang/rust:nightly-slim AS builder

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
# Using cargo +nightly to be explicit about the toolchain for Edition 2024 support
# Disabling ZK for stability. Explicitly targeting sods-cli.
RUN cargo +nightly build --release -p sods-cli --bin sods --no-default-features

# Runtime stage
FROM gcr.io/distroless/cc-debian12

# Copy the binary
COPY --from=builder /app/target/release/sods /usr/local/bin/sods

ENTRYPOINT ["/usr/local/bin/sods"]
