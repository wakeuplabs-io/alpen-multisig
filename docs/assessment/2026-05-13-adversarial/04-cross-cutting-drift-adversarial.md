# Cross-cutting Drift (Rust↔TS) — Adversarial Assessment

## Scope & Threat Model

**What we're trying to break:**
- Type contracts between Rust backend (orchestrator-be HTTP + desktop-app/src-tauri IPC) and TypeScript frontend (`desktop-app/src/types/`, `api/tauri-bridge.ts`, `api/signing.ts`)
- Error semantics and UX consistency across layer boundaries
- Serialization alignment (camelCase vs snake_case, u64/bigint handling, hex vs base64)
- Domain model duplication (Authority, Proposal, ProposalSignature modeled in multiple places)
- Idempotency and replay safety in concurrent retry scenarios
- Silent divergence under API evolution: what happens when backend v2 ships with desktop v1?
- Signer safety signals: high-signal errors lost in translation across boundaries

**Attack surface:**
1. A signer submits a valid proposal twice (network retry).
2. Frontend receives backend HTTP error but renders wrong UX due to error transformation loss.
3. Orchestrator status field value (Rust enum) doesn't round-trip through JSON (TS string union).
4. Authority names mismatch in Tauri IPC vs HTTP (backend uses `snake_case`, Tauri commands use `camelCase`).
5. `seq_no: u64` in Rust becomes `seqNo: number` in TS; JavaScript `Number` loses precision at 2^53.
6. Retried broadcast operation executes twice if idempotency key isn't propagated correctly.

---

## Top Findings (Ranked) — Blocking/High | Medium | Low

### 🔴 BLOCKER-1: Type Contract Asymmetry — Authority Serialization Drift

**Issue:** `Authority` enum is modeled identically in both `orchestrator-be` and `desktop-app/src-tauri`, both use `snake_case` serialization for wire format (`"strata_admin"`). But the desktop Tauri commands, handlers, and React frontend diverge:

- **Rust backend** (`orchestrator-be/src/domain/authority.rs:5`): `#[serde(rename_all = "snake_case")]`  
  - Serializes Authority as `"strata_admin"`, `"alpen_admin"`, etc.
- **Tauri desktop domain** (`desktop-app/src-tauri/src/domain/authority.rs:7`): `#[serde(rename_all = "snake_case")]`  
  - Claims to match orchestrator wire format (comment: "snake_case strings (`"strata_admin"`) to match the orchestrator HTTP contract")
- **Tauri commands** (`desktop-app/src-tauri/src/commands/proposals.rs` + `orchestrator_auth.rs`): `#[serde(rename_all = "camelCase")]`  
  - These convert to/from frontend, BUT no explicit round-trip tests exist
- **React frontend** (`desktop-app/src/types/auth-role.ts`): `export enum AuthRole { StrataAdministrator = 'strata_administrator', ... }`  
  - Uses different casing entirely (`StrataAdministrator` not `StrataAdmin`)

**Risk:** Frontend sends `strata_administrator` but backend expects `strata_admin`. Deserialize fails silently or triggers 400 BadRequest. If error is swallowed, signer sees no action at all. Meanwhile, in dev/test, mocked data doesn't catch this—real integration tests need explicit Authority round-trip validation.

**HYPOTHESIS:** Desktop Tauri commands use camelCase internally but the frontend enum names don't match the wire strings the backend expects. No schema validation or runtime contract test prevents divergence.

**Evidence:**
- `orchestrator-be/src/domain/authority.rs:5` — `snake_case`
- `desktop-app/src-tauri/src/domain/authority.rs:7` — `snake_case` (aligns with backend)
- `desktop-app/src-tauri/src/commands/proposals.rs:11,21,…` — `camelCase` (DIVERGES for serialization to React)
- `desktop-app/src/types/auth-role.ts:1-4` — React enum values mismatch (`StrataAdministrator` vs `strata_admin`)

**Smallest Fix:** Add exhaustive round-trip serde test in Tauri:
```rust
#[test]
fn test_authority_serde_from_backend() {
    let json = r#"{"authority":"strata_admin"}"#;
    let parsed: Proposal = serde_json::from_str(json).expect("backend JSON parses");
    assert_eq!(parsed.authority, Authority::StrataAdmin);
}
```

