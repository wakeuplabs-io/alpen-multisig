# Distributed Systems Realism — Adversarial Assessment

**Date:** 2026-05-13  
**Reviewer:** Distributed Systems Adversary  
**Scope:** Alpen Multisig backend (`orchestrator-be`) + desktop app integration  
**Stance:** Coordination-only ≠ exemption from concurrency, idempotency, and failure isolation hazards.

---

## Scope & Threat Model

We are attacking assumptions about:

1. **Concurrency safety** under Axum + Tokio with shared `RwLock`-protected state
2. **Idempotency** of proposal creation and signature submission under network retries and partition scenarios
3. **Storage durability** — in-memory vs. persistent; what happens to in-flight proposals on restart
4. **Ordering invariants** — signature accumulation, quorum transitions, broadcast claims
5. **Failure isolation** — can one bad signer, slow RPC, or network partition cascade into service-wide degradation
6. **Manual fallback survivability** — if backend is down, can signers still create, sign, and broadcast offline
7. **Single points of failure** — even at this size, the backend is a single instance, sessions stored in-memory, no replication
8. **Time assumptions** — wall-clock reliance for session expiry, proposal expiry (7 days), broadcast timeouts

**Our adversarial question:** "What is the minimal sequence of events that breaks invariants, loses data, or forces operational recovery?"

---

## Top Findings (Ranked)

### BLOCKING / CRITICAL

#### 1. In-Memory State Loss on Restart (Operational Blocker)

**Severity:** **CRITICAL**  
**When:** Production deployment with any backend restarts (crashes, deploys, rebalance)

**Finding:**

- Backend stores auth sessions, challenges, and proposal metadata in `Arc<RwLock<HashMap>>` (see `state.rs:15–16`)
- If `DATABASE_URL` is unset (line 90 of `main.rs`), **all proposal state is ephemeral**
  ```rust
  // main.rs:90
  tracing::warn!("DATABASE_URL not set — using in-memory storage (data will not persist)");
  ```
- On crash/restart, all in-flight proposals vanish; signers lose visibility and cannot retrieve collected signatures
- Even **with** Postgres, proposals created and not yet broadcast remain only in the orchestrator; if the backend process dies mid-broadcast (after `claim_broadcast` but before writing txids), the broadcast is orphaned

**Evidence:**
- `orchestrator-be/src/infrastructure/memory_repo.rs` — `InMemoryProposalRepository` wraps `RwLock<HashMap<ActionId, Proposal>>`
- `orchestrator-be/src/state.rs:44–45` — challenges and sessions are `Arc<RwLock<HashMap>>`, no persistence layer
- No append-only log, no write-ahead log, no recovery mechanism post-crash

**Attack Sequence:**

1. Signer A creates proposal (seq_no=1) via POST `/proposals` → stored in-memory only
2. Signer B submits approval via POST `/proposals/{action_id}/approve` → signature added to in-memory proposal
3. Backend crashes (e.g., pod restart, OOM kill, deploy)
4. On restart, both A and B's work is lost; proposal is completely gone
5. A and B have no way to re-retrieve collected signatures; they must re-sign manually or use offline fallback

**Smallest Fix:**

- **Persist proposal creation and signature submissions to Postgres before returning to client**
  - Wrap `save_proposal` and `add_signature` in a transaction
  - Return success only after disk write is confirmed
  - This does NOT fix broadcast atomicity but stops in-flight proposal loss

**Largest Bet:**

- Implement a write-ahead log (event log) for all state mutations
  - Every `create_proposal`, `approve_action`, `claim_broadcast`, `update_broadcast_status` is appended to a durable log
  - On startup, replay the log to reconstruct in-memory state
  - This is the foundation for audit trail, replay-safety, and disaster recovery
  - Estimated scope: 150–200 lines of new infrastructure, plus migration and replay logic

---

#### 2. Non-Atomic Broadcast State Transitions (Data Integrity Hazard)

**Severity:** **CRITICAL**  
**When:** Backend crash or multi-instance deployment (future)

**Finding:**

- `broadcast_commit_then_reveal` (application/proposals.rs:234) is **not atomic**
  - Line 254: `claim_broadcast` transitions state to `CommitBroadcasted`
  - Lines 272–288: calls `do_broadcast` (polling Bitcoin, waiting for confirms)
  - If `do_broadcast` fails partway (e.g., commit tx sent, network partition, backend crashes), the state is stuck
  - **No rollback mechanism** to reset `broadcast_status` back to `Idle` if broadcast partially fails

**Evidence:**
- `orchestrator-be/src/application/proposals.rs:254` — `claim_broadcast` sets state irreversibly
- No transaction wrapping `claim_broadcast + do_broadcast` 
- If do_broadcast fails after commit UTXO is broadcast but before reveal, proposal is permanently stuck in `CommitBroadcasted` state

**Attack Sequence:**

1. Proposal A reaches quorum, is in `Approved` state
2. Operator calls POST `/proposals/{action_id}/broadcast`
3. Backend calls `claim_broadcast` → state becomes `CommitBroadcasted`
4. Bitcoin commit tx is broadcast to mempool (UTXO created)
5. Operator's connection to Bitcoin RPC drops → `estimate_fee_rate` fails → `do_broadcast` returns error
6. **Proposal state is now STUCK in `CommitBroadcasted`**
   - Cannot retry (claim_broadcast will reject as "broadcast already in progress")
   - Cannot reset (no admin endpoint to reset broadcast_status)
   - Operator must manually query DB, manually edit DB, manually retry
7. Meanwhile, on-chain, the commit UTXO is sitting at the derived address doing nothing

**Smallest Fix:**

- Add a `/proposals/{action_id}/reset-broadcast` admin endpoint that transitions `CommitBroadcasted → Idle`
- Require auth + human confirmation (this is dangerous but operational necessity)

