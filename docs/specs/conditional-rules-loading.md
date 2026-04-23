# Spec: Conditional Rules Loading via Path-Scoped Frontmatter

## Objective

Reduce context window consumption by making AI assistant rules (Claude Code and Cursor) load conditionally based on which files are being discussed, instead of loading all rules on every conversation.

## Scope

**Included:**
- Add `paths` frontmatter to `.claude/rules/*.md` files (Claude Code's native field — see [docs](https://code.claude.com/docs/en/memory.md#path-specific-rules))
- Sync `.cursor/rules/*.mdc` files with matching content and corrected `globs` (Cursor's native field)
- Eliminate `general.md` (Claude) by redistributing its non-duplicated content into AGENTS.md and scoped rules
- Slim `general.mdc` (Cursor) to Key Conventions only — Cursor needs `alwaysApply: true` since it has no AGENTS.md equivalent
- Update AGENTS.md to document the path-based conditional loading

**Not included:**
- No production code changes

## Technical Design

### Path/glob assignments

Claude (`paths`) and Cursor (`globs`) use different field names but the same glob patterns:

| Rule | Globs | Rationale |
|------|-------|-----------|
| `typescript-standards` | `desktop-app/src/**/*.{ts,tsx}` | Only relevant when editing React/TS frontend code |
| `react-frontend-patterns` | `desktop-app/src/**/*.{ts,tsx}` | Only relevant when editing React components/hooks |
| `rust-backend-standards` | `orchestrator-be/**/*.rs`, `desktop-app/src-tauri/**/*.rs` | Applies to all Rust service code (backend + Tauri shell) |
| `backend-api-conventions` | `orchestrator-be/**/*.rs` | Specific to the orchestrator HTTP API — not Tauri |

### Frontmatter format differences

**Claude** (`.claude/rules/*.md`) — field is `paths`, YAML block list:
```yaml
---
paths:
  - "pattern1"
  - "pattern2"
---
```

**Cursor** (`.cursor/rules/*.mdc`) — field is `globs`, comma-separated inline:
```yaml
---
description: Rule description
globs: pattern1, pattern2
alwaysApply: false
---
```

> Using `globs` in a Claude rule file silently disables conditional loading — the rule loads unconditionally into every session. This was the defect in the initial version of this spec.

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
| `rust-backend-standards.mdc` | `**/*.rs` | `orchestrator-be/**/*.rs, desktop-app/src-tauri/**/*.rs` |
| `backend-api-conventions.mdc` | `**/backend/*.{rs}` | `orchestrator-be/**/*.rs` |

## Test Cases

- Verify each Claude rules file has valid YAML frontmatter with a `paths` block list
- Verify each Cursor rules file has valid frontmatter with `description`, `globs`, `alwaysApply`
- Verify no `.claude/rules/*.md` uses `globs` (Cursor-only field — silently ignored by Claude Code)
- In a fresh Claude Code session opened without touching any matching files, run `/context` and confirm `.claude/rules/*.md` files are NOT listed under "Memory files"
- Verify `.claude/rules/general.md` no longer exists
- Verify `.cursor/rules/general.mdc` exists with `alwaysApply: true` and slimmed content
- Verify AGENTS.md Key Conventions contains the redistributed lines
- Verify content is identical between `.md` and `.mdc` counterparts (excluding frontmatter format)
- Verify glob patterns match the actual project directory structure

## Module structure

N/A — no new modules. Only file modifications.