**Largest Bet:** Codegen Authority type from shared schema; validate all serialization paths (backend → HTTP → Tauri → React) in E2E test.

---

### 🔴 BLOCKER-2: ProposalStatus + BroadcastStatus as Opaque Strings in TypeScript

**Issue:** Proposal lifecycle states are Rust enums with deterministic serialization but arrive at React as untyped strings. Two separate state machines collide:

- **Rust backend** (`orchestrator-be/src/domain/proposal.rs:60-73`): Enum `ProposalStatus` with 5 variants (`Pending`, `Approved`, `Enacted`, `Canceled`, `Expired`) — serialized as snake_case strings.
- **Rust backend** (`orchestrator-be/src/domain/proposal.rs:18-26`): Enum `BroadcastStatus` with 6 variants (`Idle`, `CommitBroadcasted`, …) — also snake_case.
- **Desktop Tauri domain** (`desktop-app/src-tauri/src/domain/proposal.rs:8-21`): Proposal struct deserializes both as `status: String` and `broadcast_status: String` (no validation).
- **React frontend** (`desktop-app/src/…`): No `ProposalStatus` type; uses loose string comparison (`proposal.status === 'pending'` or the `string` type).

**Risk:** 
1. Backend pushes new status (e.g., `"pending_expired"` for future feature) → React doesn't know it exists → renders as fallback/unknown.
2. Reverse: if a frontend/Tauri layer needs to update status client-side (cache refresh), it must hardcode strings. Typo in a string literal is a silent bug.
3. Broadcast status transitions can race: if two clients call `broadcast_commit_then_reveal` concurrently, one claims `Idle→CommitBroadcasted`, the other doesn't see the state change and duplicates the broadcast.

**HYPOTHESIS:** React never validates status against a known enum; any unrecognized status silently fails form rendering or action routing.

**Evidence:**
- `orchestrator-be/src/domain/proposal.rs:62`: `#[serde(rename_all = "snake_case")]` + 5 enum variants.
- `orchestrator-be/src/domain/proposal.rs:18`: `#[serde(rename_all = "snake_case")]` + 6 broadcast status variants.
- `desktop-app/src-tauri/src/domain/proposal.rs:13,17`: `status: String` and `broadcast_status: String` — no validation.
- `desktop-app/src/screens/sign-poc-screen.tsx:62`: String comparison but no enum type guard.

**Smallest Fix:** Create TypeScript `const` enum or branded string type in `desktop-app/src/types/`:
```typescript
export const ProposalStatus = {
  Pending: 'pending',
  Approved: 'approved',
  Enacted: 'enacted',
  Canceled: 'canceled',
  Expired: 'expired',
} as const;
export type ProposalStatus = (typeof ProposalStatus)[keyof typeof ProposalStatus];
```

**Largest Bet:** Add runtime validation in Tauri layer; reject unknown statuses before sending to React.

---

### 🔴 BLOCKER-3: Error Model Divergence → Silent Failures in React

**Issue:** Backend error handling (Rust `thiserror` + Axum HTTP mapping) and desktop Tauri error handling (Rust `thiserror` + IPC serialization) diverge with no shared schema:

- **Backend** (`orchestrator-be/src/error.rs:10-45`): `AppError` enum (Unauthorized, NotFound, BadRequest, Conflict, Internal) → HTTP status + JSON error message. Example: `Conflict("signer already signed")` → 409 with body `{"error":"conflict: signer already signed"}`.
- **Tauri desktop** (`desktop-app/src-tauri/src/application/orchestrator_client.rs`): `OrchestratorError` enum (Request, Backend {status, message}, Deserialization) — catches HTTP errors and re-wraps as `Backend {status, message}`.
- **React** (`desktop-app/src/types/index.ts:3`): `ApiResult<T> = { ok: true; data: T } | { ok: false; error: string }` — **ALL errors become a single string**, no status code or discriminant.

**Risk:**
1. Signer gets 409 Conflict "signer already signed" from backend. Tauri layers it as `OrchestratorError::Backend {409, "signer already signed"}`. React receives `{ ok: false, error: "signer already signed" }`. UI shows generic error; doesn't show "this signer already approved (can proceed)" vs "invalid signatures (must retry)".
2. Network timeout (HTTP error) and business logic error (409 Conflict) both become `{ ok: false, error: "..." }`. Frontend can't distinguish retry-safe from non-idempotent.
3. High-signal errors (Unauthorized, Conflict) lose their meaning; all errors are treated as "something broke".

