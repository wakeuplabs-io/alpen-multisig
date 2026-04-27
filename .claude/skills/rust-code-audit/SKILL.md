---
name: rust-code-audit
description: Technical Rust audit focused on risks, invariants, signer safety, and PRD/SPS alignment.
---

# Rust Code Audit

Goal: audit Rust code (backend and/or tauri) with focus on correctness, safety, and regressions.

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

## Hard Principles

- Protocol alignment with SPS-50, SPS-51, and SPS-65.
- Backend as offchain coordinator; do not reimplement canonical onchain validity.
- Signer safety with high-signal errors.
- Preserve manual fallback if backend becomes unavailable.

## Audit Checklist

1. **Domain and boundaries**
   - clear separation between application/domain/infrastructure;
   - proposal lifecycle and authority scoping invariants.
2. **Auth and access control**
   - proof-of-possession;
   - explicit scope by multisig authority;
   - no leakage across authorities.
3. **Proposal semantics**
   - `ActionId` idempotency;
   - duplicates rejected without mutating existing proposals;
   - `SeqNo` modeled as `u64`.
4. **Errors and result handling**
   - binaries use `anyhow::Result`;
   - libraries use `thiserror`;
   - actionable and unambiguous error messages.
5. **Concurrency and state**
   - race conditions;
   - lock contention;
   - shared state and atomicity.
6. **Cryptography and signatures**
   - structural validation vs canonical validation boundaries;
   - no unsafe assumptions;
   - replay/context-binding checks.
7. **Testing**
   - unit + integration coverage in critical and negative paths.

## Severity and Output

Classify findings as:
- `critical`: exploitable bug, authority bypass, or state corruption.
- `high`: PRD/SPS violation or serious functional regression.
- `medium`: relevant risk or impactful technical debt.
- `low`: robustness/clarity improvements.

Format:
1. Findings ordered by severity.
2. Exact evidence (code/tests) per finding.
3. Risk and failure scenario.
4. Fix recommendation + missing test.
