# syntax=docker/dockerfile:1
#
# Staging-only multi-process image: bitcoind (regtest) + strata-asm-runner + orchestrator-be
# All three run in a single container via supervisord.
#
# NOT for production use. Exposes DEV_MINE_ENABLED and mnemonic signing.

# ── Stage 1: Build strata-asm-runner ─────────────────────────────────────────
FROM rust:slim-bookworm AS build-asm

RUN apt-get update && apt-get install -y --no-install-recommends \
    git pkg-config libssl-dev ca-certificates \
    build-essential \
    libzmq3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build/asm

# Copy the asm workspace
COPY asm/ .

# rustup picks up rust-toolchain.toml automatically
RUN rustup show

RUN cargo build --release --bin strata-asm-runner

# ── Stage 2: Build orchestrator-be ───────────────────────────────────────────
FROM rust:slim-bookworm AS build-orchestrator

RUN apt-get update && apt-get install -y --no-install-recommends \
    git pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY rust-toolchain.toml ./
RUN rustup show

# Cache dependency fetches as a layer
COPY Cargo.toml Cargo.lock ./
COPY orchestrator-be/Cargo.toml orchestrator-be/
COPY desktop-app/src-tauri/Cargo.toml desktop-app/src-tauri/
COPY e2e-tests/Cargo.toml e2e-tests/

RUN mkdir -p orchestrator-be/src \
    && echo 'fn main(){}' > orchestrator-be/src/main.rs \
    && mkdir -p desktop-app/src-tauri/src \
    && echo 'fn main(){}' > desktop-app/src-tauri/src/main.rs \
    && mkdir -p e2e-tests/src \
    && touch e2e-tests/src/lib.rs

RUN cargo fetch

COPY orchestrator-be/src orchestrator-be/src/
COPY orchestrator-be/migrations orchestrator-be/migrations/

RUN cargo build --release -p orchestrator-be

# ── Stage 3: Runtime ─────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    libssl3 \
    supervisor \
    && rm -rf /var/lib/apt/lists/*

# Install Bitcoin Core from the official release (no PPA needed on Debian).
# TARGETARCH is set automatically by Docker based on --platform (amd64 / arm64).
ARG BITCOIN_VERSION=28.1
ARG TARGETARCH
RUN case "${TARGETARCH}" in \
      amd64) BITCOIN_ARCH=x86_64-linux-gnu ;; \
      arm64) BITCOIN_ARCH=aarch64-linux-gnu ;; \
      *)     echo "Unsupported arch: ${TARGETARCH}" && exit 1 ;; \
    esac \
    && curl -fsSL "https://bitcoincore.org/bin/bitcoin-core-${BITCOIN_VERSION}/bitcoin-${BITCOIN_VERSION}-${BITCOIN_ARCH}.tar.gz" \
    | tar -xz -C /tmp \
    && install -m 755 /tmp/bitcoin-${BITCOIN_VERSION}/bin/bitcoind  /usr/local/bin/bitcoind \
    && install -m 755 /tmp/bitcoin-${BITCOIN_VERSION}/bin/bitcoin-cli /usr/local/bin/bitcoin-cli \
    && rm -rf /tmp/bitcoin-${BITCOIN_VERSION}

# Binaries from build stages
COPY --from=build-asm        /build/asm/target/release/strata-asm-runner /usr/local/bin/strata-asm-runner
COPY --from=build-orchestrator /build/target/release/server               /usr/local/bin/orchestrator

# Staging config files
COPY staging/supervisord.conf          /etc/supervisor/conf.d/staging.conf
COPY staging/entrypoint.sh             /entrypoint.sh
COPY staging/asm-params.template.json  /app/asm-params.template.json
COPY staging/asm-config.toml           /app/asm-config.toml

RUN chmod +x /entrypoint.sh /usr/local/bin/strata-asm-runner /usr/local/bin/orchestrator

# Orchestrator port
EXPOSE 3000

ENTRYPOINT ["/entrypoint.sh"]
