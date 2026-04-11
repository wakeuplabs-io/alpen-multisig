# Spec: CI Pipeline Optimization

## Objective

Fix bugs and optimize the GitHub Actions CI pipeline. Current clean build takes ~745s (12+ min), cached ~202s. The pipeline has stale configuration from before e2e-tests was integrated into the workspace, and uses the wrong Rust toolchain declaration.

## Scope

### Included

- Fix toolchain: replace `dtolnay/rust-toolchain@stable` with nightly from `rust-toolchain.toml`
- Remove stale `e2e-tests -> target` cache path
- Remove redundant `cd e2e-tests && cargo test` step
- Ensure `~/.cargo/git/db/` is cached (Alpen git deps ~21s clone on miss)
- Add Cargo.lock to rust-cache key for better invalidation

### NOT included

- Splitting CI into separate backend/desktop jobs (future optimization — would save Tauri/GTK compilation when only backend changes, but adds workflow complexity)
- Adding `default-members` to Cargo.toml (developer-facing optimization, separate concern)
- Changing the nightly toolchain pin
- Adding sccache or mold linker (require additional CI setup, can be done later)

## Technical Design

### Changes to `.github/workflows/ci.yml`

**1. Toolchain fix:**

Replace:
```yaml
- uses: dtolnay/rust-toolchain@stable
  with:
    components: rustfmt, clippy
```

With:
```yaml
- uses: dtolnay/rust-toolchain@master
  with:
    toolchain: nightly-2026-01-01
    components: rustfmt, clippy
```

This matches the `rust-toolchain.toml` pin and avoids downloading both stable and nightly.

Note: We use `@master` with explicit `toolchain:` because `@nightly` would give us the latest nightly, not our pinned version. Reading the channel from `rust-toolchain.toml` would be ideal but `dtolnay/rust-toolchain` doesn't support that directly.

**2. Cache fix:**

Replace:
```yaml
- uses: Swatinem/rust-cache@v2
  with:
    workspaces: |
      . -> target
      e2e-tests -> target
```

With:
```yaml
- uses: Swatinem/rust-cache@v2
  with:
    cache-on-failure: true
```

Remove the stale `e2e-tests -> target` workspace. The default (`. -> target`) is sufficient. Add `cache-on-failure: true` so that even failed CI runs populate the cache (saves time on retry).

Swatinem/rust-cache already caches `~/.cargo/git/db/` and `~/.cargo/registry/` by default, so no extra config needed for Alpen git deps.

**3. Remove redundant e2e step:**

Remove:
```yaml
- name: E2E tests
  run: cd e2e-tests && cargo test
```

`cargo test` already runs all workspace members including `alpen-multisig-e2e-tests`.

**4. Run tests with `--workspace` explicitly:**

Change:
```yaml
- name: Test workspace
  run: cargo test
```

To:
```yaml
- name: Test workspace
  run: cargo test --workspace
```

Explicit is better than implicit — ensures all members are tested even if `default-members` is added later.

Same for clippy:
```yaml
- name: Clippy
  run: cargo clippy --workspace -- -D warnings
```

### Production code vs. test helpers

N/A — this is a CI configuration change only. No Rust code modified.

## Test Cases

- CI pipeline runs successfully on the PR itself (self-validating)
- `cargo test --workspace` passes locally
- `cargo clippy --workspace -- -D warnings` passes locally
- `cargo fmt --check` passes locally
- Verify Swatinem/rust-cache default behavior covers `~/.cargo/git/db/`

## Module structure

N/A — single file change (`.github/workflows/ci.yml`).
