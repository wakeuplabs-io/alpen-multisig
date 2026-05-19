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

### 2. Decision #2 — Secret custody (Wave 2 Slice-0)

**Status:** Decided (2026-05-18).

**Owners:** Alpen security + Wakeup platform.

**Canonical policy:** In production, the React webview must never pass a full mnemonic or operator hex to Tauri; the operator key is loaded from process env at startup; mnemonic-over-IPC is allowed only for dev/E2E behind an explicit flag.

#### Operator / broadcast (commit–reveal)

- We kept the current model: `OPERATOR_SECRET_KEY_HEX` and related broadcast configuration load from **Tauri process environment at startup** (`broadcast_env`).
- Not from the React webview over IPC. Not on the orchestrator.
- **P-001** (reject well-known test operator key unless explicit dev mode) aligns with this and ships independently of P-003 / P-040.

#### Multisig signer material (mnemonic / software signing)

- Mnemonic-based signing over IPC (`sign_with_mnemonic_path`, etc.) **remains required** for development, local POC, E2E, and CI.
- In **production / release builds**, the webview **must not** pass a full BIP39 mnemonic or raw secret key over Tauri IPC.
- Dev/E2E use is allowed only behind an explicit flag (e.g. `ALLOW_DEV_MNEMONIC_SIGNING`) and/or debug builds, and must be **excluded from the production capability set** (P-040).

#### Deferred (not part of this decision)

- OS keychain, HSM, and secrets manager for operator or signer storage → **Wave 3** (ops / runbook). Not a blocker for Wave 2 implementation on this basis.

**Unblocks:** P-003, P-040, Track A (secrets off IPC), Wave 2 exit criterion “no key/mnemonic across IPC” with a **documented dev/E2E exception**; Track E test strategy for mnemonic paths (dev builds / flags only). **P-001** desktop test-key gate continues on its own track.

**Does not decide:** US-H5 manual-fallback scope (see pending §3). Trezor-first signing UX remains product default; this decision only bounds IPC secret transport.

---

## Pending

### 3. US-H5 manual-fallback scope (P-052, P-053, Track E)

**Question:** Is coordinator-down broadcast (export hex + local RPC) Slice-0 invariant or deferred?

**Blocked:** Track E orchestrator-down WDIO matrix scope.

**Note (post P-012):** Quorum does not auto-`approved`; tabletop scenarios should include explicit `PATCH` to `approved` before claim, or document export path when coordinator is down after signatures only.

---

### 4. P-055 — SPS excerpts in repository

**Question:** May we archive SPS-50/51/65 excerpts under `docs/specs/sps-reference/`?

**Stakeholder:** Alpen legal-of-record.

**Blocked:** Track F `P-055` content import.

