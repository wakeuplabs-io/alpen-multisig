# Alpen Multisig — Consolidated Action Plan (2026-05-14)

> **Historical.** Snapshot from 2026-05-14 synthesis. For current backlog and P-ID closure, use [`deferred-backlog.md`](./deferred-backlog.md) and [`action-plan-progress.md`](./action-plan-progress.md) — see [`assessment/README.md`](../README.md).

**Inputs:** Historical synthesis from May 2025 adversarial assessments (folders removed when stale). **Current resolution:** [ADR-006](../../architecture/adrs/006-backend-coordination-boundary.md), [wave2-exit-gap-review.md](wave2-exit-gap-review.md), [action-plan-progress.md](../action-plan-progress.md).
**Method:** Cross-audit synthesis. Each item carries a stable **P-###** ID inherited from the 2026-05-13 meta-review, with deltas from 2026-05-14 marked **[Δ 05-14]**.
**Status:** Read-only synthesis. No runtime probes were executed; severities are code-read.

---

## 1. Executive summary

**Historical context:** This document synthesizes May 2025 adversarial assessments (source folders removed when stale). Several Tier-0 findings from the 2026-05-14 re-read are **closed in code** — see [action-plan-progress.md](../action-plan-progress.md) and [wave2-exit-gap-review.md](wave2-exit-gap-review.md).

**Closed since synthesis:**
- **Broadcast boundary (P-066, P-062):** Desktop executes commit/reveal; orchestrator exposes `claim_broadcast` and PATCH coordination only. IPC returns persisted `approved` / `reveal_broadcasted` statuses.
- **Coordination-only boundary ([ADR-006](../../architecture/adrs/006-backend-coordination-boundary.md)):** Explicit `pending → approved` transition; signature ingest does not auto-approve.
- **Persistent proposals (NFR-PERSIST):** Postgres when `DATABASE_URL` is set.

**Still open:** [deferred-backlog.md](../deferred-backlog.md) and Wave 2/3 track follow-ups. US-H5 manual fallback is partial — `/manual` and [manual-execution-flow.md](../../specs/manual-execution-flow.md).

The risk themes below remain a useful historical map; severities are not re-audited in this edit.

1. **Signer-key surface is wide open.** Operator secret key defaults to the well-known test key; mnemonics and private keys cross the Tauri IPC boundary in plaintext; CSP is `null`; releases are unsigned; there is no SCA in CI; the frontend even has a `VITE_OPERATOR_SECRET_KEY_HEX` path that can leak through sourcemaps.
2. **Broadcast boundary — resolved (P-066, P-062).** See §2.1 and [ADR-006](../../architecture/adrs/006-backend-coordination-boundary.md).
3. **Type, error, and identity drift across Rust ↔ TS.** Backend `Authority` has 5 variants, Tauri shell has a subset, React has 2 (`StrataAdministrator`, `StrataSequencerManager`). [Δ 05-14] Duplicate-signer detection compares pubkeys with `==` while session auth uses `eq_ignore_ascii_case` — the same Trezor signer can pass dedup twice. Errors collapse to `error: string` at every boundary; no Zod validation at the bridge; `u64 seq_no` is exposed as a JSON `number`.
4. **Coordination state durability — partially resolved.** Postgres persistence is available; in-memory remains the dev default. Broadcast idempotency, append-only audit log, and correlation IDs remain open.

Doc/process layer: manual-fallback story (US-H5) is partially implemented; README/AGENTS Diataxis work and shared types (P-022) remain.

**Production-readiness window:** Historical estimate from May 2025; see [action-plan-progress.md](../action-plan-progress.md) for current closure status.

---

## 2. What changed between 2026-05-13 and 2026-05-14

> **Status (2026-06):** Rows 1–2 are **resolved in code**. Row 7 (P-061/P-066) **implemented**. See [action-plan-progress.md](../action-plan-progress.md).

| # | Change | Direction |
|---|---|---|
| 1 | **Desktop broadcast bypasses orchestrator state machine** — local `broadcast_commit_then_reveal` runs commit/reveal; `claim_broadcast` is dead on the happy path. | **RESOLVED** — `submit_commit_then_reveal` calls `claim_broadcast`; coordinator PATCH records txids |
| 2 | **Hard-coded `BroadcastResultDto` returns `"enacted"` / `"reveal_confirmed"`** — UI shows finality without persisted truth. | **RESOLVED** — returns `approved` / `reveal_broadcasted` from persisted state |
| 3 | **Pubkey case mismatch between dedup and auth** — `==` vs `eq_ignore_ascii_case` allows the same signer to be counted twice. | Promoted to BLOCKER |
| 4 | **Tauri `Authority` subset vs backend's 5 variants** — non–Strata-admin proposals fail deserialization in the shell. | Promoted to BLOCKER |
| 5 | **`/api/v1` confirmed to exist.** Prior "no API versioning" item is **retracted**. | Retraction |
| 6 | The remainder of Tier 0/1 from 2026-05-13 stands; no item from that list was closed in code. | No movement |
| 7 | **P-061 superseded by P-066 (2026-05-16):** Original P-061 text (“never broadcast locally”) withdrawn. Desktop-owned execution + coordinator metadata is the target architecture. | **Resolved in plan + code** |

