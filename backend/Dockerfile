# Builder stage
FROM lukemathwalker/cargo-chef:latest-rust-1-slim AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build our project dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json
# Up to this point, if our dependency tree stays the same,
# all layers should be cached.
COPY . .
RUN cargo build --release --bin openlexer-backend

# Runtime stage
FROM debian:bullseye-slim AS runtime
WORKDIR /app
# Install OpenSSL/Ca-Certificates (Required for Reqwest/HTTPS)
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    # Clean up to keep image small
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder environment
COPY --from=builder /app/target/release/openlexer-backend /app/openlexer-backend

# Use non-root user for security
RUN useradd -ms /bin/bash appuser
USER appuser

# Set environment variables
ENV PORT=8080
ENV RUST_LOG=info

# Expose the port
EXPOSE 8080

# Run the binary
ENTRYPOINT ["./openlexer-backend"]
