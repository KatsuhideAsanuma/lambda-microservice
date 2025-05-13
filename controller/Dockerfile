FROM rust:latest as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to build dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code
COPY . .

# Build the application with static linking
ENV RUSTFLAGS="-C target-feature=+crt-static"
RUN cargo build --release

# Runtime stage
FROM rust:latest

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates wget && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/lambda-microservice-controller /app/lambda-microservice-controller

# Copy .env file
COPY .env .

# Expose the port
EXPOSE 8080

# Set the entrypoint
CMD ["/app/lambda-microservice-controller"]
