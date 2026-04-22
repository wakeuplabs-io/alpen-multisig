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

- Architecture overview: `docs/architecture/overview.md`
- Discovery index: `docs/2-discovery/README.md`
- Story map / slices: `docs/3-stories/story-map.md`
- Formal deliverable: `docs/deliverable/research.md`
- Agent guidance: `AGENTS.md` and `CLAUDE.md`

## Project Status

Phase 1 discovery and pre-main consolidation are complete on `develop`; implementation work resumes on top of this consolidated baseline.