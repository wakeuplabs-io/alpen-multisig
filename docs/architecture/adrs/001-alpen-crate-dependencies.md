# ADR-001: Alpen Crate Dependency Strategy

**Status:** Accepted
**Date:** 2026-04-08
**Context:** Alpen Multisig depends on several Rust crates from the Alpen ecosystem that are not published to crates.io.

## Decision

### Consumption method

All Alpen/Strata crates are consumed as **git dependencies** since none are published to crates.io. There is no alternative distribution channel.

### Pinning strategy

| Source repo | Pin method | Rationale |
|-------------|-----------|-----------|
| `alpenlabs/alpen` | `rev` | Current dependency (`308211f`) sits past the latest tag (`v0.2.0-rc9`). We need features only available after that tag. Switch to `tag` pinning once a suitable release exists. |
| `alpenlabs/strata-common` | `tag` | Tags follow `v0.1.0-alpha-rcN` convention. Pin to the latest compatible release candidate. |

**Prefer `tag` over `rev` whenever possible.** Tags are human-readable, auditable, and tied to explicit release decisions. Use `rev` only when required features are not yet tagged.

### Workspace-level centralization

Shared dependencies are declared once in the root `Cargo.toml` under `[workspace.dependencies]`. Workspace members reference them with `{ workspace = true }`. This prevents version drift across `orchestator-be`, `desktop-app/src-tauri`, and any future crates.

```toml
# Root Cargo.toml
[workspace.dependencies]
strata-crypto = { git = "https://github.com/alpenlabs/alpen", rev = "308211f" }

# Member Cargo.toml
[dependencies]
strata-crypto = { workspace = true }
```

### Nightly toolchain for the entire workspace

Alpen crates have transitive dependencies (notably `ssz`) that use `#![feature]` and require nightly Rust. Since the desktop app (Tauri) and backend both need to consume Alpen crates directly (for signing, sighash computation, and signature verification), **the entire workspace uses nightly**.

A root `rust-toolchain.toml` pins the nightly version for all workspace members. The `e2e-tests` crate (not a workspace member) maintains its own `rust-toolchain.toml` that should be kept in sync.

**This is a temporary decision.** When Alpen publishes crates that compile on stable (either via crates.io or by removing nightly-only transitive deps), the workspace should migrate back to stable. Track upstream progress on the `ssz` crate (`alpenlabs/ssz-gen`).

### e2e-tests remains a separate crate

The `e2e-tests` crate is not a workspace member because it depends on additional test-utils crates (`strata-asm-txs-test-utils`, `strata-test-utils`) that are only needed for integration testing. It maintains its own `Cargo.toml` with explicit dependency pins that **must be kept in sync** with the workspace-level pins manually. When updating Alpen dependency versions, update both locations.

### Third-party version alignment

Third-party crates used alongside Alpen crates (`bitcoin`, `borsh`, `secp256k1`) must match the versions used in the Alpen workspace to avoid duplicate types at compile time. These are also centralized in `[workspace.dependencies]`.

## Crate inventory

### From `alpenlabs/alpen` (rev: `308211f`)

| Crate | Purpose | Used by |
|-------|---------|---------|
| `strata-asm-txs-admin` | Admin transaction types and construction | e2e-tests, desktop-app |
| `strata-crypto` | Signature verification, key types | e2e-tests, desktop-app |
| `strata-asm-params` | Role enum, AdministrationInitConfig | e2e-tests, desktop-app |
| `strata-primitives` | Shared primitive types | e2e-tests |
| `strata-asm-common` | Subprotocol trait, MsgRelayer | e2e-tests |
| `strata-asm-txs-test-utils` | Test helpers for admin tx construction | e2e-tests (test only) |
| `strata-test-utils` | General test utilities | e2e-tests (test only) |

### From `alpenlabs/strata-common` (tag: `v0.1.0-alpha-rc11`)

| Crate | Purpose | Used by |
|-------|---------|---------|
| `strata-l1-txfmt` | SPS-50/51 transaction formatting (OP_RETURN, witness envelope) | e2e-tests, signing-lib (planned) |

## Risks

1. **Untagged rev pin** — `308211f` does not correspond to any release. Harder to audit and communicate. Mitigated by documenting the rev in this ADR and switching to tags when available.
2. **Version drift** — If `e2e-tests` and workspace members diverge on Alpen crate versions, compile errors or subtle behavior differences may occur. Mitigated by manual sync discipline and documenting both locations.
3. **Build time** — Git deps clone the full repo on clean builds. Unavoidable without crates.io publication. CI caching helps.
4. **Nightly requirement** — The entire workspace uses nightly due to transitive deps (`ssz`). This couples us to nightly stability and may introduce unexpected breakage on toolchain updates. Mitigated by pinning a specific nightly version.
5. **Upstream breaking changes** — Alpen crates are pre-1.0 (`v0.2.0-rc`, `v0.1.0-alpha`). API breakage is expected. Pin updates should be deliberate and tested.

## Update procedure

1. Check latest tags on both repos
2. Update `rev` or `tag` in root `Cargo.toml` `[workspace.dependencies]`
3. Update the same pins in `e2e-tests/Cargo.toml`
4. Run `cargo build` (workspace) and `cd e2e-tests && cargo test`
5. Update this ADR with the new pin values