---

## 2.1 Broadcast boundary — SSOT (reconciles PRD, discovery, assessments)

| Layer | Owns | Must not |
|-------|------|----------|
| **ASM / Bitcoin** | Protocol validity, enactment | — |
| **Desktop (Tauri)** | Commit/reveal construction, `send_to_address` / `sendrawtransaction`, operator key in Rust process, HW signing | Re-implement SPS-65 threshold rules; trust hard-coded IPC status strings |
| **Orchestrator** | Proposals, signatures, quorum/off-chain lifecycle, **optional** broadcast coordination metadata (`claim_broadcast`, txids, `broadcast_status`, errors) | Be required for a signer to broadcast; hold sole copy of operator key for production happy path; re-validate protocol rules |
| **React** | UX, explicit verify gates | Private keys, direct Bitcoin RPC |

**What the 2026-05-14 audits actually proved (still valid):**

- Signers need a **shared** view of whether a proposal is already being broadcast (avoid duplicate commit/reveal across machines).
- UI must show **persisted** `proposal_status` / `broadcast_status`, not literals.
- Cross-authority reads must be scoped (P-002).

**What they incorrectly concluded:**

- That the fix is “desktop must call `POST …/broadcast` so the **backend** runs `broadcast_commit_then_reveal`.” That solves desync by centralizing execution — but violates PRD §2 (“signers MUST … broadcast transactions directly to Bitcoin” if the backend is down) and piles operator-key + bitcoind coupling onto the server (fights P-003, P-015).

**Implemented pattern (codified in `docs/specs/proposal-broadcast-commit-reveal.md`, ADR-006 still pending):**

1. **Desktop (Tauri):** `proposals_prepare_broadcast` and `proposals_broadcast` use `broadcast_env` (process env) for Bitcoin RPC + operator key; build and submit commit/reveal locally.
2. **Orchestrator:** `POST /proposals/:id/broadcast/claim` then `PATCH /proposals/:id/broadcast` for `broadcast_status`, txids, and errors — no server-side `sendrawtransaction`.
3. **Manual fallback (US-H5):** export hex; broadcast via any RPC; report progress when coordinator is back (PRD §2).

**Removed from orchestrator:** `POST …/broadcast/prepare`, `POST …/broadcast` (execute), `broadcast_tx` module, `OPERATOR_SECRET_KEY_HEX` in server config.

---

## 3. Consolidated backlog (merged, deduped)

IDs are stable across audits. Severity uses a single legend: **BLOCKER** (Tier 0), **HIGH** (Tier 1), **MEDIUM** (Tier 2).
"Effort" is engineering days for one focused engineer: **S** ≤2d, **M** 3–10d, **L** >10d.
"Sources" cite the axis number(s); `Δ` = newly evidenced or strengthened on 2026-05-14.

### 3.1. Tier 0 — BLOCKERS (security, signer safety, integrity)

