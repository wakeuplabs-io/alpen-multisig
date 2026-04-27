---
name: use-case-testing-plan
description: Designs a strong business use-case testing plan with functional, negative, and regression coverage.
---

# Use Case Testing Plan

Goal: create a robust use-case test plan traceable to PRDs and executable across testing layers.

## Business Source of Truth (mandatory)

Before analyzing code or tests, read and anchor conclusions to:
- `docs/0-prd/01-multisig-ui.md`
- `docs/0-prd/02-multisig-backend.md`

Rules:
1. Never treat implementation as source of truth when it conflicts with PRD requirements.
2. If a requirement is ambiguous or undefined, do not assume behavior.
3. Mark it as `BLOCKED: business undefined`.
4. Always include:
   - the exact missing decision,
   - the impacted flow/module,
   - A/B options with tradeoffs.
5. Every test case must cite PRD requirement and expected evidence.

## Test Layers

- Unit tests (local logic and module rules).
- Integration tests (backend + stores + adapters).
- E2E tests (full signer flow from wallet to final action).
- Manual exploratory tests (hardware wallet and operational fallback).

## Minimum Required Coverage

1. Happy paths per supported role/authority.
2. Negative scenarios:
   - non-signer,
   - invalid signature,
   - signer from another authority,
   - expired session.
3. Lifecycle:
   - pending -> approved/send -> past,
   - cancellation,
   - 7-day expiration (where applicable).
4. Proposal semantics:
   - duplicates,
   - idempotency,
   - sequence handling.
5. Resilience:
   - backend unavailable,
   - continuity via manual fallback.

## Test Case Template

- `Case ID`
- `Requirement (PRD ref)`
- `Preconditions`
- `Test steps`
- `Expected result`
- `Evidence to capture` (logs/UI/API)
- `Automation level` (`unit`, `integration`, `e2e`, `manual`)
- `Priority` (`P0`, `P1`, `P2`)

## Output

1. Prioritized suite (`P0` first).
2. Traceability map `requirement -> test cases`.
3. Coverage gaps.
4. `BLOCKED: business undefined` list with concrete questions.
