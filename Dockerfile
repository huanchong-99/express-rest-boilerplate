# ============================================================
# Stage 1: Chef – prepare a recipe for dependency caching
# ============================================================
FROM rust:1.83-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

# ============================================================
# Stage 2: Planner – analyse the dependency graph
# ============================================================
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ============================================================
# Stage 3: Builder – compile dependencies then the application
# ============================================================
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies – this layer is cached as long as Cargo.toml
# / Cargo.lock do not change.
RUN cargo chef cook --release --recipe-path recipe.json

# Copy the full source and build the binary
COPY . .
RUN cargo build --release --bin express_rest_boilerplate

# ============================================================
# Stage 4: Runtime – minimal image with the compiled binary
# ============================================================
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/express_rest_boilerplate /app/express_rest_boilerplate
COPY migrations/ /app/migrations/

ENV RUST_ENV=production
ENV HOST=0.0.0.0
ENV PORT=3000

EXPOSE 3000

CMD ["/app/express_rest_boilerplate"]
