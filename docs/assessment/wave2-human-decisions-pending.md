# Wave 2 — human decisions (gate log)

Per [action-plan-2026-05-14.md](action-plan-2026-05-14.md) §6. **Do not implement blocked items until approved.**

This file is the gate log for Wave 2: **resolved** entries record what was decided and where it landed; **pending** entries still block named tracks.

---

## Resolved

### 1. P-012 / ADR-006 — threshold / `approved` policy

**Status:** Resolved (2026-05-18) — merged via Track B / PR #138.

**Decision:** **Remove auto-approve on signature ingest** (Option A). Off-chain `approved` is explicit coordination state:

- `POST …/approve` only appends signatures; proposal stays `pending` at quorum.
- Desktop calls `PATCH /proposals/:action_id` with `{ "proposal_status": "approved" }` after quorum (Tauri application layer).
- Broadcast claim requires `approved` + `broadcast_status == idle`; threshold checked at transition and at claim (P-035).

**SSOT:** [ADR-006: Backend coordination boundary](../architecture/adrs/006-backend-coordination-boundary.md) (Accepted).

**Unblocks:** Track B `P-012`; ADR-006 final wording.

---

## Pending

### 2. Operator-key custody (P-001, P-003, P-040)

**Options:** process env at Tauri startup (current + P-001 gate), OS keychain, HSM, hardware-wallet-only operator.

**Blocked:** Track A `P-003` (mnemonic off IPC) and `P-040` (capabilities) design.

**Interim shipped:** P-001 desktop rejects well-known test key unless `ALLOW_DEV_OPERATOR_KEY=1`.

---

### 3. US-H5 manual-fallback scope (P-052, P-053, Track E)

**Question:** Is coordinator-down broadcast (export hex + local RPC) Slice-0 invariant or deferred?

**Blocked:** Track E orchestrator-down WDIO matrix scope.

**Note (post P-012):** Quorum does not auto-`approved`; tabletop scenarios should include explicit `PATCH` to `approved` before claim, or document export path when coordinator is down after signatures only.

---

### 4. P-055 — SPS excerpts in repository

**Question:** May we archive SPS-50/51/65 excerpts under `docs/specs/sps-reference/`?

**Stakeholder:** Alpen legal-of-record.

**Blocked:** Track F `P-055` content import.

---

### 5. Production vs test mnemonic path

**Question:** Is mnemonic-over-IPC acceptable only in dev/E2E (`ALLOW_DEV_*`), or must production builds compile it out?

**Blocked:** Track A `P-003` and Track E E2E strategy.
