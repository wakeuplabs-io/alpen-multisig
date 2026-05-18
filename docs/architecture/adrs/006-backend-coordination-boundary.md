# ADR-006: Backend coordination boundary

**Status:** Proposed (skeleton — threshold policy TBD)  
**Date:** 2026-05-18  
**Supersedes:** Informal “coordination only” claim in PRD §1 without enforcement.

## Context

The orchestrator coordinates proposals, signatures, and broadcast metadata. Desktop Tauri executes commit/reveal (see [§2.1 broadcast SSOT](../../assessment/action-plan-2026-05-14.md#21-broadcast-boundary--ssot-reconciles-prd-discovery-assessments) and [proposal-broadcast-commit-reveal.md](../../specs/proposal-broadcast-commit-reveal.md)).

## Decision

| Layer | Allowed | Forbidden |
|-------|---------|-----------|
| **Orchestrator** | Hygiene (hex/SSZ shape, duplicate signer, session auth, authority scope), lifecycle persistence, broadcast coordination (`claim`, `PATCH`) | Canonical SPS-65 validity, threshold enforcement as protocol truth (unless advisory carve-out — **TBD**) |
| **Desktop** | Broadcast execution, HW signing, operator key in process env | Re-implement threshold rules; trust hard-coded IPC status |
| **React** | UX, verify gates | Private keys, direct Bitcoin RPC |

## Threshold / auto-Approve (P-012 — human decision pending)

See [wave2-human-decisions-pending.md](../../assessment/wave2-human-decisions-pending.md).

- **Option A:** Remove `Pending → Approved` when signature count reaches `required_signatures`.
- **Option B:** Keep as advisory quorum hint; document here; require threshold-resync vs ASM before broadcast (P-035).

## Consequences

- P-028 limits Strata crate imports to infrastructure codecs.
- P-026 SSZ-decodes `action_hex` at create; does not validate enactment.
- Tests must prove cross-authority denial (P-002) and hygiene boundaries.
