# Build stage
FROM rust:latest as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
  pkg-config \
  libssl-dev \
  && rm -rf /var/lib/apt/lists/*

# Create a new empty shell project
WORKDIR /usr/src/keystonelight

# Copy the source code
COPY . .

# Build with release optimizations
RUN cargo build --release --verbose

# Create a smaller runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
  libssl3 \
  && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -u 1000 keystonelight

# Create necessary directories
RUN mkdir -p /var/lib/keystonelight && \
  chown keystonelight:keystonelight /var/lib/keystonelight

# Copy the binary from the builder stage
COPY --from=builder /usr/src/keystonelight/target/release/database /usr/local/bin/keystonelight
COPY --from=builder /usr/src/keystonelight/target/release/client /usr/local/bin/keystonelight-client

# Create entrypoint script
RUN echo '#!/bin/sh\nif [ "$1" = "--version" ]; then\n  keystonelight --version\nelif [ "$1" = "serve" ]; then\n  exec keystonelight serve\nelse\n  exec "$@"\nfi' > /usr/local/bin/entrypoint.sh && \
  chmod +x /usr/local/bin/entrypoint.sh

# Set the working directory
WORKDIR /var/lib/keystonelight

# Switch to non-root user
USER keystonelight

# Expose the server port
EXPOSE 7878

# Set environment variables
ENV RUST_LOG=info

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]

# Default command
CMD ["serve"]