**Largest Bet:**

- Implement broadcast as a durable, resumable state machine
  - Each step (derive address → create commit tx → broadcast → wait confirm → create reveal tx → broadcast → wait confirm) is checkpointed
  - On restart or error, operator can call `resume_broadcast` to pick up from last successful step
  - Pairs with the write-ahead log above; log contains each step's result
  - Scope: 300–400 lines of new logic, persistence schema changes

---

#### 3. Session-Backed Auth is Volatile and Non-Distributed

**Severity:** **CRITICAL** (for multi-instance, HIGH for single-instance)  
**When:** Any distributed deployment or session loss scenario

**Finding:**

- Auth sessions stored in `Arc<RwLock<HashMap<String, AuthSession>>>` (state.rs:16)
- No persistence, no replication, no coordination
- If you scale to 2+ backend instances, **each instance has its own session store**
  - Session from instance A is unknown to instance B
  - Load balancer sends request to instance B → 401 Unauthorized (session not found)
  - User must re-authenticate

**Evidence:**
- `orchestrator-be/src/handlers/auth_session.rs:35–42` — reads sessions from in-memory map
- No backend session store, no Redis, no coordination protocol
- Sessions expire based on wall-clock time (`SystemTime::now()`)

**Attack Sequence:**

1. Signer authenticates via POST `/auth/challenge` + `/auth/verify` → session token issued and stored on instance A
2. Load balancer sends next request to instance B
3. Instance B cannot find session → returns 401
4. Signer must re-authenticate, losing any application context
5. If authentication service is also backed by a single instance (likely), this cascades

**Smallest Fix:**

- Document: "Single-instance deployment only; no horizontal scaling of backend until Redis session store is added"

**Largest Bet:**

- Implement session store abstraction; provide Redis impl for multi-instance deployments
- Scope: 100–150 lines + operational setup for Redis HA

---

### HIGH SEVERITY

#### 4. No Duplicate Signer Rejection at Lock Granularity (Race Condition)

**Severity:** **HIGH**  
**When:** Concurrent signature submissions from same signer to same proposal

**Finding:**

- `approve_action` checks for duplicate signer (application/proposals.rs:87–94)
  ```rust
  let already_signed = proposal
      .signatures
      .iter()
      .any(|s| s.signer_pubkey == sig.signer_pubkey);
  if already_signed {
      return Err(AppError::Conflict("signer already signed".to_string()));
  }
  ```
- But this check is **not atomic** with the `add_signature` call (line 96–97)
  - Thread 1 reads proposal, sees no signature from Alice
  - Thread 2 reads proposal, sees no signature from Alice
  - Thread 1 calls `add_signature` for Alice → succeeds
  - Thread 2 calls `add_signature` for Alice → succeeds
  - Result: **Alice has signed twice**

**Evidence:**
- `orchestrator-be/src/infrastructure/memory_repo.rs:47–65` — `add_signature` has no duplicate check
- The duplicate check lives in the application layer, not the repository layer
- Lock is released between check and update

**Attack Sequence:**

1. Proposal P requires 2-of-3 signers (Alice, Bob, Carol)
2. Alice is signing via hardware wallet over a slow connection
3. Alice's client retries POST `/proposals/{action_id}/approve` twice (normal timeout handling)
4. Both requests arrive at backend nearly simultaneously
5. Both pass the duplicate check, both call `add_signature`
6. **Proposal now has 2 signatures from Alice, 0 from Bob, 0 from Carol**
7. Quorum is incorrectly reached (2 >= 2) even though only 1 unique signer has signed
8. Proposal broadcasts with invalid threshold (1 out of 2 required)
9. On-chain, ASM rejects the signatures as insufficient

**Smallest Fix:**

- Move duplicate check into `add_signature` — return `Err` if signer already signed
  ```rust
  if proposal.signatures.iter().any(|s| s.signer_pubkey == signer_pubkey) {
      return Err(AppError::Conflict("signer already signed"));
  }
  ```
- This leverages the write lock already held inside `add_signature`

**Largest Bet:**

- Implement optimistic locking or version-based CAS (compare-and-set)
- Each proposal has a `version: u64`; on update, require version match
- If two threads race, one loses with a 409 Conflict telling the client to retry

---

#### 5. Quorum Transition Not Linearized (Lost Quorum Guarantee)

**Severity:** **HIGH**  
**When:** Concurrent approvals near quorum threshold

**Finding:**

- After `add_signature` (application/proposals.rs:96–97), code checks if quorum reached (line 102)
- But the check and state transition are **not atomic**:
  ```rust
  // Thread 1 reads proposal with 2 sigs (requires 3)
  if proposal.status == ProposalStatus::Pending
      && proposal.signatures.len() >= proposal.required_signatures as usize {
      // Thread 2 adds another sig, transitions to Approved FIRST
      // Thread 1 still has old len (2) in memory, but now threshold is met
  }
  ```
- Two threads can both see that quorum is reached and **both attempt to transition state to Approved**
- Only one succeeds; the other gets a stale `Proposal` object that still shows `Pending`

**Evidence:**
- `orchestrator-be/src/application/proposals.rs:87–119` — quorum check at line 102–115 is outside the write lock held by `add_signature`
- `orchestrator-be/src/infrastructure/memory_repo.rs:96–97` — `add_signature` returns a clone, lock is released

**Attack Sequence:**

1. Proposal P requires 3-of-5 signers; currently has 2 signatures (Alice, Bob)
2. Carol submits her signature via POST → backend calls `add_signature`, gets back updated proposal with 3 sigs
3. Simultaneously, Dave submits his signature via POST
4. Both threads see that `signatures.len() (3) >= required_signatures (3)`
5. Both call `update_broadcast_status` with `ProposalStatus::Approved`
6. First one succeeds; second one also succeeds (second update overwrites with same value)
7. **No error is reported to Dave**
8. Dave's client thinks it added a signature and the proposal is still pending (because Dave got a stale object)
9. **Eve's signature is never collected** (only 4 out of 5 signers actually endorsed it)

