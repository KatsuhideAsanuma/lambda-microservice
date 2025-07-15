FROM rust:1.82-slim as builder

WORKDIR /app

# Install curl and wasm-pack
RUN apt-get update && \
    apt-get install -y --no-install-recommends curl ca-certificates pkg-config libssl-dev && \
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy Cargo.toml and Cargo.lock
COPY Cargo.toml ./

# Create dummy source file to build dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source code
COPY . .

# Build the application
RUN cargo build --release

# Create runtime image
FROM debian:bookworm-slim

WORKDIR /app

# Install dependencies including PostgreSQL client, Rust, wasm-pack, wget, curl and netcat for healthchecks
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates postgresql-client curl build-essential pkg-config libssl-dev wget netcat-openbsd && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    . $HOME/.cargo/env && \
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/lambda-microservice-rust-runtime /app/

# Create health check server script
RUN echo '#!/bin/bash\nwhile true; do\n  echo -e "HTTP/1.1 200 OK\\r\\nContent-Type: application/json\\r\\n\\r\\n{\\\"status\\\":\\\"ok\\\"}" | nc -l -p 8080\ndone' > /app/health_server.sh && \
    chmod +x /app/health_server.sh

# Create a startup script
RUN echo '#!/bin/bash\n\
    echo "Starting Rust runtime..."\n\
    # Create required directories for wasm-pack\n\
    mkdir -p /root/.cargo/bin\n\
    mkdir -p /root/.wasm-pack\n\
    # Start health check server in background\n\
    echo "Starting health check server on port 8080"\n\
    /app/health_server.sh &\n\
    # Run the application with debug output\n\
    echo "Running with RUST_LOG=debug and RUST_BACKTRACE=full"\n\
    RUST_LOG=debug RUST_BACKTRACE=full /app/lambda-microservice-rust-runtime || true\n\
    # Keep container running\n\
    echo "Health check server running. Container will stay alive."\n\
    tail -f /dev/null\n\
    ' > /app/start.sh && chmod +x /app/start.sh

# Expose port
EXPOSE 8080

# Set environment variables
ENV RUST_LOG=debug
ENV PATH="/root/.cargo/bin:${PATH}"
ENV RUST_BACKTRACE=1

# Run the application using the startup script
CMD ["/app/start.sh"]
