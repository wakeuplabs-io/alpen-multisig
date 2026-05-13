# Rust Backend (orchestrator-be) — Adversarial Assessment

## Scope & threat model

**What we're trying to break:**
- Authority isolation: can signers of one multisig view/modify proposals belonging to other authorities?
- Idempotency: can duplicate proposals be exploited to corrupt state or bypass rate limits?
- Session security: can tokens be replayed, stolen, or reused across authorities?
- Locking safety: do RwLock poisoning scenarios cause information leakage or silent failures?
- Broadcast atomicity: race conditions between concurrent broadcast claims?
- Configuration defaults: production secrets accidentally hardcoded in fallback values?
- Error handling and observability: are error messages leaking proposal existence or PII?
- Signature validation boundary: does backend re-implement canonical protocol rules it shouldn't?

**Audit lenses applied:**
- Backend is coordination only per AGENTS.md/PRD; any signature/fee/sequence validation re-implementation is a boundary violation.
- Authority scoping must be strict: per PRD §3, non-signers MUST NOT infer proposal existence.
- Idempotency and duplicate prevention per PRD §4: same `(action, seq_no)` must reject without mutation.
- Error model: `anyhow::Result` for binaries, `thiserror` for libraries.
- Ownership, async/await, locking, panics, unwraps.

---

## Top findings (ranked by severity)

### BLOCKER: F1 — Authority Scope Leakage (`list_proposals` and `get_proposal` handlers)

**Risk:** A signer of Authority A can view ALL proposals across all authorities, bypassing strict isolation required by PRD §3.2.

**Location:** `orchestrator-be/src/handlers/proposals.rs:108–126`

```rust
pub async fn list_proposals(
    State(state): State<AppState>,
    _auth: AuthenticatedSession,  // ← Extractor validates auth but DISCARDS authority!
    Query(query): Query<ListProposalsQuery>,
) -> Result<Json<ProposalListResponse>> {
    let proposals = proposals::list_proposals(state.repo.as_ref(), query.status).await?;
    Ok(Json(ProposalListResponse { proposals }))
}

pub async fn get_proposal(
    State(state): State<AppState>,
    _auth: AuthenticatedSession,  // ← Same issue
    Path(action_id): Path<String>,
) -> Result<Json<Proposal>> {
    let proposal = proposals::get_update_action(state.repo.as_ref(), &ActionId(action_id)).await?;
    Ok(Json(proposal))
}
```

**Failure scenario:**
1. Alice (Strata Admin signer) authenticates and receives a session token.
2. Alice calls `GET /proposals` → backend returns ALL proposals across Alpen Admin, Security Council, etc.
3. PRD §3.2 explicitly states: "A non-signer MUST NOT be able to view any pending proposals or infer the existence of pending proposals." 
4. By extension, a signer of Authority A should NOT see Authority B's proposals.
5. **Result:** Authority scope is globally permissive, not per-signer-set.

**Evidence of breach:**
- `proposals::list_proposals(state.repo.as_ref(), query.status)` at line 113 has NO authority filter.
- Repository trait `list_by_status()` at `src/application/traits.rs:26` takes only status, not authority.
- Even `approve_action()` at `src/handlers/proposals.rs:142–154` correctly checks `if proposal.authority != session.authority` (line 83 in application layer), but list/get handlers SKIP this check.

**Smallest fix:**
1. Modify `list_by_status()` trait to accept optional `authority` parameter.
2. Update both handlers to pass `Some(auth.authority)` when calling repo.
3. Update in-memory and postgres repos to filter by authority.
4. Add test: same signer token cannot list Authority B proposals.

**Largest bet:**
- Full authority scoping refactor across all endpoints (list, get, broadcast preparation).
- Audit every handler to verify `auth.authority` is used in business logic.

**Disconfirming probe (REJECTED):** I searched for "authority" usage in list_proposals handler and found the extractor correctly extracts authority into `auth.authority`, but the handler does NOT use it. This is not a misunderstanding of the code; it is a missing security check.

---

### HIGH: F2 — Default Operator Secret Key Hardcoded to Deterministic Test Value

**Risk:** Production backend falls back to test secret key if `OPERATOR_SECRET_KEY_HEX` environment variable is missing. This key is used to sign all reveal transactions, compromising signer authority.

**Location:** `orchestrator-be/src/config.rs:56–61`

```rust
operator_secret_key_hex: std::env::var("OPERATOR_SECRET_KEY_HEX").unwrap_or_else(
    |_| {
        // Deterministic test key (32 bytes, value = 1); override in production.
        "0000000000000000000000000000000000000000000000000000000000000001".to_string()
    },
),
```

**Failure scenario:**
1. DevOps deploys backend to production but forgets to set `OPERATOR_SECRET_KEY_HEX`.
2. Backend silently falls back to `0x0000...0001`.
3. All reveal transactions are signed with this publicly-known test key.
4. An attacker observes the commit transaction, computes the reveal script, and can forge a competing reveal (double-spending the commit UTXO to a different address).
5. Result: signer authority is bypassed, funds stolen.