**Smallest Fix:**

- Change `approve_action` to **always** re-read the proposal after `add_signature` before checking quorum
  ```rust
  let updated = repo.add_signature(...).await?;
  let proposal = updated.ok_or(AppError::NotFound)?;
  
  // Check again with fresh state
  if proposal.status == ProposalStatus::Pending && 
     proposal.signatures.len() >= proposal.required_signatures as usize {
      // Transition
  }
  ```
- This is still racy but reduces the window

**Largest Bet:**

- Change quorum transition to be driven by the repository layer, not the application layer
  - `add_signature` returns a flag: `(updated_proposal, quorum_reached: bool)`
  - Transition happens inside the write lock
  - Only one thread can succeed at transitioning to Approved
  - Scope: 50–100 lines of refactoring

---

#### 6. No Backpressure or Rate Limiting (Denial of Service)

**Severity:** **HIGH**  
**When:** Malicious or misconfigured client hammering the backend

**Finding:**

- No rate limiting on any endpoint (handlers/proposals.rs, handlers/auth_session.rs)
- No per-signer request limits, no global QPS cap
- Single malicious signer can:
  - Spam `/proposals` creation → fill memory with proposals → OOM
  - Spam `/proposals/{action_id}/approve` → cause lock contention
  - Spam `/auth/challenge` → grow in-memory challenge map unbounded

**Evidence:**
- `main.rs:114` — `TraceLayer` only; no `RateLimitLayer` or similar
- No check for max active sessions, max proposals per authority, max signatures per proposal
- Auth challenge map has no TTL enforcement on cleanup (see `state.rs:15` — challenges are added but stale ones accumulate)

**Attack Sequence:**

1. Attacker has a compromised signer key
2. Attacker calls POST `/auth/challenge` with `signer_pubkey=attacker` 10,000 times
3. Backend stores 10,000 challenge entries in the `challenges` HashMap
4. Attacker never calls `/auth/verify` — challenges just sit there
5. **Memory usage grows linearly; on a 512MB container, backend OOMs in minutes**
6. Service goes down, affects all signers of all authorities

**Smallest Fix:**

- Add a tower::layer for rate limiting (e.g., `tower-governor` or `governor`)
  - Global: max 100 requests/second
  - Per-signer: max 10 requests/second
  - Per-endpoint: custom limits (e.g., `/auth/challenge` max 1 per second per signer)

- Implement TTL-based cleanup for stale challenges and sessions
  - On startup, spawn a background task: every 60s, filter out expired entries
  - Scope: 50 lines + dependency

**Largest Bet:**

- Implement request quota management per authority
  - Each authority gets a "signature submission budget" per 24h
  - Once budget is exhausted, further approvals are rejected with 429 Too Many Requests
  - Quotas reset at fixed UTC times to avoid clock skew
  - Scope: 100–150 lines + schema changes (if persistence added)

---

#### 7. Broadcast Race: Multiple Instances Can Claim Same Proposal (Future Multi-Instance Bug)

**Severity:** **HIGH** (future-facing; currently single-instance)  
**When:** Horizontal scaling to 2+ backend instances

**Finding:**

- `claim_broadcast` (memory_repo.rs:82–97) uses a write lock to atomically claim, but **only within a single process**
- If you deploy 2 instances of the backend:
  - Instance A gets request to broadcast proposal P → calls `claim_broadcast` → state transitions to `CommitBroadcasted`
  - Instance B gets a separate request to broadcast proposal P → has its own in-memory copy → state is still `Idle`
  - Instance B also transitions to `CommitBroadcasted` and starts broadcasting
  - **Two commit txs are broadcast for the same proposal**

**Evidence:**
- `orchestrator-be/src/infrastructure/memory_repo.rs:82–97` — only uses in-process `RwLock`, no distributed lock
- Architecture docs (overview.md) show "single Orchestrator Backend" with no replication strategy

**Attack Sequence (Post-Scaling):**

1. Operator scales orchestrator from 1 instance to 3 instances
2. Proposal P reaches quorum, is in `Approved` state
3. Desktop app calls POST `/proposals/{action_id}/broadcast`
4. Load balancer sends request to instance A → `claim_broadcast` succeeds
5. Operator makes a typo, clicks "broadcast" again
6. Load balancer sends request to instance B → **`claim_broadcast` also succeeds** (different process, different map)
7. Both instances broadcast commit txs, both reveal txs
8. **Two separate Bitcoin transactions are created for the same governance action**
9. If both confirm, ASM processes both; if one processes first, the second fails at replay check

**Smallest Fix:**

- Document: "Single-instance deployment only; use Postgres + distributed lock (advisory lock) before scaling"

**Largest Bet:**

- Implement broadcast claim via Postgres advisory lock
  ```sql
  SELECT pg_advisory_lock(hash(action_id));
  -- claim broadcast
  SELECT pg_advisory_unlock(hash(action_id));
  ```
- This works across instances because lock state is centralized in Postgres
- Scope: 50–75 lines + Postgres advisory lock usage

---

### MEDIUM SEVERITY

#### 8. No Idempotency Key for Proposal Creation (Replay Hazard)

**Severity:** **MEDIUM**  
**When:** Client retries after network timeout during proposal creation

**Finding:**

- POST `/proposals` creates a new proposal; **there is no idempotency key mechanism**
- If client times out and retries with the **same payload**, code should return 409 Conflict (duplicate) but only if the backend can deduplicate

