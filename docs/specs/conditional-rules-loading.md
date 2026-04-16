# Spec: Conditional Rules Loading via Globs Frontmatter

## Objective

Reduce context window consumption by making Claude Code rules load conditionally based on which files are being discussed, instead of loading all 5 rules files on every conversation.

## Scope

**Included:**
- Add YAML `globs` frontmatter to 4 `.claude/rules/*.md` files
- Eliminate `general.md` by redistributing its non-duplicated content into AGENTS.md and the scoped rules files
- Update AGENTS.md to document the glob-based conditional loading

**Not included:**
- No production code changes
- No restructuring of `.claude/rules/` directory beyond removing `general.md`

## Technical Design

### Glob assignments

| File | Globs | Rationale |
|------|-------|-----------|
| `typescript-standards.md` | `desktop-app/src/**/*.{ts,tsx}` | Only relevant when editing React/TS frontend code |
| `react-frontend-patterns.md` | `desktop-app/src/**/*.{ts,tsx}` | Only relevant when editing React components/hooks |
| `rust-backend-standards.md` | `orchestator-be/**/*.rs`, `desktop-app/src-tauri/**/*.rs` | Applies to all Rust service code (backend + Tauri shell) |
| `backend-api-conventions.md` | `orchestator-be/**/*.rs` | Specific to the orchestrator HTTP API — not Tauri |

### `general.md` elimination

Most content is already duplicated in AGENTS.md or the scoped rules. Non-duplicated lines are redistributed:

**To AGENTS.md Key Conventions:**
- "Use kebab-case for directories and file names"
- "Match existing project patterns before introducing new abstractions"
- "All generated code must pass CI checks — verify locally before considering work done"

**To `typescript-standards.md`:**
- "Favor named exports for functions and components"
- "Use descriptive boolean names (`isLoading`, `hasError`, `canSubmit`)"
- "Omit semicolons unless required for correctness"

**To `rust-backend-standards.md`:**
- "`cargo clippy -- -D warnings` for linting (zero tolerance for warnings)"

### AGENTS.md changes

- Add redistributed lines to Key Conventions
- Update Rule Files section to remove `general.md` entry

## Test Cases

- Verify each rules file has valid YAML frontmatter (manual review)
- Verify `general.md` no longer exists
- Verify AGENTS.md Key Conventions contains the redistributed lines
- Verify `typescript-standards.md` contains the 3 new lines
- Verify `rust-backend-standards.md` contains the clippy line
- Verify glob patterns match the actual project directory structure

## Module structure

N/A — no new modules. Only file modifications.
