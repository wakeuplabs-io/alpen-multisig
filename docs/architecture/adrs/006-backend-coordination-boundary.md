# ADR-006: Backend coordination boundary

**Status:** Accepted  
**Date:** 2026-05-18  
**Supersedes:** Informal “coordination only” claim in PRD §1 without enforcement.

## Context

The orchestrator coordinates proposals, signatures, and broadcast metadata. Desktop Tauri executes commit/reveal (see [proposal-broadcast-commit-reveal.md](../../specs/proposal-broadcast-commit-reveal.md)).

Product required a clear split between **coordination state** and **protocol validity** (P-012).

## Decision

| Layer | Allowed | Forbidden |
|-------|---------|-----------|
| **Orchestrator** | Hygiene (hex/SSZ shape, duplicate signer, session auth, authority scope), lifecycle persistence, explicit `pending → approved` on desktop request, broadcast coordination (`claim`, `PATCH …/broadcast`) | Canonical SPS-65 validity; auto-`approved` when signature count reaches quorum on ingest |
| **Desktop** | Broadcast execution, HW signing, operator key in process env; after quorum on `POST …/approve`, call `PATCH …/proposals/:id` with `proposal_status: approved` | Re-implement threshold rules; trust hard-coded IPC status only |
| **React** | UX, verify gates | Private keys, direct Bitcoin RPC |

### Off-chain `approved` (P-012)

- `approved` is **coordination state**, not on-chain quorum proof.
- `POST …/approve` **only appends signatures**; proposal stays `pending` even when `signatures.len() >= required_signatures`.
- Any authenticated signer for the proposal authority may call `PATCH /proposals/:action_id` with `{ "proposal_status": "approved" }`. Idempotent if already `approved`.
- Orchestrator validates at write time: session authority matches proposal; signature count ≥ snapshot `required_signatures`; live ASM threshold matches snapshot (P-035) or rejects with a clear conflict.
- Broadcast: desktop executes commit/reveal locally; orchestrator mirrors via `POST …/broadcast/claim` and `PATCH …/broadcast`. Claim requires `approved` + `broadcast_status == idle`; second claim returns **409 Conflict**.

### State model

Two axes only:

- `proposal_status`: `pending`, `approved`, `enacted`, …
- `broadcast_status`: `idle` → commit/reveal substates → `failed`

“Not broadcast yet” = `approved` + `broadcast_status == idle`.

## Consequences

- P-028 limits Strata crate imports to infrastructure codecs.
- P-026 SSZ-decodes `action_hex` at create; does not validate enactment.
- Tests prove: quorum ingest stays `pending`; explicit transition; threshold drift at transition and claim; idempotent approved + claim 409.
- Desktop `approve` path must call transition after quorum (Tauri application layer).