**Evidence:**
- Config file has explicit comment saying "override in production" but provides no enforcement.
- No validation that the key is NOT the test value.
- Similar issue with `bitcoin_magic_bytes_hex` at line 63–64, though magic bytes are less critical (they're protocol-specific, not secret).

**PRD alignment:**
- PRD §2.2 states backend "MUST NOT be a single point of failure for the ability of signers to execute valid administrative updates."
- A compromised operator key is a single point of failure.

**Smallest fix:**
1. Remove the fallback for `OPERATOR_SECRET_KEY_HEX`; make it mandatory (no `.unwrap_or_else()`).
2. Add a validation check: if the key equals the test key, return a loud error.

**Largest bet:**
- Implement a key rotation mechanism (not in scope here, but blocks production readiness).
- Use a secrets management vault (e.g., AWS Secrets Manager, HashiCorp Vault) instead of env vars.

**Disconfirming probe (REJECTED):** I checked if the key is used elsewhere with warnings or protection. Found only basic hex decode/slice construction; no safeguards against test key usage.

---

### HIGH: F3 — List/Get Proposals Leak Existence to Non-Signers via 404 vs 200

**Risk:** A non-signer can enumerate all `action_id` values and determine which proposals exist, inferring governance activity.

**Location:** `orchestrator-be/src/handlers/proposals.rs:118–126`, returns 404 NotFound when proposal missing.

```rust
pub async fn get_proposal(
    State(state): State<AppState>,
    _auth: AuthenticatedSession,
    Path(action_id): Path<String>,
) -> Result<Json<Proposal>> {
    let proposal = proposals::get_update_action(state.repo.as_ref(), &ActionId(action_id)).await?;
    Ok(Json(proposal))
}
```

**Failure scenario:**
1. Eve (non-signer, or signer of different authority) authenticates (she IS a signer of SOME authority).
2. Eve loops through `action_id` values (sequential SHA256 hashes, or brute-force guesses if she knows `seq_no` and `action_hex`).
3. Eve receives:
   - 200 OK + full proposal body → proposal exists in her authority
   - 404 NOT_FOUND → proposal may exist in another authority or doesn't exist at all
   - 401 UNAUTHORIZED → could mean missing/expired token (caught at extractor level)
4. With authority mixing (F1), Eve can see all 404s are from AuthA, all 200s from AuthB. **Result:** she infers the existence and distribution of governance activity.

**Evidence:**
- `get_proposal()` calls `get_update_action()` which returns `AppError::NotFound` (line 128 in `src/application/proposals.rs`).
- `AppError::NotFound` maps to HTTP 404 (line 14 in `src/error.rs`).
- No check that proposal belongs to authenticated signer's authority before responding 200.

**PRD alignment:**
- PRD §3.2: "A non-signer MUST NOT be able to view any pending proposals or **infer the existence** of pending proposals."

**Smallest fix:**
1. On 404, return 401 UNAUTHORIZED (fail-closed) instead of 404.
2. Only return 200 if `proposal.authority == auth.authority`.
3. On authority mismatch, return 401 UNAUTHORIZED, not 404.

**Interaction with F1:** F1 makes this worse because Eve can call `list_proposals` without authority filtering and see everything anyway. Fix F1 first; this is a secondary hardening.

---

### HIGH: F4 — RwLock Poisoning Silent Failure Risk (Auth Challenge/Session Storage)

**Risk:** If a panic occurs while holding a write lock on challenges or sessions HashMap, the lock becomes poisoned. Subsequent read operations silently fail with generic "lock poisoned" error, returning 500. This can lead to denial of service or undetected state inconsistency.

**Location:** `orchestrator-be/src/handlers/auth.rs:67–71, 94–97, 142–146, 164–167`

```rust
state
    .challenges
    .write()
    .map_err(|_| AppError::Internal(anyhow::anyhow!("challenge lock poisoned")))?
    .insert(challenge_id.clone(), challenge);
```

**Failure scenario:**
1. A challenge is stored in the RwLock-protected HashMap.
2. A panic occurs during `.insert()` (e.g., OOM, but RwLock is not Panic-Safe by default in all scenarios).
3. Lock becomes poisoned.
4. All subsequent auth operations fail with 500 "internal error" because lock cannot be re-acquired.
5. Backend becomes unavailable for auth even though data is intact.

**Evidence:**
- `state.rs:15–16` use `Arc<RwLock<HashMap<...>>>`.
- Multiple handlers (auth.rs) call `.write().map_err()` and `.read().map_err()`.
- `auth_session.rs:35–38` also does the same for sessions.
- No recovery mechanism (no auto-unlock or poison check).

**Technical context:**
- RwLock::write() returns `Result<RwLockWriteGuard<T>, PoisonError<...>>`.
- Poisoning occurs if a thread panics while holding the lock.
- If a lock is poisoned, future lock acquisitions fail (intentionally, to signal the lock is unsafe).
- Code currently treats poisoning as an internal error and returns 500.
- For in-memory storage, this is acceptable (backend restart clears the state), but it's a liveness issue.

**Smallest fix:**
1. Use `.unwrap()` instead of `.map_err()` — let panics propagate (intentional fail-fast). OR
2. Use a separate crate like `parking_lot::RwLock`, which is panic-safe and never poisons. OR
3. Replace in-memory storage with durable storage (PostgreSQL) for auth sessions (in progress, see F5).

**Largest bet:**
- Replace all in-memory auth state with database-backed sessions, eliminating in-memory RwLock entirely.

**Disconfirming probe (ACCEPTED):** RwLock poisoning is intentional Rust safety feature, not a hidden bug. However, in request paths, poisoning should either:
- Never occur (ensure no panics while holding locks), OR
- Be handled gracefully (failover or restart).
Current code does not guarantee either. If the issue manifests in production, the backend silently degrades.

---

### HIGH: F5 — In-Memory Repository Non-Persistence; Silent Data Loss on Restart

**Risk:** When `DATABASE_URL` is not configured, the backend silently falls back to in-memory storage with no durability. A crash or restart results in total data loss, breaking the offline-fallback guarantee in PRD §2.3.

**Location:** `orchestrator-be/src/main.rs:90–104`

```rust
} else {
    tracing::warn!("DATABASE_URL not set — using in-memory storage (data will not persist)");
    let repo = Arc::new(infrastructure::memory_repo::InMemoryProposalRepository::new());
    // ... AppState uses in-memory repo
}
```

**Failure scenario:**
1. DevOps deploys backend without configuring `DATABASE_URL`.
2. Signers create and approve proposals offchain.
3. Backend crashes (OOM, network issue, deployment update).
4. All proposals are lost; signers cannot see what they were signing.
5. PRD §2.3 states: "In the event that the backend becomes unavailable, signers MUST still be able to... Aggregate signatures manually, ... Broadcast transactions directly to Bitcoin."
6. **Result:** Signers cannot aggregate signatures because they don't know which proposal to sign; the proposal state is gone.

**Evidence:**
- `main.rs:90` logs a warning but continues with in-memory repo.
- No enforcement that production deploys MUST have `DATABASE_URL` set.
- In-memory repo at `src/infrastructure/memory_repo.rs` uses `HashMap<ActionId, Proposal>` backed by `RwLock`.
- On restart, the HashMap is empty.

**PRD alignment:**
- PRD §2.2: "The backend MUST NOT be a single point of failure for the ability of signers to execute valid administrative updates."
- PRD §2.3: "signers MUST still be able to construct valid approval or cancellation transactions, aggregate signatures manually, broadcast transactions directly to Bitcoin."
- Offline fallback requires signers to know what proposals exist. Without durable storage, this is impossible.

**Smallest fix:**
1. Make `DATABASE_URL` mandatory (remove `.ok()`, fail startup if not set).
2. Run migrations to ensure schema exists.

**Largest bet:**
- Implement read-only mode: if database connection fails at startup, log an error and exit (do not fall back to in-memory).

**Disconfirming probe (REJECTED):** The warning log is present but provides false confidence. A production operator might see the warning, not realize it's a critical issue, and leave `DATABASE_URL` unset.

---

### MEDIUM: F6 — No Authority Validation on Challenge Creation

**Risk:** A user can request an auth challenge for any authority (including ones they are not a signer of), and the challenge is issued without validation. Signature verification against the wrong authority may accept an invalid signer.

**Location:** `orchestrator-be/src/handlers/auth.rs:45–79`

```rust
pub async fn auth_challenge(
    State(state): State<AppState>,
    Json(body): Json<StartAuthChallengeRequest>,
) -> Result<Json<AuthChallengeResponse>> {
    let authority = body.authority;  // ← No validation that authority exists
    // ... create challenge for requested authority
}
```

**Failure scenario:**
1. Alice can request a challenge for a non-existent or misspelled authority.
2. If the authority doesn't exist in ASM state, the subsequent `auth_verify` step should fail (during `is_signer_member_for_authority`).
3. However, challenge generation is stateless; no validation is done until verify time.
4. This creates a possible attack vector if ASM state is not always available.

**Evidence:**
- `auth_challenge()` takes `body.authority` and creates a challenge without validating the authority exists in ASM state.
- Authority validation happens at `auth_verify` time (line 125) when calling `asm_role_membership::is_signer_member_for_authority()`.
- If ASM RPC is unavailable at verify time but was available at challenge time, the backend may accept an invalid challenge.

**Test evidence:** `test_auth_verify_unmapped_authority_is_fail_closed()` at line 208 in `src/handlers/mod.rs` shows that an unmapped authority fails at verify time (returns 400 BadRequest), not at challenge time. This is correct fail-closed behavior, but challenge generation should also validate.

**Smallest fix:**
1. In `auth_challenge()`, call `asm_role_membership` to validate that the requested authority exists in ASM state.
2. Return 400 BadRequest if authority is not found.

**Largest bet:**
- Cache the authority list from ASM state on backend startup and validate challenges against the cache.

---

### MEDIUM: F7 — Challenge Expiry Not Enforced on Challenge Retrieval

**Risk:** A challenge can be reused or extended in validity if expiry is checked only at verify time, not at retrieval time. If an attacker intercepts the challenge and delays verification, the window may close but challenges accumulate in memory.

**Location:** `orchestrator-be/src/handlers/auth.rs:93–123`

```rust
let challenge = challenges
    .get_mut(&body.challenge_id)
    .ok_or(AppError::Unauthorized)?;
if challenge.consumed || now > challenge.expires_at_unix_ms {
    return Err(AppError::Unauthorized);
}
```

**Failure scenario:**
1. Alice requests a challenge at time T0 (expires at T0 + 120s).
2. Challenge is issued; Alice does not submit verification immediately.
3. At time T0 + 150s (after expiry), Alice submits verification with the expired challenge ID.
4. Backend correctly rejects (line 101–102).
5. Challenge is marked `consumed = true` even though it was expired.
6. Challenges HashMap accumulates expired entries; there is no garbage collection.
7. Over time, memory usage grows unbounded if clients request many challenges.

**Evidence:**
- `challenges: Arc<RwLock<HashMap<String, PendingAuthChallenge>>>` has no cleanup mechanism.
- `challenge.consumed` is set to `true` (line 121) regardless of expiry status.
- Expired challenges remain in the HashMap forever.

**Memory DoS risk:**
- If an attacker sends many `auth_challenge` requests, the HashMap grows indefinitely.
- Each request allocates a challenge with 16-byte random ID and expires after 120s.
- With no cleanup, memory grows by ~100 bytes per request (challenge struct + HashMap overhead).
- At 100 req/s, memory grows by 10 MB per second → OOM in minutes.

**Smallest fix:**
1. Add a cleanup method that removes expired challenges from the HashMap.
2. Call cleanup in a background task (e.g., every 60s) or on challenge retrieval.
3. Optionally, use a TTL cache (crate: `moka` or `cached`) instead of raw HashMap.

**Largest bet:**
- Replace in-memory challenge storage with PostgreSQL (same as sessions in F5).

---

### MEDIUM: F8 — Broadcast Status Transitions Not Atomic (Race Between Claim and Do-Broadcast)

**Risk:** Between `claim_broadcast()` and `do_broadcast()`, the proposal can be modified by another concurrent request, leading to inconsistent state or double-broadcast attempts.

**Location:** `orchestrator-be/src/application/proposals.rs:234–289`

```rust
let proposal = repo.claim_broadcast(action_id).await?;  // ← Atomic claim

// --- Derive broadcast artifacts (not atomic, can be interrupted) ---
let canonical_keys = ordered_keys_for_authority(...).await?;  // RPC call
let sighash = compute_sighash_for_proposal(&proposal)?;
let payload = broadcast_tx::build_signed_payload_bytes(...)?;
// ... more computation

let result = do_broadcast(...).await;  // ← Actual broadcast (not atomic)
```

**Failure scenario:**
1. Thread A calls `broadcast_commit_then_reveal()` and claims broadcast successfully (proposal.broadcast_status = CommitBroadcasted).
2. Thread A is computing sighash, making ASM RPC calls (lines 257–265).
3. Thread B concurrently calls `approve_action()` and adds a new signature to the same proposal.
4. Thread A resumes and broadcasts with signature set A.
5. Thread B's signature is added AFTER broadcast, but `broadcast_status` is already set.
6. Result: proposal shows broadcast_status=CommitBroadcasted but signatures list was mutated after claim (inconsistent state).

**Evidence:**
- `claim_broadcast()` at line 254 is atomic (CAS-like operation in memory_repo and postgres_repo).
- But `do_broadcast()` at line 272–289 is NOT atomic; it's a long async sequence.
- If `add_signature()` is called between claim and broadcast, the proposal's signatures list is mutated.
- The broadcasted transaction includes signatures from the point of claim, but the proposal object reflects later signatures.

**Test:** No test exists for concurrent broadcast + signature addition. This is a gap in test coverage.

**Smallest fix:**
1. Add a `locked` flag to Proposal (or use a separate "locked for broadcast" state).
2. Prevent `add_signature()` from succeeding if proposal is locked.
3. Unlock after broadcast completes (success or failure).

**Largest bet:**
- Implement optimistic locking using a version field (`version: u64`) on Proposal.
- Compare-and-swap on update to prevent concurrent modifications.

**Disconfirming probe (REJECTED):** I checked if there's a documented ordering guarantee between claim and broadcast. There isn't. The code assumes sequential execution, but Tokio's async runtime can interleave tasks.

---

### MEDIUM: F9 — Missing Authority Check on Broadcast Operations

**Risk:** A signer can initiate broadcast for a proposal belonging to a different authority if the signer's token somehow has cross-authority scope (defense-in-depth failure from F1).

**Location:** `orchestrator-be/src/handlers/proposals.rs:156–212`

```rust
pub async fn prepare_broadcast(
    State(state): State<AppState>,
    _auth: AuthenticatedSession,  // ← Extractor validates token exists, but handler doesn't check authority
    Path(action_id): Path<String>,
) -> Result<Json<PrepareBroadcastResponse>> {
    // ... no check that auth.authority matches proposal.authority
}

pub async fn execute_broadcast(
    State(state): State<AppState>,
    _auth: AuthenticatedSession,  // ← Same
    Path(action_id): Path<String>,
) -> Result<Json<BroadcastResponse>> {
    // ... no check that auth.authority matches proposal.authority
}
```

**Failure scenario:**
1. Alice (Strata Admin) authenticates and gets a token for Strata Admin authority.
2. Alice calls `POST /proposals/:action_id/broadcast` for an Alpen Admin proposal.
3. Handler does NOT verify that Alice's authority matches the proposal's authority.
4. Result: Alice broadcasts an Alpen Admin proposal without authorization.

**Evidence:**
- Both handlers receive `_auth: AuthenticatedSession` but never use `auth.authority`.
- No call to check `if proposal.authority != auth.authority`.
- Compare to `create_proposal()` (line 81–92) which correctly builds SessionContext with authority.

**Mitigation from F1:** If F1 is fixed and `get_proposal()` returns 401 on authority mismatch, this handler will fail at proposal fetch time. But it's still a missing check.

**Smallest fix:**
1. Fetch the proposal first.
2. Check `if proposal.authority != auth.authority` before proceeding.
3. Return 401 UNAUTHORIZED if mismatch.

---

### MEDIUM: F10 — Idempotency of Create-Proposal Vulnerable to Data Hiding (Partial)

**Risk:** The `create_update_action()` function correctly rejects duplicate ActionId (line 61 in `src/application/proposals.rs`), but only AFTER saving to repo. If the save fails partway (e.g., network partition in PostgreSQL), a half-written proposal could be left in the database, and retry with identical inputs would fail.

**Location:** `orchestrator-be/src/application/proposals.rs:30–64`

```rust
pub(crate) async fn create_update_action(...) -> Result<Proposal, AppError> {
    // ... validation ...
    let proposal = Proposal { ... };
    repo.save_proposal(proposal.clone()).await?;  // ← If this partially succeeds, retry fails
    Ok(proposal)
}
```

**Failure scenario:**
1. Request 1: Create proposal with ActionId X.
2. Backend calls `repo.save_proposal()` (line 61).
3. PostgreSQL saves the proposal, but the response packet is lost.
4. Timeout; client retries with identical request.
5. Request 2: Create proposal with ActionId X again.
6. `repo.save_proposal()` now fails with Conflict (ActionId already exists).
7. Client receives error; proposal creation is not idempotent from the client's perspective.

**Evidence:**
- `save_proposal()` is an async operation that can fail midway.
- No transactional wrapper or idempotency key mechanism.
- Check is done at repo level (`if proposals.contains_key()` at memory_repo.rs:32), but this is after the insert.

**Mitigation:** PostgreSQL does have UNIQUE constraints that prevent double-insert, but the error is returned to the client as Conflict, not as "already created, here's the proposal." This is correct fail-closed behavior but not idempotent.

**Smallest fix:**
1. In `save_proposal()`, catch UNIQUE constraint violation and return the existing proposal instead of error (read-after-write).
2. Return the existing proposal to the client (idempotent).

**Largest bet:**
- Implement request-scoped idempotency keys (client sends `idempotency_key` header, backend deduplicates).

**Disconfirming probe (ACCEPTED):** Idempotency is a "nice-to-have" for resilience, not a critical security issue. The system is fail-safe (duplicates are rejected), which is correct per PRD §4.

---

### LOW: F11 — Signature Validation Not Enforced on Proposal Approval

**Risk:** The `approve_action()` handler accepts a `signature_hex` but does NOT validate that the signature is valid for the challenge or payload. The signature is stored as-is and later used in broadcast.

**Location:** `orchestrator-be/src/handlers/proposals.rs:128–154`

```rust
pub async fn approve_action(
    State(state): State<AppState>,
    auth: AuthenticatedSession,
    Path(action_id): Path<String>,
    Json(body): Json<ApproveActionRequest>,
) -> Result<Json<Proposal>> {
    // ... no validation that body.signature_hex is a valid signature for anything
    let sig = ProposalSignature {
        signer_pubkey: body.signer_pubkey,
        signature_hex: body.signature_hex,  // ← Accepted without validation
    };
    let proposal = proposals::approve_action(..., &sig).await?;
    Ok(Json(proposal))
}
```

**Failure scenario:**
1. Alice submits a signature for a proposal, but the signature_hex is garbage or invalid.
2. Backend accepts it and stores it.
3. Later, when attempting to broadcast, the signature is passed to the onchain protocol.
4. The protocol rejects it, causing broadcast failure.
5. This is caught onchain (correct), but backend wasted cycles and stored invalid data.

**Evidence:**
- `approve_action()` calls `proposals::approve_action()` which stores the signature without validation.
- The signature is not validated until broadcast time (at `broadcast_tx::build_signed_payload_bytes()`).
- No unit test validates that invalid signatures are rejected at approve time.

**Mitigation:** Backend is coordination-only per AGENTS.md. Signature validation belongs at broadcast time, not at collection time. This is CORRECT per the protocol spec. Invalid signatures are caught by the onchain protocol.

**Smallest fix:**
- No fix needed. This is by design.

**Why I'm flagging it:**
- There's a possible **hygiene check** (structural validity) that COULD be done at approve time to fail-fast.
- Validating signature format (compact ECDSA, correct length) would prevent obvious garbage.

**This is MEDIUM priority for robustness, not a security issue.**

---

### LOW: F12 — Session Token Format Is Random Hex, No Expiry Embedded

**Risk:** Session tokens are 32-byte random hex strings with expiry stored only in backend memory (RwLock HashMap). If RwLock is poisoned or corrupted, tokens may remain valid indefinitely (if not re-accessed).

**Location:** `orchestrator-be/src/handlers/auth.rs:135–146`

```rust
let token = auth_crypto::random_hex(32);  // ← Random 64-char hex, no claims
let expires_at_unix_ms = now + state.auth_session_ttl_ms;
let session = AuthSession { ... };
state.sessions.write()...insert(token.clone(), session);
```

**Failure scenario:**
1. Alice obtains a session token (64-char hex).
2. Token is valid for 240 seconds.
3. Backend crashes at 100 seconds; RwLock HashMaps are cleared.
4. Alice's token is now orphaned (no entry in HashMap).
5. Subsequent requests with the token fail with 401 UNAUTHORIZED (correct).
6. But if backend recovers AND the token happens to be re-created with identical bits (collides), it could be valid again (extremely low probability, essentially zero).

**Evidence:**
- Tokens are opaque random values; no JWT or signature.
- Expiry is stored server-side only; there's no embedded claim.
- This is a standard approach for server-side sessions (not a vulnerability per se).

**Why I'm flagging it:**
- No protection against replay in case of clock skew or time-travel attacks.
- If system time is adjusted backward, tokens could regain validity.

**Smallest fix:**
- Store token creation timestamp and add a check that `created_at` is not in the future (clock-skew detection).

**Largest bet:**
- Replace random tokens with JWTs (self-signed, containing authority + expiry + nonce).
- Eliminates server-side session storage (stateless).

**Note:** This is VERY LOW priority for a private backend operated by Alpen Labs. Server time integrity is assumed. If this is a concern, upgrade to JWTs.

---

## Attack narratives

### Narrative 1: Authority Cross-Contamination

**The attacker:** Carol, a Security Council signer with access to one authority.

**The attack:**
1. Carol authenticates and receives a session token for Security Council.
2. Carol calls `GET /api/v1/proposals` → receives ALL proposals across all authorities (F1).
3. Carol sees a pending Alpen Admin proposal and deduces the governance action.
4. Carol is not an Alpen Admin signer, but she can call `GET /api/v1/proposals/:action_id` to read its details (F1 + F3).
5. Carol observes that other Alpen Admin signers are approving, and broadcasts to a front-running service.
6. **Impact:** Confidentiality breach; governance activity is exposed to unauthorized signers.

**Required fixes:** F1, F3, F9.

---

### Narrative 2: Production Deployment with Test Operator Key

**The attacker:** Dave, a Bitcoin L1 observer who knows the test operator key is `0x0000...0001`.

**The attack:**
1. Alpen Labs deploys the backend without setting `OPERATOR_SECRET_KEY_HEX`.
2. Backend silently uses the test key (F2).
3. Dave monitors the Bitcoin network and observes a commit transaction with the known commit address.
4. Dave computes the reveal script (it's deterministic from the commit and the test key).
5. Dave creates a competing reveal transaction that spends the same commit UTXO to a different address (e.g., his own).
6. Dave broadcasts his reveal slightly before the backend's reveal.
7. **Impact:** Signer authority is bypassed; funds are stolen from the multisig.

**Required fixes:** F2.

---

### Narrative 3: Denial of Service via Challenge Spam

**The attacker:** Eve, an automated bot.

**The attack:**
1. Eve calls `POST /api/v1/auth/challenge` 10,000 times per second.
2. Each challenge allocates ~100 bytes in the challenges HashMap.
3. After 10 seconds, 1 MB is allocated; after 100 seconds, 10 MB.
4. The HashMap never garbage-collects expired challenges (F7).
5. After 5 minutes, 50 MB is consumed.
6. After 1 hour, 3.6 GB is consumed → OOM.
7. Backend crashes.
8. **Impact:** Availability; honest users cannot authenticate.

**Required fixes:** F7 (cleanup) or F5 (move to durable storage).

---

### Narrative 4: Race Between Signature Addition and Broadcast

**The attacker:** Frank, a signer with access to legitimate proposals.

**The attack:**
1. Proposal P has 2 signatures; threshold is 3.
2. Signer X initiates `POST /broadcast` to prepare the broadcast.
3. Signer Y simultaneously calls `POST /approve` to add the 3rd signature.
4. Backend's broadcast thread claims the broadcast lock (F8).
5. But signature addition is not blocked; Signer Y's signature is added to the proposal.
6. Broadcast proceeds with 2 signatures (from the point of claim), not 3.
7. Broadcast fails onchain (insufficient signatures).
8. User sees `broadcast_status = CommitBroadcasted` but proposal has 3 signatures in the database (inconsistent state).
9. **Impact:** State corruption; operational confusion.

**Required fixes:** F8 (lock during broadcast).

---

### Narrative 5: Information Leakage via HTTP Status Codes

**The attacker:** Grace, a Security Council signer (different authority from Alpen Admin).

**The attack:**
1. Grace authenticates as Security Council.
2. Grace collects a list of possible `action_id` values (by guessing seq_no + action_hex, or by seeing them in logs).
3. For each action_id, Grace calls `GET /api/v1/proposals/:action_id`.
4. Responses:
   - 200 OK → proposal exists in her authority
   - 404 NOT_FOUND → proposal exists in another authority or doesn't exist
   - (Note: with F1 fixed, all 404s are from other authorities)
5. Grace cross-references against known seqno values and infers the Alpen Admin governance timeline.
6. **Impact:** Confidentiality breach; governance timing is exposed.

**Required fixes:** F3 (return 401 on authority mismatch instead of 404).

---

## Evidence index (paths)

| Finding | File | Line(s) | Issue |
|---------|------|---------|-------|
| F1 | `src/handlers/proposals.rs` | 108–126 | list_proposals, get_proposal missing authority filter |
| F1 | `src/application/traits.rs` | 26 | list_by_status() trait missing authority parameter |
| F1 | `src/application/proposals.rs` | 132–137 | list_proposals() no authority filtering |
| F2 | `src/config.rs` | 56–61 | operator_secret_key_hex fallback to test value |
| F3 | `src/error.rs` | 14 | NotFound maps to HTTP 404 |
| F3 | `src/handlers/proposals.rs` | 118–126 | get_proposal returns 404 instead of 401 on authority mismatch |
| F4 | `src/handlers/auth.rs` | 67–71, 94–97, 142–146, 164–167 | RwLock.write().map_err() poisoning handling |
| F4 | `src/state.rs` | 15–16 | `Arc<RwLock<HashMap>>` for challenges and sessions |
| F5 | `src/main.rs` | 90–104 | Database fallback to in-memory storage |
| F6 | `src/handlers/auth.rs` | 45–79 | auth_challenge no validation of authority existence |
| F7 | `src/handlers/auth.rs` | 94–123 | Challenge expiry checked at verify, not cleaned up |
| F7 | `src/state.rs` | 15 | HashMap<String, PendingAuthChallenge> with no TTL or cleanup |
| F8 | `src/application/proposals.rs` | 234–289 | claim_broadcast then do_broadcast race window |
| F8 | `src/application/traits.rs` | 31–35 | claim_broadcast() is atomic, but do_broadcast is not |
| F9 | `src/handlers/proposals.rs` | 156–177, 180–212 | prepare_broadcast, execute_broadcast missing authority check |
| F10 | `src/application/proposals.rs` | 61 | save_proposal() not idempotent on retry |
| F11 | `src/handlers/proposals.rs` | 128–154 | approve_action no signature format validation |
| F12 | `src/handlers/auth.rs` | 135–146 | session tokens are opaque random hex, no expiry claim |

---

## Smallest fixes vs largest bets

### Smallest Fixes (1–2 hours each)

1. **F1:** Add authority parameter to `list_by_status()` trait and filter in handlers.
2. **F2:** Remove fallback for `OPERATOR_SECRET_KEY_HEX`; make mandatory.
3. **F3:** Return 401 UNAUTHORIZED instead of 404 on authority mismatch in get_proposal.
4. **F6:** Validate authority in `auth_challenge()` against ASM state.
5. **F9:** Check authority in `prepare_broadcast()` and `execute_broadcast()`.

### Medium Fixes (4–8 hours)

1. **F7:** Implement challenge cleanup (background task or TTL cache).
2. **F8:** Add broadcast-lock state to prevent signature addition during broadcast.
3. **F10:** Read-after-write on UNIQUE constraint violation to achieve idempotency.
4. **F11:** Add signature format validation (compact ECDSA, length check) at approve time.

### Largest Bets (1–2 weeks)

1. **F5:** Migrate auth session storage from RwLock HashMap to PostgreSQL.
2. **F4:** Replace RwLock with `parking_lot::RwLock` or move to durable storage.
3. **F12:** Replace random tokens with JWTs; make sessions stateless.
4. **F8:** Implement optimistic locking (version field) on Proposal.

---

## What would change my mind

### Missing Evidence (need to verify)

1. **Authority scoping in list/get:** I saw that `_auth` is discarded in the handler, but did not trace the full repository query path. If there's a secret authority filter in the repo layer, F1 would be a WONT_FIX.

   **Verification needed:** Read `src/infrastructure/postgres_repo.rs` fully and search for WHERE authority in list_by_status() and find_by_action_id().

2. **CORS and public access:** I saw `allow_origin(Any)` in CORS but did not check if there's an additional auth middleware. If requests are entirely blocked at middleware level, then authority scope in handlers may be defense-in-depth only.

   **Verification needed:** Check all routes; confirm all proposal endpoints require AuthenticatedSession.

3. **RwLock poisoning in practice:** RwLock poisoning is a documented Rust safety feature, not a latent bug. I classified it as HIGH because in-memory storage is not production-ready anyway (F5 is the real fix). If the backend is already migrated to database-backed sessions, F4 is MOOT.

   **Verification needed:** Confirm database URL is always set in production config.

4. **Signature validation scope:** I flagged F11 as LOW because signature validation belongs onchain, not offchain. But PRD §1.5 states backend MAY perform "basic hygiene checks." Validating signature format (compact ECDSA format, 64 bytes) is hygiene, not canonical validation. Confirming the signature is mathematically valid would be canonical validation (belongs onchain).

   **Verification needed:** Re-read PRD §1.5 for the explicit list of allowed hygiene checks.

### Experiments That Would Invalidate Findings

1. **F1 Authority Leakage:** Write a test that:
   - Authenticates as Strata Admin.
   - Creates a proposal for Strata Admin.
   - Authenticates as Alpen Admin.
   - Calls `GET /proposals` → verify NO proposals from Strata Admin are returned.
   - **If test fails:** F1 is CONFIRMED BLOCKER.
   - **If test passes:** Investigate repo layer; authority filter exists but is hidden.

2. **F2 Test Key Usage:** In production config:
   - Comment out `OPERATOR_SECRET_KEY_HEX`.
   - Start backend; check logs and crash signals.
   - **If backend starts silently:** F2 is CONFIRMED HIGH.
   - **If backend fails to start:** F2 is already mitigated.

3. **F3 Existence Leakage:** Write a test that:
   - Creates a proposal.
   - As a non-signer (or signer of different authority), calls `GET /proposals/:action_id` on both existing and non-existing IDs.
   - Verify status codes are identical (both 401 or both 404).
   - **If status codes differ:** F3 is CONFIRMED MEDIUM.

4. **F5 Data Loss:** Write a test that:
   - Comment out `DATABASE_URL`.
   - Create proposals.
   - Restart backend.
   - Verify proposals are lost.
   - **If proposals are lost:** F5 is CONFIRMED HIGH.
   - **If proposals persist:** Investigate database fallback logic.

5. **F7 Memory Growth:** Load test:
   - Send 1,000 `auth/challenge` requests per second for 60 seconds.
   - Monitor backend memory usage.
   - **If memory grows unbounded:** F7 is CONFIRMED MEDIUM.
   - **If memory stabilizes (cleanup detected):** F7 is already mitigated.

---

## Conclusion

**Overall Security Posture:** CRITICAL ISSUES BLOCKING PRODUCTION

The backend has **5 Blockers (F1, F2)** and **5 High-severity issues (F3–F7)** that must be addressed before production deployment:

1. **Authority scope leakage (F1)** breaks the core security model (signers can see other authorities' proposals).
2. **Hardcoded test operator key (F2)** exposes signer authority to key compromise.
3. **In-memory storage without durability (F5)** breaks the offline fallback guarantee.
4. **Missing authority checks on broadcast (F9)** and **status code leakage (F3)** weaken defense-in-depth.

**Recommended order of fixes:**
1. F1 (authority filtering) — fixes root cause of F3, F9, and narrative 1.
2. F2 (mandatory operator key) — fixes narrative 2.
3. F5 (database durability) — fixes narratives 3 and 4 if auth moves to DB.
4. F3, F6, F7, F8, F9 — defense-in-depth hardening.

**Estimated effort:** 2–3 weeks of coordinated development to achieve production readiness. Current state is pre-alpha.
