# Backend Deployment Guide

**Audience:** infrastructure teams deploying the Strata Multisig backend.

## Components

| Component | Deploy? | Notes |
|-----------|---------|-------|
| **Orchestrator** | Yes | REST API written in Rust, port `3000`, routes under `/api/v1`. Run **a single instance**: sign-in sessions live in memory. |
| **PostgreSQL** 15+ | Yes | Stores proposals and signatures. The orchestrator runs its migrations at startup, so its user needs DDL rights on the database. |
| **Bitcoin Core** (mainnet) | No (use the existing node) | The orchestrator connects to an existing mainnet node over RPC, e.g. the one the ASM uses. It must run with `-txindex=1`. |
| **ASM runner** | No (already deployed) | The orchestrator only needs its RPC URL. |
| **Desktop app** | No (installed by each signer) | Needs several endpoints reachable from signers' machines; see [Desktop app](#desktop-app-not-deployed). |

Health checks: `GET /api/v1/health` (liveness), `GET /api/v1/ready` (Bitcoin RPC reachable).

### Orchestrator

A standard REST API written in Rust. Deploy it however you prefer (VM, container, PaaS). It is a single binary with no runtime dependencies beyond OpenSSL and CA certificates.

```bash
cargo build --release -p orchestrator-be   # from the repo root; produces target/release/server
```

The Rust toolchain is pinned in [`rust-toolchain.toml`](../../rust-toolchain.toml). Database migrations run automatically at startup. TLS and rate limiting are not built in; put it behind your usual reverse proxy or load balancer.

## Environment variables

### Orchestrator

| Variable | Required | Mainnet value |
|----------|----------|---------------|
| `DATABASE_URL` | Yes | `postgres://<user>:<pass>@<db-host>:5432/<db>?sslmode=require` |
| `ORCHESTRATOR_PROFILE` | Yes | `production` |
| `STRATA_ADMIN_STATE_RPC_URL` | Yes | `https://<asm-rpc-host>` |
| `BITCOIN_RPC_URL` | Yes | `https://<bitcoin-rpc-host>` |
| `BITCOIN_RPC_USER` | Yes | `<rpc-user>` |
| `BITCOIN_RPC_PASS` | Yes | `<rpc-password>` |
| `BITCOIN_NETWORK` | Yes | `bitcoin` |
| `SERVER_HOST` | Yes | `0.0.0.0` |
| `SERVER_PORT` | No | `3000` |
| `PROPOSAL_EXPIRY_DAYS` | No | `7` |
| `AUTH_CHALLENGE_TTL_MS` | No | `120000` |
| `AUTH_SESSION_TTL_MS` | No | `86400000` |
| `RUST_LOG` | No | `info` |

What each variable is for:

- **`DATABASE_URL`**: the Postgres database where the orchestrator stores proposals, collected signatures, and broadcast attempts. Pending proposals exist only here, so back it up.
- **`ORCHESTRATOR_PROFILE`**: set to `production` so the service refuses to start without `DATABASE_URL`. Without it, a missing database URL silently falls back to in-memory storage and all proposals are lost on restart.
- **`STRATA_ADMIN_STATE_RPC_URL`**: the RPC endpoint of the already-deployed ASM runner. The orchestrator asks the ASM whether a wallet belongs to an authority's signer set when someone signs in, what the current signature threshold is, and which sequence numbers are already used on-chain.
- **`BITCOIN_RPC_URL`**: the Bitcoin Core RPC endpoint. The orchestrator only reads from the node: chain height to compute when an approved proposal activates, and transactions to confirm which broadcast of a proposal was mined. It never signs or broadcasts.
- **`BITCOIN_RPC_USER` / `BITCOIN_RPC_PASS`**: credentials for that RPC, matching `rpcauth` (or `rpcuser`/`rpcpassword`) in the node's `bitcoin.conf`. Cookie authentication is not supported. When using a hosted RPC provider that carries its token in the URL, these can be any value.
- **`BITCOIN_NETWORK`**: the chain the orchestrator works on (`bitcoin` = mainnet). It is part of the message signers sign, so a wrong value makes every signature check fail. The default is `regtest`, so it must be set.
- **`SERVER_HOST`**: the address the HTTP API listens on. The default is `127.0.0.1`, which only accepts connections from the same machine, so set `0.0.0.0` for the API to be reachable from your proxy or load balancer.
- **`SERVER_PORT`**: the port the HTTP API listens on (`3000` by default).
- **`PROPOSAL_EXPIRY_DAYS`**: days a proposal stays open. If it is not broadcast within this window it moves to Expired.
- **`AUTH_CHALLENGE_TTL_MS`**: how long a sign-in challenge is valid (2 minutes by default). The signer must sign it with their wallet within this time.
- **`AUTH_SESSION_TTL_MS`**: how long a signer stays signed in (24 hours by default) before having to sign in again.
- **`RUST_LOG`**: log verbosity (`info`, `debug`, …). Logs go to stdout.

