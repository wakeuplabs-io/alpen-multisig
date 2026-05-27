# syntax=docker/dockerfile:1

# ── Build stage ───────────────────────────────────────────────────────────────
FROM rust:slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    git pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy toolchain pin first so rustup installs it before anything else
COPY rust-toolchain.toml ./
RUN rustup show

# Copy workspace manifests + lock file to cache dependency fetches as a layer.
# If only src changes (not Cargo files), Docker reuses the cached cargo fetch.
COPY Cargo.toml Cargo.lock ./
COPY orchestrator-be/Cargo.toml orchestrator-be/
COPY desktop-app/src-tauri/Cargo.toml desktop-app/src-tauri/
COPY e2e-tests/Cargo.toml e2e-tests/

# Stub out workspace members so `cargo fetch` resolves without real source
RUN mkdir -p orchestrator-be/src \
    && echo 'fn main(){}' > orchestrator-be/src/main.rs \
    && mkdir -p desktop-app/src-tauri/src \
    && echo 'fn main(){}' > desktop-app/src-tauri/src/main.rs \
    && mkdir -p e2e-tests/src \
    && touch e2e-tests/src/lib.rs

RUN cargo fetch

# Copy real source (migrations are embedded at compile time via sqlx::migrate!)
COPY orchestrator-be/src orchestrator-be/src/
COPY orchestrator-be/migrations orchestrator-be/migrations/

RUN cargo build --release -p orchestrator-be

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/server /usr/local/bin/orchestrator

# Bind to all interfaces by default; override via env at runtime
ENV SERVER_HOST=0.0.0.0
EXPOSE 3000

ENTRYPOINT ["/usr/local/bin/orchestrator"]
