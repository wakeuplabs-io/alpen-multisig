# E2E Tests — Alpen Multisig

End-to-end integration tests that exercise the moving parts of the multisig stack against real Alpen/Strata crates and a real orchestrator process.

This crate is a member of the Cargo workspace and is gated by the workspace `rust-toolchain.toml` (nightly; see [`docs/2-discovery/15-nightly-dependency-finding.md`](../docs/2-discovery/15-nightly-dependency-finding.md)).

## Test suites

| Test | What it covers | External requirements |
|---|---|---|
| [`e2e_admin_subprotocol`](./tests/e2e_admin_subprotocol.rs) | Full admin action flow against the upstream Alpen/Strata crates: generate signer keys → build `MultisigAction` → SPS-65 tagged sighash → ECDSA threshold signatures → SSZ-encoded SPS-50+51 transaction → parse back and verify. Stops before broadcast. Background in [`docs/2-discovery/03-poc1-findings.md`](../docs/2-discovery/03-poc1-findings.md). | None |
| [`e2e_admin_commit_reveal`](./tests/e2e_admin_commit_reveal.rs) | On-chain commit → reveal with regtest + harness: funded envelope, mined commit, reveal tx, parse SPS payload and verify threshold signatures. | `bitcoind` in `PATH` |
| [`e2e_propose_sign`](./tests/e2e_propose_sign.rs) | Desktop ↔ orchestrator integration. Spawns the real `orchestrator-be` binary as a subprocess and drives the desktop `application::proposals` layer over real HTTP (create → get → approve → verify_threshold). Exercises domain + action_codec + signing without pulling Strata crates into the desktop app. | None (builds and launches the orchestrator binary on a random port) |
| [`e2e_harness_hello_world`](./tests/e2e_harness_hello_world.rs) | Smoke test for the reusable ASM test harness (`src/test_harness.rs`). Boots Bitcoin regtest, launches the ASM worker, mines one block, and asserts that the processed height advances. Skipped at runtime if `bitcoind` is not in `PATH`. | `bitcoind` in `PATH` |
| [`e2e_signer_update_enacted_light`](./tests/e2e_signer_update_enacted_light.rs) | Desktop signing + `broadcast_tx` against regtest + ASM: multisig update is committed, revealed, mined past confirmation depth, and **enacted** state is asserted via `AdministrationSubprotoState`. Uses shared fixtures in [`src/fixtures/`](./src/fixtures/). Two cases: `DEFAULT_REPO_ASM` (confirmation depth 144) and `FAST_ENACTMENT` (depth 5). Skipped if `bitcoind` is missing. | `bitcoind` in `PATH` |
| [`e2e_enactment_predicate`](./tests/e2e_enactment_predicate.rs) | `asm_enactment::is_multisig_update_enacted_in_admin_state` is **false** right after reveal is queued and **true** after mining past `confirmation_depth` (`FAST_ENACTMENT`). Guards coordination-layer enacted timing. | `bitcoind` in `PATH` |

## Shared fixtures (`src/fixtures`)

Deterministic JSON + mnemonic profiles live in the library crate so integration tests stay thin. [`fixtures::signer_update_enacted`](./src/fixtures/signer_update_enacted.rs) defines `DEFAULT_REPO_ASM` and `FAST_ENACTMENT`; the derivation prefix must stay aligned with `desktop_app::infrastructure::signing::list_mnemonic_addresses` (`m/84'/0'/73'/0/{n}`).

## Requirements

- **Rust nightly** pinned via the workspace `rust-toolchain.toml`.
- Internet access on first build (git dependencies from `alpenlabs/asm` and `alpenlabs/strata-common`).
- **`bitcoind` in `PATH`** for any test that boots regtest (`e2e_harness_hello_world`, `e2e_admin_commit_reveal`, `e2e_signer_update_enacted_light`). GitHub Actions installs **Bitcoin Core 29.0** from [bitcoincore.org](https://bitcoincore.org/en/download/) (Ubuntu 24.04 runners do not provide an apt `bitcoind` package). Locally, install `bitcoind` however you prefer (distro package when available, or the same tarball) and ensure it is on `PATH`.

## Run

```sh
cargo test -p alpen-multisig-e2e-tests
```

Run a single test:

```sh
cargo test -p alpen-multisig-e2e-tests --test e2e_admin_subprotocol
```

Enacted signer-update tests (default + fast confirmation fixture):

```sh
cargo test -p alpen-multisig-e2e-tests --test e2e_signer_update_enacted_light -- --nocapture
```
