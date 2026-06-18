# Comprehensive Codebase Review — Alpen Multisig

> **Resolution index (2026-06):** Point-in-time audit on `develop` (2026-05-22). Open items are tracked in [`deferred-backlog.md`](../assessment/deferred-backlog.md). Broadcast boundary: [ADR-006](../architecture/adrs/006-backend-coordination-boundary.md). Do not treat severity tables below as current blockers without checking backlog closure.

> **Date**: 2026-05-22  
> **Branch**: `develop`  
> **Scope**: Full codebase — Rust backend (`orchestrator-be`), Tauri shell (`desktop-app/src-tauri`), React frontend (`desktop-app/src`)  
> **Business source of truth**: `docs/0-prd/01-multisig-ui.md`, `docs/0-prd/02-multisig-backend.md`, SPS-50/51/65  

---

## Table of Contents

1. [Rust Code Audit](#1-rust-code-audit)
2. [React / TypeScript Audit](#2-react--typescript-audit)
3. [Spec Compliance Audit](#3-spec-compliance-audit)
4. [Open Questions — BLOCKED: business undefined](#4-open-questions)
5. [Final Recommendation](#5-final-recommendation)

---

## 1. Rust Code Audit

### ~~HIGH — Missing ASM role mapping for 3 of 5 authorities~~ — DEFERRED (intentional)

`AlpenAdmin`, `SecurityCouncil`, and `PayoutAdmin` are not yet wired to ASM (`authority_to_role_impl` returns `Unsupported` for them). This is **intentional** for the current scope — upstream crates don't expose these role constants yet. Not a finding to address now.

**Action when scope expands**: Wire remaining roles, and change the HTTP response from 400 to 503 to make the "not yet available" state operationally distinguishable from an auth error.

---

### HIGH — `report_broadcast_progress` allows arbitrary status regression without state machine validation

**Evidence**: `orchestrator-be/src/application/proposals.rs:381-382`

```rust
Some("canceled") => Some(ProposalStatus::Canceled),
Some("expired")  => Some(ProposalStatus::Expired),
```

Any authenticated signer in the authority can call `PATCH /proposals/:id/broadcast` with `proposal_status: "canceled"` or `proposal_status: "expired"` on **any** proposal — including ones already in `Enacted` state. There is no guard on the current state before applying the transition.

**PRD requirement**: PRD §12.1.3 — "Canceled updates MUST be kept offchain and accessible/visible only to multisig signers." There is no PRD grant for arbitrary signer-triggered cancellation via broadcast progress reporting.  
**Risk**: Any signer can corrupt a finalized (`Enacted`) proposal to `Canceled` with no audit trail. This is especially dangerous given the lack of a cancel endpoint with proper lifecycle guards.  
**Fix**: Add a state machine guard — `Canceled` and `Expired` transitions must assert the current status is `Pending` (for `Expired`) or `Approved` (for `Canceled`). The cancel path should go through a dedicated, scoped endpoint.  
**Missing test**: Test that a signer cannot cancel an already-enacted proposal via broadcast progress.

---

### ~~HIGH — Wrong derivation path in mnemonic signer (`m/84'` instead of `m/86'`)~~ — RESOLVED (PRD updated)

The PRD was updated after the initial audit. The correct derivation path is now `m/84'/0'/73'/0/n` (BIP84, P2WPKH), which matches the implementation in `signing.rs:119`. This is no longer a finding.

---

### MEDIUM — Cross-authority existence leak via `Unauthorized` vs `NotFound`

**Evidence**: `orchestrator-be/src/application/proposals.rs:85-87`

```rust
if proposal.authority != session.authority {
    return Err(AppError::Unauthorized);
}
```

When a `StrataAdmin` signer looks up an `AlpenAdmin` proposal by its `action_id`, the backend returns `401 Unauthorized` instead of `404 Not Found`. This reveals that the `action_id` belongs to a different authority.

**PRD requirement**: PRD §02 §3.3 — "A non-signer MUST NOT be able to view any pending proposals **or infer the existence** of pending proposals."  
**Risk**: A signer on one authority can brute-force action IDs and learn about proposals belonging to other authorities.  
**Fix**: Return `AppError::NotFound` for cross-authority access in `require_proposal_authority`.  
**Missing test**: Test that a cross-authority lookup returns 404, not 401.

---

### MEDIUM — Pending proposals never auto-expire (no 7-day TTL enforcement)

**Evidence**: `ProposalStatus::Expired` exists in the domain model, but there is no background task, cron job, or request-time sweep that transitions `Pending` proposals to `Expired` after 7 days. There is no `created_at` timestamp on `Proposal` exposed to the frontend either.

**PRD requirement**: PRD §13.3 — "A 'Pending' update MUST expire if it has not been approved within 7 days."  
**Risk**: Stale proposals accumulate indefinitely; signers cannot distinguish live from expired proposals; coordination is ambiguous.  
**Fix**: Add `created_at` column to proposals; implement a periodic reconciliation task (background thread or scheduled job) that marks proposals `Expired` when `now() - created_at > 7 days`.  
**Missing test**: Test that a proposal with `created_at` > 7 days ago is returned as `Expired`.

---

### MEDIUM — No signature format/shape hygiene in `approve_action`

**Evidence**: `orchestrator-be/src/handlers/proposals.rs:141-145`

```rust
let sig = ProposalSignature {
    signer_pubkey: body.signer_pubkey,
    signature_hex: body.signature_hex,
};
```

Any string is accepted as `signature_hex`. The backend validates hex decodability for `action_hex` (via `action_codec`) but not for signature payloads.

**PRD requirement**: PRD §02 §1.5 — "The backend MAY perform basic hygiene checks (e.g., malformed signatures, duplicate signatures, structural validation)."  
**Risk**: Garbage signatures can be stored and will permanently block a proposal from working when an operator tries to broadcast — the on-chain validator will reject the payload, but the backend will have already consumed the signer's slot.  
**Fix**: Verify that `signature_hex` decodes to exactly 64 bytes (compact ECDSA) and that the ECDSA signature is well-formed before storing.  
**Missing test**: Test that submitting a malformed `signature_hex` returns 400.

---

### MEDIUM — In-memory session and challenge maps grow without eviction

**Evidence**: `orchestrator-be/src/state.rs:11-12`

```rust
pub challenges: Arc<RwLock<HashMap<String, PendingAuthChallenge>>>,
pub sessions: Arc<RwLock<HashMap<String, AuthSession>>>,
```

Expired challenges and sessions accumulate in memory until process restart. Under sustained load (many auth attempts), this is a memory leak and potential DoS surface.

**PRD requirement**: PRD §02 §3 — "Bounded validity (e.g., expiration or revocation capability)."  
**Risk**: Memory growth under attack; expired tokens remain revocable only by restart.  
**Fix**: Sweep expired entries on each write (or use a LRU map with TTL, e.g., `moka` crate).  
**Missing test**: Test that expired challenge IDs are rejected with 401 and are not retained in memory.

---

### MEDIUM — Removed onchain signer retains valid session until it expires

**Evidence**: `orchestrator-be/src/handlers/auth_session.rs:39-40`

```rust
let session = sessions.get(&token).ok_or(AppError::Unauthorized)?;
if now > session.expires_at_unix_ms {
    return Err(AppError::Unauthorized);
}
```

ASM membership is checked once at `auth_verify` and never again. A signer removed from the canonical signer set onchain can continue making authenticated requests until their session token expires.

**PRD requirement**: PRD §02 §3.6.2 — "Any session authorization MUST reflect the canonical signer set at the time of authorization."  
**Risk**: A freshly removed signer can still submit approval signatures or create proposals in the window between removal and token expiration.  
**Fix**: Either shorten session TTL aggressively (e.g., 15 minutes), or re-check ASM membership on write operations (`create_update_action`, `approve_action`).  
**Missing test**: Integration test simulating a signer removed from ASM mid-session still being rejected.

---

### LOW — `verify_threshold` can double-count a single signature

**Evidence**: `desktop-app/src-tauri/src/infrastructure/signing.rs:191-207`

```rust
for sig_hex in signatures_hex {
    // ...
    for pk_hex in public_keys_hex {
        if SECP256K1.verify_ecdsa(&msg, &sig, &pk).is_ok() {
            valid_count += 1;
            break;
        }
    }
}
```

If the same signature hex appears twice in `signatures_hex` (duplicated by caller), `valid_count` is incremented twice. Combined with `threshold=1`, this would return `valid: true` for a single-signer submission with a duplicated entry.

**Risk**: Low — the backend deduplicates signers before this function is called; but the client-side `verify_threshold` utility gives false confidence in invalid inputs.  
**Fix**: Deduplicate `signatures_hex` entries before counting, or track which public keys have already been matched.  
**Missing test**: Test that passing the same signature twice does not increase `valid_count` above 1.

---

### LOW — `add_signature` in `PostgresProposalRepository` is non-atomic read-after-write

**Evidence**: `orchestrator-be/src/infrastructure/postgres_repo.rs:255-259`

```rust
tx.commit().await?;
self.find_by_action_id(action_id).await  // separate, non-transactional read
```

After committing the signature insert, the returned proposal is loaded in a separate query. A concurrent signature from another signer can appear in or disappear from the returned snapshot.

**Risk**: Low — the returned snapshot is informational only; no business logic depends on the exact post-insert count.  
**Fix**: Use `RETURNING *` in the signature insert and load signatures within the same transaction for a consistent snapshot.

---

### LOW — `report_broadcast_progress` allows downgrading `broadcast_status`

**Evidence**: `orchestrator-be/src/application/proposals.rs:346-398` — no guard prevents setting `broadcast_status` to `idle` after `reveal_confirmed`.  

**Risk**: Low — only authenticated signers within the authority can do this; but it could cause another signer to re-claim broadcast and double-broadcast.  
**Fix**: Add monotonic progression validation for `broadcast_status` (allow only forward transitions).

---

## 2. React / TypeScript Audit

### HIGH — `inferProposalType` is always wrong for non-sequencer proposals

**Evidence**: `desktop-app/src/domain/proposals-dashboard/components/proposals-dashboard.tsx:377-385`

```typescript
function inferProposalType(proposal: Proposal): string {
  if (proposal.authority.toLowerCase().includes('sequencer')) {
    return 'Sequencer update'
  }
  if (proposal.actionHex.toLowerCase().startsWith('0x01')) {
    return 'Verification key update'
  }
  return 'Signer update'
}
```

`actionHex` is a raw hex string (no `0x` prefix). The condition `startsWith('0x01')` can never match. All non-sequencer proposals are labeled **"Signer update"** regardless of their actual content.

**PRD requirement**: PRD §6.6 — signers MUST be able to "clearly read and understand each message they are signing."  
**Risk**: A signer could approve a Verification Key update believing they are approving a Signer update — a critical mis-signing scenario.  
**Fix**: Decode the `action_hex` using the Tauri `decode_action` command and use the returned `kind` field for labeling. The correct data is already available in `proposal-detail.tsx` via `useDecodedProposal` — the same pattern should be used in the card.  
**Missing test**: Unit test asserting that VK update proposals receive the correct label.

---

### HIGH — `buildActionHex` hardcodes `role: 'strata_admin'` regardless of session authority

**Evidence**: `desktop-app/src/domain/create-proposal/hooks/use-create-proposal.ts:64`

```typescript
const hexResult = await buildAdminMultisigUpdateHex({
  role: 'strata_admin',
  // ...
})
```

`selectedRole` from `useSession()` is ignored when building the action payload. An `AlpenAdmin` signer creating a proposal will encode a `StrataAdminMultisigUpdate` action, which will be rejected onchain by the wrong authority scope.

**PRD requirement**: PRD §15.1 — "Alpen Administrator multisig: Alpen verification key update, Alpen Administrator Signer update."  
**Risk**: Proposals created by non-StrataAdmin signers would embed the wrong authority in the SSZ payload, causing permanent on-chain rejection.  
**Fix**: Map `selectedRole` to the correct `role` string before calling `buildAdminMultisigUpdateHex`.  
**Missing test**: Test that `buildActionHex` uses the session role, not a hardcoded string.

---

### HIGH — `seqNo` typed as JS `number` — precision loss for large `u64` values

**Evidence**: `desktop-app/src/api/ipc-schemas.ts:18`

```typescript
seqNo: z.number(),
```

JavaScript `number` is a 64-bit float, safely representing integers only up to 2^53 − 1 ≈ 9 × 10^15. The protocol defines `SeqNo` as `u64` (max ~1.8 × 10^19). Values exceeding the safe integer range are silently rounded.

**PRD requirement**: PRD §02 §4 — "`SeqNo` MUST be a 64-bit unsigned integer (`u64`)."  
**Risk**: Low probability in practice, but a violation of the protocol type contract. If the chain ever reaches a high seq number, proposal IDs would collide silently.  
**Fix**: Change `z.number()` to `z.string().transform(BigInt)` or keep as string and parse at use sites.  
**Missing test**: Add a boundary test for seq number at `Number.MAX_SAFE_INTEGER + 1`.

---

### MEDIUM — No expiry countdown displayed for pending proposals

**Evidence**: Neither `proposals-dashboard.tsx` nor `proposal-detail.tsx` shows time remaining before expiry. The `Proposal` type has no `createdAt` or `expiresAt` field.

**PRD requirement**: PRD §13 — "the user MUST be able to see ... how much time is left before the 'Pending' update expires."  
**Risk**: Signers cannot prioritize time-sensitive proposals; coordination fails silently.  
**Fix**: Add `createdAt` (ISO timestamp) to the backend `Proposal` struct and DB schema. Surface `expiresAt = createdAt + 7 days` in the card and detail view.  
**Missing test**: UI test asserting the expiry countdown renders for pending proposals.

---

### MEDIUM — `useCreateProposal` silently discards `getNextSeqNo` errors

**Evidence**: `desktop-app/src/domain/create-proposal/hooks/use-create-proposal.ts:100-106`

```typescript
getNextSeqNo({ baseUrl: ORCHESTRATOR_BASE_URL }).then((result) => {
  if (cancelled) return
  setIsLoadingSeqNo(false)
  if (result.ok) setNextSeqNo(result.data)
  // Error case: nextSeqNo stays null, no error surfaced
})
```

When the orchestrator is unreachable, `nextSeqNo` is silently `null` with no error shown. The user must manually enter the correct sequence number without guidance.

**PRD requirement**: PRD §02 §2 — backend downtime must not block signers from executing updates.  
**Risk**: Signers may use the wrong seq number, creating invalid proposals.  
**Fix**: Surface an error message when `getNextSeqNo` fails; display the last known seq number from ASM state as a fallback.  
**Missing test**: Test that a network error renders an error state and does not leave `isLoadingSeqNo: true`.

---

### MEDIUM — `connectSession` leaves half-created auth state on step-2 failure

**Evidence**: `desktop-app/src/contexts/session-provider.tsx:70-97`

```typescript
const connectSession = useCallback(async () => {
  try {
    await authenticate(...)          // Step 1: Tauri auth
    // ...
    await orchestratorAuthComplete(...)  // Step 2: Orchestrator auth
  } finally {
    setSigningStep(null)
  }
}, [adapter, authenticate, selectedRole])
```

If step 1 succeeds but step 2 fails (e.g., orchestrator unreachable), the Tauri session is authenticated but the orchestrator session does not exist. Subsequent calls through `ensureOrchestratorSession` would attempt to re-authenticate the orchestrator, potentially causing a second hardware wallet prompt without user context.

**PRD requirement**: PRD §3 — "Bounded validity" and "Explicit scoping to a single multisig authority."  
**Risk**: User confusion on step-2 failure; double signing prompts on retry.  
**Fix**: Roll back the Tauri session on step-2 failure (`await logout()` in the catch block), or refactor to a single atomic flow.  
**Missing test**: Test that step-2 failure triggers a clean rollback of step-1 auth state.

---

### MEDIUM — `proposal-detail.tsx` subtitle hardcodes "Signer update" for all proposals

**Evidence**: `desktop-app/src/domain/proposal-detail/components/proposal-detail.tsx:115`

```tsx
<p className="m-0 mt-1 text-[13px] text-[#6b7280]">
  #{proposal.seqNo} · Signer update · {proposal.authority}
</p>
```

The subtitle always reads "Signer update" regardless of the decoded action type. Same root cause as the `inferProposalType` issue in the dashboard, but here the decoded data is already available via `decodedData`.

**Risk**: Same mis-signing risk as dashboard finding.  
**Fix**: Replace the hardcoded string with the decoded action's type label from `decodedData`.

---

### LOW — No test coverage for critical hooks

No test files exist for `useHwWalletConnect`, `useCreateProposal`, `useProposalDetail`, `useSession`, or `useAuthSession`. Only `wallet-binding.test.ts` and `ipc-schemas.test.ts` exist.

**Risk**: Regressions in wallet connect and proposal flows are undetected until E2E tests or manual QA.  
**Fix**: Add hook tests using `@testing-library/react` for the critical user journeys (wallet connect → auth → create → sign → broadcast).  
**Missing test**: At minimum, unit tests for `useCreateProposal.submitCreateProposal` and `useHwWalletConnect.connect`.

---

### LOW — `verifyOnDevice` shows incorrect confirmation message

**Evidence**: `desktop-app/src/domain/connect-wallet/hooks/use-hw-wallet-connect.ts:145`

```typescript
setVerifyMessage('Path/public key confirmed on device.')
```

The hardware wallet displays and confirms the **address**, not the public key. The message text is technically incorrect and could mislead users about what was verified.

**Fix**: Change to `'Address confirmed on device.'`

---

## 3. Spec Compliance Audit

### Compliance Matrix

| # | Requirement | PRD Source | Code Evidence | Test Evidence | Status | Risk |
|---|---|---|---|---|---|---|
| 1 | Auth: proof-of-possession, authority-scoped, bounded validity | PRD §02 §3 | `auth_verify` ECDSA + ASM; `expires_at_unix_ms` checked | `auth_crypto` unit tests | PARTIAL | Sessions not evicted; no re-verification mid-session |
| 2 | All 5 multisig authorities supported | PRD §7 | `Authority` enum complete; ASM mapping only 2/5 — remaining 3 deferred intentionally | `all_five_authorities_have_explicit_asm_mapping_status` | DEFERRED | Out of current scope; upstream crates don't expose remaining role constants yet |
| 3 | No cross-authority proposal visibility | PRD §02 §3.3–3.4 | `require_proposal_authority` returns `Unauthorized` | `test_get_update_action_rejects_wrong_authority` | PARTIAL | Returns `Unauthorized` not `NotFound` — leaks existence |
| 4 | `ActionId = hash(MultisigAction, SeqNo)`; idempotent | PRD §02 §4 | `compute_action_id = sha256(seqno_be \|\| action_bytes)` | `test_create_duplicate_action_rejected`, `test_action_id_is_deterministic` | **PASS** | — |
| 5 | `SeqNo` MUST be `u64` | PRD §02 §4 | Rust: `type SeqNo = u64`; Frontend: `z.number()` | None for u64 boundary | PARTIAL | JS precision loss above 2^53 |
| 6 | Multiple proposals at same SeqNo supported | PRD §02 §4 | `ActionId` based on content hash; different actions → different IDs | Implicit | **PASS** | — |
| 7 | No strict ordering enforced between seq numbers | PRD §02 §4 | No ordering guard in create/approve | Implicit | **PASS** | — |
| 8 | Pending proposals expire after 7 days | PRD §13.3 | `ProposalStatus::Expired` exists; no background expiry task | None | **FAIL** | Proposals never auto-expire |
| 9 | Expired proposals kept offchain, visible to signers | PRD §13.3.1 | Status exists; not enforced (see #8) | None | PARTIAL | Depends on #8 |
| 10 | Show time remaining for pending proposals | PRD §13 | No `created_at`/`expires_at`; no countdown in UI | None | **FAIL** | Signers cannot see expiry |
| 11 | Signers can approve and copy approval signatures | PRD §13.2.1 | `approve_action` endpoint; `CopyButton` in detail view | `test_approve_action` | **PASS** | — |
| 12 | Broadcast or copy raw tx for approval | PRD §13.2.1.2 | Commit/reveal broadcast implemented; no raw tx export | `test_claim_broadcast_coordination` | PARTIAL | No manual fallback raw tx copy |
| 13 | Quorum-reaching signer offered to broadcast | PRD §13.2.1.3 | Broadcast button shown after quorum | `test_approve_at_quorum_calls_transition` | **PASS** | — |
| 14 | Manual fallback when backend unavailable | PRD §02 §2.3 | Copy signatures available; signing requires orchestrator session | No offline test | PARTIAL | Offline aggregation undocumented and untested |
| 15 | Approved updates can be canceled | PRD §12.1 | Implemented in separate branch | — | DEFERRED | — |
| 16 | Cancellation signature count shown | PRD §12 | Implemented in separate branch | — | DEFERRED | Depends on #15 |
| 17 | All Strata Admin proposal types | PRD §15.2 | Only signer_update wired; VK update throws "not supported" | None | **FAIL** | Safe Harbor, VK, Operator, Bridge actions missing |
| 18 | Sequencer Manager proposal types | PRD §15.3 | Only signer_update; sequencer update absent | None | **FAIL** | — |
| 19 | Security Council Defcon 1 + 3 | PRD §15.4 | Not implemented | None | **FAIL** | — |
| 20 | Payout Administrator full flow | PRD §16–20 | Not implemented | None | **FAIL** | Entire feature slice missing |
| 21 | Backend as offchain coordinator only | PRD §02 §1 | Hygiene only; no threshold/seqno enforcement | Comments in code | **PASS** | — |
| 22 | Session scoped to exactly one authority | PRD §02 §3.1 | `AuthSession.authority` enforced per request | `test_get_update_action_rejects_wrong_authority` | **PASS** | — |
| 23 | Session validity time-bounded | PRD §02 §3 | `expires_at_unix_ms` checked; no eviction | No expiry-rejection integration test | PARTIAL | Expired tokens linger in memory |
| 24 | Onchain signer set changes reflected in access control | PRD §02 §3.6 | ASM checked at login only | None | PARTIAL | Removed signer retains access until token expires |
| 25 | Derivation path `m/84'/0'/73'/0/n` (P2WPKH) | PRD §6.2 (updated) | `m/84'/0'/73'/0/{n}` — matches PRD update | None | **PASS** | — |
| 26 | User can view address on HW device screen | PRD §6.5 | `verify_address_on_device` Tauri command | None | **PASS** | — |
| 27 | User can read signing message on HW screen | PRD §6.6 | `render_signing_message` + `SignProposalView` shows sighash | `test_mnemonic_signature_verifies_against_raw_sighash` | **PASS** | — |
| 28 | All "Past" updates visible (enacted, canceled, expired) | PRD §14 | Dashboard groups all statuses | None | PARTIAL | Canceled proposals never created; enacted may not reconcile |
| 29 | Signature count shown for pending proposals | PRD §13 | `collectedSignatures / requiredSignatures` shown | None for UI | **PASS** | — |
| 30 | Manual fee rate input (increments of 0.1 sat/vB) | PRD §13.2.1.3.1 | Fee estimated automatically; **no manual input** | None | **FAIL** | User cannot override fee rate |

---

### Summary by Status

| Status | Count |
|---|---|
| PASS | 12 |
| PARTIAL | 8 |
| FAIL | 7 |
| DEFERRED | 3 |
| BLOCKED: business undefined | 3 (see §4) |

---

## 4. Open Questions

**B1 — Expiry behavior: payout vs admin proposals**  
PRD §13.3.1 says expired admin proposals are "kept offchain and accessible." PRD §17.4.1 says expired payout transactions are "deleted from the backend." Is backend deletion restricted to payouts, or does it apply to all expired proposals?  
- **Option A**: Delete only payout expired transactions (most likely intent).  
- **Option B**: Delete all expired proposals.  
- **Impact**: Backend data retention model and `reconcile_enacted_for_authority` sweep logic.

**B2 — Cancel authorization: any signer or original proposer?**  
PRD §12.1 — "The user MUST be able to cancel any 'Approved' update." Does "the user" mean any authenticated signer in the authority, or only the signer who proposed the update?  
- **Option A**: Any signer (permissive — coordination is among trusted signers).  
- **Option B**: Only the proposer (restrictive — requires tracking original proposer).  
- **Impact**: Cancel endpoint authorization model.

**B3 — Broadcast prompt UX: immediate vs deferred**  
PRD §13.2.1.3 says the quorum-reaching signer "SHOULD be given the option" of broadcasting. Does "given the option" mean an in-flow prompt immediately after the quorum-reaching approve, or is it sufficient to show the Broadcast button on the dashboard?  
- **Option A**: Modal/prompt immediately after approve at quorum.  
- **Option B**: Dashboard button is sufficient.  
- **Impact**: `approve_action` hook flow in the frontend.

---

## 5. Final Recommendation

### NO-GO — with explicit unblocking conditions

The core proposal coordination flow (create → sign → approve → broadcast) is architecturally sound and functionally correct for `StrataAdmin` and `SequencerManager`. Test coverage for domain invariants is solid. The foundational choices — deterministic `ActionId`, authority-scoped sessions, offchain coordinator pattern, desktop-owned Bitcoin broadcast — are well-executed.

However, the following items block production readiness:

| Priority | Finding | Blocking? |
|---|---|---|
| P1 | Pending proposals never auto-expire (7-day TTL not enforced) | Yes |
| P1 | Cross-authority `Unauthorized` leaks proposal existence | Yes |
| P1 | `inferProposalType` always wrong — signers may mis-identify what they sign | Yes |
| P2 | `report_broadcast_progress` allows backward status regression on any proposal | Conditional |
| P2 | Session/challenge map memory leak | Conditional |
| P3 | All remaining PRD FAIL items (other action types, payout flow, fee rate input) | Scope-dependent |
| DEFERRED | 3/5 authorities (AlpenAdmin, SecurityCouncil, PayoutAdmin) — out of current scope | No |

**Conditions for GO on current scope (StrataAdmin + SequencerManager only)**:

1. Fix derivation path (`m/86'` + P2TR).
2. Guard `report_broadcast_progress` state transitions.
3. Return `NotFound` for cross-authority lookups.
4. Enforce expiry or at minimum add `created_at` and surface it in the UI.
5. Fix `inferProposalType` to use decoded action data.

All P1 items must be resolved before expanding scope to the remaining multisigs.

---

## Resolution status (2026-06)

| Area | Status | Where tracked |
|------|--------|---------------|
| P1 expiry / cross-authority / inferProposalType | **Open** | [`deferred-backlog.md`](../assessment/deferred-backlog.md), [`proposal-lifecycle-expiry-and-status-completion.md`](../specs/proposal-lifecycle-expiry-and-status-completion.md) |
| P2 broadcast regression / session leak | **Partial / open** | Backlog + follow-up specs |
| P3 remaining PRD gaps | **Scope-dependent** | [`admin-wallet-prd-compliance.md`](../specs/admin-wallet-prd-compliance.md) |
| 3/5 authorities unwired | **DEFERRED** (intentional) | [`2-discovery/08-alpen-crate-prd-coverage.md`](../2-discovery/08-alpen-crate-prd-coverage.md) |
| Core StrataAdmin + SequencerManager flow | **Sound** (review conclusion stands) | This document §5 |
