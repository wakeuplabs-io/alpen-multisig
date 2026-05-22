# Testing Strategy & Quality — Adversarial Assessment

**Date:** May 13, 2026  
**Scope:** Orchestrator backend, Desktop Tauri app, E2E test suite (Cargo)  
**Stance:** Test quality attacks + test-optimizer lens (brittle tests, coverage theater, missing negative paths)

---

## Scope & Threat Model (What We're Trying to Break)

### Multisig Signers Are Under Attack

The application controls the *creation, approval, and broadcast of transactions that update onchain multisig authorities* (Alpen Admin, Strata Admin, Sequencer Manager, Security Council, Payout Admin). A signerack, a signer can:
- **Understand what they're signing** (does the UI show the right action?)
- **Be unable to replay old signatures** (seqNo prevents it)
- **Never be tricked into signing something they didn't authorize** (domain separation + challenge-binding)
- **Never proceed if the backend fails** (offline fallback works)
- **Know when signature collection is complete and safe** (threshold tests real)

### Testing Layers We're Asking To Hold This Line

1. **Backend (orchestrator-be):** HTTP API routes, auth sessions, proposal creation, signature aggregation, threshold tracking
2. **Desktop Tauri Rust layer:** IPC commands, signing, serialization (SSZ), action encoding, orchestrator client
3. **Frontend (React):** UI forms, state management, wallet connect, signer confirmation flows
4. **E2E (Cargo test suite):** Real Bitcoin, real orchestrator subprocess, proposal creation to broadcast

---

## Top Findings (Ranked) — Blocking/High | Medium | Low

### BLOCKING (Severity: Critical — Signers At Risk)

#### B1: **No Test For Signer Race (Simultaneous Threshold Breach)**

**Location:** `orchestrator-be/src/application/proposals.rs:253` (comment: "Returns Conflict if another caller already claimed (race-safe).")

**The Bug That Ships:**

Two signers sign simultaneously. Signer A and B *both* meet the threshold at the exact same moment (timing window exists). Test suite *only* tests sequential approval (A signs, B signs, threshold met). No test for:
- Concurrent `POST /proposals/{action_id}/approve` calls
- Proposal status transitions to `Approved` under race conditions
- Idempotency of threshold crossing (calling approve twice returns same state, not "already approved" error)

**Evidence:**

```rust
// orchestrator-be/src/application/proposals.rs:253
// "Returns Conflict if another caller already claimed (race-safe)."
// BUT: no tokio::test spawning concurrent tasks testing this claim
```

No async test with `tokio::spawn` or `futures::join_all` testing two `approve_action` calls on the same proposal, same seqno, simultaneously.

**Why It Matters:**

If `approve_action` is not atomic or properly synchronized at the in-memory level, backend state could show 2 signatures collected but status still `Pending`. Frontend would show "needs 2 more signatures" when in fact the threshold is met. Signers would think approval failed and re-sign, creating duplicates that the on-chain multisig rejects.

**Adversarial Test That Should Fail Today:**

```rust
#[tokio::test]
async fn test_approve_action_race_condition() {
    let repo = Arc::new(InMemoryProposalRepository::new());
    let (sig_a, sig_b) = (sig_a(), sig_b());
    
    // Create proposal, threshold = 2
    let proposal = create_update_action(&repo, session_a, 1, ACTION_HEX, &sig_a, 2).await.unwrap();
    
    // Race: two threads both try to add sig_b
    let repo_clone = repo.clone();
    let action_id = proposal.action_id.clone();
    let handle_1 = tokio::spawn(async move {
        approve_action(&repo_clone, session_b, &action_id, &sig_b).await
    });
    
    let repo_clone = repo.clone();
    let action_id_2 = proposal.action_id.clone();
    let handle_2 = tokio::spawn(async move {
        approve_action(&repo_clone, session_b, &action_id_2, &sig_b).await
    });
    
    let (r1, r2) = tokio::join!(handle_1, handle_2);
    let p1 = r1.unwrap().unwrap();
    let p2 = r2.unwrap().unwrap();
    
    // CLAIM: status should be Approved, exactly 2 sigs, not duplicated
    assert_eq!(p1.status, ProposalStatus::Approved);
    assert_eq!(p1.signatures.len(), 2);  // NOT 3
}
```

