# Spec: Migrate ASM dependencies from `alpenlabs/alpen` to `alpenlabs/asm`

> Status: Implemented on `develop` (2026-04-22).

> **Issue:** [#36](https://github.com/wakeuplabs-io/alpen-multisig/issues/36)
> **Branch:** `feature/migrate-asm-deps-to-alpenlabs-asm`
> **Scope:** one PR, replace one crate source with another, drop anything unused, adjust the minimum code needed because upstream replaced Borsh with SSZ for the admin wire format.

## Objective

The workspace has zero references to `alpenlabs/alpen` and zero references to the `308211f` rev after this PR. Our code consumes the same Strata/ASM types it consumes today, pointed at the new dedicated repo (`alpenlabs/asm`). The admin transaction signing and verification flow keeps producing byte-identical sighashes and byte-identical on-chain payloads (delegated to upstream `create_test_admin_tx` / `parse_tx`, which already do SSZ).

## Context

Upstream state, verified against GitHub on 2026-04-17:

- `alpenlabs/alpen/main` (commit `afb3683`, today) already consumes `strata-asm-txs-admin` and the other `strata-asm-*` crates from `https://github.com/alpenlabs/asm` tag `v0.1-alpha.5`. The old path-local copy at `crates/asm/txs/admin/` was deleted. We are 10 days behind.
- The initial copy into `alpenlabs/asm` still used Borsh. PR #8 (`feat!(ssz): finish ASM serialization migration`, commit `fd57abb`, 2026-03-25) replaced Borsh with SSZ for every admin tx type. **No tag in `alpenlabs/asm` is Borsh-based.**
- `sighash_payload()` is hand-coded in upstream (see `cancel.rs` and `updates/multisig.rs`) and is byte-identical across the Borsh→SSZ rewrite. Signatures produced against the old crate remain valid against the new crate. The wire format change is limited to the enclosing envelope payload (`SignedPayload`), which upstream itself produces via `create_test_admin_tx` — not something we encode locally.

Our current Borsh usage is entirely **internal** (between `action_codec.rs`, `signing.rs`, and the POC-4 compatibility test). No byte output of ours goes to L1 — the reveal transaction is built upstream. Therefore the migration is a repoint + a small internal wire-format swap.

## Scope

### In

- Repoint `strata-asm-txs-admin`, `strata-asm-params`, `strata-asm-common`, `strata-asm-txs-test-utils` from `alpenlabs/alpen` rev `308211f` → `alpenlabs/asm` rev `a8559d3` (== tag `v0.1-alpha.5`).
- Move `strata-crypto` from `alpenlabs/alpen` rev `308211f` → `alpenlabs/strata-common` tag `v0.1.0-alpha-rc16`.
- Bump `strata-l1-txfmt` from `alpenlabs/strata-common` tag `v0.1.0-alpha-rc11` → tag `v0.1.0-alpha-rc16` so every `strata-common` consumer resolves to one version.
- Add the `ssz` workspace dep (`alpenlabs/ssz-gen` tag `v0.15.0`) since `MultisigAction` now requires it to be (de)serialized.
- Rewrite the six Borsh call sites in our code to use SSZ (`action_codec.rs`, `signing.rs`, their tests).
- Rename the POC-4 "Borsh roundtrip" test to a "SSZ roundtrip" test. It keeps the same role: verify our local wire layer is byte-compatible with what the upstream crate produces directly.
- Drop `strata-primitives` and `strata-test-utils` from the workspace (zero imports).
- Drop unused `strata-asm-common`, `strata-asm-txs-test-utils`, `strata-l1-txfmt` from `desktop-app/src-tauri/Cargo.toml` (declared but never imported — verified by grep).
- Drop the `borsh` workspace dep if unused after the migration. Verify with `cargo tree` before removing; keep if any transitive or feature-gated consumer still needs it.
- Update ADR-001, research.md, discovery doc `08-alpen-crate-prd-coverage.md`, and add a new discovery doc `11-asm-repo-migration.md` covering the SSZ transition.

### Out

- No changes to sighash computation, tagged-hash format (SPS-65), or signature scheme. On-chain semantics are untouched.
- No integration of new upstream crates (`strata-asm-proto-administration`, `strata-l1-envelope-fmt`, etc.). Those are separate follow-ups.
- No closing of PRD gaps (missing roles, missing update types) — upstream status is unchanged on that front.
- No new Tauri commands, no new UI, no API changes.

## Technical design

### Cargo.toml (root) — target state

```toml
[workspace.dependencies]
# From alpenlabs/asm (rev-pinned — main HEAD == v0.1-alpha.5; no stable tag adopted yet)
strata-asm-txs-admin = { git = "https://github.com/alpenlabs/asm", rev = "a8559d3" }
strata-asm-params = { git = "https://github.com/alpenlabs/asm", rev = "a8559d3" }
strata-asm-common = { git = "https://github.com/alpenlabs/asm", rev = "a8559d3" }
strata-asm-txs-test-utils = { git = "https://github.com/alpenlabs/asm", rev = "a8559d3" }

# From alpenlabs/strata-common (tag-pinned)
strata-crypto = { git = "https://github.com/alpenlabs/strata-common", tag = "v0.1.0-alpha-rc16" }
strata-l1-txfmt = { git = "https://github.com/alpenlabs/strata-common", tag = "v0.1.0-alpha-rc16" }

# SSZ serialization (required by strata-asm-* after upstream PR #8)
ssz = { git = "https://github.com/alpenlabs/ssz-gen", tag = "v0.15.0" }

# Third-party — unchanged
bitcoin = { version = "0.32.6", features = ["serde"] }
secp256k1 = { version = "0.29.1", features = ["global-context", "std"] }
rand = "0.8"
hex = "0.4"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
# `borsh` is removed if `cargo tree` confirms no remaining consumer after the code migration.
```

### Code changes

Six touch points, all internal. No public API change, no Tauri command signature change.

#### `desktop-app/src-tauri/src/infrastructure/action_codec.rs`

| Current | Target |
|---|---|
| `use borsh` not needed explicitly, but used via `borsh::to_vec` / `borsh::from_slice` | `use ssz::{Decode, Encode};` |
| `borsh::to_vec(&strata)` (line 38) | `Ok(strata.as_ssz_bytes())` |
| `borsh::from_slice(bytes)` (line 44) | `StrataMultisigAction::from_ssz_bytes(bytes).map_err(`…`)` — note `ssz::DecodeError` is not `Display` so we format it manually |
| POC-4 test `test_encode_matches_direct_strata_borsh` with `borsh::to_vec(&strata_action)` | Renamed to `test_encode_matches_direct_strata_ssz` with `strata_action.as_ssz_bytes()` |
| Doc comment in module header: "Borsh-encoded `MultisigAction`" | "SSZ-encoded `MultisigAction`" |
| Error enum variants `CodecError::Encode` / `Decode` | Unchanged — variant names are neutral to the format |

#### `desktop-app/src-tauri/src/infrastructure/signing.rs`

| Current | Target |
|---|---|
| `use borsh::BorshDeserialize;` (line 4) | `use ssz::Decode;` |
| `MultisigAction::try_from_slice(&action_bytes)` (line 43) | `MultisigAction::from_ssz_bytes(&action_bytes)` |
| Doc comment "MultisigAction uses Borsh, not serde" (line 40) | "MultisigAction uses SSZ, not serde" |
| Test helper `borsh::to_vec(&build_demo_action())` (line 171) | `build_demo_action().as_ssz_bytes()` |

All 11 existing tests in `signing.rs` (`test_compute_sighash_*`, `test_sign_sighash_*`, `test_verify_threshold_*`) keep the same assertions and keep passing — they exercise the sighash/signing/verification semantics, which the upstream migration did not change.

#### `e2e-tests/tests/e2e_admin_subprotocol.rs`

No code change required. The test calls upstream `create_test_admin_tx` and `parser::parse_tx` — SSZ is hidden behind the upstream API. Only a small comment fix: the doc comment at line 12 currently says "Borsh-serialize and build the Bitcoin transaction", update to "SSZ-serialize …".

### Member manifests

**`e2e-tests/Cargo.toml`:**
- `strata-asm-*` grouped under a "From alpenlabs/asm" comment.
- `strata-crypto` and `strata-l1-txfmt` under "From alpenlabs/strata-common".
- Drop `strata-primitives`, `strata-test-utils`.
- Drop `borsh` if unused (verify with `cargo tree`).

**`desktop-app/src-tauri/Cargo.toml`:**
- Keep `strata-asm-txs-admin`, `strata-asm-params`, `strata-crypto`.
- Drop `strata-asm-common`, `strata-asm-txs-test-utils`, `strata-l1-txfmt` (declared but no import).
- Add `ssz = { workspace = true }`.
- Drop `borsh` if unused after the code migration.

**`orchestrator-be/Cargo.toml`:** no changes.

### Production code vs. test helpers

- **Production code touched:** `action_codec.rs::encode_action`, `action_codec.rs::decode_action`, `signing.rs::compute_sighash`. Same function names, same signatures, same error shapes. Only the serialization call inside them changes.
- **Test helpers touched:** `signing.rs::demo_action_hex` (produces hex input for sighash tests) and the POC-4 compatibility test in `action_codec.rs`. Both remain test-only, gated under `#[cfg(test)]`, and keep doing the same job — just swapped to SSZ.
- No test helper becomes a production path. No new Tauri command is added.

## Test cases

All existing tests continue to be the gate. No new test scenarios are added beyond the POC-4 rename, because the feature set did not change.

| Test | File | Expectation |
|---|---|---|
| `test_encode_matches_direct_strata_ssz` (was `…_borsh`) | `action_codec.rs` | Our `encode_action` output equals `strata_action.as_ssz_bytes()` byte-for-byte |
| `test_roundtrip_hex`, `test_roundtrip_bytes` | `action_codec.rs` | Domain → SSZ → domain is lossless |
| `test_compute_sighash_returns_valid_32_byte_hash`, `_deterministic`, `_different_seqno` | `signing.rs` | Sighash shape and determinism unchanged |
| `test_sign_sighash_success`, `_invalid_secret_key`, `_invalid_sighash_length` | `signing.rs` | Signing behaviour unchanged |
| `test_verify_threshold_full_flow_2_of_3`, `_below_threshold`, `_empty_signatures`, `_invalid_signature` | `signing.rs` | Threshold verification unchanged |
| `e2e_build_and_verify_admin_signer_update` | `e2e-tests/tests/e2e_admin_subprotocol.rs` | Full reveal-tx construction + SPS-50 parse + SPS-51 envelope + signature verification, now all through the new `alpenlabs/asm` crate |

Plus the global gates:
- `cargo build --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --check`
- `cd desktop-app && npm run build`

## Module structure

No new modules. Each edited file retains its existing single responsibility:

- `action_codec.rs` — domain ↔ strata-asm type conversion and byte serialization for the internal hand-off (one sentence). Responsibility unchanged, only the byte format swapped.
- `signing.rs` — SPS-65 sighash computation, ECDSA signing, threshold verification (one sentence). Responsibility unchanged.
- `e2e_admin_subprotocol.rs` — end-to-end reveal-tx build+parse+verify (one sentence). Responsibility unchanged.

Dependency direction is preserved: `action_codec.rs` and `signing.rs` depend on `strata_asm_txs_admin` (and now `ssz`), never the reverse.

## Docs to update

| File | Change |
|---|---|
| `docs/architecture/adrs/001-alpen-crate-dependencies.md` | Rewrite "Crate inventory" and "Pinning strategy" sections to reflect the new sources and the SSZ format. Add a note on the `alpenlabs/asm` tag schemes (`v0.1.0-rc*` and `v0.1-alpha.*` coexist, pre-stable). |
| `docs/2-discovery/crate-inventory.md` | Update the "in use" table with new sources + SSZ note. |
| `docs/external/research-assessment.md` §1 | Add re-validation note after migration (see crate-inventory for pins). |
| `docs/2-discovery/08-alpen-crate-prd-coverage.md` | Update source paths. Add the same re-validation note. |
| `docs/2-discovery/11-asm-repo-migration.md` | **New file.** Covers: what changed in `Cargo.toml`, timeline of the upstream split and SSZ migration, verification that signatures remain on-chain-compatible, confirmation that PRD gaps remain open upstream, bump procedure for future rev/tag updates. |

## Commit layout (single PR)

Each commit compiles and tests pass — except commit A which is pure additive (ssz dep added, unused), and commit B which is the atomic repoint+migration (must land together because the Borsh APIs of the old crates and the SSZ APIs of the new crates are mutually exclusive).

- **A** `chore(deps): add ssz workspace dependency for upcoming ASM SSZ migration`
  Adds `ssz = { git = "https://github.com/alpenlabs/ssz-gen", tag = "v0.15.0" }` to `[workspace.dependencies]`. No other change. Build still green.

- **B** `feat(deps)!: migrate ASM crates to alpenlabs/asm and switch internal codec to SSZ`
  Atomic: root `Cargo.toml` + `e2e-tests/Cargo.toml` + `desktop-app/src-tauri/Cargo.toml` pins, plus the six Borsh→SSZ call sites in `action_codec.rs` and `signing.rs`, plus the comment fix in `e2e_admin_subprotocol.rs`. Includes the POC-4 test rename.

- **C** `chore(deps): drop unused strata-primitives and strata-test-utils`
  Removes the two workspace deps and their member-level declarations in `e2e-tests/Cargo.toml`.

- **D** `chore(deps): clean up unused strata deps in desktop-app manifest`
  Removes `strata-asm-common`, `strata-asm-txs-test-utils`, `strata-l1-txfmt` from `desktop-app/src-tauri/Cargo.toml`. Optionally drops `borsh` here and in the root workspace if `cargo tree` confirms no consumer.

- **E** `docs: record ASM repo migration, SSZ transition, and reassess PRD coverage`
  Updates ADR-001, research.md §1.1 / §1.3, `08-alpen-crate-prd-coverage.md`, creates `11-asm-repo-migration.md`.

The `!` in commit B signals a breaking change at the dep graph layer (pins differ, wire format differs) even though user-visible behaviour is unchanged.

## Risks

| Risk | Mitigation |
|---|---|
| `ssz` crate (from `alpenlabs/ssz-gen`) brings transitive deps we can't resolve | It's already a transitive dep via `strata-asm-*` today when pinned to the new repo; adding it explicitly does not change the resolution, just makes it direct and usable. |
| Our `as_ssz_bytes()` output differs from upstream's, breaking the POC-4 compatibility test | Low: both sides call the same SSZ derive on the same `MultisigAction` enum. If it differs, it's a real bug upstream, and the test is exactly there to catch it. |
| `ssz::DecodeError` is not `Display` | Format it manually in the `map_err` closure, same idiom the upstream admin parser uses. |
| `borsh` still needed by a transitive dep we don't see | Run `cargo tree -i borsh` before removing the dep; keep it if there's any consumer. |
| Sighash changes on-chain and old signatures stop verifying | Confirmed not the case: `sighash_payload()` byte-identical pre/post migration. The test `test_compute_sighash_deterministic` stays green against stored fixtures if we add any. |

## Definition of Done

- [ ] Zero references to `alpenlabs/alpen` or rev `308211f` in any `*.toml`.
- [ ] `cargo build --workspace` green.
- [ ] `cargo test --workspace` green, including the 12 tests listed in "Test cases".
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo fmt --check` clean.
- [ ] `cd desktop-app && npm run build` succeeds.
- [ ] ADR-001, research.md §1.1 and §1.3, `08-alpen-crate-prd-coverage.md` updated.
- [ ] `docs/2-discovery/11-asm-repo-migration.md` created.
- [ ] PR opened against `develop` with @juandahl as reviewer.