| ID | Title | Effort | Sources |
|---|---|---|---|
| P-001 | Operator secret key: **desktop** `broadcast_env` must reject well-known test key unless explicit dev flag; orchestrator no longer loads operator key. Remove any `VITE_OPERATOR_*` path. | S | 01, 02, 05, 11, 14, 16 (Δ 14) |
| P-002 | Authority-scope leakage on proposal reads and broadcast coordination (`claim` / `PATCH`). Authority filter; 401 on mismatch. | S | 01, 03, 06, 08 (Δ 04) |
| P-003 | Plaintext secrets across Tauri IPC (`mnemonic`, `operator_secret_key_hex`). Accept derivation indices only; load operator key in Rust at startup; wrap sensitive fields in `ZeroizeOnDrop`. | M | 02, 05 |
| P-004 | CSP disabled (`"csp": null`). Set strict CSP; pair with Tauri 2 capabilities per window. | S | 02, 05 |
| P-005 | No client-side check that the backend returned the proposal the user submitted (MITM / malicious-backend window). Hash submitted action and compare before signing/broadcast. | S | 02, 03 |
| P-006 | Payload divergence — Trezor only shows the 32-byte SPS-65 sighash; authority/action are not displayed on-device. Add explicit on-screen "verify this hex matches your device" gate that names the authority. | M | 02, 03, 12, 13, 15, 16 |
| P-007 | Sighash swap between preview and sign in `create-proposal-form.tsx`. Freeze form values at preview; force re-preview on edit. | S | 03, 13 |
| P-008 | No runtime IPC validation at the Tauri bridge. Add Zod schemas for `Proposal`, `ProposalStatus`, `BroadcastStatus`, `AuthSession`; treat unknown enum variants as errors. | S | 03, 04, 14 |
| P-009 | Session token has no authority binding; cross-authority reuse. Validate `session.authority === authorityFromRole(selectedRole)` before reuse; `await authLogout()` before re-auth. | S | 03, 04, 13 |
| P-010 | Deep-link `/proposals/:actionId/sign` bypasses authority context. Refuse to render if `proposal.authority` ≠ selected role. | S | 03, 13 |
| P-011 | Unsigned releases, no SCA, no committed lockfile, npm `^`-ranges, git-rev–pinned `alpen-*` with no signature verification. Commit `package-lock.json`, `npm ci`, `cargo audit` + `cargo deny`, pre-commit secret-scanning. | M | 02, 05 |
| P-012 | ~~Backend auto-approves on threshold~~ — **CLOSED:** [ADR-006](../../architecture/adrs/006-backend-coordination-boundary.md) documents explicit `approve_action`; ingest does not auto-transition. Threshold resync tests remain in Wave 3. | — | 06, 13, 16 |
| P-013 | `parse_network` defaults to `regtest`. Require explicit `bitcoin`/`testnet`/`signet`/`regtest`; fail otherwise. | S | 02 |
| P-014 | Bearer token transported over user-supplied `base_url`; no HTTPS enforcement. Reject non-`https://` in `build_client` (allow `http://localhost` only in dev). | S | 02 |
| P-015 | `VITE_OPERATOR_SECRET_KEY_HEX` env path can leak via sourcemaps. Delete env var; load operator key only in Rust at startup. | S | 02, 05 |
| **P-061** | **[Δ 05-14] Superseded (2026-05-16).** Original wording (“route all broadcast through orchestrator”) conflicts with PRD. Do not implement. Use **P-066**. | — | 02, 04 |
| **P-062** | **[Δ 05-14] UI reflects persisted coordinator state.** Re-fetch proposal after broadcast; no hard-coded `proposal_status` / `broadcast_status` in IPC. | S | 02, 04 |
| **P-066** | **[2026-05-16] Broadcast boundary — desktop executes, orchestrator coordinates.** Tauri local commit/reveal; `claim` + `PATCH` APIs; spec + architecture docs updated; server execute/prepare routes removed. | M | PRD, 09-functional-analysis |
| **P-063** | **[Δ 05-14] Pubkey case mismatch: dedup uses `==`, auth uses `eq_ignore_ascii_case`.** Normalize hex pubkeys to lowercase at every ingress; DB `CHECK (signer_pubkey ~ '^[a-f0-9]{66}$')`. | S | 04, 09 |
| **P-064** | **[Δ 05-14] Tauri `Authority` is a subset of backend's 5 variants.** Non–Strata-admin proposals fail deserialization. Promote shared `Authority` to a shared crate (see P-022/L1) or, short-term, extend the Tauri enum and add a serde round-trip test. | S→M | 04, 06, 08 |

### 3.2. Tier 1 — HIGH (durability, idempotency, observability)