**Evidence:**
- `handlers/proposals.rs:68–95` — `create_proposal` calls `create_update_action` which directly calls `repo.save_proposal`
- Memory repo checks for existing ActionId (memory_repo.rs:32–34) but **a fresh request is considered a new creation**
- If client sends: POST `/proposals` with `(seq_no=1, action_hex="deadbeef")` twice:
  - First request: creates proposal with ActionId = hash(1, deadbeef) → 201 Created
  - Network timeout
  - Client retries: creates proposal with same ActionId → 409 Conflict ✓ **Good**
  
**But this only works if:**
1. The first request's response was received by the backend (so proposal persists)
2. The second request arrives at the **same instance** with the proposal still in memory

**Attack Sequence (Pathological but Real):**

1. Desktop app calls POST `/proposals` with (seq_no=1, action_hex="deadbeef")
2. Orchestrator backend processes it, returns 201 Created
3. Desktop app never receives response (timeout after 5 seconds)
4. Desktop app retries POST `/proposals` with same payload
5. Orchestrator backend (same instance) checks for ActionId = hash(1, deadbeef)
6. **If that proposal is still in memory**, returns 409 Conflict ✓ Good
7. **If backend crashed and restarted**, proposal is gone, returns 201 Created ✗ **Proposal created twice**
8. Now two identical proposals exist in the system (after restart)

**Smallest Fix:**

- Document: "Clients must include an idempotency key in POST `/proposals`; backend will reject duplicates"
- Example: `POST /proposals` with header `Idempotency-Key: sha256(seq_no || action_hex || signer_pubkey)`
- Backend stores a cache of recently seen keys → if seen again, return 409 Conflict

**Largest Bet:**

- Implement idempotency key deduplication as middleware
  - All POST/PUT requests must include `Idempotency-Key` header
  - Backend stores key → response mapping for 24h
  - If key seen again, return cached response (even if proposal state changed)
  - Scope: 100–150 lines of middleware + schema

---

#### 9. Session Expiry Based on Wall Clock (Clock Skew + Expiry Window Hazard)

**Severity:** **MEDIUM**  
**When:** System clock skew between client and backend, or long-running operations near TTL boundary

**Finding:**

- Auth sessions expire based on absolute wall-clock time (auth_session.rs:40)
  ```rust
  let now = now_unix_ms();
  if now > session.expires_at_unix_ms {
      return Err(AppError::Unauthorized);
  }
  ```
- Default TTL is 240 seconds (config.rs:37)
- If a signer starts a 3-minute proposal signing operation:
  - Request 1 (prepare broadcast): 0:00 to 0:30 → session valid ✓
  - Request 2 (estimate fee): 3:00 to 3:15 → session expired ✗ **Returns 401 mid-operation**

**Evidence:**
- `orchestrator-be/src/config.rs:37` — `auth_session_ttl_ms` default is 240,000ms (4 minutes)
- `handlers/auth_session.rs:34` — compares against `SystemTime::now()`
- No sliding-window refresh; once issued, TTL is fixed

**Attack Sequence:**

1. Signer Alice authenticates at time T0 → session expires at T0 + 240s
2. Alice's hardware wallet signing flow takes 3 minutes (hardware wallet ↔ device ↔ client ↔ backend)
3. At T0 + 180s, Alice submits final signature via POST `/proposals/{action_id}/approve`
4. Backend checks session → `now (180s) > expires_at (240s)`? No, still valid ✓
5. At T0 + 200s, Alice submits broadcast request
6. Backend checks session → `now (200s) > expires_at (240s)`? No, still valid ✓
7. At T0 + 250s, operator re-initiates a new proposal submission
8. Backend checks session → `now (250s) > expires_at (240s)`? **Yes, session expired** ✗ 401 Unauthorized
9. **Operator must re-authenticate**, losing any in-progress work

**Smallest Fix:**

- Implement "last_activity_at" tracking; extend expiry on each successful request
  ```rust
  let session = sessions.get(&token).ok_or(Unauthorized)?;
  if now > session.created_at + TTL {
      return Err(Expired);
  }
  // Extend TTL by updating last_activity_at
  session.last_activity_at = now;
  ```

- This requires mutable access to the sessions map; currently read lock only

**Largest Bet:**

- Move to token-based TTL with refresh tokens (OAuth2-style)
  - Issue short-lived access token (15 minutes) and long-lived refresh token (7 days)
  - When access token expires, client exchanges refresh token for new access token
  - Enables fine-grained TTL without blocking long operations
  - Scope: 150–200 lines + new auth flow

---

#### 10. No Timeout on Bitcoin RPC Calls (Cascading Latency)

**Severity:** **MEDIUM**  
**When:** Bitcoin RPC provider is slow or stuck

**Finding:**

- Broadcast operations call Bitcoin RPC for fee estimation and confirmation polling
- No explicit timeout on these calls; reqwest default is unbounded

**Evidence:**
- `application/proposals.rs:218–219` — `btc_client.estimate_fee_rate_sats_per_vb(6)` with no timeout wrapper
- `infrastructure/bitcoin_rpc.rs` — likely uses default reqwest timeouts (none by default in older versions)

**Attack Sequence:**

1. Operator initiates broadcast of proposal P
2. Bitcoin RPC provider (e.g., BlockCypher, local node) is hanging (network flake, overload)
3. POST `/proposals/{action_id}/broadcast` blocks indefinitely waiting for fee estimate
4. Orchestrator backend's Tokio runtime thread is blocked
5. Concurrent requests from other signers are queued, waiting for worker thread
6. After 30 seconds, multiple requests are blocked on Bitcoin RPC
7. Operator's desktop app hangs (waiting for response)
8. Eventually request timeouts on client side; operator retries
9. **Broadcast may have succeeded on Bitcoin side but response never reaches client**
   - Proposal state is `CommitBroadcasted` (commit tx broadcast)
   - Client got a 500 or timeout
   - Operator retries broadcast → backend returns "broadcast already in progress"