**HYPOTHESIS:** When a signer re-attempts to approve a proposal they already signed, the error message is correct but the UI has no way to know it's idempotent/safe to retry or proceed to the next action.

**Evidence:**
- `orchestrator-be/src/error.rs:10-45` — 5 error variants with HTTP mapping.
- `desktop-app/src-tauri/src/application/orchestrator_client.rs:14-17` — 3 error variants.
- `desktop-app/src/types/index.ts:3` — `error: string` (no discriminant).
- `desktop-app/src/api/tauri-bridge.ts:11-17` — Passes through Tauri error as plain string.

**Smallest Fix:** Expand `ApiResult` to include error type:
```typescript
export type ApiResult<T> = 
  | { ok: true; data: T } 
  | { ok: false; error: string; errorCode?: 'CONFLICT' | 'UNAUTHORIZED' | 'NOT_FOUND' | 'BAD_REQUEST' };
```

**Largest Bet:** Full error schema versioning; backend and desktop publish compatible error codes.

---

### 🔴 HIGH-4: `seq_no: u64` Precision Loss in JavaScript

**Issue:** Sequence numbers are `u64` (0–18,446,744,073,709,551,615) in Rust. JavaScript `number` is IEEE 754 float64, safe integer range is 0–2^53−1 (9,007,199,254,740,991). Any `seq_no > 2^53−1` silently loses precision when deserialized in React.

**Risk:**
1. Backend creates a proposal with `seq_no = 18,446,744,073,709,551,600`. JSON serializes as number. JavaScript receives `18446744073709552000` (rounded). Frontend submits approve request with wrong `seq_no`. Backend rejects as "different action". Double-signature attempt fails.
2. No error is thrown; Math.js sees it as a valid number.

**HYPOTHESIS:** Current test data uses small `seq_no` values (1, 2, 3), masking the issue. Only surfaces at scale or in fuzz tests.

**Evidence:**
- `orchestrator-be/src/domain/proposal.rs:9` — `pub type SeqNo = u64`.
- `desktop-app/src-tauri/src/domain/proposal.rs:11` — `pub seq_no: u64` in Rust struct (correct).
- `desktop-app/src/domain/create-proposal/hooks/use-create-proposal.ts:114` — `const seqNo = Number(formData.seqNo.trim())` — no range check, silently converts.
- No bigint usage; all `seqNo` fields are plain `number` type in TS.

**Smallest Fix:** Use `string` for `seq_no` in JSON, convert to `bigint` in TS:
```typescript
export type Proposal = {
  seq_no: string; // "18446744073709551615"
};
// Parse: const seqNoBigInt = BigInt(proposal.seq_no);
```

**Largest Bet:** Codegen from Rust types; ensure all u64 fields are strings in JSON.

---

### 🔴 HIGH-5: Broadcast Idempotency — Concurrent Calls Can Double-Broadcast

**Issue:** `broadcast_commit_then_reveal` is called from React (user clicks "Send"), but there's no idempotency key or distributed lock. If the same user (or concurrent requests) calls it twice, the state machine races:

- **Backend** (`orchestrator-be/src/application/proposals.rs:234-305`): Has `claim_broadcast()` that atomically transitions `Idle → CommitBroadcasted`. Returns Conflict if already claimed. ✓ Good.
- **Tauri desktop** (`desktop-app/src-tauri/src/application/proposals.rs:109-229`): **No such claim mechanism**. Directly builds commit + reveal without exclusive access. If called twice concurrently, both threads may call `btc_rpc.send_to_address()` with the same address, resulting in two distinct UTXOs funding the same commit address.
- **React** (`desktop-app/src/screens/…`): No loading state or button disable to prevent double-click.

**Risk:** Two broadcasts of the same proposal, both on-chain, causing signature verification to fail (ASM sees two competing reveal txs). Or worse: both succeed, and the governance action executes twice.

**HYPOTHESIS:** If React uses `tauriCall('broadcast_proposal', {...})` and the user clicks "Send" twice (or the request hangs and they retry), Tauri doesn't prevent concurrent broadcast attempts.

