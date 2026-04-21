# POC 3 Findings — Signing Library with Alpen Crate Integration

> **Post-discovery note (2026-04-17).** When this POC ran, the ASM crates used Borsh and were pinned to `alpenlabs/alpen` rev `308211f`. Upstream has since migrated to SSZ (`alpenlabs/asm` rev `a8559d3`, `alpenlabs/strata-common` tag `v0.1.0-alpha-rc16`). The `compute_sighash` byte layout is byte-identical across both — all signing claims in this document still hold. See [`11-asm-repo-migration.md`](./11-asm-repo-migration.md) and current pins in root `Cargo.toml`.
>
> The signing library has also moved to `desktop-app/src-tauri/src/infrastructure/signing.rs` (layered architecture, ADR-005).

## Overview

This document captures findings from POC 3: validating that the desktop app can compute SPS-65 sighashes, sign them with ECDSA keys, and verify threshold signatures — all using Alpen/Strata crates as the canonical implementation.

### Sources

- **Signing library** — [`desktop-app/src-tauri/src/infrastructure/signing.rs`](../../desktop-app/src-tauri/src/infrastructure/signing.rs) (288 lines, 10 tests)
- **Alpen crate dependency strategy** — [ADR-001](../architecture/adrs/001-alpen-crate-dependencies.md)
- **E2E admin subprotocol test** — [`e2e-tests/tests/e2e_admin_subprotocol.rs`](../../e2e-tests/tests/e2e_admin_subprotocol.rs) (POC-1, validates full tx construction + verification)

---

## 1. What Was Validated

POC-3 proved that the desktop app can perform all signing operations required for the multisig flow using Alpen crates directly — no reimplementation of protocol logic.

### Three core operations

| Operation | Function | What it does |
|-----------|----------|-------------|
| **Sighash computation** | `compute_sighash(seqno, action_hex)` | Deserializes a Borsh-encoded `MultisigAction`, calls `action.compute_sighash(seqno)` from `strata-asm-txs-admin` |
| **Signing** | `sign_sighash(secret_key_hex, sighash_hex)` | ECDSA signature over the 32-byte sighash using `secp256k1` |
| **Threshold verification** | `verify_threshold(sighash_hex, signatures, pubkeys, threshold)` | Verifies each signature against its public key and checks `valid_count >= threshold` using `ThresholdConfig` from `strata-crypto` |

### Key design decisions

- **Borsh serialization for actions** — `MultisigAction` uses Borsh (not JSON/serde). The signing library accepts hex-encoded Borsh bytes and deserializes internally. This matches the on-chain format defined in SPS-65.
- **Hex encoding at boundaries** — All inputs/outputs use hex strings. This keeps the library agnostic to transport (IPC, HTTP, CLI) and simplifies testing.
- **No key management** — The library receives keys as parameters. It does not store, derive, or manage keys. This separates signing logic from wallet integration (POC-3 uses software keys; Slice 3 will add HWI).

---

## 2. Alpen Crate Dependencies

POC-3 introduced direct dependencies on Alpen crates in the desktop app. The full dependency strategy is documented in [ADR-001](../architecture/adrs/001-alpen-crate-dependencies.md).

### Crates used by signing.rs

> Pins below are the ones observed at POC-3 time. Current workspace pins (post-migration) are `alpenlabs/asm` rev `a8559d3` and `alpenlabs/strata-common` tag `v0.1.0-alpha-rc16`; see [`11-asm-repo-migration.md`](./11-asm-repo-migration.md) and root `Cargo.toml`.

| Crate | Source (POC-3 time) | Purpose |
|-------|--------|---------|
| `strata-asm-txs-admin` | `alpenlabs/alpen` (rev: `308211f`) | `MultisigAction`, `Sighash`, `compute_sighash()` |
| `strata-crypto` | `alpenlabs/alpen` (rev: `308211f`) | `CompressedPublicKey`, `ThresholdConfig` |
| `secp256k1` | crates.io (version-aligned with Alpen) | ECDSA signing and verification |
| `borsh` | crates.io (version-aligned with Alpen) | `BorshDeserialize` for `MultisigAction` (replaced by `ssz::Decode` post-migration) |

### Implications discovered

- **Nightly Rust required** — Alpen crates have transitive dependencies (`ssz`) that require `#![feature]`. The entire workspace uses nightly, pinned via `rust-toolchain.toml`. See ADR-001 for details.
- **Version alignment is critical** — Third-party crates (`bitcoin`, `secp256k1`, and formerly `borsh`) must match Alpen's versions exactly to avoid duplicate types at compile time. Centralized in `[workspace.dependencies]`.
- **Git dependencies only** — Alpen crates are not on crates.io. Consumed via `rev` pin. (At POC-3 time the pin tag was `v0.2.0-rc9` on the old monorepo; current workspace uses `alpenlabs/asm` pins — see ADR-001.)

---

## 3. Test Coverage

The signing library has 10 unit tests covering:

| Category | Tests | What they verify |
|----------|-------|-----------------|
| **Sighash computation** | Deterministic output, valid Borsh input, invalid hex/Borsh rejection | Same `(action, seqno)` always produces the same sighash |
| **Signing** | Valid signature, invalid key/sighash rejection | ECDSA signatures are correct and verifiable |
| **Threshold verification** | 2-of-3 quorum, below-threshold rejection, invalid signature rejection, non-signer rejection | Quorum logic matches protocol requirements |
| **Round-trip** | Compute → sign → verify end-to-end | All three operations compose correctly |

All tests use software keys (secp256k1 keypairs generated in-test). Hardware wallet signing is deferred to Slice 3.

---

## 4. Relationship to Other POCs

```
POC-1 (e2e-tests/)                    POC-3 (desktop-app/src-tauri/)
├── Full tx construction               ├── Sighash computation
├── ASM state machine validation        ├── ECDSA signing
├── Witness envelope (SPS-51)           └── Threshold verification
└── Proved: protocol crates work
                                        Proved: desktop app can sign
         ↓                                        ↓
         └──────────── POC-4 ──────────────────────┘
                 Coordination flow (this plan)
```

- **POC-1** proved that Alpen crates can construct and verify complete admin transactions (full `SignedPayload` with SPS-50/51 envelope). Tests live in `e2e-tests/` because they need additional test-utils crates.
- **POC-3** proved that the **desktop app** can perform the signing subset of that flow — the operations a human signer needs. The library lives in `src-tauri/` because it will be called from Tauri commands.
- **POC-4** will connect POC-3 signing to the orchestrator for coordination (propose → sign → quorum).

---

## 5. What Comes Next

| Next step | Builds on POC-3 |
|-----------|-----------------|
| **POC-4 Step 1** | Desktop application layer orchestrates `signing.rs` + backend client to create/sign proposals |
| **Slice 2** | SPS-50/51 Bitcoin tx construction wraps the signed payload into a broadcastable transaction |
| **Slice 3** | HWI integration replaces software keys with hardware wallet signing (same `sign_sighash` interface) |
