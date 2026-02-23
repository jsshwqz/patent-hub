# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY templates ./templates
COPY static ./static

# Build release
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/patent-hub /app/patent-hub

# Copy templates and static files
COPY --from=builder /app/templates /app/templates
COPY --from=builder /app/static /app/static

# Copy env example
COPY .env.example /app/.env.example

# Expose port
EXPOSE 3000

# Run
CMD ["./patent-hub"]