**Evidence:**
- `orchestrator-be/src/application/proposals.rs:254` — `let proposal = repo.claim_broadcast(action_id).await?;` — atomic claim.
- `desktop-app/src-tauri/src/application/proposals.rs:52-102` — **No claim; builds and sends directly.**
- `orchestrator-be/src/application/proposals.rs:272-288` — Backend has exclusive broadcast region.
- **No e2e test for concurrent broadcasts.**

**Smallest Fix:** Add idempotency key to `broadcast_commit_then_reveal` call; Tauri memoizes in-flight requests.

**Largest Bet:** Implement distributed lock (Redis, DB) or require backend to coordinate all broadcasts.

---

### 🟡 MEDIUM-6: Signer Public Key Case Sensitivity Mismatch

**Issue:** Backend and desktop disagree on hex pubkey comparison rules:

- **Backend** (`orchestrator-be/src/application/proposals.rs:40`): `sig.signer_pubkey.eq_ignore_ascii_case(session.signer_pubkey)` — **case-insensitive**.
- **Tauri** (`desktop-app/src-tauri/src/application/proposals.rs:90`): `s.signer_pubkey == sig.signer_pubkey` — **case-sensitive**.

**Risk:** A signer's public key is stored in lowercase from one source but uppercase from another. When approving a proposal:
1. Backend accepts the lowercase variant.
2. Tauri rejects the uppercase variant as "already signed" because the string doesn't match exactly.

**HYPOTHESIS:** If hardware wallet returns uppercase hex and backend session stores lowercase hex, duplicate-signer check fails to detect the duplicate.

**Evidence:**
- `orchestrator-be/src/application/proposals.rs:38-42` — `eq_ignore_ascii_case`.
- `desktop-app/src-tauri/src/application/proposals.rs:87-90` — `==` operator.
- **No test for hex case normalization.**

**Smallest Fix:** Normalize all hex pubkeys to lowercase on entry.

---

### 🟡 MEDIUM-7: No Correlation ID Propagation Across Layers

**Issue:** Backend logs request errors with internal tracing; desktop Tauri logs local errors. No correlation ID ties them together for debugging.

**Risk:** A signer submits a proposal, gets an error, retries. Backend logs "duplicate proposal" at timestamp T1. Desktop logs "HTTP 409" at T2. Operators can't correlate the two logs without manual timestamp/ID matching.

**Evidence:**
- `orchestrator-be/src/error.rs:34-39` — Logs internal errors but no correlation ID.
- `desktop-app/src-tauri/src/infrastructure/orchestrator_client.rs:43-61` — Parses HTTP error; no correlation ID extraction or forward.
- **No tracer setup** in Tauri; logs are ad-hoc.

**Smallest Fix:** Backend returns `X-Request-ID` header; Tauri forwards it to React; React logs it on error.

---

### 🟡 MEDIUM-8: Naming Convention Chaos — Backend API Uses snake_case, Tauri Commands Use camelCase

**Issue:** Serialization format inconsistency:

- **Backend HTTP API** (`orchestrator-be/src/handlers/proposals.rs` + domain types): Field names use `snake_case` (e.g., `action_hex`, `broadcast_status`, `commit_txid`).
- **Tauri command DTOs** (`desktop-app/src-tauri/src/commands/proposals.rs`, `orchestrator_auth.rs`): Use `#[serde(rename_all = "camelCase")]` for IPC to React (e.g., `actionHex`, `broadcastStatus`, `commitTxid`).

**Risk:**
1. If a frontend developer adds a new field to `CreateProposalRequest` (Rust side) and forgets to mark it with camelCase, it silently serializes to snake_case when sent to the backend, causing a 400 BadRequest.
2. No static check prevents field name misalignment.

**Evidence:**
- `orchestrator-be/src/domain/proposal.rs:17,61` — `snake_case`.
- `orchestrator-be/src/handlers/proposals.rs:10-98` — Request/response DTOs don't explicitly mark naming.
- `desktop-app/src-tauri/src/commands/proposals.rs:11,21,27,…` — `#[serde(rename_all = "camelCase")]` **explicitly** marks transformation.

**Smallest Fix:** Add explicit `#[serde(rename_all = "snake_case")]` to all backend DTOs (they currently rely on default, which is `snake_case` but not explicit).

---

### 🟡 MEDIUM-9: No API Versioning Strategy

