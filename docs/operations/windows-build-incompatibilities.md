# Windows build incompatibilities (D5)

This document records the Windows-specific problems hit while trying to build the
desktop app on a **native Windows runner**, why a native build was abandoned in
favour of **cross-compiling from Linux**, and what must be escalated to Alpen.

It exists so the next person does not re-discover these the hard way, and so the
upstream issues are reported with enough detail to be fixed at the source.

## Summary

The desktop app depends (transitively, through `desktop-app/src-tauri`) on Alpen
crates whose source and build tooling are **not portable to Windows**. A native
`windows-latest` build hits three independent, sequential blockers. Two are
worked around at checkout time; the third is an upstream code bug that cannot be
worked around without patching Alpen source.

| # | Blocker | Layer | Workaround on native Windows | Status |
|---|---------|-------|------------------------------|--------|
| 1 | Reserved `aux` directory in `alpenlabs/asm` | git checkout | git-CLI pre-checkout (libgit2 cannot) | Worked around |
| 2 | CRLF conversion breaks SSZ schema parsing | git checkout | `core.autocrlf=false` + `core.eol=lf` | Worked around |
| 3 | `ssz-gen` codegen splits paths on `/` only | build script | none — upstream bug | **Blocks native Windows** |

Because blocker #3 cannot be fixed from our repository, the Windows artifact is
produced by **cross-compiling on Linux** instead (see
[`executable-delivery-plan.md`](./executable-delivery-plan.md), D5). Cross-compiling
sidesteps all three: the checkout happens on a Linux filesystem (no reserved-name
or CRLF issue) and the codegen runs with `/` path separators.

## Blocker 1 — reserved `aux` directory (`alpenlabs/asm`)

`strata-asm-common` (a direct dependency of `desktop-app/src-tauri`) ships
`crates/common/src/aux/collector.rs`. `aux` is a **reserved DOS device name** on
Windows. cargo checks out git dependencies with **libgit2**, which *unconditionally*
rejects such paths during validation:

```
error: failed to get `strata-asm-common` as a dependency of package `orchestrator-be`
Caused by:
  Unable to update https://github.com/alpenlabs/asm?rev=e0461f8...
Caused by:
  cannot checkout to invalid path 'crates/common/src/aux/collector.rs'; class=Checkout (20)
```

- This is **not** configurable: `core.protectNTFS=false` does *not* disable
  libgit2's DOS-device-name rejection (verified in CI — see PR #240).
- The `aux/` directory exists at the pinned rev *and* at upstream HEAD, so bumping
  the pin does not help.
- The git **CLI** (git-for-windows) *can* write reserved paths via the `\\?\`
  namespace with `core.protectNTFS=false` + `core.longpaths=true`.

**Native workaround (PR #242):** let `cargo fetch` populate the bare git db (it
fails at libgit2's checkout, as expected), then materialise the `asm` worktree
with the git CLI into cargo's `checkouts/asm-<hash>/<short-oid>/` dir and write
cargo's `.cargo-ok` marker so the build reuses our checkout.

## Blocker 2 — CRLF breaks SSZ schema parsing

git-for-windows defaults `core.autocrlf` on. The git-CLI pre-checkout from
blocker 1 therefore converted the `.ssz` schema files' `LF` to `CRLF`, and the
SSZ code generator rejects `\r`:

```
error: failed to run custom build command for `strata-asm-manifest-types`
Caused by:
  Failed to generate SSZ types: Token(UnexpectedChar('\r', 25))
```

cargo's own libgit2 checkout applies **no** EOL filters (which is why Linux/macOS
never hit this). 

**Native workaround (PR #242):** pin `core.autocrlf=false` + `core.eol=lf` so our
git-CLI checkout is byte-identical to libgit2's.

## Blocker 3 — `ssz-gen` codegen is not path-separator portable (UPSTREAM BUG)

With blockers 1 and 2 worked around, the asm crates compile until the
`strata-asm-manifest-types` build script runs the SSZ code generator, producing
a module tree that is **missing nested modules** on Windows:

```
error[E0433]: failed to resolve: could not find `ssz` in `ssz_generated`
  use crate::{..., ssz_generated::ssz::log::AsmLogEntry};
error[E0432]: unresolved import `ssz_generated::ssz`
error: could not compile `strata-asm-manifest-types` (lib) due to 5 previous errors
```

### Root cause

`ssz_codegen` derives the generated module hierarchy from file paths but splits
on the forward slash only. In `alpenlabs/ssz-gen` @ `v0.15.0`,
`crates/ssz_codegen/src/codegen.rs`, `module_tokens_to_rust_code()`:

```rust
let path_str = path.to_string_lossy().to_string();
// Split path into components
let components: Vec<&str> = path_str.split('/').collect();   // ← '/' only
```

The path keys come from `crates/ssz_codegen/src/files.rs`:

```rust
let path = Path::new(base_dir).join(entry_point);   // native separator
file_map.insert(path.with_extension(""), content);
```

On Windows `to_string_lossy()` yields `ssz\log` (backslash), so `split('/')`
returns a single component `ssz\log` instead of `["ssz", "log"]`. The nested
module `ssz` is never created, hence `ssz_generated::ssz::log` does not exist.

### Why it cannot be worked around locally

The separator is introduced by `PathBuf::join` at **build-script runtime on the
Windows host**. No environment variable or git/cargo config changes it. The only
fixes are to patch `ssz-gen` (e.g. `split(['/', '\\'])` or iterate
`Path::components()`) or to patch `asm`'s `build.rs` — both require forking Alpen
source.

### Suggested upstream fix (report to Alpen)

In `alpenlabs/ssz-gen`, `crates/ssz_codegen/src/codegen.rs`:

```rust
// Portable across path separators
let components: Vec<&str> = path_str.split(['/', '\\']).collect();
```

or, preferably, build the hierarchy from `path.components()` / `Path::iter()`
instead of a stringly-typed split, and avoid pushing `std::path::MAIN_SEPARATOR`
into a value that is later compared against split results.

## Decision

Per maintainer direction (2026-06-05): **pivot to cross-compiling the Windows
artifact on Linux.** If cross-compilation does not work (e.g. native USB/HID C
dependencies — `trezor-client`, `ledger-transport-hid` — cannot cross-compile to
the Windows target), **escalate blocker 3 (and 1) to Alpen** for an upstream fix
and treat native Windows as blocked until then.

Cross-compiling trades away the `.msi` (WiX is Windows-only) for the NSIS `.exe`,
and avoids all three blockers because the build runs in a Linux environment.

## References

- PR #238, #240 — native attempts (config-only) — did not work.
- PR #242 — native pre-checkout (blockers 1 + 2 worked around).
- `executable-delivery-plan.md` — D5 status.
- `docs/architecture/adrs/001-alpen-crate-dependencies.md` — Alpen dependency strategy.
