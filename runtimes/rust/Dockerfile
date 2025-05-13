FROM rust:latest as builder

WORKDIR /app

# Copy Cargo.toml and Cargo.lock
COPY Cargo.toml ./

# Create dummy source file to build dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source code
COPY . .

# Build the application with static linking
ENV RUSTFLAGS="-C target-feature=+crt-static"
RUN cargo build --release

# Create runtime image
FROM rust:latest

WORKDIR /app

# Install dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/lambda-microservice-rust-runtime /app/

# Expose port
EXPOSE 8080

# Set environment variables
ENV RUST_LOG=info

# Run the application
CMD ["/app/lambda-microservice-rust-runtime"]
