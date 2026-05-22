---
name: react-ui-screen-implementation
description: Implement React screens for desktop-app using Alpen branding and domain-first frontend rules. Use when building or refactoring route screens, feature UI, hooks, and screen flows in desktop-app/src.
---

# React UI Screen Implementation

Goal: implement production-ready screens in `desktop-app/src` that follow Alpen branding and the project React/TypeScript rules.

## Required Sources (read first)

1. Branding and UI behavior:
	- `branding/`
	- `branding/uploads/Alpen_Ui_v0_1_updated 34b7b3842f0c80f096f3d5c846d74adf.md`
2. Frontend architecture and responsibilities:
	- `.claude/rules/react-frontend-patterns.md`
	- `.claude/rules/typescript-standards.md`

If any branding artifact conflicts with frontend rules, preserve the rule constraints and adapt visuals without breaking architecture boundaries.

## Architecture Contract (must follow)

- `screens/*` = route root and composition only.
- `domain/<feature>/components/*` = feature UI components (presentational).
- `domain/<feature>/hooks/use*.ts` = state, side effects, async orchestration, validation.
- `domain/<feature>/model/*` = feature-local UI/domain types and mappers.
- `domain/<feature>/services/*` = typed API/adapters at the boundary.
- `components/*` = global design-system/branding primitives only.

Do not place feature business flow in `screens` or visual components.

## Implementation Workflow

Copy this checklist and keep it updated during implementation:

```md
Screen Implementation Checklist
- [ ] Identify target route and feature domain folder
- [ ] Split responsibilities: screen composition vs hook logic vs visual components
- [ ] Reuse global branded components from `src/components` where possible
- [ ] Create/adjust feature hooks for state/effects and typed handlers
- [ ] Map backend DTOs to feature view-models at service/model boundary
- [ ] Cover loading, empty, error, and success states from branding flow
- [ ] Validate architecture placement (`screens`, `domain`, `components`)
- [ ] Run lint/format/type checks relevant to changed code
```

## Screen Composition Rules

- Keep route files thin: compose sections, pass props, wire handlers.
- Prefer one screen hook (or a small hook composition) per screen flow.
- Use explicit handler names (`handleConnect`, `handleRetry`, `handleSubmit`).
- Keep authority/session context explicit in hook state and props.

## Component Rules (presentational only)

- Components receive prepared props and emit intent callbacks.
- No direct API calls inside visual components.
- No business-rule branching that belongs to domain hooks.
- Keep copy and visual states aligned with branding docs for each screen state.

## Hook Rules (state + effects)

- Hooks own:
	- async calls and retries
	- state transitions
	- derived UI flags (`isLoading`, `canSubmit`, `hasError`)
	- transport-to-view-model mapping
- Hooks return typed contracts (state + actions), never `any`.
- Keep side effects scoped and cleanup explicit.

## Branding Application Rules

- Reuse existing brand primitives before creating new ones.
- Preserve typography, spacing rhythm, state messaging, and hierarchy from `branding/`.
- For repeated patterns (status badge, monospace box, proposal card), extract shared component on second use.
- Keep high-signal UX for multisig actions: explicit confirmation, clear errors, visible quorum progress.

## Definition of Done

- Code placement follows domain-first structure.
- Components are presentational; hooks own behavior.
- Screen states match branding flow (idle/loading/success/error/empty when applicable).
- Type boundaries are respected (DTO vs view-model).
- Updated code passes project checks (`lint`, `format`, and related frontend validations).
