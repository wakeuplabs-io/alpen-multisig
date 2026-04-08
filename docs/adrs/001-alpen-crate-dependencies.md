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

### e2e-tests remains a separate crate

The `e2e-tests` crate requires `nightly-2026-01-01` (via its own `rust-toolchain.toml`) because some Alpen test-utils crates depend on nightly features. The main workspace uses stable Rust. Including `e2e-tests` as a workspace member would force nightly on all members.

Therefore `e2e-tests` maintains its own `Cargo.toml` with explicit dependency pins that **must be kept in sync** with the workspace-level pins manually. When updating Alpen dependency versions, update both locations.

### Third-party version alignment

Third-party crates used alongside Alpen crates (`bitcoin`, `borsh`, `secp256k1`) must match the versions used in the Alpen workspace to avoid duplicate types at compile time. These are also centralized in `[workspace.dependencies]`.

## Crate inventory

### From `alpenlabs/alpen` (rev: `308211f`)

| Crate | Purpose | Used by |
|-------|---------|---------|
| `strata-asm-txs-admin` | Admin transaction types and construction | e2e-tests, signing-lib (planned) |
| `strata-crypto` | Signature verification, key types | e2e-tests, signing-lib (planned) |
| `strata-asm-params` | Role enum, AdministrationInitConfig | e2e-tests |
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
4. **Nightly requirement** — Some Alpen test-utils crates require nightly. Production code (backend, desktop app) must not depend on nightly-only crates.
5. **Upstream breaking changes** — Alpen crates are pre-1.0 (`v0.2.0-rc`, `v0.1.0-alpha`). API breakage is expected. Pin updates should be deliberate and tested.

## Update procedure

1. Check latest tags on both repos
2. Update `rev` or `tag` in root `Cargo.toml` `[workspace.dependencies]`
3. Update the same pins in `e2e-tests/Cargo.toml`
4. Run `cargo build` (workspace) and `cd e2e-tests && cargo test`
5. Update this ADR with the new pin values
