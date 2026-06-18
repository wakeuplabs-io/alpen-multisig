# Deferred backlog — post-assessment

> **Current — SSOT:** Open user-story and NFR backlog after Waves 1–3. For P-ID closure status, use [`action-plan-progress.md`](./action-plan-progress.md). See [`assessment/README.md`](./README.md).

**Created:** 2026-05-20  
**Source:** [action-plan-2026-05-14.md](action-plan-2026-05-14.md) — items not addressed in Waves 1–3.  
**Status:** Assessment closed. Items below are captured as User Stories or Non-Functional Requirements for future implementation planning. No Wave 4 is scheduled; pick up individual items as standalone PRs or group them into a new wave as needed.

---

## User Stories

### US-H5 — Manual coordinator-down fallback

**Status:** Partial.

**As a** signer, **I want to** be able to aggregate signatures and broadcast the commit/reveal transaction manually when the orchestrator is unavailable, **so that** a coordinator outage never permanently blocks an approved proposal from being enacted.

**Already shipped:**
- `/manual` UI route and `proposals_broadcast_manual` Tauri command.
- [manual-execution-flow.md](../specs/manual-execution-flow.md) spec for coordinator-down broadcast.

**Acceptance criteria (remaining):**
- Signer can export the aggregated `actionHex` + collected `signatureHex` list from the UI as a portable artifact.
- Offline path is documented in the operations runbook with a step-by-step procedure.
- When the orchestrator comes back, the signer can report broadcast progress (txids) to reconcile state.

**Acceptance criteria (met):**
- Signer can construct and broadcast the commit/reveal pair using any Bitcoin RPC endpoint without the orchestrator being online (via manual flow).

**Source:** PRD §2.3; [wave2-human-decisions-pending.md](wave2-human-decisions-pending.md) §3; P-052.  
**Priority:** High.

---

### US-EXP — Proposal expiry enforcement

**As a** signer, **I want** proposals that exceed their validity window to be automatically cancelled, **so that** stale pending proposals do not accumulate and require manual cleanup.

**Acceptance criteria:**
- Orchestrator enforces a configurable TTL on pending proposals (e.g. 48 h).
- Proposals past TTL transition to `canceled` status automatically (background sweep or on-read).
- Expired proposals are displayed as `Expired` in the dashboard; signing is blocked.
- TTL value is documented and configurable via environment variable.

**Source:** P-011 (expiry enforcement slice).  
**Priority:** High.

---

### US-RESET — Broadcast reset endpoint

**As a** multisig administrator, **I want** an admin endpoint to reset a stranded `broadcasting` proposal back to `approved`, **so that** a failed or partial broadcast does not permanently strand a proposal.

**Acceptance criteria:**
- `POST /proposals/:id/broadcast/reset` is available to authenticated admins.
- Reset transitions the proposal from `broadcasting` → `approved`, clears `commitTxid`/`revealTxid`, and resets `broadcastError`.
- Reset is idempotent; calling it on a non-broadcasting proposal returns 409.
- The endpoint is documented in the runbook.

**Source:** P-018 (resumable broadcast FSM).  
**Priority:** High.

---

### US-DISC — Signer discovery & digest usability

**As a** product owner, **I want** structured interviews with 5–8 real signers and a controlled usability test for digest verification, **so that** the UX for the hardware-wallet signing flow is validated against real operator mental models before the flow is considered production-ready.

**Acceptance criteria:**
- 5–8 signer interviews conducted per [wave2-p053-interview-plan.md](../2-discovery/wave2-p053-interview-plan.md).
- Digest verification usability test run per [wave2-p053-digest-usability.md](../2-discovery/wave2-p053-digest-usability.md).
- ≥ 80% task-success rate on "verify digest matches device display" task.
- Findings document published in `docs/2-discovery/`.
- US-H5 tabletop scenario (coordinator-down) covered in at least one session.

**Source:** P-053.  
**Priority:** Medium.

---

## Non-Functional Requirements

### NFR-AUTH — Session TTL and rate limiting

**Description:** Authentication sessions and challenge nonces must expire and must be rate-limited to prevent replay and brute-force attacks.

**Requirement:**
- Challenge nonces expire in ≤ 5 minutes; sessions expire in ≤ 8 hours.
- Session store uses TTL-aware storage (background sweep or `parking_lot` + expiry field).
- Auth endpoints (`/auth/challenge`, `/auth/verify`) are rate-limited via `tower-governor` (e.g. 10 req/min per IP).
- Rate-limit headers (`X-RateLimit-*`) are returned on 429 responses.

**Source:** P-017.  
**Priority:** High.

---

### NFR-PERSIST — Persistent proposal storage

**Status:** Done.

**Implemented:**
- `orchestrator-be` Postgres backend (`postgres_repo.rs`) behind the `ProposalRepository` trait when `DATABASE_URL` is set.
- In-memory backend remains available for local dev/testing (`memory_repo.rs`).
- Versioned migrations under `orchestrator-be/migrations/`.
- Postgres service in `staging/docker-compose.yml` for the local stack.

**Source:** P-031.  
**Priority:** Medium (closed).

---

### NFR-TYPES — Shared authority and status types

**Description:** `Authority`, `ProposalStatus`, and `BroadcastStatus` must have a single canonical definition shared between the Rust backend, Tauri layer, and TypeScript frontend to eliminate deserialization mismatches.