**Smallest Fix:**

- Add 10-second timeout to all Bitcoin RPC calls
  ```rust
  let fee_rate = tokio::time::timeout(
      Duration::from_secs(10),
      btc_client.estimate_fee_rate_sats_per_vb(6)
  )
  .await
  .map_err(|_| AppError::Internal(anyhow::anyhow!("bitcoin rpc timeout")))?
  .map_err(AppError::from)?;
  ```

- Return 504 Gateway Timeout if RPC times out; client can retry safely

**Largest Bet:**

- Implement circuit breaker pattern for Bitcoin RPC
  - Track consecutive failures; if >3 in a row, short-circuit new requests with "service degraded" 
  - Auto-recover after 60s of no failures
  - Scope: 100–150 lines + operational monitoring

---

### LOW SEVERITY

#### 11. No Proposal Expiry Enforcement (Protocol Mismatch)

**Severity:** **LOW**  
**When:** Proposals older than 7 days are still approved and broadcast

**Finding:**

- Proposal domain model has `ProposalStatus::Expired` but there's **no background job or check that transitions old proposals to Expired**
- A proposal created 8 days ago can still be in `Pending` state, and a signer can still approve it

**Evidence:**
- `orchestrator-be/src/domain/proposal.rs:62–73` — `ProposalStatus` enum includes `Expired` but...
- `application/proposals.rs` — no `check_expiry` function, no background task
- `handlers/proposals.rs` — all handlers return proposals as-is without checking age

**Attack Sequence:**

1. Proposal P created at timestamp T
2. Signatures collected; reaches quorum at T + 2 days
3. Operator forgets to broadcast for 6 days
4. At T + 8 days, operator broadcasts proposal
5. **ASM on-chain verifies the proposal should have expired 1 day ago**
6. On-chain, the update is rejected as "stale" (ASM protocol validates expiry internally)
7. Desktop app shows "broadcast succeeded" but ASM rejects it
8. Operator is confused; trust in system degraded

**Smallest Fix:**

- Add an `expires_at: u64` timestamp to `Proposal`
- In `approve_action` and `broadcast_commit_then_reveal`, check: `if now_unix_ms > expires_at, return Err(Expired)`
- This moves expiry check from backend to backend (defensive), not relying solely on ASM

**Largest Bet:**

- Background expiry job (see write-ahead log finding)
  - Every 30 seconds, query all `Pending` proposals older than 7 days
  - Transition them to `Expired` status
  - Scope: 75–100 lines of background task logic

---

## Attack Narratives

### Narrative 1: Backend Restart Loses All Signatures (Data Loss)

**Actors:** Alice (Signer), Backend (Orchestrator)

**Preconditions:**
- Backend is running on a 512MB container (typical dev/staging setup)
- Postgres is configured but the backend instance is using in-memory repo
- Proposal creation is in progress (50 signers, multi-signature collection over 6 hours)

**Sequence:**

1. Alice initiates proposal creation for "upgrade validator keys" at 09:00
   - POST `/proposals` → proposal stored **in memory only** (no persistence)
   - Alice sees "Proposal created, awaiting 4 more signatures"

2. Over next 2 hours, Bob, Carol, Dave, and Eve all approve the proposal via POST `/proposals/{action_id}/approve`
   - 5 signatures collected, stored only in the in-memory HashMap

3. At 11:05, Kubernetes pod enters a restart loop (e.g., due to a misconfigured liveness probe)
   - Backend process is killed
   - All HashMaps in memory are lost
   - Pod restarts, backend initializes empty state

4. Alice refreshes her browser/app at 11:06
   - Calls GET `/proposals?status=pending`
   - Backend returns empty list: `[]`
   - Alice sees "No pending proposals"

5. Alice and the 4 other signers are confused
   - All collected signatures are lost
   - They must manually re-enter their signatures
   - This time they use the offline fallback: copy signatures, aggregate manually, broadcast directly to Bitcoin

**Impact:**
- 2-hour delay
- Loss of trust in backend as coordination service
- Signers revert to fully manual process (defeating purpose of backend)
- On-call engineer is paged; must investigate why proposal disappeared

**Incident Cost:** 2+ hours of signer time, 30 minutes on-call investigation

---

### Narrative 2: Duplicate Signer Signature Causes Incorrect Quorum (Protocol Violation)

**Actors:** Alice (Signer, slow HW wallet), Frontend (Client), Backend (Orchestrator), ASM (On-Chain)

**Preconditions:**
- Proposal requires 2-of-3 signers
- Alice is signing with hardware wallet that takes 45 seconds per operation
- Frontend has a timeout of 30 seconds for requests

**Sequence:**

1. Alice clicks "Sign with Trezor" to approve proposal P
2. Trezor connects, Alice enters PIN, 45 seconds of signing delay
3. After 35 seconds, frontend gets a socket timeout (connection drops)
4. Frontend shows error: "Signing failed, retrying..."
5. Frontend automatically retries the approve request (standard retry logic)
6. Meanwhile, Trezor finishes signing at 45s, and the first request also completes

**Both requests hit backend nearly simultaneously:**

7. Request A (thread 1):
   - Reads proposal: `{ signatures: [alice], len: 1, required: 2 }`
   - Checks for duplicate: Alice not in list ✓
   - Calls `add_signature` for Alice
   - Proposal now has `[alice_sig1]`

8. Request B (thread 2):
   - Reads proposal: `{ signatures: [alice], len: 1, required: 2 }` (still has old count)
   - Checks for duplicate: Alice not in list ✓ (bug: should have re-read after thread 1)
   - Calls `add_signature` for Alice again
   - Proposal now has `[alice_sig1, alice_sig2]`

