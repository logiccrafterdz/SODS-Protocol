# Builder stage
# Using bookworm (full image) to ensure all tools are present and definitive version 1.85
FROM rust:1.85-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    musl-tools \
    pkg-config \
    libssl-dev \
    libdbus-1-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Check toolchain version (Debug)
RUN cargo --version

# Copy the entire workspace
COPY . .

# Build the binary in release mode
# Explicitly build the sods-cli package without default features (disables ZK)
# to avoid heavy RISC Zero build-time dependencies in the builder env.
# Adding a comment to force a change in the line count/hash: Final Build Trigger
RUN cargo build --release -p sods-cli --bin sods --no-default-features

# Runtime stage  
FROM gcr.io/distroless/cc-debian12

# Copy only the final binary
COPY --from=builder /app/target/release/sods /usr/local/bin/sods

# Set entrypoint to the binary
ENTRYPOINT ["/usr/local/bin/sods"]
