# Alpen Multisig

**Desktop multisig client and off-chain coordination backend for Alpen/Strata governance.**

Alpen Multisig lets authorized signers prepare governance payloads, collect signatures off-chain, and broadcast Bitcoin transactions that the Alpen State Machine (ASM) processes deterministically. It targets the administrative multisigs defined by the Strata and Alpen protocols — the Alpen Administrator, Strata Administrator, Sequencer Manager, Security Council, and Payout Administrator — with a focus on signer safety, explicit confirmation, and high-signal feedback.

---

## Download

Signed installers for Linux, macOS, and Windows are published on the **[GitHub Releases page](https://github.com/wakeuplabs-io/alpen-multisig/releases/latest)**. Each release includes checksums, a detached GPG signature, and reproducible build digests — **verify artifacts before use** ([Verifying Releases](docs/external/verifying-releases.md)).

Installation steps per platform: [Setup Guide](docs/external/setup-guide.md).

## Capabilities

- Governance proposals across the Strata and Alpen administrative multisigs — create, review, sign, and broadcast.
- Off-chain signature collection, with a manual fallback when the backend is unavailable.
- Admin Wallet: send and receive BTC, fee control, and fee-bump (RBF / CPFP).
- Hardware wallet integration (Trezor, Ledger) — see the [compatibility matrix](docs/external/hardware-wallet-matrix.md).
- Signed, reproducible builds for Linux, macOS, and Windows.

## Documentation

Comprehensive documentation lives in **[`docs/external/`](docs/external/README.md)**. Each document is self-contained and can be read independently of the source tree.

| Topic | Document |
|-------|----------|
| Install a packaged release | [Setup Guide](docs/external/setup-guide.md) |
| Run the full stack from source, end-to-end | [Local Dev Smoke Test Guide](docs/external/local-dev-smoke-test-guide.md) |
| System design and data flow | [Architecture Overview](docs/external/architecture-overview.md) |
| Backend endpoints | [API Reference](docs/external/api-reference.md) |
| Supported hardware wallets | [Compatibility Matrix](docs/external/hardware-wallet-matrix.md) |
| Build, package, distribute | [Build and Release Process](docs/external/build-and-release-process.md) |
| Verify authenticity | [Verifying Releases](docs/external/verifying-releases.md) · [Reproducible Builds](docs/external/reproducible-builds.md) |
| Quality and risk | [Integration Test Report](docs/external/integration-test-report.md) · [Security Review Summary](docs/external/security-review-summary.md) |

## Architecture

The system is a Cargo workspace plus a React/Tauri desktop frontend.

- **`desktop-app`** (Tauri 2 + React + TypeScript) — signer-facing UI, wallet integration, and hardware-wallet bridge.
- **`orchestrator-be`** (Axum) — off-chain coordination service exposing `/api/v1`: proposal creation, signature collection, and lifecycle tracking. It **never** re-implements protocol validity rules; SPS-50/51/65 remain the source of truth.
- **`e2e-tests`** (Rust) — integration coverage against upstream ASM/Strata crates and the orchestrator flow.

**Design principle:** the backend is coordination-only. If it is unavailable, signers can still aggregate signatures and broadcast manually. Full detail in the [Architecture Overview](docs/external/architecture-overview.md).

## Repository Layout

| Path | Contents |
|------|----------|
| `orchestrator-be/` | Backend API (`/api/v1`), domain-driven layout |
| `desktop-app/src-tauri/` | Tauri Rust shell, signing and wallet integration |
| `desktop-app/src/` | React UI (wallet connect, signing, proposal, and admin wallet screens) |
| `e2e-tests/` | Protocol and cross-component end-to-end tests |
| `docs/external/` | Reference documentation |
| `docs/` | PRDs, proposal, discovery, architecture, and per-feature specs |

## Building from Source

For development and reproducible verification. To simply run the application, install a packaged release instead.

### Prerequisites

- Rust toolchain pinned by `rust-toolchain.toml` (nightly)
- Node.js 20.x and npm
- Tauri system dependencies for your OS
- Optional, for the harness test: `bitcoind` on `PATH`

### Rust workspace

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

### Backend API

```bash
cargo run -p orchestrator-be   # Axum server on port 3000
```

### Desktop app

```bash
cd desktop-app
npm install
npm run dev        # frontend only (no Tauri IPC)
npm run tauri dev  # full desktop app
npm run build
```

To bring up the full system locally — backend, desktop app, and a regtest governance action — follow the [Local Dev Smoke Test Guide](docs/external/local-dev-smoke-test-guide.md).

## Security

- Release artifacts are cryptographically signed and reproducible; verify before use ([Verifying Releases](docs/external/verifying-releases.md)).
- Protocol alignment is enforced against SPS-50, SPS-51, and SPS-65; the backend never redefines governance or validity rules.
- See the [Security Review Summary](docs/external/security-review-summary.md) for the current analysis and recommendations.

To report a vulnerability, please contact the maintainers privately rather than filing a public issue.

## Support

- **Issues and feedback:** [github.com/wakeuplabs-io/alpen-multisig/issues](https://github.com/wakeuplabs-io/alpen-multisig/issues)
- **Documentation:** [`docs/external/`](docs/external/README.md)