**Issue:** There is no versioning header, path prefix, or compatibility layer. If backend changes the `Proposal` response shape (adds a field, removes a field, renames a field), all desktop clients break immediately.

**Risk:** Backend v2 ships with a new `last_updated_timestamp` field. Desktop v1 deserializes, ignores the new field (serde default). But if backend v2 *removes* `broadcast_error`, desktop v1's deserialization fails (missing required field).

**Evidence:**
- **No version header in HTTP requests/responses.**
- `desktop-app/src-tauri/src/infrastructure/orchestrator_client.rs:43-61` — HTTP parsing does not check version.
- No content-type negotiation or API version path (e.g., `/api/v1` is absent; current is `/api/...`).

**Smallest Fix:** Add `/api/v1` prefix to all backend endpoints; document compatibility rules (new optional fields are OK; removed fields require major version bump).

---

### 🟠 LOW-10: Missing End-to-End Tests for Rust↔TS Round-Trip

**Issue:** There are no tests that exercise the full Rust→JSON→TS→Rust serialization cycle for key types.

**Evidence:**
- `e2e-tests/src/e2e_propose_sign.rs` — Tests create→approve→verify but doesn't test JSON serialization round-trip.
- **No tests in TS for deserializing Rust-generated JSON.**
- **No tests in Tauri for Tauri command IPC serialization.**

**Smallest Fix:** Add TS test:
```typescript
import { Proposal } from '@/types'; // doesn't exist yet, but should
const rustJson = { action_id: '...', seq_no: 1, status: 'pending', ... };
const proposal: Proposal = JSON.parse(JSON.stringify(rustJson));
// Assert all fields are present and typed correctly
```

---

## Attack Narratives (3–6): "How This Fails in Production / for a Signer / for Maintainers"

### Narrative 1: Double-Signature in Retry (Broadcast Idempotency)
**Signer POV:** "I approved the proposal, but the UI hung. I clicked 'Send' again. Now the backend shows two broadcasts for the same proposal. On-chain, both reveal txs are mined. The governance action fired twice."

**Root cause:** No idempotency key in `broadcast_commit_then_reveal`. Tauri layer has no distributed lock mechanism. React button wasn't disabled during the first request, so user could retry manually.

**Detection:** Operators see two identical `commit_txid`/`reveal_txid` pairs in proposal history. They don't realize it's from the same user clicking twice; they assume it's two different signers.

---

### Narrative 2: Authority Name Mismatch on Authority Change
**Signer POV:** "I started in the 'Strata Administrator' role but want to switch to 'Sequencer Manager'. I re-authenticate. The backend rejects my session with Unauthorized 401. I can't proceed."

**Root cause:** Frontend enum (`AuthRole.StrataAdministrator`) uses different casing than the backend wire format (`strata_admin`). During re-auth, the mismatch surfaces in the `StartOrchestratorAuthRequest` — the JSON contains the wrong authority string.

**Detection:** Frontend shows generic "Unauthorized" error (from `ApiResult` — no error code). Operator logs show 401 from backend. No context about which authority was sent or expected.

---

### Narrative 3: Lost Error Context in Broadcast Failure
**Signer POV:** "I tried to send a proposal. I got an error message: 'Something went wrong.' It doesn't tell me what to do next."

**Root cause:** Backend returns 503 "Bitcoin RPC unavailable." Tauri wraps it as `OrchestratorError::Backend {503, "Bitcoin RPC unavailable"}`. React receives `{ ok: false, error: "Bitcoin RPC unavailable" }` — no error code, no retry flag. UI renders a generic error; doesn't suggest "try again in 5 minutes" vs "contact support."

**Detection:** No error code in `ApiResult`. React can't branch on error type. All errors get the same retry/dismiss UX.

---

### Narrative 4: Sequence Number Collision from Precision Loss
**Operator POV:** "The proposal history shows seq_no 18446744073709552000, but the on-chain ASM never saw it. Looking at the Bitcoin txs, I see a txid with the correct seq_no, but it's written to the chain as 18446744073709551999. Why the mismatch?"

**Root cause:** Frontend received `seq_no` as a JSON number (u64), JavaScript silently rounded it to 2^53−1 range, submitted a different `seq_no` to the backend. Backend created a new proposal with the wrong ActionId. On-chain, the ASM parsed the original (correct) seq_no from the Bitcoin tx and rejected the signature (mismatched ActionId).