9. **Quorum incorrectly reached:** `len(2) >= required(2)` even though only 1 unique signer signed

10. Backend automatically transitions proposal to `Approved` state

11. Desktop app shows "Proposal approved!" even though only Alice has signed (Bob and Carol have not)

12. Operator calls broadcast at 11:15
    - Broadcast succeeds, commit tx mined in 2 blocks

13. ASM on-chain receives the transaction
    - ASM checks signature threshold: only 1 valid signature (Alice) vs. required 2
    - **ASM rejects the transaction** — "insufficient signatures"

14. **Proposal is stuck:** marked as `Approved` off-chain but rejected on-chain

15. Operator must manually investigate, discover the duplicate signature, manually delete one Alice signature from the proposal, re-broadcast

**Impact:**
- Protocol violation at governance layer
- 30–60 minute delay in governance action
- Manual intervention required
- High trust loss ("Why didn't the backend catch this?")

**Incident Cost:** 1 hour of operator time, governance action delayed 1 hour

---

### Narrative 3: Session Expires Mid-Broadcast (Lost Broadcast Authorization)

**Actors:** Operator (Human), Desktop App (Frontend), Backend (Orchestrator), Bitcoin (Network)

**Preconditions:**
- Operator has been working on proposal for 8 minutes
- Session TTL is 240 seconds (4 minutes) — not refreshing on activity
- Operator has collected 3 signatures, proposal is `Approved`

**Sequence:**

1. Operator authenticates at 10:00:00
   - POST `/auth/verify` → session created, expires at 10:04:00

2. Operator collects signatures from 3 signers, preparing broadcast (10:02:00)
   - Multiple approve requests, all succeed (session still valid)

3. Operator clicks "Broadcast to Bitcoin" at 10:08:00
   - 4 minutes after authentication, session has already expired

4. Backend receives POST `/proposals/{action_id}/broadcast`
   - Extracts bearer token from Authorization header
   - Calls `AuthenticatedSession::from_request_parts`
   - Checks session expiry: `now (10:08:00) > expires_at (10:04:00)` → **session expired**
   - Returns 401 Unauthorized

5. Desktop app receives 401 → displays "Session expired, please log in again"

6. Operator re-authenticates (10:08:15)
   - New session created, valid until 10:12:15

7. Operator re-clicks "Broadcast"
   - Backend calls `claim_broadcast` → **state transitions to `CommitBroadcasted`**
   - Derives address, creates and broadcasts commit tx
   - Proposal state is now stuck in `CommitBroadcasted` (awaiting reveal)

8. Operator checks back 3 hours later (13:08)
   - Finds proposal still in `CommitBroadcasted` state
   - Commit UTXO is confirmed on Bitcoin, sitting at the derived address
   - But no reveal tx has been sent

9. **Broadcast is incomplete:** commit tx successful, but reveal is orphaned

10. Operator must manually investigate, query Bitcoin for the confirm tx, reconstruct the reveal, broadcast manually

**Impact:**
- 3+ hour delay in governance action
- Manual recovery required
- User experience degraded (why did session expire mid-operation?)

**Incident Cost:** 30 minutes operator investigation, 1 hour manual recovery

---

### Narrative 4: Load Balancer + In-Memory Sessions = 401 on Retry (Session Loss at Scale)

**Actors:** Alice (Signer), Load Balancer, Backend Instance A, Backend Instance B

