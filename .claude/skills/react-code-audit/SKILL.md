---
name: react-code-audit
description: Technical React/TypeScript audit for critical multisig flows, UX safety, and state consistency.
---

# React Code Audit

Goal: audit React/TypeScript frontend for critical signer flows and business behavior compliance.

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

## Audit Checklist

1. **Auth/session**
   - authority-scoped session initialization;
   - expiration and revocation handling;
   - session cleanup when changing wallet/multisig.
2. **Authority isolation**
   - protected routes and screens;
   - no exposure of data from other multisigs;
   - no inference of foreign proposals.
3. **Flows and lifecycle**
   - `Pending`, `Approved`, `Past`, `Expired`;
   - allowed actions per state;
   - correct messaging and CTAs.
4. **Signing UX**
   - explicit confirmation steps;
   - invalid-signature/non-signer errors;
   - clear context before signing/sending.
5. **TS/React quality**
   - consistent domain types;
   - correct effect/dependency handling;
   - robust loading/error/empty states.
6. **Frontend testing**
   - hooks/contexts tests and critical UI flow coverage.

## Severity and Output

Classify findings as:

- `high`: data exposure, permission bypass, or broken critical flow.
- `medium`: meaningful inconsistencies, state errors, or risky UX.
- `low`: maintainability/clarity improvements.

Format:

1. Prioritized findings.
2. Technical evidence (components/hooks/tests).
3. User/business impact.
4. Fix recommendation and missing test.
