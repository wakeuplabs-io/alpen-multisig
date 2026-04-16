# Spec: Conditional Rules Loading via Globs Frontmatter

## Objective

Reduce context window consumption by making AI assistant rules (Claude Code and Cursor) load conditionally based on which files are being discussed, instead of loading all rules on every conversation.

## Scope

**Included:**
- Add `globs` frontmatter to `.claude/rules/*.md` files (Claude Code)
- Sync `.cursor/rules/*.mdc` files with matching content and corrected globs (Cursor)
- Eliminate `general.md` (Claude) by redistributing its non-duplicated content into AGENTS.md and scoped rules
- Slim `general.mdc` (Cursor) to Key Conventions only — Cursor needs `alwaysApply: true` since it has no AGENTS.md equivalent
- Update AGENTS.md to document the glob-based conditional loading

**Not included:**
- No production code changes

## Technical Design

### Glob assignments

Both Claude (`.md`) and Cursor (`.mdc`) rules use the same glob patterns:

| Rule | Globs | Rationale |
|------|-------|-----------|
| `typescript-standards` | `desktop-app/src/**/*.{ts,tsx}` | Only relevant when editing React/TS frontend code |
| `react-frontend-patterns` | `desktop-app/src/**/*.{ts,tsx}` | Only relevant when editing React components/hooks |
| `rust-backend-standards` | `orchestator-be/**/*.rs`, `desktop-app/src-tauri/**/*.rs` | Applies to all Rust service code (backend + Tauri shell) |
| `backend-api-conventions` | `orchestator-be/**/*.rs` | Specific to the orchestrator HTTP API — not Tauri |

### Frontmatter format differences

**Claude** (`.claude/rules/*.md`):
```yaml
---
globs: ["pattern1", "pattern2"]
---
```

**Cursor** (`.cursor/rules/*.mdc`):
```yaml
---
description: Rule description
globs: pattern1, pattern2
alwaysApply: false
---
```

### `general` rule handling

**Claude (`general.md`)** — Eliminated. Content redistributed:

- **To AGENTS.md Key Conventions:** kebab-case naming, match existing patterns, CI gate
- **To `typescript-standards.md`:** named exports, boolean names, omit semicolons
- **To `rust-backend-standards.md`:** `cargo clippy -- -D warnings` zero tolerance

**Cursor (`general.mdc`)** — Kept with `alwaysApply: true`, slimmed to Key Conventions only (mirrors AGENTS.md). Cursor has no AGENTS.md equivalent, so this is the only place for global rules.

### Cursor glob fixes

Previous Cursor globs did not match the actual project structure:

| File | Before (broken) | After (correct) |
|------|-----------------|-----------------|
| `typescript-standards.mdc` | `**/*.{ts,tsx}` | `desktop-app/src/**/*.{ts,tsx}` |
| `react-frontend-patterns.mdc` | `**/frontend/*.{tsx,jsx}` | `desktop-app/src/**/*.{ts,tsx}` |
| `rust-backend-standards.mdc` | `**/*.rs` | `orchestator-be/**/*.rs, desktop-app/src-tauri/**/*.rs` |
| `backend-api-conventions.mdc` | `**/backend/*.{rs}` | `orchestator-be/**/*.rs` |

## Test Cases

- Verify each Claude rules file has valid YAML frontmatter with array globs
- Verify each Cursor rules file has valid frontmatter with `description`, `globs`, `alwaysApply`
- Verify `.claude/rules/general.md` no longer exists
- Verify `.cursor/rules/general.mdc` exists with `alwaysApply: true` and slimmed content
- Verify AGENTS.md Key Conventions contains the redistributed lines
- Verify content is identical between `.md` and `.mdc` counterparts (excluding frontmatter format)
- Verify glob patterns match the actual project directory structure

## Module structure

N/A — no new modules. Only file modifications.
