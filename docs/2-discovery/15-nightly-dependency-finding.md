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
| `orchestrator-be` | **No** (zero strata crate dependencies currently) | Yes — shares `rust-toolchain.toml` with workspace |
| `e2e-tests` | Yes (all strata crates) | Yes — separate `rust-toolchain.toml` |

Notable: **the backend does not depend on any strata crate** and could compile on stable Rust if it were in a separate workspace.

---

## 6. Build Time Analysis

A clean workspace build takes **~1m 48s locally** (108s wall clock). Using `cargo build --timings`, the breakdown by category:

| Category | Compile time | % of total | Top crates |
|----------|-------------|------------|------------|
| **Tauri/GTK/UI** | 151.7s | ~20% | gtk (23s), tauri-utils (18s+12s), gio (16s), glib (14s) |
| **Other** (tokio, syn, regex, etc.) | 540.8s | ~72% | tokio (16s), regex-automata (12s), syn (10s+9s), h2 (11s) |
| **Alpen/SSZ** | 31.0s | ~4% | ssz_codegen (4s), strata-crypto (3s), strata-identifiers (2s) |
| **Crypto** (bitcoin, secp256k1) | 30.8s | ~4% | bitcoin (15s) |

Note: compile times are cumulative across cores; wall-clock is shorter due to parallelism.

**Key finding:** Alpen/SSZ crates are only ~4% of build time. The main bottleneck is Tauri/GTK (20%) and generic dependencies (72%).

### Git dependency download times

| Scenario | Time | When it occurs |
|----------|------|----------------|
| Clone from scratch (no cache) | ~21s | First build, or after clearing `~/.cargo/git/` |
| Re-checkout (bare repo cached) | ~15s | After `cargo clean` |
| Incremental (fully cached) | ~0s | Day-to-day builds |

The Alpen git repos are small (alpen 17MB, ssz-gen 1.1MB, strata-common 552KB = 19MB total). Download time is only significant on the first build.

### CI pipeline times

The CI pipeline (GitHub Actions) takes ~451s (7.5 min) on a clean run:

| Step | Time | Note |
|------|------|------|
| Clippy | 169s | Compiles in check mode + analyzes |
| Test | 203s | Recompiles with test profile + executes |
| Tauri system deps | 37s | `apt-get install` GTK/WebKit |
| Cache save | 25s | Saves artifacts for next runs |
| Toolchain | 10s | Downloads nightly |

The main CI cost is double compilation: clippy (check mode) and test (test mode) use different profiles. A shared `cargo build --workspace --all-targets` step before both helps reuse artifacts.

---

## 7. Risks

1. **Nightly compiler breakage** — Nightly features can change or break between releases. Mitigated by pinning a specific nightly date in `rust-toolchain.toml`, but pin updates require full validation.
2. **Ecosystem friction** — Some third-party crates may have bugs or incompatibilities specific to nightly Rust, limiting dependency choices.
3. **CI complexity** — CI must use the exact pinned nightly toolchain. Standard stable Rust Docker images and caching strategies do not apply.
4. **Blast radius** — The backend is forced to nightly despite not needing it, increasing its exposure to nightly-specific issues.
5. **No mitigation path** — Cannot vendor SSZ without the feature flag, cannot substitute a different serializer (strata crates hardcode SSZ), and the Rust feature will not stabilize soon.

---

## 8. Mitigation Strategies

### Current (implemented)

- **Pin nightly version** — `rust-toolchain.toml` pins a specific nightly date (`nightly-2026-01-01`), preventing surprise breakage from compiler updates.
- **e2e-tests in workspace** — Integrated into the Cargo workspace (PR #20), eliminating dual compilation of ~600 crates and centralizing all Alpen dependency pins in one location.
- **CI toolchain fix** — CI uses `dtolnay/rust-toolchain@master` pinned to the same nightly version as `rust-toolchain.toml` (PR #21).
- **CI shared build step** — `cargo build --workspace --all-targets` runs before clippy and test to maximize artifact reuse between compilation profiles (PR #21).
- **CI remains blocking** — `continue-on-error` was attempted but GitHub still waits for all checks to finish before allowing merge regardless. The rust job blocks PRs until complete (~7.5 min). Future optimization: split into faster backend-only job.
- **CI cache-on-failure** — `Swatinem/rust-cache` configured with `cache-on-failure: true` so failed runs still populate the cache for faster retries.

### Potential (not yet implemented)

- **`default-members`** — Add `default-members = ["orchestrator-be"]` to root `Cargo.toml` so `cargo build` only builds the backend by default, skipping Tauri/GTK (~150s savings for backend-only work).
- **`sccache`** — Cache compiled artifacts across clean builds. Saves 50-80% on rebuilds.
- **`mold` linker** — Faster linking via `.cargo/config.toml` configuration.
- **Separate backend workspace** — Since `orchestrator-be` has no strata crate dependencies, it could be extracted into its own workspace with a stable toolchain. This would reduce the blast radius of nightly issues to only the desktop app.
- **Split CI jobs** — Separate backend-only job (no Tauri deps) from full workspace job. Backend changes would get faster feedback.
- **Monitor upstream** — Track `alpenlabs/ssz-gen` and `rust-lang/rust#76560` for changes. If `generic_const_exprs` stabilizes or Alpen rewrites SSZ to avoid it, the workspace can migrate to stable.