| ID | Title | Effort | Sources |
|---|---|---|---|
| P-016 | In-memory storage is the silent default; `DATABASE_URL` unset → no durability. Fail startup in production mode. | S | 01, 05, 07, 09, 11 |
| P-017 | Auth challenges and sessions in `Arc<RwLock<HashMap>>` with no TTL, no persistence, no rate limit. Add TTL sweep, `tower-governor` rate limit, swap `RwLock` for `parking_lot`. | M | 01, 02, 07, 09, 11 |
| P-018 | Broadcast is non-atomic; partial state strands proposals. Add `tokio::time::timeout` on every BTC/ASM RPC call; add admin `/reset-broadcast` endpoint. Foundation for resumable FSM. | M | 01, 06, 07, 11 |
| P-019 | Duplicate-signer race + non-linearized quorum transition. Move dedup inside `add_signature` under the write lock; return `(proposal, quorum_reached)`. Add optimistic-locking `version: u64`. | M | 07, 08, 10 |
| P-020 | Tauri broadcast has no idempotency, no in-flight dedupe, no disabled Send button. Disable button while broadcasting; cache in-flight `action_ids`; thread `Idempotency-Key` end-to-end. | S | 02, 04, 07, 10 |
| P-021 | `u64 seq_no` silently rounded by JavaScript above 2^53−1. Serialize as string; parse via `BigInt`. | S | 04 |
| P-022 | Status enums travel as opaque strings; no enum guard. Branded TS unions for `ProposalStatus`, `BroadcastStatus`, `Authority`; Zod parse at bridge. | S | 04, 06, 08, 14 |
| P-023 | Error model collapses to `error: string` across the bridge. Add `errorCode` discriminant to `ApiResult`; thread HTTP status + Tauri error category. | M | 04, 14, 11 |
| P-024 | Pubkey case sensitivity mismatch — see P-063 (Tier 0). | — | (merged) |
| P-025 | Mock RPC via URL pattern matching is wired into production code paths (`is_signer_member_for_authority`). Inject `AsmStateRpc` trait; mocks only in `#[cfg(test)]`. | M | 06, 08, 14 |
| P-026 | No SSZ validation at the create-proposal boundary. SSZ-decode in handler; reject early. | S | 06, 14 |
| P-027 | No timeouts/retries/backoff on Bitcoin or ASM RPC. `tokio::time::timeout` on every external call + retries with jitter; circuit breaker per dependency. | M | 07, 11 |
| P-028 | Strata crates leak from `infrastructure/action_codec.rs` into application layer. Route all SSZ decode through `action_codec`; enforce via clippy/`deny` lint. | S | 06, 14 |
| P-029 | No structured logging, no correlation IDs, no `/ready` probe. `#[tracing::instrument(action_id, authority, seq_no)]` on handlers; generate request UUID in `tauri-bridge.ts`; `/health` + `/ready` that actually check Postgres/BTC/ASM. | M | 04, 05, 11 |
| P-030 | No persistent auth-challenge / consumed-nonce store. Persist consumed `challenge_id` with TTL in Postgres; HTTP-layer replay-rejection test. | S | 09, 10 |
| P-031 | No append-only event/audit log. Add `proposal_events` table (`event_type`, `data jsonb`, `created_at`); foundation for event sourcing. | M→L | 08, 09, 11 |
| P-032 | No frontend tests, no Tauri IPC contract tests, no concurrent-approval test, no broadcast-error-path test. Implement axis-10 test inventory (≥20 tests). | L | 10 |
| P-033 | BIP-137 recovery-id normalization missing in `broadcast_tx::build_signed_payload_bytes`. Implement or import `normalize_recovery_id`. (Confirm still present before fixing.) | S | 11 |
| P-034 | No proposal expiry enforcement. `ProposalStatus::Expired` exists but nothing transitions to it. Check `now > expires_at` in `approve_action` and broadcast paths; background expiry job. | S | 07, 13 |
| P-035 | Threshold snapshot vs on-chain change (pairs with P-012). Refuse broadcast if backend `required_signatures` ≠ freshly-fetched ASM threshold, or document explicitly. | S | 13 |
| P-036 | Hardcoded constants (`REVEAL_TX_VBYTES`, `COMMIT_DUST_SATS`, `0x414c504e`, `m/86'/0'/73'/0/*`) duplicated across binaries; role 73 is Strata-Admin only. Centralize in config; fetch role at auth. | S | 02, 06, 14 |
| P-037 | Authority mapping incomplete (`authority_to_role`). Add test that fails until all 5 authorities map; feature-gate or grey-out unsupported authorities in the UI. | S | 06, 08, 16 |
| P-038 | No signature golden-test against SPS-65 across the 5×N grid. Add parameterized e2e test. | M | 10, 14, 16 |
| P-039 | Wallet address not validated against connected wallet in `session-provider.tsx`. Compare `signature.publicKeyHex` to `wallet.publicKeyHex` before submission. | S | 03 |
| P-040 | Tauri commands lack a capability/OCAP model. Gate high-risk commands on `get_session()` + role; long-term, use Tauri 2 capabilities per window. | M | 02, 05 |

### 3.3. Tier 2 — MEDIUM (maintainability, docs, process)