**Detection:** Sequence number audit fails; database shows one seq_no, blockchain shows another.

---

### Narrative 5: Untyped Status String Breaks New Feature Rollout
**Operator POV:** "We shipped backend v2 with a new status 'PendingExpired' for proposals that are about to expire. Desktop v1 still sees them as 'Pending'. Signers don't notice the proposals are expiring and don't approve in time. Multiple proposals expire."

**Root cause:** React has no `ProposalStatus` enum; uses loose string comparison. Backend adds `"pending_expired"` status; React doesn't recognize it and renders it the same as `"pending"`. Signer sees no urgency signal.

**Detection:** Post-mortem: proposals showed `pending_expired` in the DB but signers never saw the warning. No TS type guard caught the unknown status.

---

### Narrative 6: Signature Already Submitted, But Case Mismatch Hides It
**Signer POV:** "I approved the proposal this morning with my Trezor (uppercase hex). Now I'm trying to approve it again from a different wallet import (lowercase hex). The backend says OK and accepts the second signature. But the ASM verification fails because it sees two signatures that look like the same key but with different casings."

**Root cause:** Tauri uses case-sensitive string comparison for pubkeys; desktop doesn't normalize to lowercase before checking for duplicates. Backend uses case-insensitive comparison (correct behavior) but Tauri bypasses it. The proposal is sent to the backend with two "different" signer pubkeys (same key, different case). ASM verification fails during threshold check (duplicate key not detected).

**Detection:** Backend threshold verification fails with "threshold not met" even though all required signers signed.

---

## Evidence Index (Paths)

### Type Contracts & Serialization
- `orchestrator-be/src/domain/authority.rs:5` — Backend Authority, snake_case
- `desktop-app/src-tauri/src/domain/authority.rs:7` — Tauri Authority, snake_case
- `desktop-app/src-tauri/src/commands/proposals.rs:11,21,27,34,41,50,57,73,91,100` — Tauri commands, camelCase for IPC
- `desktop-app/src/types/auth-role.ts:1-4` — React AuthRole enum, non-matching values
- `desktop-app/src/types/index.ts:3` — ApiResult<T>, error is plain string

### Error Handling
- `orchestrator-be/src/error.rs:10-45` — AppError enum, 5 variants
- `desktop-app/src-tauri/src/application/orchestrator_client.rs:10-18` — OrchestratorError enum
- `desktop-app/src/api/tauri-bridge.ts:11-17` — Tauri bridge, error as string

### Proposal State Management
- `orchestrator-be/src/domain/proposal.rs:60-73` — ProposalStatus enum, snake_case
- `orchestrator-be/src/domain/proposal.rs:18-26` — BroadcastStatus enum, snake_case
- `desktop-app/src-tauri/src/domain/proposal.rs:8-21` — Proposal struct, status/broadcast_status as String
- `desktop-app/src/screens/sign-poc-screen.tsx:62` — String comparison, no enum guard

### Sequence Number Handling
- `orchestrator-be/src/domain/proposal.rs:9` — SeqNo = u64
- `desktop-app/src-tauri/src/domain/proposal.rs:11` — seq_no: u64
- `desktop-app/src/domain/create-proposal/hooks/use-create-proposal.ts:114` — Number(formData.seqNo), no bigint

### Broadcast & Idempotency
- `orchestrator-be/src/application/proposals.rs:234-305` — broadcast_commit_then_reveal, has claim_broadcast()
- `desktop-app/src-tauri/src/application/proposals.rs:109-229` — broadcast_commit_then_reveal, no claim mechanism
- `orchestrator-be/src/application/proposals.rs:254` — claim_broadcast() atomic

### Signer Pubkey Comparison
- `orchestrator-be/src/application/proposals.rs:38-42` — Case-insensitive comparison
- `desktop-app/src-tauri/src/application/proposals.rs:87-90` — Case-sensitive comparison

### API Versioning
- No `/api/v1` prefix in backend routes.
- No version header in HTTP responses.
- No content-type negotiation in Tauri client.

---

## Smallest Fixes vs Largest Bets

### Quick Wins (1–2 days, high impact)

1. **Normalize hex pubkeys to lowercase on input:**
   - Tauri: Add `fn normalize_pubkey(pk: &str) -> String { pk.to_lowercase() }` before all comparisons.
   - Impact: Eliminates signature deduplication bypass.