**Proposal:** Add 3 concurrent approval tests covering (a) threshold-exact race, (b) over-threshold race, (c) same signer re-approving.

---

#### B2: **Frontend Has No Error Path Tests for Signer UI**

**Location:** `desktop-app/src/domain/sign-proposal/components/sign-proposal-view.tsx` (exists, no tests found in glob/grep)

**The Bug That Ships:**

Frontend has zero tests for the signer confirmation screen. Can't be sure:
- User sees the correct action (not a different proposal)
- Signature input is validated before submission
- Backend 500 errors show in UI (not silent failure)
- Partial signature (truncated sig_hex) is rejected visibly
- Invalid signer pubkey is surfaced
- Offline state (backend unavailable) has fallback copy button for manual aggregation

**Evidence:**

No `*.test.tsx` or `*.spec.tsx` files found in `desktop-app/src`.  
`desktop-app/src-tauri/src/application/proposals.rs` has unit tests with mock orchestrator; frontend has none.

```bash
# No test files found
$ find desktop-app/src -name '*.test.ts' -o -name '*.spec.ts' -o -name '*.test.tsx'
# (empty)
```

**Why It Matters:**

Multisig UI is a **high-signal interface** — a signer trusts the UI to show them what action they're signing. If the frontend doesn't test error paths (backend down, malformed action, network timeout), signers could:
- Sign the wrong action (UI showed cached/stale proposal)
- Paste a truncated signature without realizing
- Think their signature was sent when it wasn't
- Never know the backend failed (no error message)

**Adversarial Test That Should Exist:**

```tsx
describe('SignProposalView', () => {
  it('should reject truncated signature with visible error', () => {
    const { getByText, getByLabelText } = render(
      <SignProposalView proposal={mockProposal} />
    );
    const sigInput = getByLabelText('Signature hex');
    const submitBtn = getByText('Confirm and Send');
    
    // User pastes truncated sig (64 chars, needs 128)
    fireEvent.change(sigInput, { target: { value: 'deadbeef' } });
    fireEvent.click(submitBtn);
    
    // Should NOT submit, should show error
    expect(getByText(/Invalid signature format/)).toBeInTheDocument();
  });

  it('should show backend error when approving fails', async () => {
    const mockClient = { approve_action: jest.fn().mockRejectedValue(new Error('500')) };
    const { getByText } = render(
      <SignProposalView client={mockClient} proposal={mockProposal} />
    );
    
    fireEvent.click(getByText('Confirm and Send'));
    await waitFor(() => {
      expect(getByText(/Failed to send signature/)).toBeInTheDocument();
    });
  });
});
```

**Proposal:** Add frontend test suite with error-path coverage for sign flow, wallet connect, and offline fallback.

---

#### B3: **Tauri IPC Contracts Are Never Tested**

**Location:** `desktop-app/src-tauri/src/commands/` (12 command files, zero IPC tests)

**The Bug That Ships:**

Tauri commands (`authentication`, `proposals`, `signing`, `asm_state`, etc.) are the boundary between React frontend and Rust backend. Each command is a port. **Zero tests invoke the IPC boundary.**

Example: `desktop-app/src-tauri/src/commands/signing.rs`:
```rust
#[tauri::command]
pub async fn sign_action_with_key(action_hex: String, seq_no: u64, ...) -> Result<Signature, String> { ... }
```

Tests exist for the underlying `signing::compute_sighash()` function, but *no test calls `sign_action_with_key` through Tauri's IPC*.

**Evidence:**

- No `#[tauri::test]` tests found in repo
- `desktop-app/src-tauri/Cargo.toml` lists no `tauri-test` dev-dependency
- `e2e-tests/tests/e2e_propose_sign.rs` tests the full app via subprocess, but not isolated IPC contract testing

**Why It Matters:**

Tauri IPC has its own serialization layer (serde JSON for args, return values). A contract break (frontend sends wrong shape, Rust expects different field name) would only surface at runtime when the user clicks a button. Example failures:

- Frontend sends `{ actionHex: "..." }` (camelCase), Rust expects `{ action_hex: "..." }` (snake_case) → serde error, no helpful UI message
- Frontend sends signature as base64, Rust expects hex → silently fails or corrupts
- Rust throws an error with a struct that doesn't serialize — frontend gets cryptic "serde error"

**Adversarial Test That Should Exist:**

```rust
#[tokio::test]
async fn test_sign_action_with_key_ipc_contract() {
    // Simulate frontend calling via IPC
    let action_hex = "deadbeef".to_string();
    let seq_no = 1u64;
    let secret_key = "...".to_string();

    // Should NOT panic, should return valid Signature struct
    let result = sign_action_with_key(action_hex, seq_no, secret_key).await;
    
    assert!(result.is_ok());
    let sig = result.unwrap();
    assert!(!sig.signature_hex.is_empty());
    assert!(!sig.public_key_hex.is_empty());
    assert_eq!(sig.public_key_hex.len(), 66); // 33 bytes = 66 hex chars
}
```

**Proposal:** Add integration tests that (a) call each Tauri command with valid/invalid inputs, (b) verify serde round-trip, (c) check error messages serialize properly.

---

### HIGH (Severity: High — Critical Paths Uncovered)

#### H1: **Proposal Finalization (Broadcast) Path Has Only Happy-Path Tests**

**Location:** `orchestrator-be/src/handlers/proposals.rs:prepare_broadcast`, `execute_broadcast` (lines ~160+)

**The Bug That Ships:**

`/proposals/{action_id}/broadcast/prepare` and `execute_broadcast` endpoints have:
- ✅ Happy path: sufficient signatures, valid sighash, broadcast succeeds
- ❌ Negative paths: no tests for:
  - Insufficient signatures (3/5 required, only 2 collected)
  - Malformed signature (not valid hex, invalid secp256k1 sig)
  - Network error during broadcast (Bitcoin RPC down)
  - Transaction already on-chain (idempotent broadcast)
  - Expired state (proposal too old, Bitcoin block height advanced)

**Evidence:**

```bash
# orchestrator-be/src/handlers/proposals.rs around execute_broadcast
# Grep shows only one test pattern: test_auth_verify_...
# No test_broadcast_with_insufficient_signatures
# No test_broadcast_with_malformed_signature
```

`e2e-tests/tests/e2e_propose_sign.rs` tests happy path (propose → sign → broadcast). No variation for:
- What if we try to broadcast before 2/2 signatures collected?
- What if Bitcoin RPC is unreachable mid-broadcast?
- What if the Bitcoin node rejects the transaction (fee too low)?

**Why It Matters:**

Broadcast is the *point of no return* — once a commit+reveal tx pair hits Bitcoin, the multisig update becomes enforceable on-chain. If the backend doesn't test error cases:

- Signer clicks "broadcast" twice (idempotent?) — might submit duplicate tx
- Network glitch during broadcast — signer doesn't know if tx landed (UI freezes)
- Insufficient sigs case — backend might not clearly reject it

**Adversarial Test That Should Fail:**

```rust
#[tokio::test]
async fn test_broadcast_with_insufficient_signatures_rejected() {
    let repo = Arc::new(InMemoryProposalRepository::new());
    let sig_a = sig_a();
    
    // Create proposal, threshold = 2, but only 1 sig collected
    let proposal = create_update_action(&repo, session_a, 1, ACTION_HEX, &sig_a, 2)
        .await
        .unwrap();
    
    // Try to broadcast without 2nd signature
    let broadcast_result = prepare_broadcast(
        &state,
        proposals::SessionContext { ... },
        &proposal.action_id,
    )
    .await;
    
    // Should error with "insufficient signatures" not 500
    assert!(matches!(broadcast_result.unwrap_err(), AppError::BadRequest(_)));
}
```

**Proposal:** Add 5 broadcast error-path tests (insufficient sigs, malformed sig, network down, idempotent retry, expiry).

---