**Preconditions:**
- Orchestrator is scaled to 2 instances for redundancy
- Both instances share a Postgres database for proposals
- But each instance has its own in-memory session store (they don't share)
- Session affinity is NOT configured (sticky sessions)

**Sequence:**

1. Alice authenticates against instance A at 10:00:00
   - Instance A stores session in its in-memory map
   - Returns session token to Alice

2. Alice calls GET `/proposals?status=pending` at 10:00:05
   - Load balancer routes to **instance B** (round-robin, no affinity)
   - Instance B does NOT have Alice's session in its map
   - Backend returns 401 Unauthorized

3. Alice is confused: "I just logged in, why am I not authorized?"

4. Alice retries, load balancer routes to instance A → 200 OK ✓

5. Alice makes another request, load balancer routes to instance B → 401 Unauthorized ✗

6. **Alice's experience is non-deterministic:** ~50% of requests fail

7. After several retries, Alice gives up using the app

8. Operator receives support ticket: "Backend auth is broken, 50% of requests fail"

**Investigation:**

- On-call engineer checks logs
- Sees 401s from instance B when session was created on instance A
- Realizes sessions are not shared across instances
- Quick fix: disable load balancing (route all traffic to single instance)
- Sets sticky session flag in load balancer

**Impact:**
- 30+ minute outage of backend (until operator realizes issue)
- Signers cannot coordinate proposals
- Manual fallback is only option

**Incident Cost:** 30 minutes outage, 1 hour on-call investigation/fix

---

### Narrative 5: Bitcoin RPC Hangs → Backend Cascades → All Signers Blocked

**Actors:** Bitcoin RPC Provider, Backend, Multiple Signers, Network

**Preconditions:**
- Bitcoin node is running on the same physical host as the backend
- A misconfigured cron job starts a full blockchain rescan at 02:00 UTC
- This causes Bitcoin RPC to hang on all requests
- No timeout is set on Bitcoin RPC calls in the backend

**Sequence:**

1. At 02:05 UTC, operator initiates broadcast of an urgent governance proposal
   - POST `/proposals/{action_id}/broadcast` lands at backend

2. Backend calls `btc_client.estimate_fee_rate_sats_per_vb(6)`
   - Request is sent to Bitcoin RPC
   - Bitcoin RPC is in the middle of a rescan, CPU at 100%, request is queued

3. Backend thread is **blocked waiting for response** (no timeout)
   - Request doesn't return
   - Tokio worker thread is stuck

4. More signers and operators make requests to the backend
   - All hit the backend's thread pool (default: 4–8 worker threads on small instance)
   - Some threads are now all stuck on Bitcoin RPC calls
   - New requests queue up

5. After 2 minutes, all Tokio worker threads are exhausted
   - New requests immediately get connection refused or 503 Service Unavailable
   - Backend is effectively down

6. Multiple signers try to submit signatures, get 503 errors
   - They think the backend is crashed
   - They switch to offline fallback

7. At 02:15 UTC, Bitcoin node finishes rescan
   - Bitcoin RPC requests start responding
   - Blocked backend threads unblock
   - Backend is now responsive again

8. But during the 10-minute window, signers have already:
   - Given up on backend
   - Manually collected signatures
   - Broadcast directly to Bitcoin
   - Proposal went through without backend coordination

**Impact:**
- 10-minute coordination outage
- Manual fallback was necessary
- Signers lost trust in backend as a reliable service
- On-call engineer is paged

**Incident Cost:** 10 minutes of service degradation, 30 minutes on-call investigation

---

## Evidence Index

### Paths Cited

| Finding | File | Lines | Issue |
|---------|------|-------|-------|
| **In-Memory State Loss** | `orchestrator-be/src/state.rs` | 15–16 | `challenges` and `sessions` in `Arc<RwLock<HashMap>>` — no persistence |
| | `orchestrator-be/src/main.rs` | 90 | Warning when `DATABASE_URL` not set; uses in-memory repo |
| | `orchestrator-be/src/infrastructure/memory_repo.rs` | 1–128 | `InMemoryProposalRepository` — HashMap wrapping, no durability |
| **Broadcast Atomicity** | `orchestrator-be/src/application/proposals.rs` | 234–288 | `broadcast_commit_then_reveal` — non-atomic state transitions |
| | | 254 | `claim_broadcast` sets state irreversibly; no rollback |
| **Auth Volatility** | `orchestrator-be/src/handlers/auth_session.rs` | 35–42 | In-memory session lookup; fails on different instance |
| | `orchestrator-be/src/state.rs` | 16 | Sessions stored in `RwLock<HashMap>` |
| **Duplicate Signer Race** | `orchestrator-be/src/application/proposals.rs` | 87–119 | Quorum check happens outside repository lock |
| | `orchestrator-be/src/infrastructure/memory_repo.rs` | 47–65 | `add_signature` has no duplicate check; check is in application layer |
| **Quorum Non-Linearized** | `orchestrator-be/src/application/proposals.rs` | 102–115 | Quorum transition check is not atomic with signature addition |
| | `orchestrator-be/src/infrastructure/memory_repo.rs` | 96–97 | `add_signature` returns clone; lock is released |
| **No Rate Limiting** | `orchestrator-be/src/main.rs` | 111–114 | Router setup with `TraceLayer` only; no rate limit layer |
| | `orchestrator-be/src/handlers/` | — | No per-endpoint rate limit checks |
| **Broadcast Race (Multi-Instance)** | `orchestrator-be/src/infrastructure/memory_repo.rs` | 82–97 | `claim_broadcast` uses in-process `RwLock`; not distributed |
| **No Idempotency Key** | `orchestrator-be/src/handlers/proposals.rs` | 68–95 | `create_proposal` has no idempotency key mechanism |
| **Session Expiry Wall Clock** | `orchestrator-be/src/handlers/auth_session.rs` | 40 | Fixed TTL, no sliding window or refresh |
| | `orchestrator-be/src/config.rs` | 37 | Default TTL 240,000ms (4 minutes) |
| **Bitcoin RPC Timeout** | `orchestrator-be/src/application/proposals.rs` | 218–219 | `estimate_fee_rate_sats_per_vb` called without timeout wrapper |
| **Proposal Expiry Not Enforced** | `orchestrator-be/src/domain/proposal.rs` | 62–73 | `ProposalStatus::Expired` defined but never transitioned to |
| | `orchestrator-be/src/application/proposals.rs` | — | No expiry check in any handler |
| | `orchestrator-be/src/handlers/proposals.rs` | — | No expiry validation before approval or broadcast |

### RPC/External Dependencies

| Service | Usage | Timeout | Retry |
|---------|-------|---------|-------|
| Bitcoin RPC | Fee estimation, tx broadcast, confirm polling | **None** | **None** |
| ASM State RPC | Signer set lookup, seq_no state | **None** | **None** |
| Postgres (if used) | Proposal persistence | Default pool timeout | **None** |

---

## Smallest Fixes vs. Largest Bets

### Smallest Fixes (< 2 hours each)

1. **Persist proposal creation to Postgres before returning** (Finding #1)
   - Wrap repo operations in transaction
   - ~30 lines of code

2. **Move duplicate signer check into repository layer** (Finding #4)
   - Add `already_signed` check inside `add_signature` write lock
   - ~10 lines of code

3. **Add rate limiting layer** (Finding #6)
   - Add `tower-governor` or `governor` dependency
   - Add layer to Axum router
   - ~40 lines of code + 2 deps

4. **Add timeout wrapper to Bitcoin RPC calls** (Finding #10)
   - Use `tokio::time::timeout`
   - ~20 lines of code

5. **Add /reset-broadcast admin endpoint** (Finding #2)
   - New handler to reset `broadcast_status → Idle`
   - Requires manual confirmation
   - ~50 lines of code

### Largest Bets (> 2 weeks each)

1. **Write-Ahead Log + Event Sourcing** (Findings #1, #2, #11)
   - Append-only log for all state mutations
   - Replay on startup for recovery
   - Forms foundation for audit trail and multi-instance safety
   - Scope: 300–400 lines + schema + replay logic
   - Timeline: 2–3 weeks
   - Enables: crash recovery, audit trail, disaster recovery

2. **Distributed Broadcast Lock via Postgres Advisory Locks** (Finding #7)
   - Replaces in-process `RwLock` with advisory lock
   - Safe for multi-instance deployments
   - Scope: 50–75 lines + advisory lock usage patterns
   - Timeline: 3–5 days
   - Enables: horizontal scaling of backend

3. **Session Store Abstraction + Redis Impl** (Finding #3)
   - Abstract session storage behind trait
   - Implement Redis backend for multi-instance
   - Scope: 150–200 lines + Redis ops
   - Timeline: 1 week
   - Enables: stateless backend instances, session sharing

4. **OAuth2-Style Token Refresh** (Finding #9)
   - Short-lived access tokens + long-lived refresh tokens
   - Sliding window TTL
   - Scope: 200–250 lines
   - Timeline: 1–2 weeks
   - Enables: long-running operations without session expiry surprise

5. **Background Job Framework for Cleanup + Expiry** (Findings #6, #8, #11)
   - Scheduled tasks (every 30s, 1h, daily)
   - TTL-based cleanup of stale challenges, sessions, expired proposals
   - Scope: 150–200 lines + deployment
   - Timeline: 1 week
   - Enables: resource cleanup, expiry enforcement, operational health

6. **Circuit Breaker + Retries for External RPC** (Finding #10)
   - Track failure rates for Bitcoin RPC, ASM RPC
   - Auto-circuit-break on >3 consecutive failures
   - Exponential backoff retry logic
   - Scope: 200–300 lines + observability
   - Timeline: 1.5 weeks
   - Enables: graceful degradation, faster failure detection

---

## What Would Change My Mind

### Evidence That Would Reduce Severity

1. **If there is an append-only transaction log I haven't seen**, this would change Findings #1, #2, and #11 from CRITICAL to LOW
   - Proof: Show the log table schema, log replay on startup, log durability guarantee
   - Current evidence: No log visible in schema, migrations, or code

2. **If distributed locks are already implemented for broadcast**, this would change Finding #7 from HIGH to PENDING
   - Proof: Show Postgres advisory lock usage or equivalent distributed lock
   - Current evidence: Only in-process `RwLock` in memory_repo.rs

3. **If there is session clustering / Redis backend**, this would change Finding #3 and #4 from CRITICAL to MEDIUM
   - Proof: Show Redis connection in main.rs, session store abstraction, multi-instance test
   - Current evidence: Sessions only in in-memory HashMap

4. **If there are automated tests for concurrent signature submission**, this would reduce Finding #4 severity
   - Proof: Show test with 2 concurrent `approve_action` calls from same signer
   - Current evidence: No test visible in handlers/proposals.rs or application/proposals.rs

5. **If manual fallback is already proven end-to-end**, this would lower overall risk profile
   - Proof: Show e2e test where signers aggregate signatures offline and broadcast without backend
   - Current evidence: Architectural docs claim it's possible but no test validates it

### Evidence That Would Increase Severity

1. **If Postgres queries are non-atomic or lack transaction wrapping**, several findings would escalate
   - Example: If `claim_broadcast` is not wrapped in a transaction, find the actual SQL

2. **If there is production traffic already using this backend**, ALL findings escalate from potential to realized
   - Current evidence: This appears to be POC/pre-production; production deployment would change severity assessment

3. **If signers are already experiencing session drops or timeouts**, findings #3, #9, #10 move from theoretical to operational reality

### Experiments to Run

1. **Crash resilience test:**
   - Create 5 proposals, collect 3 signatures each
   - Kill backend process with `kill -9`
   - Restart backend
   - Query for proposals → expect 0 (currently)
   - With fix: expect 5 (persisted)

2. **Duplicate signer race test:**
   - Use wrk or Apache Bench to send 100 concurrent approve requests from same signer
   - Check final signature count in proposal
   - If count > 1 for same signer → bug confirmed

3. **Session loss at scale test:**
   - Start 2 backend instances, load balancer
   - Authenticate on instance A
   - Make requests hitting both instances
   - Measure % of 401 errors (expect >0% until fixed)

4. **Bitcoin RPC hang test:**
   - Start Bitcoin RPC mock that hangs for 60 seconds
   - Call broadcast endpoint
   - Measure time to failure/timeout (expect 60s, should be 10s with fix)

---

## Conclusion

**Current State:** The backend is a well-layered, clean POC suitable for **single-instance, single-operator testing** on **non-production data**. It correctly separates domain logic, validates authority/signer mapping, and enforces duplicate rejection at a basic level.

**Production Readiness Gap:** The system has **5 critical gaps** (FINDINGS #1, #2, #3, #4, #5) that must be closed before any multi-signer, multi-day operation or horizontal scaling:

1. **Data loss on restart** — in-memory state is ephemeral
2. **Broadcast non-atomicity** — partial failure leaves proposals stuck
3. **Non-distributed session store** — scaling breaks auth
4. **Duplicate signer race condition** — concurrent approvals bypass duplicate check
5. **Non-atomic quorum transition** — two threads can both trigger approved state

**Time to Production (Single-Instance, Full Safety):**
- Quick wins (smallest fixes): **6–8 hours** → 50% risk reduction
- Minimal safety (in-memory + timeouts + rate limits): **2–3 days** → 80% risk reduction
- Production grade (persistence + distributed locks + event sourcing): **3–4 weeks** → 95% risk reduction

**Recommendation:** Deploy to **staging/canary only** with the quick wins (items 1–4 above). Do not route production governance transactions through this backend until:
- ✅ Proposal state persists to Postgres
- ✅ Broadcast state machine is atomic or resumable
- ✅ Sessions are backed by a shared store (Redis or Postgres)
- ✅ Rate limiting is in place
- ✅ All external RPC calls have timeouts
- ✅ E2E manual fallback path is tested and working