The URLs are used as given: without a port, `https` uses 443. The examples assume the ASM and Bitcoin RPCs are exposed behind HTTPS. When connecting directly to the services instead, include the port, e.g. `http://<bitcoind-host>:8332` (`8332` is the default Bitcoin Core RPC port on mainnet) and the port the ASM RPC was configured with.

Example `.env` for mainnet:

```bash
ORCHESTRATOR_PROFILE=production
DATABASE_URL=postgres://<user>:<pass>@<db-host>:5432/<db>?sslmode=require
STRATA_ADMIN_STATE_RPC_URL=https://<asm-rpc-host>
BITCOIN_NETWORK=bitcoin
BITCOIN_RPC_URL=https://<bitcoin-rpc-host>
BITCOIN_RPC_USER=<rpc-user>
BITCOIN_RPC_PASS=<rpc-password>
SERVER_HOST=0.0.0.0
```

### PostgreSQL

No special settings. Create a database and a user that owns it, then pass them in `DATABASE_URL`.

### Bitcoin Core (existing node)

Not deployed as part of this backend. The existing node must have these `bitcoind` flags (or `bitcoin.conf` entries):

| Setting | Value |
|---------|-------|
| `txindex` | `1` (required) |
| `server` | `1` |
| `rpcuser` / `rpcpassword` (or `rpcauth`) | Same credentials as `BITCOIN_RPC_USER` / `BITCOIN_RPC_PASS` |
| `rpcbind` / `rpcallowip` | Allow the orchestrator's host |
| `chain` | `main` (mainnet) |

## Desktop app (not deployed)

Each signer installs the desktop app from GitHub Releases. It is not deployed by infra, but it connects to these endpoints, which must be reachable from signers' machines. Each signer sets them in **Settings > Node**:

| Endpoint | Used for |
|----------|----------|
| Orchestrator | Proposals, signatures and broadcast coordination. The public HTTPS URL of the orchestrator, e.g. `https://<orchestrator-host>/api/v1` |
| ASM RPC | Reading on-chain governance state |
| Bitcoin Core RPC (mainnet) | Broadcasting transactions and estimating fees. Signers enter the RPC credentials in the app |
| Electrum server (mainnet, `ssl://`) | Syncing the wallet that pays broadcast fees |

The app's **Trusted endpoint** mode uses `https://rpc.stratabtc.org` (ASM), `https://btc-rpc.stratabtc.org` (Bitcoin RPC) and `ssl://electrum.stratabtc.org:50002` (Electrum). If those are available on mainnet, signers only need to set the orchestrator URL.

### Desktop release after the backend is deployed

Once the backend is deployed and the production URLs are known, a new desktop release will be built with mainnet defaults, so signers do not have to configure anything by hand:

| Default | Value |
|---------|-------|
| Bitcoin network | `bitcoin` (mainnet) |
| Orchestrator URL | The deployed orchestrator, e.g. `https://<orchestrator-host>/api/v1` |
| Trusted endpoints | Production ASM, Bitcoin RPC and Electrum URLs |
| Connection mode | Trusted endpoint |
| Magic bytes | Mainnet value, to be confirmed by Alpen |

To prepare this release, infra needs to share the final public URLs of the orchestrator, ASM RPC, Bitcoin RPC and Electrum server.