#### H2: **ASM Membership RPC Failures Are Mocked, Not Integration-Tested**

**Location:** `orchestrator-be/src/infrastructure/asm_role_membership.rs`

**The Bug That Ships:**

`threshold_for_authority()` and `last_seqno_for_authority()` call the ASM membership RPC. Handlers depend on these returning correct thresholds. Tests mock the RPC:

```rust
// orchestrator-be/src/handlers/mod.rs:94
fn test_app_with_rpc_url(rpc_url: &str) -> Router {
    // ... creates app with "mock://asm-membership" or "http://127.0.0.1:1" ...
}
```

**What's Not Tested:**

- RPC actually returns threshold = 2 for Strata Admin (not 1, not 999)
- Timeout handling if RPC is slow
- Malformed RPC response (missing `threshold` field)
- Authority not found in ASM state (returns 404)
- Different authorities have different thresholds (Alpen Admin = 3, Strata Admin = 2)

**Evidence:**

Tests use hardcoded RPC URLs that don't actually call ASM. `e2e-tests` harness (`test_harness.rs`) sets up Bitcoin but doesn't embed the ASM subprotocol to verify real threshold state.

**Why It Matters:**

If the ASM RPC returns garbage (e.g., threshold=0), the handler would create a proposal with `required_signatures=0`. Any single signer could "approve" it and mark it ready for broadcast — bypassing the multisig entirely.

**Adversarial Test That Should Fail:**

```rust
#[tokio::test]
async fn test_get_threshold_from_real_asm_state() {
    // Embed strata-asm-proto-administration, replay Bitcoin, query ASM state
    let asm_state = AdministrationSubprotoState::from_bitcoin(&harness).await;
    let strata_admin_threshold = asm_state.authority_for_role(Role::StrataAdmin).threshold;
    
    // Must be > 1 (sanity check)
    assert!(strata_admin_threshold.get() > 1);
    
    // Must match what backend requests
    let backend_threshold = threshold_for_authority("http://...", Authority::StrataAdmin).await;
    assert_eq!(strata_admin_threshold.get() as u64, backend_threshold);
}
```

**Proposal:** Add integration test querying real ASM state (via e2e harness), verify all 5 authorities have correct thresholds.

---

#### H3: **Signature Verification Coverage: Only Happy Path**

**Location:** `desktop-app/src-tauri/src/infrastructure/signing.rs:verify_threshold` (line 267+)

**The Bug That Ships:**

`verify_threshold()` is the gate that decides "are these N signatures valid for this sighash?" Tests cover:

