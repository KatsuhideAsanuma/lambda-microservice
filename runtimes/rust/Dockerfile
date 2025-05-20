FROM rust:1.68-slim as builder

WORKDIR /app

# Copy Cargo.toml (Cargo.lock is optional)
COPY Cargo.toml ./
# Create empty Cargo.lock if it doesn't exist
RUN touch Cargo.lock

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
FROM debian:bullseye-slim

WORKDIR /app

# Install dependencies including PostgreSQL client
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates postgresql-client && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/lambda-microservice-rust-runtime /app/

# Expose port
EXPOSE 8080

# Set environment variables
ENV RUST_LOG=info

# Run the application
CMD ["/app/lambda-microservice-rust-runtime"]
