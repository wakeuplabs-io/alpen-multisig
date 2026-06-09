# syntax=docker/dockerfile:1
#
# Staging asm image: strata-asm-runner only.
# Connects to alpen-multisig-bitcoin via Render private network.
# Exposes Strata admin-state RPC on port 8080 (Render web port).
#
# NOT for production use.

# ── Stage 1: Build strata-asm-runner ─────────────────────────────────────────
FROM rust:slim-bookworm AS build

RUN apt-get update && apt-get install -y --no-install-recommends \
    git pkg-config libssl-dev ca-certificates \
    build-essential libzmq3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build/asm
COPY asm/ .
RUN rustup show
RUN cargo build --release --bin strata-asm-runner

# ── Stage 2: Runtime ─────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl libssl3 jq \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build /build/asm/target/release/strata-asm-runner /usr/local/bin/strata-asm-runner
COPY staging/entrypoint-asm.sh           /entrypoint.sh
COPY staging/asm-params.template.json    /app/asm-params.template.json
COPY staging/asm-config-regtest.toml     /app/asm-config.toml

RUN chmod +x /entrypoint.sh /usr/local/bin/strata-asm-runner

# Strata admin-state RPC — Render routes HTTPS → this port
EXPOSE 8080

ENTRYPOINT ["/entrypoint.sh"]
