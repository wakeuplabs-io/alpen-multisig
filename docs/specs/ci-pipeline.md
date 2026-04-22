# Spec: CI Pipeline (GitHub Actions)

## Objective

Implement a CI pipeline that enforces code quality, correctness, and build integrity on every pull request. Follows decisions documented in [ADR-004](../architecture/adrs/004-ci-pipeline-strategy.md).

## Scope

**Included:**
- GitHub Actions workflow file (`.github/workflows/ci.yml`)
- 2 parallel jobs: rust (lint/build/test), frontend (lint/format/build)
- Clippy with `-D warnings` (zero tolerance)
- Workspace tests + e2e tests (orchestrator subprocess)
- Tauri system dependency installation (both Rust jobs)
- Rust caching (`Swatinem/rust-cache@v2`) and Node caching (`actions/cache`)
- ESLint setup for React + TypeScript frontend (`eslint`, `@eslint/js`, `typescript-eslint`, `eslint-plugin-react-hooks`, `eslint-plugin-react-refresh`)
- Prettier setup for frontend formatting enforcement (`prettier`), config aligned with project rules (tabs, single quotes, no semicolons)
- Clean up any remaining clippy/build warnings in the workspace

**NOT included:**
- `cargo audit`, code coverage, multi-OS matrix
- Branch protection rules (manual GitHub settings, documented in ADR)
- Release/deploy workflows

## Technical Design

### File: `.github/workflows/ci.yml`

Single workflow file with 2 parallel jobs (Rust + frontend).

#### Shared configuration

- **Runner:** `ubuntu-latest`
- **Rust toolchain:** stable (via `dtolnay/rust-toolchain@stable`)
- **Triggers:**
  - `on: pull_request` — branches: `develop`, `main`
  - `on: push` — branches: `develop`, `main`

#### Job 1: `rust`

Purpose: Lint, format-check, build, and test all Rust code in a single serial job. Clippy compiles the full workspace, so `cargo test` reuses the build artifacts — no double compilation.

Steps:
1. `actions/checkout@v4`
2. `dtolnay/rust-toolchain@stable` with `components: rustfmt, clippy`
3. Install Tauri system deps (`libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `librsvg2-dev`, `libappindicator3-dev`)
4. `Swatinem/rust-cache@v2`
5. `cargo fmt --check`
6. `cargo clippy -- -D warnings`
7. `cargo test`
8. `cd e2e-tests && cargo test`

#### Job 2: `frontend`

Purpose: Lint, TypeScript type-check, and Vite production build.

Steps:
1. `actions/checkout@v4`
2. `actions/setup-node@v4` with `node-version: 20`
3. `actions/cache@v4` with path `desktop-app/node_modules`, key based on `desktop-app/package-lock.json` hash
4. `cd desktop-app && npm ci`
5. `cd desktop-app && npm run format:check`
6. `cd desktop-app && npm run lint`
7. `cd desktop-app && npm run build`

### ESLint setup

Add ESLint to the frontend with flat config (`eslint.config.js`).

**Dev dependencies to add:**
- `eslint` (^9)
- `@eslint/js`
- `typescript-eslint`
- `eslint-plugin-react-hooks`
- `eslint-plugin-react-refresh`
- `globals`

**Config:** Flat config extending `@eslint/js` recommended + `typescript-eslint` recommended + React hooks plugin. Ignore `dist/` output.

**npm script:** Add `"lint": "eslint src"` to `package.json`.

### Prettier setup

Add Prettier for frontend formatting enforcement.

**Dev dependencies to add:**
- `prettier`

**Config (`.prettierrc`):** Aligned with project rules from `.claude/rules/general.md`:
```json
{
  "useTabs": true,
  "singleQuote": true,
  "semi": false,
  "printWidth": 120,
  "trailingComma": "all"
}
```

**npm script:** Add `"format:check": "prettier --check src"` to `package.json`.

Existing frontend code must be formatted with Prettier before enabling the check (run `npx prettier --write src` once).

### Warning cleanup

If any clippy or build warnings exist in the workspace, they must be resolved before the workflow is added. This includes:
- Dead code in `orchestrator-be` (domain types, middleware, error variants)
- Any other warnings surfaced by `cargo clippy -- -D warnings`

Resolution options per warning type:
- **Dead code that will be used soon** (auth types, middleware, domain models): Add `#[allow(dead_code)]` with a comment explaining it's planned for future use
- **Dead code that is truly unused**: Remove it

### Production code vs. test helpers

- **Production:** `.github/workflows/ci.yml` — the workflow file
- **Production (cleanup):** Warning fixes across workspace crates
- **Test helpers:** None — this is infrastructure, not application code

## Test Cases

1. **Workflow validates locally:** `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo build`, `cargo test`, `cd e2e-tests && cargo test`, `cd desktop-app && npm run format:check`, `cd desktop-app && npm run lint`, `cd desktop-app && npm run build` all pass locally before pushing
2. **PR triggers CI:** Push a branch and open a PR against `develop` — both jobs should run and pass

## Module structure

- **`.github/workflows/ci.yml`** — Single CI workflow with 2 parallel jobs (rust, frontend)
- **`desktop-app/eslint.config.js`** — ESLint flat config for React + TypeScript
- **`desktop-app/.prettierrc`** — Prettier config (tabs, single quotes, no semicolons, 120 chars)
- **`desktop-app/package.json`** — Updated with ESLint + Prettier dev deps, `lint` and `format:check` scripts