2. **Add `errorCode` to `ApiResult`:**
   - TS type: `errorCode?: 'CONFLICT' | 'UNAUTHORIZED' | 'NOT_FOUND'`
   - React: Branch on error code for UX (retry vs dismiss).
   - Impact: Signers can see actionable error context.

3. **Disable "Send" button during broadcast:**
   - React: `disabled={isBroadcasting}` on send button.
   - Tauri: Return loading state from bridge.
   - Impact: Prevents accidental double-clicks.

4. **Add round-trip serde tests:**
   - Rust: Authority serialization round-trip test.
   - TS: Proposal deserialization test (once types are generated).
   - Impact: Catches type contract divergence early.

### Medium Bets (1–2 weeks, systemic)

5. **Create TypeScript types from Rust with serde codegen:**
   - Generate TS types for Proposal, Authority, errors from Rust #[serde].
   - Keep in sync with `typescript-codegen` or similar.
   - Impact: No more hand-written TS types; contracts are enforceable.

6. **Implement idempotency keys for broadcast:**
   - Tauri: Generate UUID for each broadcast attempt; memoize in-flight.
   - Backend: Store `(action_id, idempotency_key) → (commit_txid, reveal_txid)` to deduplicate retries.
   - Impact: Eliminates double-broadcast risk.

7. **Add API versioning:**
   - Backend: Prefix all routes with `/api/v1`.
   - Tauri client: Include `Accept: application/json; version=1` header.
   - Impact: Enables independent versioning; breaking changes are explicit.

### Large Bets (3–4 weeks, architectural)

8. **Shared error schema & codegen:**
   - Define error types in a `.proto` or JSON schema.
   - Generate Rust `thiserror` enum and TS error type.
   - Update all layers (backend, Tauri, React) to use generated types.
   - Impact: Error contracts are enforceable; no silent loss.

9. **Distributed broadcast lock:**
   - Introduce Redis or DB lock for `broadcast_commit_then_reveal`.
   - Both backend and Tauri respect the lock.
   - Requires backend integration (not just in-memory).
   - Impact: Eliminates race condition entirely; safe concurrent retries.

10. **Full E2E contract testing:**
    - E2E test suite exercises Rust → JSON → TS → Rust for all key types.
    - Includes serialization, deserialization, error cases.
    - Runs on every commit.
    - Impact: No more silent divergence; contracts are continuously validated.

---

## What Would Change My Mind (Missing Evidence / Experiments)

1. **Evidence that idempotency keys are already implemented elsewhere** (e.g., in Bitcoin RPC layer). If true, BLOCKER-5 is not critical.

2. **Evidence that TS types ARE codegen'd or auto-validated** (e.g., via a build step I missed). If true, BLOCKER-1 and BLOCKER-2 impact is reduced.

3. **Evidence that error codes ARE propagated in `ApiResult.errorCode`** (even if not used in React yet). If true, BLOCKER-3 is medium, not blocker.

4. **Evidence that `seq_no` is already serialized as string in JSON** (not number). If true, BLOCKER-4 is not critical.

5. **Evidence that all `seq_no` deserialization in React uses `BigInt` or has explicit range checks.** If true, BLOCKER-4 risk is mitigated.

6. **Evidence that there IS an E2E test for Authority serialization round-trip.** If true, BLOCKER-1 risk is lower.

7. **Evidence that broadcast operations are logged with request IDs and correlation chains exist.** If true, MEDIUM-7 (observability) impact is lower.

**How to test these:** Run grep for `BigInt`, `string` type in Tauri Proposal struct, correlation ID setup, and check E2E test coverage in `e2e-tests/`.

---

## Summary

**Verdict:** The system has 3 blocking contract drift risks (Authority serialization, Status enums, Error model) and 1 critical idempotency gap (broadcast concurrency). Together, these can lead to silent failures in production: duplicate signatures, missed retries, lost error context, and signer confusion.

Most findings are **fixable with small changes** (add type guards, disable buttons, normalize pubkeys). The largest bet—shared error schema + E2E contract tests—is architectural but yields the highest confidence.

**Confidence level:** HIGH. The drift is structural and present in the code; the attack narratives are plausible given current error handling and type choices.

