# Alpen Multisig

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Documentation Structure

```
docs/
├── 0-prd/          # Product Requirements from the client (Alpen) — do not modify
├── 1-proposal/     # WakeUp Labs technical proposal and scope — do not modify
└── 2-discovery/    # Research, ecosystem understanding, and POC plans/results
```

## Commands

### Rust Workspace

```bash
cargo build                                # Build entire workspace
cargo test                                 # Run all tests
cargo test -p orchestator-be               # Test backend only
cargo test -p alpen-multisig-e2e-tests     # Run e2e tests only
cargo test -p orchestator-be -- test_name  # Run a single test
cargo clippy                               # Lint
cargo fmt --check                          # Format check
```

### Desktop App

```bash
cd desktop-app && npm install     # Install dependencies
cd desktop-app && npm run dev     # Vite dev server
cd desktop-app && npm run build   # TypeScript + Vite build
cd desktop-app && npm run tauri   # Tauri CLI
```

### Running the System

1. Backend: `cargo run -p orchestator-be` (starts Axum server on port 3000)
2. Desktop: `cd desktop-app && npm run tauri dev`

## Architecture

**Cargo workspace** with 2 members: `orchestator-be`, `desktop-app/src-tauri`

- **`orchestator-be`** — Offchain coordination backend (Axum HTTP). Domain-driven layout: `domain/`, `handlers/`, `middleware/`, `state.rs`, `config.rs`, `error.rs`. Entry: `src/main.rs`
- **`desktop-app/src-tauri`** — Tauri 2 desktop shell. Entry: `src/main.rs`
- **`e2e-tests`** — Separate crate (not a workspace member) with integration tests against Alpen/Strata protocol crates. Depends on rev-pinned `alpenlabs/alpen` and tag-pinned `alpenlabs/strata-common` for `strata-asm-*`, `strata-crypto`, `strata-primitives`, `strata-l1-txfmt`.

**Frontend** (`desktop-app/src/`): React 18 + TypeScript + TailwindCSS + Vite + react-router-dom. Layout: `api/`, `hooks/`, `types/`, `App.tsx`, `main.tsx`

## Key Conventions

- **Protocol alignment**: SPS-50, SPS-51, SPS-65 are source of truth
- **Backend is coordination only**: proposal creation, signature collection, lifecycle tracking — never re-implement protocol validity rules
- **Signer safety**: Explicit confirmation steps, authority context, high-signal errors
- **Manual fallback**: Users can aggregate signatures and broadcast if backend unavailable
- **Error handling**: `anyhow::Result` for binaries, `thiserror` for libraries
- **Frontend**: Use tabs, single quotes, ~120 char lines, strict equality

## Additional Rule Files

- `.claude/rules/general.md` — Global defaults and formatting
- `.claude/rules/rust-backend-standards.md` — Backend-specific Rust rules
- `.claude/rules/backend-api-conventions.md` — API, auth, and data lifecycle rules
- `.claude/rules/typescript-standards.md` — TypeScript conventions
- `.claude/rules/react-frontend-patterns.md` — React patterns