| ID | Title | Effort | Sources |
|---|---|---|---|
| P-041 | `AppState` god-object (12 unrelated concerns). Split into per-concern services. | M | 06 |
| P-042 | `Proposal` anemic aggregate. Private fields + `add_signature_if_pending`; split into `ProposalAggregate` + `BroadcastAggregate`. | M | 08, 09 |
| P-043 | Authority duplication & ubiquitous-language drift across Rust↔TS. Fix `'Strata' → 'Alpen Administrator'` label bug; shared `multisig-types` crate. | M | 04, 06, 08, 13, 14 |
| P-044 | Bitcoin/Strata types in the application layer. Push to infrastructure. | S | 06, 08 |
| P-045 | `SessionContext` lives in `application/` not `domain/`. Move; enables cron/batch authorization. | S | 08 |
| P-046 | ~~No API versioning~~ — **RETRACTED 2026-05-14**: `/api/v1` is present. | — | (Δ 04) |
| P-047 | No data retention, no soft delete, no FSM enforcement on `BroadcastStatus`. Add `can_transition_to` predicate; `deleted_at` columns. | M | 09 |
| P-048 | No encryption at rest; broadcast errors echo RPC URLs / credentials. `pgcrypto` for `signer_pubkey`/`signature_hex`; sanitize broadcast errors. | M | 09 |
| P-049 | No desktop local persistence; drafts vanish on crash. | S | 09 |
| P-050 | Diataxis collapse in `README.md` and `AGENTS.md`. Rewrite README as 5-line tutorial; AGENTS.md as reference. | S | 15 |
| P-051 | Missing docs: backend ops runbook, signer-safety model, threat model, incident playbook, capability cross-links, build-and-release reproducibility guide, testing-strategy doc. **ADR-006 landed.** **Partial resolution (post-2026-05-14):** runbook, threat model, signer-safety exist — see [`operations/runbook.md`](../../operations/runbook.md), [`operations/threat-model.md`](../../operations/threat-model.md), [`specs/signer-safety-model.md`](../../specs/signer-safety-model.md); remainder open in [`action-plan-progress.md`](../action-plan-progress.md). | L | 15 |
| P-052 | `docs/3-stories/` has no DoR/DoD; US-H5 manual fallback unspecified; no cancellation flow; no signer-rotation story. Add 8-item DoR checklist; story-by-story audit. | M | 12, 13, 15 |
| P-053 | Zero user discovery (5–8 signer interviews, digest-verification usability test, manual-fallback tabletop sim). | L | 12 |
| P-054 | Rule/skill stack drifts (`.claude/rules/` vs `.cursor/rules/`; missing `description:` for auto-trigger; `rust-specialist` vs `rust-backend-standards` disagreement on `.unwrap()`). Verify Cursor IDE loading semantics first; consolidate. | S | 17 |
| P-055 | SPS-65 cited as source-of-truth but not in the repo. Archive key SPS-50/51/65 excerpts under `docs/specs/sps-reference/`; add section-id comments in `signing.rs`, `proposals.rs`, `action_codec.rs`. | M | 16 |
| P-056 | ADR-001 crate-pinning rationale incomplete. Expand pro/con + defined "convergence signal" to migrate. | S | 16 |
| P-057 | Vestigial Tauri `custom-protocol` feature flag, deprecated GraphQL in `sprint-board`, unused config fields. | S | 14, 17 |

---

## 4. Cross-cutting themes (do not lose these in the per-ticket churn)

- **"Backend coordination only" is documented in [ADR-006](../../architecture/adrs/006-backend-coordination-boundary.md).** Remaining work: forbidden-import lint, SPS-65 citation in code (P-028, P-055).
- **Single source of truth is broken everywhere.** Authority defined 3×, rules duplicated, constants hardcoded in multiple places. P-022 + P-043 + P-054 + P-064 share a single root cause: no shared types/codegen.
- **In-memory by default leaks across every concern.** Proposals, sessions, challenges, broadcast claims. P-016 + P-017 + P-031 are one architectural decision.
- **No correlation chain frontend → Tauri → backend → on-chain.** P-023 + P-029 + P-051 must ship together for ops to be honest.
- **Signer-safety UX is uncodified.** P-005 + P-006 + P-007 + P-009 + P-010 + P-039 are the codification of "what does the signer trust, and how do they verify it".
- **Testing pyramid is hollow above the unit layer.** P-032 + P-038 are non-optional gates for Tier 0/1 closure.
- **Broadcast boundary aligned with PRD (P-066).** Orchestrator coordinates; desktop broadcasts. P-061 retired. Remaining broadcast work: P-020 idempotency, P-018 resumable FSM, manual-fallback validation (US-H5).
- **Synthesized risk:** once Postgres replaces `RwLock`, every existing race condition (P-019, P-020, P-018) becomes worse because the in-process write lock that accidentally serialized everything is gone.
- **Synthesized risk:** the combination of CSP-null + npm `^`-ranges without a lockfile + no SCA + unsigned releases + IPC-plaintext secrets + drifting rule guidance is a textbook supply-chain attack surface — single most likely path to catastrophic compromise.

---

## 5. Action plan — sequencing

