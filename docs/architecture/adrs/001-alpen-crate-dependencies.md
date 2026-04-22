# ADR-001: Alpen Crate Dependency Strategy

**Status:** Accepted (superseded in parts on 2026-04-17 — see "Revision history")
**Date:** 2026-04-08, updated 2026-04-17
**Context:** Alpen Multisig depends on several Rust crates from the Alpen ecosystem that are not published to crates.io.

## Decision

### Consumption method

All Alpen/Strata crates are consumed as **git dependencies** since none are published to crates.io. There is no alternative distribution channel.

### Pinning strategy

| Source repo | Pin method | Rationale |
|-------------|-----------|-----------|
| `alpenlabs/asm` | `rev` | The dedicated ASM repo was spun off on 2026-03-17. Two tag schemes coexist (`v0.1-alpha.N` and `v0.1.0-rcN`), none stable. Current rev `a8559d3` equals tag `v0.1-alpha.5`. Switch to `tag` pinning once upstream converges on a single release cadence. |
| `alpenlabs/strata-common` | `tag` | Tags follow `v0.1.0-alpha-rcN` convention. Pin to the latest compatible release candidate. Currently `v0.1.0-alpha-rc16`. |

**Prefer `tag` over `rev` whenever possible.** Tags are human-readable, auditable, and tied to explicit release decisions. Use `rev` only when required features are not yet tagged.

### Wire format: SSZ for admin transactions

Upstream PR `alpenlabs/asm#8` (2026-03-25) replaced Borsh with SSZ as the wire format for `MultisigAction` and every admin tx type. `alpenlabs/alpen/main` itself now consumes the SSZ-based `alpenlabs/asm` crates, so no Borsh-compatible version of ASM crates exists upstream.

Our codec (`infrastructure/action_codec.rs`) uses `ssz::{Encode, Decode}`. **On-chain signatures are unaffected**: `sighash_payload()` is hand-coded in upstream (SPS-65) and is byte-identical across the Borsh→SSZ rewrite. Signatures produced against either version verify identically against the threshold config.

The workspace declares `ssz = { git = "https://github.com/alpenlabs/ssz-gen", tag = "v0.15.0" }` as a direct dep in `[workspace.dependencies]`.

### Workspace-level centralization

Shared dependencies are declared once in the root `Cargo.toml` under `[workspace.dependencies]`. Workspace members reference them with `{ workspace = true }`. This prevents version drift across `orchestrator-be`, `desktop-app/src-tauri`, and any future crates.

```toml
# Root Cargo.toml
[workspace.dependencies]
strata-crypto = { git = "https://github.com/alpenlabs/strata-common", tag = "v0.1.0-alpha-rc16" }

# Member Cargo.toml
[dependencies]
strata-crypto = { workspace = true }
```

### Nightly toolchain for the entire workspace

Alpen crates have transitive dependencies (notably `ssz`) that use `#![feature]` and require nightly Rust. Since the desktop app (Tauri) and backend both need to consume Alpen crates directly (for signing, sighash computation, and signature verification), **the entire workspace uses nightly**.

A root `rust-toolchain.toml` pins the nightly version for all workspace members, including `e2e-tests`.

**This is a temporary decision.** When Alpen publishes crates that compile on stable (either via crates.io or by removing nightly-only transitive deps), the workspace should migrate back to stable. Track upstream progress on the `ssz` crate (`alpenlabs/ssz-gen`).

### e2e-tests is a workspace member

The `e2e-tests` crate is a workspace member. It depends on additional crates (`strata-asm-common`, `strata-asm-txs-test-utils`) that are declared in `[workspace.dependencies]` alongside the production crates. This eliminates duplicate compilation and keeps all Alpen dependency pins in a single location.

### Third-party version alignment

Third-party crates used alongside Alpen crates (`bitcoin`, `secp256k1`, `ssz`) must match the versions used in the Alpen workspace to avoid duplicate types at compile time. These are centralized in `[workspace.dependencies]`.

## Crate inventory

### From `alpenlabs/asm` (rev: `a8559d3`, == tag `v0.1-alpha.5`)

