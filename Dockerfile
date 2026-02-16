# Base stage with shared target dir
FROM lukemathwalker/cargo-chef:latest-rust-1-slim AS chef
WORKDIR /app
# Important: Force all cargo commands to use the same target directory
# This allows us to share artifacts between /app and /app/backend
ENV CARGO_TARGET_DIR=/app/target

FROM chef AS planner
COPY . .
WORKDIR /app/backend
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/backend/recipe.json recipe.json
# Build dependencies (runs in /app, stores in /app/target)
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
COPY . .
WORKDIR /app/backend
# Inherits CARGO_TARGET_DIR, so it uses the pre-built dependencies from /app/target
RUN cargo build --release --bin openlexer-backend

# Runtime stage
# "latest-rust-1-slim" uses Debian Bookworm, so we must match it here
FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

# Copy binary (from the global target dir)
COPY --from=builder /app/target/release/openlexer-backend /app/openlexer-backend

RUN useradd -ms /bin/bash appuser
USER appuser

ENV PORT=8000
ENV RUST_LOG=info
EXPOSE 8000

ENTRYPOINT ["./openlexer-backend"]