**Requirement:**
- A shared `multisig-types` crate (or equivalent codegen path) defines `Authority` covering all 5 variants (Strata Administrator, Alpen Administrator, Sequencer Manager, Security Council, Payout Admin).
- TypeScript branded unions for `ProposalStatus`, `BroadcastStatus`, `Authority` are generated or manually kept in sync with a round-trip serde test.
- Non–Strata-admin proposals (e.g. Sequencer Manager) deserialize correctly in Tauri.

**Source:** P-022, P-064.  
**Priority:** Medium.

---

### NFR-SEC-ENCRYPT — Encryption at rest and credential sanitization

**Description:** Sensitive signing material stored in the database must be encrypted, and broadcast errors must not leak RPC credentials.

**Requirement:**
- `signer_pubkey` and `signature_hex` columns use `pgcrypto` AES encryption at rest.
- Broadcast error messages are sanitized before storage/display — RPC URLs, usernames, and passwords are redacted.
- Key management (encryption key rotation) is documented.

**Source:** P-048.  
**Priority:** Medium.

---

### NFR-SUPPLY-CHAIN — Full release pipeline and supply-chain hardening

**Description:** Releases must be signed, reproducible, and protected against supply-chain attacks.

**Requirement:**
- `package-lock.json` committed; CI uses `npm ci` (not `npm install`).
- `cargo audit` and `cargo deny` run in CI on every PR; findings block merge.
- Pre-commit secret-scanning (e.g. `gitleaks`) runs locally and in CI.
- Production releases signed for all three target OSes (Apple Developer ID, Authenticode, PGP-signed checksum manifest).
- Tauri updater verifies signatures before applying updates.
- Multi-employee signing enforced per PRD NF-3.

**Source:** P-011 full.  
**Priority:** Medium (MVP slice already done in Wave 2 Track C; this is the full pipeline).

---

### NFR-SIGNER-SAFETY — On-device payload verification gate

**Description:** Signers must be able to verify the full authority and action context on-device before confirming a signature, not just the raw 32-byte sighash.

**Requirement:**
- Signing screen displays a human-readable summary of the action (authority label, action type, affected keys/threshold) before the hardware-wallet confirmation prompt.
- An explicit "verify this matches your device display" gate is shown with the authority name.
- The gate is dismissible only after the signer confirms.
- Backend integrity check: frontend hashes the submitted action and compares the orchestrator's returned proposal before allowing signature submission.

**Source:** P-005, P-006.  
**Priority:** Medium (P-006); Low (P-005 backend hash check).

---

### NFR-PAYOUT-ADMIN — Payout-Admin authority flow

**Description:** The application must support the Payout-Admin multisig authority end-to-end, not just Strata Administrator and Sequencer Manager.

**Requirement:**
- Payout-Admin proposals can be created, co-signed, approved, and broadcast through the standard flow.
- UI displays "Payout Admin" authority label correctly.
- Backend authority enum covers all 5 variants without deserialization errors.

**Source:** P-022/P-064 (authority coverage), PRD §1 authority matrix.  
**Priority:** Medium.

---

### NFR-SEQ-MGR — Sequencer-Manager authority flow completion

**Description:** The Sequencer-Manager signing flow must be fully supported and tested end-to-end.

**Requirement:**
- Sequencer-Manager proposals follow the same create → co-sign → approve → broadcast flow as Strata Administrator proposals.
- E2E test covers at least one Sequencer-Manager proposal lifecycle.
- UI authority labels and session scoping are correct for the Sequencer-Manager role.

**Source:** P-048.  
**Priority:** Medium.

---

### NFR-CI-WEBDRIVER — Automated WebDriver smoke in CI

**Description:** The WDIO smoke tests must run automatically in CI so regressions in the full Tauri desktop flow are caught without manual intervention.

**Requirement:**
- `npm run test:e2e` (wallet connect, propose, co-sign, broadcast) runs in CI on every PR targeting `develop`.
- CI provides a headless Tauri binary, `tauri-driver`, and `WebKitWebDriver` (Linux).
- Flaky tests are quarantined, not deleted; failures block merge.

**Source:** Playbook Wave 3 exit criterion; PRD test strategy.  
**Priority:** Low.

---

### NFR-AUDIT-LOG — Append-only proposal event log

**Description:** Every significant proposal lifecycle transition must be recorded in an immutable audit log for compliance and debugging.

**Requirement:**
- `proposal_events` table: `(id, action_id, event_type, actor_pubkey, data jsonb, created_at)`.
- Events recorded for: `created`, `signature_added`, `approved`, `broadcast_claimed`, `broadcast_completed`, `broadcast_failed`, `reset`, `expired`, `canceled`.
- Events are append-only (no updates or deletes).
- Log is queryable via a read-only API endpoint for administrators.

**Source:** P-031 (audit log slice).  
**Priority:** Low.

---

## develop → main gate

| Gate | Status | Notes |
|------|--------|-------|
| Wave 1 merged | Done | PR #134 |
| Wave 2 merged | Done | PRs #136–#142 |
| Wave 3 merged | Done | PRs #151–#159 |
| Final E2E WDIO pass on `develop` tip | **Done** | PASS `cc996de` — 2026-05-20; recorded in [action-plan-progress.md](action-plan-progress.md) E2E table |
| P-055 legal OK | Pending | Awaits Alpen legal OK for SPS reference excerpts |

**`develop → main` gate is open.** PR [#162](https://github.com/wakeuplabs-io/alpen-multisig/pull/162) ready to merge.
