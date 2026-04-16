# Spec: Conditional Rules Loading via Globs Frontmatter

## Objective

Reduce context window consumption by making Claude Code rules load conditionally based on which files are being discussed, instead of loading all 5 rules files on every conversation.

## Scope

**Included:**
- Add YAML `globs` frontmatter to 4 of 5 `.claude/rules/*.md` files
- Keep `general.md` without globs (always loaded — global rules)
- Update AGENTS.md to document the glob-based conditional loading
- Remove the static "Additional Rule Files" section from AGENTS.md (no longer accurate — rules auto-load, not via `@` imports)

**Not included:**
- No production code changes
- No changes to rule content itself
- No restructuring of `.claude/rules/` directory

## Technical Design

### Glob assignments

| File | Globs | Rationale |
|------|-------|-----------|
| `general.md` | *(none — always loaded)* | Global formatting, naming, tooling, protocol alignment |
| `typescript-standards.md` | `desktop-app/src/**/*.{ts,tsx}` | Only relevant when editing React/TS frontend code |
| `react-frontend-patterns.md` | `desktop-app/src/**/*.{ts,tsx}` | Only relevant when editing React components/hooks |
| `rust-backend-standards.md` | `orchestator-be/**/*.rs`, `desktop-app/src-tauri/**/*.rs` | Applies to all Rust service code (backend + Tauri shell) |
| `backend-api-conventions.md` | `orchestator-be/**/*.rs` | Specific to the orchestrator HTTP API — not Tauri |

### Frontmatter format

```yaml
---
globs: ["pattern1", "pattern2"]
---
```

Placed at the top of each file, before the existing `# Heading`.

### AGENTS.md changes

Replace the "Additional Rule Files" section with a brief note explaining that rules in `.claude/rules/` auto-load conditionally based on globs frontmatter, and that `general.md` always loads.

## Test Cases

- Verify each rules file has valid YAML frontmatter (manual review)
- Verify `general.md` has no `globs` frontmatter
- Verify AGENTS.md no longer lists individual rule files as static references
- Verify glob patterns match the actual project directory structure

## Module structure

N/A — no new modules. Only file modifications.