The plan is structured around three sequential "waves." Earlier waves remove existential risks; later waves harden architecture and process. **Tracks A/B/C within a wave can run in parallel.**

### Wave 1 — Stop the bleeding (Weeks 1–2)

**Goal:** Eliminate the new BLOCKERs from 2026-05-14 and the cheapest Tier 0 wins from 2026-05-13. Close ≥10 of 19 BLOCKERs by end of week 2.

| Track | Items | Owner hint |
|---|---|---|
| **A** Broadcast integrity | P-066 (done), P-062, P-035, P-020 | Desktop + BE coordination APIs |
| **B** Backend authz hygiene | P-001, P-002, P-013, P-014, P-015, P-063 | BE |
| **C** Frontend trust boundary | P-004, P-008, P-009, P-010, P-007, P-064 (short-term), label-bug fix | FE + Tauri |
| **D** Ops baseline | P-016 (require `DATABASE_URL` in prod), P-029 (request IDs + `/ready` + tracing skeleton), P-054 (verify Cursor rule semantics, consolidate) | Platform |

**Exit criteria:**
- Desktop executes commit/reveal on the happy path; orchestrator records broadcast coordination state (not required to be up for broadcast to succeed).
- No `"enacted"` literal appears in `BroadcastResultDto`; UI uses persisted proposal fields.
- Strata-Admin token cannot list/get/broadcast Alpen-Admin proposals (integration test passes).
- Desktop Tauri refuses broadcast without valid `OPERATOR_SECRET_KEY_HEX` in process env (not webview); orchestrator refuses prod start without `DATABASE_URL`.
- CSP is set and verified in `tauri.conf.json`.
- Every Tauri IPC return value parsed through Zod; unknown enum variants raise.
- Skeleton `ADR-006: Backend Coordination Boundary` exists even if implementation lags.

#### 5.1 Wave 1 — execution record (2026-05-16)

**Delivered on:** `fix/action-plan-wave1-2026-05-14` → draft PR #134 (`develop` base).  
**Tracker:** [action-plan-progress.md](../action-plan-progress.md).

**How it was run**

- One branch; **one atomic commit per planned P-ID** (plus bootstrap tracker commit).
- **E2E:** single `/e2e-proposal-flow` at branch tip (`a74c817`), not per-commit runs (deviation from ideal gate in execution playbook).
- **Automated checks per commit:** `cargo fmt`, `clippy`, `cargo test` (`orchestrator-be`, `desktop-app`), `npm run build` when FE touched; `npm run test:ipc-schemas` added later.

**Outcome vs exit criteria**

| Criterion | Result |
|-----------|--------|
| Desktop-owned broadcast + coordinator metadata | Met (P-066, post–Wave 1 correction) |
| No hard-coded enacted strings | Met (P-062) |
| Cross-authority 401 | Met (P-002) |
| Prod operator key + DATABASE_URL | Met (P-001, P-016) |
| CSP | Met (P-004); capabilities deferred |
| All IPC via Zod | **Partial** — proposal/broadcast only (P-008) |
| ADR-006 skeleton | **Not done** — carry to Wave 2 Track B |

**Post–Wave 1 discoveries (fixed on same branch)**

1. **P-008 regression:** Tauri serializes `Option::None` as JSON `null`; Zod `.optional()` rejected create/list — fixed in `50d3d51` / `a74c817` (`.nullish()`).
2. **Broadcast confirmations:** `gettransaction` fails for reveal txs from `sendrawtransaction` — fixed in `e6a994d` (`getrawtransaction` verbose for confirmations).
3. **Regtest E2E:** commit/reveal waits need mined blocks during broadcast — `662e517` (`mineWhileWaitingForBroadcastDone` in WDIO + `mine-blocks.sh` env in helper).

**Partial P-IDs (acceptable short-term; full text in Wave 2/3)**

| P-ID | Shipped | Remaining |
|------|---------|-----------|
| P-004 | CSP string | Tauri 2 capabilities (P-040) |
| P-008 | Proposal/broadcast Zod | Auth/wallet IPC schemas |
| P-020 | In-flight guard | `Idempotency-Key` end-to-end |
| P-029 | `/ready` (BTC), tracing on list/get | Request UUID in bridge; Postgres/ASM on `/ready`; all handlers |
| P-063 | Lowercase ingress | DB `CHECK` when Postgres lands |
| P-064 | Tauri 5-variant enum | Shared `multisig-types` crate (Wave 3) |

**Process / tooling gaps**

- `autotest/start-stack.sh` must inject **desktop** broadcast env (`OPERATOR_SECRET_KEY_HEX`, `BITCOIN_RPC_*`, `BITCOIN_NETWORK` in `desktop-app/.env`); orchestrator no longer needs operator key for happy path.
- Extra commits beyond “21 planned”: tracker sync (`4cad4d1`, `4e1b4e7`), prettier chore, three broadcast/E2E fixes above.

