# ADR-004: CI Pipeline Strategy

**Status:** Accepted
**Date:** 2026-04-10
**Context:** The project has grown to include a Rust workspace (orchestrator backend + desktop app), a separate e2e test crate, and a React/TypeScript frontend. We need a CI pipeline to enforce code quality on every pull request without adding excessive build time.

## Decision

### Platform

**GitHub Actions** — the project is hosted on GitHub, the team already uses `gh` CLI and PRs. No need for external CI providers.

### Pipeline structure: 2 parallel jobs

| Job | Steps (serial) | Purpose | Est. time |
|-----|-------|---------|-----------|
| **rust** | `cargo fmt --check` → `cargo clippy -- -D warnings` → `cargo test` → e2e tests | Lint, build, and test in one job. Clippy compiles the full workspace, so subsequent test steps reuse the build artifacts — no double compilation. | ~5 min |
| **frontend** | `npm ci` → `prettier --check` → `eslint` → `tsc && vite build` | Lint, format-check, type-check, and bundle the React frontend | ~1 min |

Jobs run in parallel. A failure in either job blocks the PR from merging.

### Tauri system dependencies

The desktop-app binary build requires Linux system libraries (`libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `librsvg2-dev`, `libappindicator3-dev`). These are installed via `apt-get` in the build-test job. If this proves too slow, we can fall back to `cargo build -p desktop-app --lib` (lib-only, no system deps) and defer the full Tauri build to a release workflow.

### E2E tests in build-test job

The e2e tests (`e2e-tests/tests/e2e_propose_sign.rs`) compile the orchestrator binary and spawn it as a subprocess. Running them in the same job as `cargo build` avoids artifact passing overhead — the binary is already compiled.

The e2e-tests crate is excluded from the workspace, so it requires a separate `cargo test` invocation from the `e2e-tests/` directory.

### Caching

Use `Swatinem/rust-cache` for Cargo registry, git sources, and target directory. Use `actions/cache` with `package-lock.json` hash for `node_modules`. Cache is critical — without it, a clean Rust build takes 10+ minutes.

### Clippy policy

`cargo clippy -- -D warnings` — warnings are treated as errors in CI. Existing dead_code warnings in the orchestrator must be cleaned up before enabling this. No `#[allow]` exceptions unless explicitly justified.

### Trigger policy

- `on: pull_request` targeting `develop` and `main` — validates every PR
- `on: push` to `develop` and `main` — validates post-merge state
- Feature branch pushes do NOT trigger CI (the PR event covers them)

### Branch protection

Enable branch protection rules on `develop`:
- Require all 3 CI jobs to pass before merge
- Require at least 1 approval

## Alternatives considered

| Alternative | Why rejected |
|------------|-------------|
| Three separate Rust jobs (lint, build, test) | Each job pays full compilation cost. Clippy already compiles everything, so a single serial Rust job avoids double builds |
| `cargo audit` for security | Too noisy with pre-release Alpen crates; revisit when deps stabilize |
| Code coverage (`cargo llvm-cov`) | Adds significant time; coverage % doesn't correlate well with quality for this project size |
| Multi-OS matrix (macOS, Windows) | Needed for release builds but overkill for CI; Linux catches 99% of issues |
| Separate job for e2e tests | Artifact upload/download overhead isn't worth the isolation benefit |
| ESLint deferred | Initially considered deferring, but included — catches React-specific issues (hooks rules, stale refs) that `tsc` misses |

## Consequences

- Every PR must pass both CI jobs (rust + frontend) before merge
- Dead code warnings in the orchestrator must be resolved (or explicitly `#[allow]`ed with justification)
- CI feedback loop should stay under ~5 minutes for the slowest job
