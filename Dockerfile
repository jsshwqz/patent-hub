# Stage 1: Build
FROM rust:1.82-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release --bin patent-hub

# Stage 2: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/patent-hub .
COPY templates/ templates/
COPY static/ static/
COPY .env.example .env.example

EXPOSE 3000
ENV RUST_LOG=info
CMD ["./patent-hub"]