#### 5.2 Architectural correction (2026-05-16) — broadcast boundary

Aligned with PRD §1–§2, `docs/2-discovery/01-conceptual-overview.md`, and `docs/architecture/overview.md`:

| Question | Answer |
|----------|--------|
| Who submits commit/reveal txs? | **Desktop Tauri** (signer-configured Bitcoin RPC + operator key in process env). |
| What does the orchestrator do for broadcast? | **Coordination only:** `claim_broadcast`, persist `broadcast_status` / txids via `PATCH`. |
| What happened to P-061 on the branch? | Interim server-execute path **removed**; P-061 ID retired; **P-066** is the authoritative fix. |
| Operator key location | **Desktop only** (`desktop-app/.env` / Tauri env). Orchestrator `.env` no longer requires `OPERATOR_SECRET_KEY_HEX`. |

**Wave 2 should pick up:** ADR-006 skeleton; P-012; P-003; P-005–P-006; P-011; complete P-008/P-029/P-004; E2E test with coordinator stopped after claim (manual-fallback matrix).

### Wave 2 — Correctness, supply chain, operations (Weeks 3–6)

**Goal:** Close the remaining Tier 0 + the highest-leverage Tier 1; deliver a credible supply-chain story; codify signer safety.

| Track | Items | Owner hint |
|---|---|---|
| **A** Secrets off the IPC | P-003 (mnemonics/keys leave IPC; OS keychain; `Zeroize`), P-040 (Tauri capabilities), P-033 (BIP-137 — confirm and fix) | Tauri + BE |
| **B** Coordination boundary | P-012 (decision: remove auto-Approve OR ADR-006 carve-out + threshold-resync test), P-028 (`action_codec` lint), P-025 (mock injection), P-026 (early SSZ validation), P-037 (full 5-authority mapping) | BE |
| **C** Supply chain & release | P-011 MVP (lockfile, `npm ci`, `cargo audit`/`cargo deny` mandatory, secret-scan pre-commit; signed releases for at least one OS) | Platform |
| **D** Correctness & ops | P-017 (TTL + rate limit), P-018 (RPC timeouts + reset endpoint), P-019 (atomic dedup + version), P-023 (typed errors), P-027 (timeouts/retries/backoff), P-029 (full structured logging) | BE + Platform |
| **E** Test floor | P-032 partial (axis-10 inventory: 3 broadcast negatives + 6 IPC contract + 5 concurrent + 6 frontend smoke) | QA + FE + BE |
| **F** Docs & signer safety | P-051 (ops runbook, threat model, signer-safety-model spec), P-006 (on-device verification UX), P-055 (SPS-65 archive + code citations) | Docs + UX + BE |
| **G** Discovery (parallel) | P-053 starts: recruit 5–8 signer interviews, design digest-verification usability test, scope a manual-fallback tabletop | Product |

**Exit criteria:**
- No private key or mnemonic crosses the webview/IPC boundary.
- `cargo deny` / `cargo audit` / `npm audit` block CI on advisories (no "noisy warnings" exception).
- Backend either does not auto-Approve, or ADR-006 documents the carve-out and a threshold-resync test passes.
- All RPC calls (Bitcoin, ASM) wrapped in `tokio::time::timeout` with structured error metadata.
- Every Tier 0 BLOCKER from 2026-05-14 is closed.
- A backend ops runbook, threat model, and signer-safety-model spec exist and are linked from the README.

### Wave 3 — Architectural hardening & governance integrity (Weeks 7–12)

**Goal:** Make the system safe to scale, audit, and rotate. Convert the "synthesized risks" from theory to inert.

| Track | Items | Owner hint |
|---|---|---|
| **A** Shared types & codegen | P-022, P-043, P-064 full fix — shared `multisig-types` crate; codegen TS types from Rust serde; round-trip serde + JSON contract tests | BE + FE |
| **B** Event log & resumable broadcast | P-031 (append-only event log), P-042 (proper aggregates), P-018 full (resumable FSM + Postgres advisory lock) | BE |
| **C** Scale & durability | P-017 full (distributed session store), P-030 (replay-rejection at HTTP layer), P-047 (FSM enforcement, retention, soft delete), P-048 (encryption at rest) | BE + Platform |
| **D** Test pyramid | P-032 full (axis-10 inventory + concurrent-approval + e2e across 5 authorities), P-038 (SPS-65 golden tests), P-021 (`u64` as string + `BigInt`) | QA |
| **E** Release pipeline | P-011 full (Apple Developer + Authenticode + PGP-signed checksum manifest + Tauri updater verification across all 3 OSes; multi-employee signing per PRD NF-3) | Platform |
| **F** Product & docs | P-052 (DoR + story audit + signer-rotation story + offline-fallback walking-skeleton), P-053 (discovery findings inform Slice plan), P-050 + P-051 full (Diataxis rewrite, capability matrix as live doc) | Product + Docs |
| **G** Hygiene | P-036 (centralize constants), P-039 (wallet-address validation), P-044, P-045, P-049, P-056, P-057 | BE + FE |

