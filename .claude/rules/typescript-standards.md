---
paths:
  - "desktop-app/src/**/*.{ts,tsx}"
---

# TypeScript Standards

- Favor named exports for functions and components
- Use descriptive boolean names (`isLoading`, `hasError`, `canSubmit`)
- Omit semicolons unless required for correctness
- Use TypeScript for all frontend/client code and keep explicit types at API and component boundaries
- Prefer `type` for domain modeling; use `interface` only when declaration merging or extension ergonomics matter
- Model multisig domains with narrow unions (authority, lifecycle status, action kind) instead of loose strings
- Use runtime validation for external payloads at boundaries before updating UI state
- Prefer immutable updates and pure helpers for derived data (quorum counts, status grouping, time remaining)
- Handle nullable and optional values with type guards and early returns
- Keep transport DTO types separate from UI view-model types
- Keep transport DTO types in boundary modules (`domain/<feature>/services` or adapters), not in presentational components
- Expose typed hook contracts for feature boundaries (input params, returned state, and handler signatures)
- Avoid passing raw backend response shapes through screen/component trees; map once at the domain boundary
- Prefer feature-local domain types in `domain/<feature>/model` for UI-facing state
- Use utility types (`Pick`, `Omit`, `Partial`, `Record`) to remove duplication while preserving intent
- Ensure async API helpers return typed success/error shapes rather than `any` or exception-only control flow
- Prefix unused function parameters with `_` (e.g., `_actionId`) — ESLint is configured to allow `_`-prefixed unused vars
- Code must pass `npm run lint` (ESLint) and `npm run format:check` (Prettier) — see `desktop-app/eslint.config.js` and `desktop-app/.prettierrc` for active rules
