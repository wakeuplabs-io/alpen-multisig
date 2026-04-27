---
name: spec-compliance-audit
description: Audits implementation compliance against business PRDs for multisig UI/backend, with a requirement-evidence-status matrix and risk assessment.
---

# Spec Compliance Audit

Goal: validate that an implementation meets business requirements from PRDs without treating implementation as source of truth.

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
5. Every finding must cite requirement + code/test evidence.

## Scope

This skill covers business functional compliance. It does not replace deep Rust/React code audits.

## Process

1. Identify scope (feature, PR, files, module).
2. Extract applicable PRD requirements.
3. Build a `requirement -> evidence -> status` matrix.
4. Flag gaps and business risks.
5. Propose missing tests to close gaps.

## Evaluation Matrix

Use these statuses:
- `PASS`: implemented and covered.
- `PARTIAL`: incomplete implementation or insufficient coverage.
- `FAIL`: contradicts PRD or not implemented.
- `BLOCKED: business undefined`: ambiguous/incomplete PRD definition.

Minimum fields per row:
- `Requirement`
- `PRD source`
- `Code evidence`
- `Test evidence`
- `Status`
- `Risk`
- `Next action`

## Minimum Areas to Review

- Authentication and session model, with explicit authority scope.
- Isolation between multisigs and proposal visibility boundaries.
- Proposal semantics (`ActionId`, idempotency, `SeqNo`).
- Lifecycle states (`Pending`, `Approved`, `Past`, `Expired`).
- Manual fallback when backend is unavailable.
- Backend constraints as offchain coordinator (no canonical validity reimplementation).

## Output Format

1. Findings by severity (`high`, `medium`, `low`).
2. Compliance matrix.
3. Open questions marked `BLOCKED: business undefined`.
4. Final recommendation: `GO`, `GO with conditions`, or `NO-GO`.