- ✅ Valid signature, valid threshold met
- ❌ Missing: all negative paths:
  - Signature from wrong signer (pubkey doesn't match)
  - Signature for wrong sighash (changed action, changed seqno)
  - Duplicate signature (same signer twice)
  - Invalid signature format (not valid secp256k1)
  - Threshold not met (1 sig, threshold=2)
  - Empty signature array but threshold=1

**Evidence:**

```rust
// desktop-app/src-tauri/src/infrastructure/signing.rs:210-300
// Tests: verify_threshold, verify_threshold_one_of_two
// Only happy-path assertions: result.valid == true
// No test_verify_threshold_wrong_signer
// No test_verify_threshold_insufficient_sigs
```

**Why It Matters:**

This function is called when frontend tries to finalize a proposal. If it doesn't test negative paths:

- Attacker-controlled signature could pass verification (sigcheck broken)
- Frontend might think a proposal is "ready to broadcast" when only 1/2 signatures are valid
- Wrong-signer signature might be accepted (threshold check off)

**Adversarial Test That Should Fail:**

```rust
#[test]
fn test_verify_threshold_rejects_wrong_signer() {
    let keys = generate_demo_keys(2); // A, B
    let sighash = compute_sighash(1, &demo_action_hex()).unwrap();
    
    // Sign with key A
    let sig_a = sign_sighash(&keys[0].secret_key_hex, &sighash.sighash_hex).unwrap();
    
    // Verify with key B's pubkey (mismatch)
    let result = verify_threshold(
        &[keys[1].public_key_hex], // Verify against wrong key
        1,
        &[sig_a.signature_hex],
        &sighash.sighash_hex,
    );
    
    assert!(!result.unwrap().valid, "signature from wrong signer must not verify");
}
```

**Proposal:** Add 6 signature verification negative-path tests (wrong signer, wrong sighash, duplicate, insufficient count, malformed sig, empty array).

---

### MEDIUM (Severity: Medium — Surprises Likely, Fewer Signers At Risk)

#### M1: **Authentication Challenge Reuse (Replay) Is Not Fully Tested**

**Location:** `desktop-app/src-tauri/src/application/authentication.rs:355` mentions `replayed_challenge_is_rejected()`, but only tests the Tauri layer, not the HTTP API layer.

**Evidence:**

```rust
// Test in authentication.rs (Tauri layer)
fn replayed_challenge_is_rejected() { ... }

// But orchestrator-be/src/handlers/auth.rs (HTTP layer) has NO test for:
// POST /auth/verify with a challenge_id that was already used in previous request
```

**Why It Matters:**

If HTTP layer allows challenge reuse, an attacker could capture a valid `challenge_id` + `signature` pair and replay it later to forge a session for a different signer.

**Proposal:** Add backend test: `test_auth_verify_rejects_replayed_challenge_id`.

---

#### M2: **Config/Environment Validation Missing**

**Location:** `orchestrator-be/src/config.rs`

**The Bug That Ships:**

Server starts with environment variables:
- `SERVER_HOST`, `SERVER_PORT`
- `STRATA_ADMIN_STATE_RPC_URL`
- `BITCOIN_RPC_URL`
- `DATABASE_URL` (if using Postgres)
- `MAGIC_BYTES`

Tests and CI never validate:
- What if `STRATA_ADMIN_STATE_RPC_URL` is missing? Does the server start anyway?
- What if `MAGIC_BYTES` is invalid hex? (e.g., "INVALID" instead of "414c504e")
- What if port is already in use?

**Evidence:**

No test calls the config loader with missing required variables and verifies it fails fast.

**Proposal:** Add 3 config validation tests (missing RPC URL, invalid magic bytes, invalid network).

---

### LOW (Severity: Low — Edge Cases, Coverage Theater)

#### L1: **Dataclass Storage Tests Present (Anti-Pattern 2.3)**

**Location:** Various `#[test]` blocks asserting struct field assignment.

Example pattern (hypothetical, but consistent with test structure):
```rust
#[test]
fn test_proposal_stores_seq_no() {
    let proposal = Proposal { seq_no: 5, ... };
    assert_eq!(proposal.seq_no, 5);
}
```

**Why It's Theater:**

Rust's struct field assignment is guaranteed by the type system; this test adds no value.

**Proposal:** Audit and remove any tests that only verify `struct_instance.field == expected_value` without behavioral assertion.

---

#### L2: **E2E Test Depends On External `bitcoind`**

**Location:** `e2e-tests/tests/e2e_harness_hello_world.rs:6-11`

```rust
if Command::new("bitcoind").arg("--version").output().is_err() {
    eprintln!("Skipping ...");
    return;
}
```

**Why It's Low:**

Test silently skips if `bitcoind` unavailable (good graceful degradation), but it means CI might not actually run this test. Check CI logs to confirm it runs or not.

**Proposal:** Add a CI job that explicitly requires `bitcoind` (or use Docker image that includes it).

---

## Attack Narratives (3–6): "How This Fails in Production"

### Narrative 1: **Signer Thinks They've Signed, But Silence Is the Response**

**The Flow:**
1. Signer opens desktop app, sees proposal: "Add Alice to multisig"
2. Clicks "Sign"
3. **Backend is down (k8s crash, network partition)**
4. Frontend has no error message (no test for backend unreachable)
5. Signer: "Hmm, nothing happened. I'll try again."
6. Clicks sign again → network recovers, both requests land → **duplicate signature**
7. Backend accepts both signatures (no deduplication by signer pubkey tested)
8. Proposal shows "3/2 signatures" or worse, "Approved" when user only signed once

**Root Cause:** Frontend has no tests for backend errors; Tauri IPC contracts untested.

### Narrative 2: **Two Signers Race, Threshold Met, But One Signer's UI Says "Waiting For You"**

**The Flow:**
1. Alice and Bob both see "2/2 required, 0 signatures"
2. Alice clicks "Sign" (t=0ms)
3. Bob clicks "Sign" (t=1ms)
4. **Race condition:** Both `approve_action` calls hit backend simultaneously
5. Backend's in-memory repo doesn't lock/synchronize properly
6. Both complete without error, proposal shows "Approved"
7. Alice's UI, polled at t=10ms, refreshes and sees "Pending" (stale cache)
8. Bob's UI correctly sees "Approved"
9. **Alice never knows the threshold was met; she re-signs offline to be safe**

**Root Cause:** No concurrent test for threshold crossing; no real-time UI updates on approval.

### Narrative 3: **Attacker Forges a Signer's Signature (Threshold Check Broken)**

**The Flow:**
1. Eve, a non-signer, obtains a real proposal's action_hex and seqno
2. Eve generates a fake signature (or uses a captured old signature)
3. Eve calls `verify_threshold([eve_pubkey], 1, [eve_sig], sighash)`
4. **Test never validated wrong-signer rejection**
5. `verify_threshold` returns `valid: true` (bug in underlying secp256k1 wrapper)
6. Eve's signature is accepted
7. Proposal broadcasts with Eve's "approval"
8. Multisig update on-chain fails (threshold never met), but Eve's signer pubkey is broadcast in a transaction log

**Root Cause:** Signature verification has no negative-path tests.

### Narrative 4: **Backend Starts With Corrupted Authority Threshold (0 Signatures Required)**

**The Flow:**
1. Orchestrator deployed, RPC points to stale/corrupted ASM state
2. `threshold_for_authority(Strata Admin)` returns 0 (malformed data)
3. Proposal created with `required_signatures: 0`
4. Single signer clicks "Approve"
5. Threshold check passes (0 signatures required)
6. Proposal auto-transitions to `Approved`
7. **No test ever verified threshold > 0**
8. Proposal broadcasts with single signature, on-chain multisig rejects it
9. User sees cryptic Bitcoin error, doesn't know the backend was the culprit

**Root Cause:** ASM RPC integration untested against real authority state.

### Narrative 5: **Signer Never Sees the Action They're Signing (Stale Cache)**

**The Flow:**
1. Signer A creates proposal P1: "Add Alice"
2. Signer B retrieves proposal, sees it
3. Signer A cancels it, creates P2: "Add Bob"
4. **Frontend caches P1 in local state; doesn't poll for updates**
5. Signer B, still on the P1 screen, signs it
6. Frontend sends signature for P2's seqno/action_hex (logic error)
7. Backend rejects it (action mismatch)
8. **No frontend test for cache invalidation on proposal status change**

**Root Cause:** Frontend has no error-path tests; no server-push or polling tests for real-time updates.

### Narrative 6: **Tauri Command Serialization Breaks Silently**

**The Flow:**
1. Frontend (React) calls Tauri command: `invoke('sign_action_with_key', { actionHex: "..." })`
2. Tauri serializes to JSON, sends to Rust
3. Rust expects `{ action_hex: ... }` (snake_case)
4. serde fails to deserialize
5. **No test ever ran the IPC boundary** — Rust test mocks skip serde
6. Frontend's `invoke()` promise hangs or returns generic error
7. Signer thinks signing failed, retries manually via hex input
8. **UX nightmare, and no error message to help**

**Root Cause:** Tauri IPC contracts untested.

---

## Evidence Index (Paths)

### Test Files (Current)

| Path | Type | Coverage |
|------|------|----------|
| `orchestrator-be/src/handlers/mod.rs:43-end` | Unit (HTTP handlers) | Auth ✅, Proposal creation ✅, Broadcast ❌ |
| `orchestrator-be/src/application/proposals.rs:455-end` | Unit (business logic) | Create ✅, Approve ✅, Threshold ✅, Duplicate prevention ✅, Error propagation ❌ |
| `orchestrator-be/src/domain/proposal.rs:139-end` | Unit (domain types) | Action ID determinism ✅, Invalid hex ✅ |
| `desktop-app/src-tauri/src/infrastructure/signing.rs:210-end` | Unit (crypto) | Sighash ✅, Sign ✅, Verify (happy path) ✅, Verify (negative) ❌ |
| `desktop-app/src-tauri/src/infrastructure/action_codec.rs:162-end` | Unit (encoding) | Happy path ✅ |
| `desktop-app/src-tauri/src/application/authentication.rs:274-end` | Unit (Tauri auth) | Challenge generation ✅, Replay ✅ |
| `desktop-app/src-tauri/src/application/proposals.rs:317-end` | Unit (Tauri proposals) | Create ✅, Approve ✅, Errors ✅ (mocked orchestrator) |
| `e2e-tests/tests/e2e_propose_sign.rs` | Integration (happy path) | Propose → Sign → Verify ✅, Broadcast ✅, Errors ❌ |
| `e2e-tests/tests/e2e_harness_hello_world.rs` | Integration (harness setup) | Bitcoin mining ✅ |
| **`desktop-app/src/**/*.tsx`** | **Frontend tests** | **NONE FOUND** |

### Critical Paths With No/Weak Test Coverage

| Critical Path | What Should Be Tested | Status |
|---------------|----------------------|--------|
| Proposal creation | Happy ✅, Duplicate rejection ✅, Malformed action ❌, Threshold lookup failure ❌ | **WEAK** |
| Signature aggregation | Sequential approval ✅, Concurrent approval ❌, Race at threshold ❌, Duplicate signature ❌ | **WEAK** |
| Broadcast | Happy ✅, Insufficient sigs ❌, Malformed sig ❌, RPC down ❌, Idempotent retry ❌ | **CRITICAL** |
| Signer verification | Valid signature ✅, Invalid signer ❌, Wrong sighash ❌, Malformed sig ❌, Threshold not met ❌ | **CRITICAL** |
| Auth challenge | Challenge generation ✅, Replay detection ✅ (Tauri), RPC query ❌, Expired challenge ❌ | **WEAK** |
| Frontend sign flow | (No tests exist) | **CRITICAL** |
| Tauri IPC contracts | (No tests exist) | **CRITICAL** |

---

## Smallest Fixes vs Largest Bets

### Smallest Fixes (1–3 hours each)

1. **Add 3 concurrent approval tests** (`orchestrator-be`)
   - `test_approve_action_race_condition`
   - `test_approve_action_threshold_exact_race`
   - `test_approve_action_duplicate_prevention`

2. **Add 5 broadcast error-path tests** (`orchestrator-be`)
   - `test_broadcast_insufficient_signatures`
   - `test_broadcast_malformed_signature`
   - `test_broadcast_rpc_unavailable`
   - `test_broadcast_idempotent_retry`
   - `test_broadcast_proposal_expired`

3. **Add 6 signature verification negative tests** (`desktop-app/src-tauri`)
   - `test_verify_threshold_wrong_signer`
   - `test_verify_threshold_wrong_sighash`
   - `test_verify_threshold_insufficient_count`
   - `test_verify_threshold_duplicate_signer`
   - `test_verify_threshold_malformed_signature`
   - `test_verify_threshold_empty_signatures`

4. **Add 4 ASM RPC integration tests** (`orchestrator-be` + `e2e-tests`)
   - Real threshold query for all 5 authorities
   - Timeout handling
   - Malformed response handling
   - Authority not found

5. **Add Tauri IPC contract tests** (`desktop-app/src-tauri`)
   - Each command (`sign_action_with_key`, `create_proposal`, etc.) with valid/invalid inputs
   - Serde round-trip verification
   - Error serialization

### Medium Fixes (5–10 hours each)

6. **Frontend test suite** (`desktop-app/src`)
   - Sign flow: happy path, backend down, invalid signature format, threshold not met
   - Wallet connect: connection error, signer rejection
   - Create proposal: validation, server error propagation
   - Real-time update: poll/refresh on status change

7. **Config validation tests** (`orchestrator-be`)
   - Missing required env vars
   - Invalid magic bytes
   - Invalid network
   - Port already in use

### Largest Bets (2–3 days each, architectural impact)

8. **Real ASM state integration in e2e-tests**
   - Embed `strata-asm-proto-administration`, replay Bitcoin
   - Verify all authority thresholds match on-chain state
   - Test proposal creation under different authorities (Alpen Admin, Sequencer Manager, etc.)

9. **Frontend state synchronization (polling/websocket)**
   - Add real-time UI updates when proposal status changes
   - Test concurrent UIs (two browsers, one proposal)
   - Test cache invalidation

10. **Signature verification against real hardware wallets**
    - Currently only tests software key signing
    - Integration with Trezor/Ledger (if available)
    - Test rejection of BIP-137 signatures (negative path)

---

## What Would Change My Mind (Missing Evidence / Experiments)

### Evidence I Don't Currently Have

1. **Test coverage %, by path**
   - Run `cargo tarpaulin --workspace` or similar; report % for each crate
   - Current hypothesis: backend ~60–70%, frontend 0%, e2e ~30%

2. **Actual concurrent access pattern in production**
   - How often do two signers try to approve the same proposal simultaneously?
   - If rare, race condition is lower priority; if frequent, it's critical

3. **ASM RPC availability SLA**
   - Is there a fallback if ASM RPC is down?
   - Current code doesn't seem to have one (see `asm_role_membership.rs`)
   - If ASM RPC unavailable, can the backend operate at all?

4. **Hardware wallet integration maturity**
   - Are Trezor/Ledger signing tests in the suite?
   - Grep found no references to actual HW wallet testing
   - Only BIP-137 theoretical discussion in docs

5. **Frontend CI**
   - Current CI does linting and build, but **no frontend tests run**
   - Add React Testing Library tests; report if they catch real bugs

### Experiments I'd Run

1. **Inject race condition deliberately** (chaos engineering)
   - Add artificial sleep in the middle of `approve_action`
   - Run two approvals concurrently, measure if state is corrupted
   - Expected: should not corrupt; if it does, race condition is real

2. **Kill orchestrator mid-broadcast** (failure injection)
   - Start broadcast, kill process after commit tx but before reveal
   - Query backend state: is proposal "in broadcast limbo"?
   - Can a signer retry safely?

3. **Replay a captured signature**
   - Capture a valid `(challenge_id, signature_hex)` from a real auth flow
   - Replay it 10 minutes later with a different `signer_pubkey`
   - Should be rejected; if accepted, auth is broken

4. **Fuzz the signing code**
   - Generate random `action_hex`, `seq_no`, `secret_key` combinations
   - Verify no panics, all errors are user-facing
   - Run mutation testing on `signing.rs` to measure test strength

5. **Load test proposal creation**
   - 100 concurrent `/proposals` POST requests with same (seqno, action_hex)
   - Verify exactly 1 succeeds, rest get Conflict
   - Measure response time variance

---

## Summary

The test suite **holds happy-path behavior confidently** but **has critical gaps in negative-path and concurrent access testing**. Most signer-safety risks cluster around:

- **Race conditions** (simultaneous threshold crossing)
- **Error handling** (backend down, RPC unavailable, malformed input)
- **Frontend absence** (React untested entirely)
- **IPC contracts** (Tauri commands untested in isolation)

The adversarial lens surfaces **3 blocking issues** (B1–B3) that directly threaten signer integrity (race conditions, frontend errors, IPC contracts), **3 high-severity gaps** (H1–H3) in critical paths (broadcast, ASM integration, signature verification), and **2 medium issues** (M1–M2) that are surprising but less immediately dangerous.

**Estimated effort to close all gaps: 10–15 days of focused testing work** (smallest fixes first, largest bets deferred). **Highest ROI in first week:** add concurrent approval tests + broadcast error paths + signature verification negatives + frontend test suite.

---

**Generated:** 2026-05-13 by Adversarial Test Reviewer  
**Methodology:** nw-test-optimization skill + nw-tdd-methodology skill + port-to-port testing principles
