# Build stage
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
  pkg-config \
  libssl-dev \
  && rm -rf /var/lib/apt/lists/*

# Create a new empty shell project
WORKDIR /usr/src/keystonelight

# Copy the source code
COPY . .

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -u 1000 keystonelight

# Create necessary directories
RUN mkdir -p /var/lib/keystonelight && \
  chown keystonelight:keystonelight /var/lib/keystonelight

# Copy the binary from the builder stage
COPY --from=builder /usr/src/keystonelight/target/release/database /usr/local/bin/keystonelight
COPY --from=builder /usr/src/keystonelight/target/release/client /usr/local/bin/keystonelight-client

# Set the working directory
WORKDIR /var/lib/keystonelight

# Switch to non-root user
USER keystonelight

# Expose the server port
EXPOSE 7878

# Set environment variables
ENV RUST_LOG=info

# Run the server
CMD ["keystonelight", "serve"]