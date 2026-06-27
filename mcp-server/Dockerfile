# Build stage
FROM rust:1.80-slim AS builder

WORKDIR /usr/src/mcp-server
COPY . .

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Build the release binary
RUN cargo build --release

# Run stage
FROM debian:bookworm-slim

# Install runtime dependencies for TLS/HTTPS
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/mcp-server/target/release/mcp-server /app/mcp-server

EXPOSE 8080

ENV MCP_PORT=8080

CMD ["/app/mcp-server"]
