# Alpen Multisig

Desktop multisig client and coordination backend for Alpen/Strata governance flows. The project enables authorized signers to prepare governance payloads, collect signatures off-chain, and broadcast Bitcoin transactions that ASM processes deterministically.

## Architecture (high level)

- `desktop-app` (Tauri + React): signer-facing UI and hardware-wallet bridge
- `orchestrator-be` (Axum): off-chain proposal and signature coordination API
- `e2e-tests` (Rust): integration coverage against upstream ASM/Strata crates and orchestrator flow

## Repository Layout

- `orchestrator-be/`: Rust backend API (`/api/v1`) with in-memory repository
- `desktop-app/src-tauri/`: Tauri Rust shell, signing and wallet integrations
- `desktop-app/src/`: React UI (wallet connect + signing PoC screens)
- `e2e-tests/`: protocol and cross-component end-to-end tests
- `docs/`: PRDs, proposal, discovery, story map, architecture, and per-feature specs
- Ops & safety (Wave 2): [runbook](docs/operations/runbook.md), [threat model](docs/security/threat-model.md), [signer safety model](docs/specs/signer-safety-model.md)

## Prerequisites

- Rust toolchain from `rust-toolchain.toml` (nightly pinned by project)
- Node.js 20+ and npm
- Tauri system dependencies for your OS
- Optional for harness test: `bitcoind` in `PATH`

## Build & Run

### Rust workspace

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --no-deps
cargo fmt --check
```

### Backend API

```bash
cargo run -p orchestrator-be
```

### Desktop app

```bash
cd desktop-app
npm install
npm run dev        # frontend only
npm run tauri dev  # full desktop app
npm run build
```

## Documentation Pointers

### External (client-facing) documentation

Client-facing deliverables live in [`docs/external/`](docs/external/README.md) — start there for setup, architecture, API, and release verification.

- **New here? Try the app end-to-end:** [Local Dev Smoke Test Guide](docs/external/local-dev-smoke-test-guide.md) — a beginner-friendly, assume-nothing walkthrough that brings up the full local stack and runs a complete governance action on regtest.
- Install a packaged release: [Setup Guide](docs/external/setup-guide.md)
- System design: [Architecture Overview](docs/external/architecture-overview.md)
- Backend endpoints: [API Reference](docs/external/api-reference.md)
- Full index: [`docs/external/README.md`](docs/external/README.md)

### Internal references

- Architecture overview: `docs/architecture/overview.md`
- Discovery index: `docs/2-discovery/README.md`
- Story map / slices: `docs/3-stories/story-map.md`
- Formal deliverable: `docs/deliverable/research.md`
- Agent guidance: `AGENTS.md` and `CLAUDE.md`

## Project Status

Phase 1 discovery and pre-main consolidation are complete on `develop`; implementation work resumes on top of this consolidated baseline.