**Exit criteria:**
- A single shared `Authority`/`Status`/`Error` schema; no enum drift between BE/Tauri/React.
- An append-only event log is the system of record for proposal lifecycle and broadcast.
- Signed releases on all target OSes; multi-employee signing per PRD NF-3.
- US-H5 manual fallback is speced, tested, and documented; signer-rotation story exists.
- All 17 axis findings are either closed or have an open ticket with effort + acceptance criteria.

---

## 6. Decisions still owed (block work in Waves 1–2)

These are not engineering tasks — they are policy/scope decisions that must be made by the right humans before the corresponding tickets can land cleanly:

1. **Threshold-detection policy (P-012).** **Resolved** — explicit approve per ADR-006. Threshold resync on broadcast remains open (P-035).
2. **SPS-65 archival in-repo (P-055).** Are we allowed to ship excerpts of the SPS-65 Notion document under `docs/specs/sps-reference/`? Stakeholder: Alpen legal-of-record.
3. **Cursor IDE rule semantics (P-054).** Confirm whether `.cursor/rules/` is IDE-managed or source-controlled before we delete or consolidate.
4. **Operator-key custody model (P-001, P-003, P-040).** Sidecar daemon, OS keychain, HSM, or hardware-wallet–only? The choice changes Wave-2 implementation across two teams.
5. **Manual fallback scope (P-052, P-053).** **Partial** — `/manual` and [manual-execution-flow.md](../../specs/manual-execution-flow.md) shipped; export/reconcile in [deferred-backlog.md](../deferred-backlog.md) US-H5.

---

## 7. Confidence and adversarial caveats

- **Code-read severity, not measured.** None of the disconfirming probes proposed by individual axes were executed. If production deploy scripts already enforce env-var presence and reject the test key, several Tier 0 items downgrade.
- **"Coordination only" reading (P-055) depends on SPS-65 interpretation we cannot verify locally.**
- **`u64` precision (P-021) and BIP-137 (P-033) are real but may not be hit in practice today.** Confirm before allocating Wave-2 effort.
- **Race conditions (P-019, P-020, P-018) are theoretical until a load test confirms them.** Current Tokio + in-process `RwLock` may serialize accidentally; Postgres path may already use `SELECT FOR UPDATE` in places not re-read this sprint.
- **The 17 axes overlap heavily.** This document merges duplicates; raw axis-finding counts overstate scope.

---

## 8. Quick reference — minimal "first PR" for each track

If we can ship only the smallest credible change per track in Wave 1, ship these:

- **A (broadcast integrity):** P-066 (landed) + P-062 + P-020. Spec: `docs/specs/proposal-broadcast-commit-reveal.md`. APIs: `POST …/broadcast/claim`, `PATCH …/broadcast` only on orchestrator.
- **B (backend authz):** P-002. One PR: add `authority: Option<Authority>` to `list_by_status` and `find_by_action_id`; `get_proposal` returns 401 on mismatch; integration test for cross-authority denial.
- **C (frontend trust):** P-008. One PR: Zod schemas at `tauriCall<T>`; failing schema raises a typed error surfaced to the UI.
- **D (ops baseline):** P-029 skeleton. One PR: `#[tracing::instrument]` on every handler with `action_id`/`authority`/`seq_no`; `/ready` checks Postgres + RPC URLs.

These four PRs alone close 4 of 19 BLOCKERs in roughly 5 engineering days and unblock the rest of Wave 1.

---

## 9. Closure note (2026-06)

Material decisions from this plan landed in [ADR-006](../../architecture/adrs/006-backend-coordination-boundary.md), Wave 2 execution tracks ([wave2-exit-gap-review.md](wave2-exit-gap-review.md)), and ongoing backlog ([deferred-backlog.md](../deferred-backlog.md)). May 2025 adversarial assessment folders were removed when stale; use the links above for current status.

---

*End of consolidated action plan. Status tracking: [action-plan-progress.md](../action-plan-progress.md). Coordination boundary: [ADR-006](../../architecture/adrs/006-backend-coordination-boundary.md).*
