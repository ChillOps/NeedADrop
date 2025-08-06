# Multi-stage build for security and efficiency
# Use the latest stable Rust image with a specific version tag
FROM rust:1-alpine AS builder

# Install system dependencies needed for compilation
RUN apk add --no-cache \
    pkgconfig \
    openssl-dev \
    sqlite \
    sqlite-dev \
    musl-dev \
    ca-certificates

# Create a new user for building (don't use root)
RUN addgroup -g 1000 rustbuild && \
    adduser -D -s /bin/sh -u 1000 -G rustbuild rustbuild

# Create app directory
WORKDIR /app

# Copy only dependency files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only (this layer will be cached)
RUN cargo build --release && rm -f target/release/deps/needadrop*

# Copy source code
COPY src ./src
COPY templates ./templates

# Build the actual application
RUN cargo build --release

# Strip binary to reduce size
RUN strip target/release/needadrop

# Runtime stage with minimal base image
FROM alpine:3.20

# Install only runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    sqlite \
    curl \
    tini

# Create a non-root user for running the application
RUN addgroup -g 1000 appuser && \
    adduser -D -s /bin/sh -u 1000 -G appuser appuser

# Create necessary directories
RUN mkdir -p /app/uploads /app/data && \
    chown -R appuser:appuser /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/needadrop /app/
COPY --from=builder /app/templates /app/templates

# Set ownership
RUN chown -R appuser:appuser /app

# Switch to non-root user
USER appuser

# Set work directory
WORKDIR /app

# Set environment variables
ENV RUST_LOG=info
ENV DATABASE_URL=sqlite:///app/data/needadrop.db
ENV UPLOAD_DIR=/app/uploads
ENV PORT=3000

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/ || exit 1

# Use tini as init system for proper signal handling
ENTRYPOINT ["/sbin/tini", "--"]

# Run the application
CMD ["./needadrop"]
