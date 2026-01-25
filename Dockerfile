# Builder stage
FROM rust:1.78-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    musl-tools \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the entire workspace
COPY . .

# Build the binary in release mode
# Use no-default-features to ensure ZK is handled as per configuration if needed, 
# but here we build the main CLI.
RUN cargo build --release --bin sods

# Runtime stage  
FROM gcr.io/distroless/cc-debian12

# Copy only the final binary
COPY --from=builder /app/target/release/sods /usr/local/bin/sods

# Set entrypoint to the binary
ENTRYPOINT ["/usr/local/bin/sods"]
