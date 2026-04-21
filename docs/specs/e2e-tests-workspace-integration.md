# Spec: Integrate e2e-tests into the Cargo Workspace

## Objective

Move `e2e-tests` from an excluded standalone crate into the Cargo workspace. Since the entire workspace already uses nightly and depends on Alpen crates, there is no reason to keep e2e-tests separate. Integration eliminates duplicate dependency compilation (~600 crates compiled twice), removes duplicated dependency pins that can drift out of sync, and simplifies the developer experience.

## Scope

### Included

- Add `e2e-tests` to `workspace.members` in root `Cargo.toml`
- Migrate e2e-tests dependencies to use `workspace = true` where pins already exist
- Add missing workspace-level dependencies needed only by e2e-tests (`strata-asm-txs-test-utils`, `strata-test-utils`, `rand`, `hex`, `tokio`, `reqwest`)
- Remove `e2e-tests/rust-toolchain.toml` (workspace-level one applies)
- Remove the `exclude = ['e2e-tests']` directive
- Update ADR-001 to reflect the new structure
- Update CLAUDE.md commands if needed

### NOT included

- Changing any test logic or test files
- Changing the backend or desktop-app crates
- Adding new tests
- Changing the nightly toolchain pin

## Technical Design

### Changes to root `Cargo.toml`

1. Remove `exclude = ['e2e-tests']`
2. Add `'e2e-tests'` to `members`
3. Add to `[workspace.dependencies]`:
   ```toml
   # Test-only Alpen crates
   strata-asm-txs-test-utils = { git = "https://github.com/alpenlabs/alpen", rev = "308211f" }
   strata-test-utils = { git = "https://github.com/alpenlabs/alpen", rev = "308211f" }

   # Shared utilities (used by e2e-tests and potentially others)
   rand = "0.8"
   hex = "0.4"
   tokio = { version = "1", features = ["full"] }
   reqwest = { version = "0.12", features = ["json"] }
   ```

4. Add feature-activated workspace deps for crates that e2e-tests needs with extra features:
   ```toml
   # Already exist, no change needed — features are additive at workspace level
   # but e2e-tests needs test-utils/arbitrary features.
   # Solution: declare the base version in workspace, add features in the member.
   ```

### Changes to `e2e-tests/Cargo.toml`

Replace all explicit git/version pins with `workspace = true`. For crates needing extra features:

```toml
[dependencies]
strata-asm-txs-admin = { workspace = true, features = ["test-utils"] }
strata-crypto = { workspace = true, features = ["test-utils"] }
strata-asm-params = { workspace = true, features = ["arbitrary"] }
strata-primitives.workspace = true
strata-asm-common.workspace = true
strata-asm-txs-test-utils.workspace = true
strata-test-utils.workspace = true
strata-l1-txfmt.workspace = true
desktop-app = { path = "../desktop-app/src-tauri" }
bitcoin.workspace = true
borsh.workspace = true
secp256k1 = { workspace = true, features = ["recovery"] }
rand.workspace = true
hex.workspace = true
tokio.workspace = true
reqwest.workspace = true
serde_json.workspace = true
```

### Delete `e2e-tests/rust-toolchain.toml`

The workspace-level `rust-toolchain.toml` now applies.

### Delete `e2e-tests/Cargo.lock`

Workspace members share the root `Cargo.lock`.

### Update ADR-001

- Remove the section "e2e-tests remains a separate crate"
- Update the update procedure to remove the step about syncing e2e-tests pins manually
- Note that test-only Alpen crates are now in workspace dependencies

## Test Cases

- `cargo build` succeeds from workspace root (all members including e2e-tests)
- `cargo test -p alpen-multisig-e2e-tests` runs and passes both existing tests
- `cargo test` (full workspace) passes
- `cargo clippy` passes with no warnings
- `cargo fmt --check` passes
- No duplicate `Cargo.lock` exists in e2e-tests/

## Module structure

No new modules. This is a build configuration change only — no Rust source files are modified.
