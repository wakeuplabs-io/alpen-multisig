# E2E Tests — Alpen Multisig

End-to-end integration tests that exercise the moving parts of the multisig stack against real Alpen/Strata crates and a real orchestrator process.

This crate is a member of the Cargo workspace and is gated by the workspace `rust-toolchain.toml` (nightly; see [`docs/2-discovery/15-nightly-dependency-finding.md`](../docs/2-discovery/15-nightly-dependency-finding.md)).

## Test suites

| Test | What it covers | External requirements |
|---|---|---|
| [`e2e_admin_subprotocol`](./tests/e2e_admin_subprotocol.rs) | Full admin action flow against the upstream Alpen/Strata crates: generate signer keys → build `MultisigAction` → SPS-65 tagged sighash → ECDSA threshold signatures → SSZ-encoded SPS-50+51 transaction → parse back and verify. Stops before broadcast. Background in [`docs/2-discovery/03-poc1-findings.md`](../docs/2-discovery/03-poc1-findings.md). | None |
| [`e2e_propose_sign`](./tests/e2e_propose_sign.rs) | Desktop ↔ orchestrator integration. Spawns the real `orchestrator-be` binary as a subprocess and drives the desktop `application::proposals` layer over real HTTP (create → get → approve → verify_threshold). Exercises domain + action_codec + signing without pulling Strata crates into the desktop app. | None (builds and launches the orchestrator binary on a random port) |
| [`e2e_harness_hello_world`](./tests/e2e_harness_hello_world.rs) | Smoke test for the reusable ASM test harness (`src/test_harness.rs`). Boots Bitcoin regtest, launches the ASM worker, mines one block, and asserts that the processed height advances. Skipped at runtime if `bitcoind` is not in `PATH`. | `bitcoind` available in `PATH` |

## Requirements

- **Rust nightly** pinned via the workspace `rust-toolchain.toml`.
- Internet access on first build (git dependencies from `alpenlabs/asm` and `alpenlabs/strata-common`).
- A `bitcoind` binary in `PATH` to exercise `e2e_harness_hello_world`; other tests do not need it.

## Run

```sh
cargo test -p alpen-multisig-e2e-tests
```

Run a single test:

```sh
cargo test -p alpen-multisig-e2e-tests --test e2e_admin_subprotocol
```
