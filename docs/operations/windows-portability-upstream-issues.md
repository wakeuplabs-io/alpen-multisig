# Windows Build Portability — Upstream Issues

> **External document — For Alpen Labs**

## Overview

Building the desktop application natively on Microsoft Windows is currently not
possible because of two portability issues in upstream Alpen crates. Both are
platform-specific defects that only manifest on Windows; builds on Linux and
macOS are unaffected.

This document describes each issue, its root cause, the resulting build failure,
and a suggested fix, so that native Windows builds can be supported.

## Affected dependencies

| Repository | Reference | Issue |
|------------|-----------|-------|
| `github.com/alpenlabs/asm` | rev `e0461f8f520e9be814541d1f76fb961fd847e4ae` (also present on the default branch) | Reserved Windows device name used as a directory |
| `github.com/alpenlabs/ssz-gen` | tag `v0.15.0` | Code generator is not portable across path separators |

## Issue 1 — Reserved device name directory (`asm`)

### Description

The `strata-asm-common` crate contains a source directory named `aux`:

```
crates/common/src/aux/
├── collector.rs
├── data.rs
├── errors.rs
├── mod.rs
└── provider.rs
```

`aux` is a **reserved device name** on Windows (along with `con`, `prn`, `nul`,
`com1`–`com9`, and `lpt1`–`lpt9`). The Win32 file API forbids creating any file
or directory whose name is one of these, in any location. As a result, the
directory cannot be written to a normal Windows working tree.

### Impact

Tooling that checks out the repository on Windows fails. In particular, the Rust
package manager (Cargo), which uses libgit2 to materialise Git dependencies,
rejects the path during checkout validation:

```
error: failed to get `strata-asm-common` as a dependency
Caused by:
  cannot checkout to invalid path 'crates/common/src/aux/collector.rs'; class=Checkout (20)
```

This rejection is unconditional on Windows and is not affected by the
`core.protectNTFS` Git setting, so it cannot be disabled by configuration. The
checkout fails before any compilation begins.

### Suggested fix

Rename the `aux` module to a non-reserved name (for example `auxiliary`,
`aux_data`, or `auxinput`), updating the corresponding `mod` declaration and all
internal references. This is a source-only change with no effect on Linux or
macOS.

## Issue 2 — Code generator is not path-separator portable (`ssz-gen`)

### Description

The SSZ code generator derives the generated Rust module hierarchy from file
paths, but assembles those paths as strings split on the forward slash only.

In `crates/ssz_codegen/src/files.rs`, the path keys are built with the platform's
native separator:

```rust
let path = Path::new(base_dir).join(entry_point); // uses '\' on Windows
file_map.insert(path.with_extension(""), content);
```

In `crates/ssz_codegen/src/codegen.rs`, `module_tokens_to_rust_code()` then
splits those keys on `'/'` to build the module tree:

```rust
let path_str = path.to_string_lossy().to_string();
// Split path into components
let components: Vec<&str> = path_str.split('/').collect(); // '/' only
```

On Windows, `to_string_lossy()` yields a backslash-separated string (for example
`ssz\log`), so `split('/')` returns a single element (`ssz\log`) instead of the
expected `["ssz", "log"]`. The intended nested module is never created.

### Impact

The generated code is missing its nested modules on Windows, so any consumer that
references them fails to compile. For example, a crate importing a generated type
fails with:

```
error[E0433]: failed to resolve: could not find `ssz` in `ssz_generated`
error[E0432]: unresolved import `ssz_generated::ssz`
```

This occurs during the build of a crate whose build script invokes the generator,
after the dependency has otherwise compiled successfully.

### Suggested fix

Make the path-to-module mapping independent of the host path separator. Either
split on both separators:

```rust
let components: Vec<&str> = path_str.split(['/', '\\']).collect();
```

or, preferably, derive the module hierarchy from `Path::components()` /
`Path::iter()` rather than from a string split, and avoid mixing
`std::path::MAIN_SEPARATOR` into values that are later compared against
split results. This is a source-only change with no effect on Linux or macOS.

## Summary

Both issues are isolated, source-only defects that affect Windows exclusively:

1. A reserved device name (`aux`) used as a directory in `asm`, which cannot be
   checked out on Windows.
2. A path-separator assumption in the `ssz-gen` code generator, which produces
   incomplete output on Windows.

With both addressed upstream, the application can be built natively on Windows.

## Action required from Alpen Labs

- **`asm`:** Rename the reserved `aux` module directory to a non-reserved name.
- **`ssz-gen`:** Make the code generator's path handling portable across
  separators, and publish a release including the fix.

When fixed releases are available, the corresponding dependency references can be
updated to consume them.

## References

- Microsoft — Naming Files, Paths, and Namespaces (reserved device names):
  https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file
- libgit2 — path validation on Windows:
  https://github.com/libgit2/libgit2
- Rust — `std::path::Path` separator handling:
  https://doc.rust-lang.org/std/path/struct.Path.html
