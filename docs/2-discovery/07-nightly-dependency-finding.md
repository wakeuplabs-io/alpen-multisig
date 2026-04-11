# Finding — Nightly Rust Requirement via Alpen SSZ Dependency

## Overview

This document captures findings from investigating the nightly Rust toolchain requirement imposed by Alpen/Strata crate dependencies. The entire workspace is forced to use nightly due to a single transitive dependency (`ssz`), which has significant implications for build stability, CI, and project architecture.

### Sources

- **Alpen crate dependency strategy** — [ADR-001](../architecture/adrs/001-alpen-crate-dependencies.md)
- **SSZ crate source** — `alpenlabs/ssz-gen` (fork of Lighthouse SSZ libraries)
- **Rust tracking issue** — [rust-lang/rust#76560](https://github.com/rust-lang/rust/issues/76560) (`generic_const_exprs`)

---

## 1. The Dependency Chain

The nightly requirement originates from a single transitive dependency:

```
desktop-app/src-tauri
  → strata-asm-params
    → strata-btc-types
      → ssz v0.14.0            ← #![feature(generic_const_exprs)]
      → strata-identifiers
        → ssz, ssz_types, tree_hash  ← #![feature(generic_const_exprs)]
```

No code in this project uses nightly features directly. The constraint is entirely inherited.

---

## 2. Why Alpen Uses SSZ

SSZ (Simple Serialize) is the serialization format from Ethereum 2.0's Beacon Chain. Alpen uses their own fork (`alpenlabs/ssz-gen`) because:

- **Deterministic serialization** — identical output for identical input, required for consensus.
- **Merkle tree hashing** — the companion `tree_hash` crate provides content-addressable data structures for rollup state proofs and checkpoint verification.
- **Ethereum compatibility** — Strata is an EVM-compatible rollup, so sharing the consensus-layer encoding is a natural fit.

Alternatives like Borsh or Bincode lack the integrated Merkle tree hashing that SSZ provides.

---

## 3. The Specific Nightly Feature

Both `ssz` and `tree_hash` declare a single nightly feature:

```rust
#![feature(generic_const_exprs)]
```

This feature allows computing values in const generic positions:

```rust
// Example: a bitvector of N bits needs N/8 bytes
impl<const N: usize> BitVectorRef<N>
where
    [u8; N / 8]: Sized,  // ← requires generic_const_exprs
```

Without this feature, you cannot express "an array whose size is computed from a const generic parameter" in Rust. There is no stable workaround that preserves the same compile-time guarantees.

---

## 4. Upstream Status

| Aspect | Status |
|--------|--------|
| Rust feature `generic_const_exprs` | Marked "far from ready" — no stabilization timeline, likely years away |
| `alpenlabs/ssz-gen` v0.14 → v0.15 | Only CI changes — no movement toward stable Rust |
| Structural dependency | `generic_const_exprs` is fundamental to SSZ's `BitVectorRef` design — removing it requires a significant rewrite |

**Conclusion:** There is no realistic path to eliminating the nightly requirement through upstream changes in the near term.

---

## 5. Impact on Workspace Members

| Component | Needs strata crates with SSZ? | Forced to nightly? |
|-----------|-------------------------------|---------------------|
| `desktop-app/src-tauri` | Yes (`strata-asm-params`, `strata-crypto`) | Yes — direct transitive dependency |
| `orchestator-be` | **No** (zero strata crate dependencies currently) | Yes — shares `rust-toolchain.toml` with workspace |
| `e2e-tests` | Yes (all strata crates) | Yes — separate `rust-toolchain.toml` |

Notable: **the backend does not depend on any strata crate** and could compile on stable Rust if it were in a separate workspace.

---

## 6. Risks

1. **Nightly compiler breakage** — Nightly features can change or break between releases. Mitigated by pinning a specific nightly date in `rust-toolchain.toml`, but pin updates require full validation.
2. **Ecosystem friction** — Some third-party crates may have bugs or incompatibilities specific to nightly Rust, limiting dependency choices.
3. **CI complexity** — CI must use the exact pinned nightly toolchain. Standard stable Rust Docker images and caching strategies do not apply.
4. **Blast radius** — The backend is forced to nightly despite not needing it, increasing its exposure to nightly-specific issues.
5. **No mitigation path** — Cannot vendor SSZ without the feature flag, cannot substitute a different serializer (strata crates hardcode SSZ), and the Rust feature will not stabilize soon.

---

## 7. Mitigation Strategies

### Current (implemented)

- **Pin nightly version** — `rust-toolchain.toml` pins a specific nightly date (`nightly-2026-01-01`), preventing surprise breakage from compiler updates.
- **Keep `e2e-tests` separate** — Already outside the workspace, maintains its own toolchain pin.

### Potential (not yet implemented)

- **Separate backend workspace** — Since `orchestator-be` has no strata crate dependencies, it could be extracted into its own workspace with a stable toolchain. This would reduce the blast radius of nightly issues to only the desktop app.
- **Monitor upstream** — Track `alpenlabs/ssz-gen` and `rust-lang/rust#76560` for changes. If `generic_const_exprs` stabilizes or Alpen rewrites SSZ to avoid it, the workspace can migrate to stable.