| Crate | Purpose | Used by |
|-------|---------|---------|
| `strata-asm-txs-admin` | Admin transaction types (SSZ-encoded) and construction | e2e-tests, desktop-app |
| `strata-asm-params` | `Role` enum, administration init config | e2e-tests, desktop-app |
| `strata-asm-common` | Subprotocol trait, `TxInputRef`, shared types | e2e-tests |
| `strata-asm-txs-test-utils` | Test helpers for admin tx construction (`TEST_MAGIC_BYTES`, reveal-tx stub) | e2e-tests (test only) |

### From `alpenlabs/strata-common` (tag: `v0.1.0-alpha-rc16`)

| Crate | Purpose | Used by |
|-------|---------|---------|
| `strata-crypto` | Signature verification, `CompressedPublicKey`, `ThresholdConfig` | e2e-tests, desktop-app |
| `strata-l1-txfmt` | SPS-50 transaction formatting (magic bytes, tag parsing) | e2e-tests |

### Third-party with explicit pin

| Crate | Source | Purpose |
|-------|--------|---------|
| `ssz` | `alpenlabs/ssz-gen` tag `v0.15.0` | SSZ encode/decode for `MultisigAction` and related upstream types. |

## Risks

1. **Untagged rev pin on `alpenlabs/asm`** — `a8559d3` corresponds to tag `v0.1-alpha.5` but upstream hasn't converged on a single tag scheme yet (`v0.1-alpha.N` and `v0.1.0-rcN` coexist). We pin by rev to be unambiguous until upstream settles.
2. **Version drift** — Mitigated by centralizing all Alpen crate pins in the root `[workspace.dependencies]`. All members (including `e2e-tests`) use `workspace = true`.
3. **Build time** — Git deps clone the full repo on clean builds. Unavoidable without crates.io publication. CI caching helps.
4. **Nightly requirement** — The entire workspace uses nightly due to transitive deps (`ssz`). This couples us to nightly stability and may introduce unexpected breakage on toolchain updates. Mitigated by pinning a specific nightly version.
5. **Upstream breaking changes** — Alpen crates are pre-1.0 (`v0.1.0-alpha`, `v0.1-alpha`). API breakage is expected (the Borsh→SSZ migration is a recent example). Pin updates should be deliberate and tested. Mitigated inside `desktop-app` by concentrating all Strata-facing code in `infrastructure/action_codec.rs`: pin bumps that rename or reshape `MultisigAction` / `Role` / `ThresholdConfigUpdate` only break the codec, not the application, UI, or tests. `test_encode_matches_direct_strata_ssz` guards byte-level compatibility between our codec and what upstream produces.

## Update procedure

1. Check latest tags/revs on both repos (`gh api repos/alpenlabs/asm/tags`, `gh api repos/alpenlabs/strata-common/tags`).
2. Verify alignment: `alpenlabs/asm` and `alpenlabs/strata-common` must resolve to compatible versions (upstream typically bumps both together — see `alpenlabs/alpen/main` Cargo.toml as reference).
3. Update `rev` / `tag` in root `Cargo.toml` `[workspace.dependencies]`.
4. Run `cargo build` and `cargo test` (covers all workspace members including e2e-tests). Confirm `test_encode_matches_direct_strata_ssz` still passes — any divergence signals an incompatible wire format.
5. Update this ADR with the new pin values.

## Revision history

- **2026-04-17** — ASM crates migrated from `alpenlabs/alpen` (rev `308211f`) to `alpenlabs/asm` (rev `a8559d3`) following the upstream repo split. `strata-crypto` moved from `alpenlabs/alpen` to `alpenlabs/strata-common` (tag `v0.1.0-alpha-rc16`). Internal wire format for `MultisigAction` switched from Borsh to SSZ, tracking upstream PR `alpenlabs/asm#8`. Dropped unused `strata-primitives` and `strata-test-utils`. See `docs/2-discovery/11-asm-repo-migration.md` for the full migration notes.
- **2026-04-08** — Initial version. All Alpen crates pinned to `alpenlabs/alpen` rev `308211f` and `alpenlabs/strata-common` tag `v0.1.0-alpha-rc11`.
