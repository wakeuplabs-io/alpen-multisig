# Data Engineering — Adversarial Assessment

**Audit Date:** 2026-05-13  
**Scope:** Backend persistence, desktop local state, data models, migrations, PII/privacy, governance  
**Stance:** Adversarial — attack data design and the absence thereof  

---

## Scope & Threat Model

### What We're Trying to Break

Alpen Multisig coordinates multisig governance off-chain with:

1. **Signer identities** (compressed public keys) – used as authentication credentials and signature validators
2. **Proposal lifecycle** (Pending → Approved → Enacted → [Expired|Canceled]) – where state transitions gate signing authorization
3. **Signature records** (pubkey + sig_hex) – immutable append-only, must not replay or corrupt
4. **Authority isolation** (5 independent multisigs: AlpenAdmin, StrataAdmin, SequencerManager, SecurityCouncil, PayoutAdmin)
5. **Off-chain data durability** – backend restart or desktop crash must not cause signer confusion, duplicate signatures, or loss of quorum progress

### The Adversarial Questions

- **Restart loss:** Backend restarts with in-memory storage → signers retry signing → duplicate submissions bypass unique constraint?
- **Migration trap:** Schema evolution without backward compatibility → stuck transactions or signature corruption?
- **PII leakage:** Signer pubkeys logged as plaintext → data breach exposes governance authority list?
- **Local state collapse:** Desktop crashes mid-signing → orphaned signatures, corruption, or state inconsistency?
- **Authority ambiguity:** Cross-authority state leakage → signer from AlpenAdmin can infer StrataAdmin proposal existence?
- **Determinism fragility:** Minor schema change breaks `ActionId` hash computation → old signatures no longer verify?

---

## Top Findings (Ranked)

### 🔴 BLOCKING

#### 1. **Backend In-Memory Storage by Default — Zero Durability**
**Risk:** Backend restart → all proposals lost; signers retry; duplicate signature conflicts; orphaned data.

**Evidence:**
- `orchestrator-be/src/main.rs:90-91` — "DATABASE_URL not set — using in-memory storage (data will not persist)"
- `orchestrator-be/src/infrastructure/memory_repo.rs` — `InMemoryProposalRepository` with `RwLock<HashMap<ActionId, Proposal>>`
- Default config: `database_url: Option<String>` is `None`, triggering in-memory repo initialization

**Attack narrative:**
1. Orchestrator is running in production with `DATABASE_URL` unset (common in Kubernetes-first deployments where env secrets are late-loaded)
2. 3 signers have signed a proposal; 2 more needed for quorum
3. Pod restart (OOM, unscheduled eviction)
4. All proposals vanish from backend memory
5. Signers retry create + sign
6. New proposal gets different `ActionId` (if `seq_no` collision) or old `ActionId` re-created
7. Signature deduplication fails; broadcast multiplexes two transactions with same authority

**Remediation:**
- Make `DATABASE_URL` required in production. Refuse startup if `DATABASE_URL` is not set AND `ENVIRONMENT=production`.
- Implement a read-only "degraded" mode that warns but continues if Postgres is down (not forgetting).
- Add startup health check: ping Postgres and verify table schema before accepting requests.

---

#### 2. **No Data Dictionary or Schema Governance; Implicit Types in Rust**
**Risk:** Silent schema corruption; misaligned (de)serialization; breaking migrations; lost audit trail.

**Evidence:**
- No `docs/data-model.md` or schema documentation beyond migrations
- Table/column semantics inferred from Rust types: `action_hex TEXT NOT NULL` (but no length limit enforcement)
- Migration files list only SQL DDL; no "before/after" narrative or rollback tests
- `postgres_repo.rs` hard-codes column selection: `const SELECT_PROPOSAL_COLS: &str = …` (fragile to schema drift)
- No migration audit log (timestamps, creator, deployment metadata)

**Attack narrative:**
1. Developer adds `optional_metadata JSONB` to proposals table without updating `SELECT_PROPOSAL_COLS`
2. Fetched rows return `NULL` for new column; Rust `row.get()` silently ignores or panics inconsistently
3. Three weeks later: metadata queries return empty; signers see incomplete proposal history
4. Rollback attempt fails because old migration script has no rollback step
5. Manual data cleanup required; governance timeline corrupted

**Remediation:**
- Create `docs/data-model.md` documenting:
  - Entity-relationship diagram (proposals, signatures, auth_sessions, challenges)
  - Field semantics, constraints, uniqueness rules
  - Serialization format for each blob (hex strings must be lowercase, length constraints, etc.)
- Annotate every migration with:
  - **Why:** business driver (e.g., "add broadcast tracking for payout flows")
  - **Before/After:** SQL queries showing data transformation
  - **Rollback:** reverse migration (if not one-way)
  - **Testing:** "test with >1M proposal records" or "verify ActionId hashes remain valid"
- Implement `infrastructure/schema_version.rs` that verifies migration version matches code version at startup

---

#### 3. **Flat Signature Record Format — No Ordering or Deduplication Guarantees**
**Risk:** Replay attacks; out-of-order signatures causing threshold confusion; no append-only semantics.

**Evidence:**
- `orchestrator-be/src/domain/proposal.rs:98` — `signatures: Vec<ProposalSignature>` (unordered, can be shuffled)
- `proposals` table: no `created_at` or `idx_signatures_created_at` on proposal_signatures table
- `add_signature()` in both repos appends without order verification
- Unique constraint only on `(action_id, signer_pubkey)` — prevents re-signing same proposal, not replay

**Attack narrative:**
1. Signer A signs proposal with `action_id = X`
2. Backend stores signature in order: [sig_A, sig_B, sig_C]
3. Due to Postgres connection pool rebalancing, next query of signatures fetches them in reverse: [sig_C, sig_B, sig_A]
4. Threshold verification re-orders them by pubkey index; verification succeeds (Strata crypto does this)
5. But downstream code that expects monotonic timestamp finds inconsistency
6. Auditor sees "signatures out of order" and questions governance authority

**Remediation:**
- Add `created_at` to `proposal_signatures` table with `ORDER BY created_at ASC` in fetch queries
- Add database index: `CREATE INDEX idx_proposal_signatures_order ON proposal_signatures(action_id, created_at)`
- Document: "Signatures are append-only. Database guarantees immutability via `ON DELETE CASCADE` and primary key lock."
- Add a test: "Verify signature order is deterministic across 100 schema-identical queries"

---

### 🟠 HIGH

#### 4. **Auth Sessions and Challenges Stored In-Memory; No Durability; Race Condition on Restart**
**Risk:** Session token reuse; auth bypass; signer confusion during session boundaries.

**Evidence:**
- `orchestrator-be/src/state.rs:15-16` — `challenges: Arc<RwLock<HashMap<String, PendingAuthChallenge>>>` and `sessions: Arc<RwLock<HashMap<String, AuthSession>>>`
- Both populated at startup (empty), expire after TTL but never persisted
- No auth session table in migrations
- Challenge/session expiry is soft (checked in handlers, not enforced by DB)

**Attack narrative:**
1. Signer initiates auth: receives challenge `nonce_123`
2. Backend process crashes; new process starts
3. Old `nonce_123` is gone
4. Signer completes signature of challenge offline (valid, cryptographically)
5. Signer submits signature with `nonce_123`
6. New process has no record of this nonce (HashMap is empty)
7. Auth fails; signer retries, gets new nonce
8. If nonce generation is reused due to weak RNG, attacker can forge nonce → bypass auth

**Remediation:**
- Add `auth_challenges` and `auth_sessions` tables with TTL (`expires_at` column, deleted by periodic cleanup job)
- Move challenge/session logic to database:
  ```sql
  CREATE TABLE auth_challenges (
    nonce TEXT PRIMARY KEY,
    authority TEXT NOT NULL,
    challenge_hex TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    consumed BOOLEAN DEFAULT FALSE
  );
  ```
- Implement `auth_session_cleanup_job` that runs every minute, deletes expired rows
- Test: "Verify nonce persists across process restart; auth succeeds"

---

#### 5. **Signer Public Key Format Unspecified; Collisions or Case Sensitivity Issues**
**Risk:** Two "identical" signer pubkeys treated as distinct (case, formatting); access control bypass.

**Evidence:**
- `postgres_repo.rs:33` — `key.eq_ignore_ascii_case(signer_pubkey)` (case-insensitive comparison in ASM membership)
- But in `proposal_signatures` table, `signer_pubkey` is stored as-is (case preserved)
- No schema constraint: `CHECK (signer_pubkey ~ '^[a-f0-9]{66}$')` — hex format not validated at DB level

**Attack narrative:**
1. Signer's compressed pubkey from hardware wallet: `0248aAbCdEfAbCdEf…` (mixed case)
2. Same signer submits a second signature, but device returns lowercase: `0248aabcdefabcdef…`
3. Unique constraint `(action_id, signer_pubkey)` allows both (different strings)
4. Proposal now has two entries for same signer; threshold count is inflated
5. Broadcast uses first match; signature verification fails (threshold crypto binds to canonical order)

**Remediation:**
- Add database constraint:
  ```sql
  ALTER TABLE proposal_signatures
  ADD CONSTRAINT signer_pubkey_format CHECK (signer_pubkey ~ '^[a-f0-9]{66}$');
  ```
- Normalize pubkey to lowercase in all ingress handlers:
  ```rust
  let signer_pubkey = body.signer_pubkey.to_lowercase();
  // Then use signer_pubkey in all lookups
  ```
- Document: "All signer public keys are 66-character lowercase hex strings (compressed format)"
- Test: "Mixed-case inputs normalize consistently"

---

#### 6. **No Append-Only Event Log; Audit Trail Implicit in Row Mutations**
**Risk:** Cannot reconstruct proposal state at any point in time; compliance audit impossible; no forensics.

**Evidence:**
- `proposals` table has `updated_at TIMESTAMPTZ` but no change log
- `proposal_signatures` table: immutable by design (no UPDATE), but no creation metadata beyond `created_at`
- Broadcast status transitions (`Idle → CommitBroadcasted → CommitConfirmed → …`) stored as final state, not as timestamped events

**Attack narrative:**
1. Governance auditor asks: "What was the state of proposal X at block 850000?"
2. Only current row state is available; no historical snapshots
3. Auditor cannot verify:
   - Whether signatures were collected in correct order
   - When broadcast decision was made vs. when quorum was reached
   - If any signer revoked consent (there is no revoke mechanism, so this is implicit)
4. Compliance requirement (e.g., EU governance data retention) cannot be satisfied
5. Signer disputes: "I never approved this proposal" — no immutable proof otherwise

**Remediation:**
- Create `proposal_events` append-only log:
  ```sql
  CREATE TABLE proposal_events (
    event_id BIGSERIAL PRIMARY KEY,
    action_id TEXT NOT NULL,
    event_type TEXT NOT NULL, -- 'created', 'signature_added', 'broadcast_status_changed', 'finalized'
    data JSONB NOT NULL, -- event-specific metadata
    created_at TIMESTAMPTZ DEFAULT NOW()
  );
  ```
- Log every state transition: proposal creation, each signature, broadcast status change, finalization
- Implement `proposals_materialized_view` that reconstructs current state from event log
- Test: "Event log can reconstruct full proposal history; matches current row state"

---

#### 7. **No Encryption at Rest; Unencrypted Signer Public Keys and Signatures in Database**
**Risk:** Database breach exposes governance signer identities; attacker can enumerate all authorities and their signers.

**Evidence:**
- `proposals` table columns: `action_hex TEXT`, `required_signatures SMALLINT` (plaintext)
- `proposal_signatures` columns: `signer_pubkey TEXT`, `signature_hex TEXT` (plaintext)
- No `ENCRYPTED` columns or Postgres pgcrypto integration
- No mention in architecture docs of encryption strategy

**Attack narrative:**
1. Attacker gains database read access (e.g., misconfigured AWS RDS snapshot public)
2. Queries `SELECT DISTINCT signer_pubkey FROM proposal_signatures` → full list of governance signers
3. Correlates pubkeys to Strata admin state (public on-chain) → maps pubkeys to real identities
4. Queries `SELECT authority, action_hex FROM proposals WHERE status = 'enacted'` → sees all governance history
5. Constructs adversarial proposals knowing exactly who can veto them

**Remediation:**
- Enable Postgres `pgcrypto` extension; encrypt sensitive columns at rest:
  ```sql
  ALTER TABLE proposal_signatures
  ADD COLUMN signer_pubkey_encrypted BYTEA,
  ADD COLUMN signature_hex_encrypted BYTEA;
  ```
- Migrate data: `UPDATE proposal_signatures SET signer_pubkey_encrypted = pgp_sym_encrypt(signer_pubkey, 'KEY')`
- Query decrypted data transparently via database function:
  ```sql
  SELECT pgp_sym_decrypt(signer_pubkey_encrypted, 'KEY') AS signer_pubkey FROM proposal_signatures;
  ```
- Use AWS KMS or HashiCorp Vault for key rotation
- Test: "Verify plaintext signer_pubkey never leaked in query plans; EXPLAIN output shows encrypted column"

---

### 🟡 MEDIUM

#### 8. **Desktop App: No Local Persistence; All State Lost on Crash**
**Risk:** Signers lose work mid-proposal; must restart from scratch; UX frustration → security bypass (shortcuts).

**Evidence:**
- `desktop-app/src/contexts/wallet-session-context.ts` — session context lives in React state (RAM only)
- No IndexedDB or localStorage documented for proposal cache
- Tauri filesystem access available but not used for proposal state
- Architecture docs (overview.md) mention "offline survivability" for backend but not for desktop

**Attack narrative:**
1. Signer is creating a complex multisig proposal on desktop app (authority select, action build, preview)
2. Form has 5 fields; signer fills 4, takes a screenshot for record-keeping
3. Desktop app crashes (OOM, GPU driver, Tauri IPC panic)
4. Signer restarts app; all form state is gone
5. Frustrated, signer uses web UI (hypothetical competitor) with weaker isolation
6. Or, signer manually constructs action hex string, making typo → broadcast fails at-chain

**Remediation:**
- Persist proposal drafts to Tauri filesystem:
  ```rust
  #[tauri::command]
  async fn save_proposal_draft(draft: ProposalDraft) -> Result<String> {
    let path = home_dir()?.join(".config/alpen-multisig/drafts/");
    let draft_id = Uuid::new_v4().to_string();
    std::fs::write(path.join(&draft_id), serde_json::to_string(&draft)?)?;
    Ok(draft_id)
  }
  ```
- Expose `load_proposal_draft`, `delete_proposal_draft` commands
- React context loads drafts on app startup; renders "Resume draft?" UI
- Encrypt draft at rest: `let draft_encrypted = encrypt_draft(&draft, &device_key)?`
- Test: "Verify draft persists after app crash; can resume proposal with all fields intact"

---

#### 9. **No Data Retention or Expiry Policy; Proposals Accumulate Forever**
**Risk:** Database bloat; compliance violation (GDPR-ish right to deletion); forensic contamination.

**Evidence:**
- Migrations: no TTL on proposals table (`DROP COLUMN` never mentioned)
- `postgres_repo.rs::list_by_status()` — returns all proposals matching status; no pagination or time filters
- Architecture overview: "Expired" status is final, but never deleted
- Config: no `DATA_RETENTION_DAYS` parameter

**Attack narrative:**
1. System runs for 3 years; 50K governance proposals accumulate (5 authorities × ~10 per day)
2. Forensic auditor performs query: `SELECT * FROM proposals` → 10-minute query, locks table
3. Compliance officer asks: "How many proposals from 2024 are still stored?" — no deletion policy exists
4. Attacker uses old, expired proposal states to confuse governance (e.g., "Did we already vote on X?")
5. Database size balloons; backup/restore becomes impractical

**Remediation:**
- Define retention policy in `AGENTS.md`:
  - Enacted proposals: keep indefinitely (audit trail)
  - Expired/Canceled proposals: keep 1 year, then archive to cold storage
  - Pending proposals (never broadcast): keep 90 days
- Add to migrations:
  ```sql
  ALTER TABLE proposals ADD COLUMN archived_at TIMESTAMPTZ;
  CREATE INDEX idx_proposals_archived ON proposals(archived_at) WHERE archived_at IS NULL;
  ```
- Implement archival job:
  ```rust
  async fn archive_old_proposals() {
    sqlx::query(
      "UPDATE proposals SET archived_at = NOW() 
       WHERE status IN ('expired', 'canceled') 
       AND updated_at < NOW() - INTERVAL '1 year' 
       AND archived_at IS NULL"
    ).execute(&pool).await?;
  }
  ```
- Test: "Verify old expired proposals are archived; active proposals remain queryable"

---

#### 10. **Broadcast Error Field: Plaintext; Sensitive RPC Failures May Leak**
**Risk:** Backend error messages (e.g., "Bitcoin RPC auth failed: user=…") leak credentials or system internals.

**Evidence:**
- `postgres_repo.rs:125` — `proposal.broadcast_error = error.map(|s| s.to_string())`
- `error.rs:34-35` — Errors are logged with `tracing::error!("internal error: {e}")` but client sees "internal error"
- However, broadcast error field is stored in database and exposed to signers via GET `/proposals/:action_id`

**Attack narrative:**
1. Bitcoin RPC connection fails: "Error: auth failed, bitcoind at 10.0.0.5:18332 rejected credentials"
2. Error is stored in `proposals.broadcast_error`
3. Signer calls GET `/proposals/:action_id` (authenticated)
4. Response includes: `"broadcast_error": "auth failed, bitcoind at 10.0.0.5:18332 …"`
5. Signer screenshots for bug report; credential+IP leak exposed
6. Attacker can now target Bitcoin node directly

**Remediation:**
- Sanitize broadcast errors; store sanitized message + raw error separately:
  ```rust
  let sanitized_error = "Bitcoin broadcast failed (contact operator)";
  let raw_error = format!("{:?}", err); // logged only, not stored
  tracing::error!("broadcast failed: {}", raw_error);
  proposal.broadcast_error = Some(sanitized_error.to_string());
  ```
- Never expose RPC URLs, credentials, or internal hostnames in client responses
- Document: "Error messages returned to signers must not include operational details"
- Test: "Verify broadcast error messages do not contain 'rpc', 'auth', 'password', or IP addresses"

---

### 🟢 LOW / SUGGESTIONS

#### 11. **No Soft-Delete for Proposals; Accidental Deletion Irreversible**
**Risk:** Data recovery impossible; audit trail broken if proposal is mistakenly deleted.

**Evidence:**
- `postgres_repo.rs` — no soft-delete pattern (`deleted_at` or `is_deleted`)
- `proposal_signatures` — `ON DELETE CASCADE` means deleting a proposal cascades deletion of all signatures

**Remediation:**
- Implement soft delete:
  ```sql
  ALTER TABLE proposals ADD COLUMN deleted_at TIMESTAMPTZ;
  ALTER TABLE proposal_signatures ADD COLUMN deleted_at TIMESTAMPTZ;
  ```
- Update queries to `WHERE deleted_at IS NULL`
- Implement restore function for admin use only
- Test: "Verify deleted proposal can be restored; signature order preserved"

---

#### 12. **Broadcast Status Enum — No Terminal State Lock**
**Risk:** Proposal status can be overwritten mid-broadcast; inconsistent state.

**Evidence:**
- `proposal.rs:18-26` — `BroadcastStatus` enum: `Idle, CommitBroadcasted, CommitConfirmed, RevealBroadcasted, RevealConfirmed, Failed`
- `update_broadcast_status()` accepts any `BroadcastStatus` transition without state machine validation
- No FSM enforcement; e.g., can jump from `Idle` directly to `RevealConfirmed`

**Remediation:**
- Implement state machine:
  ```rust
  impl BroadcastStatus {
    fn can_transition_to(&self, next: BroadcastStatus) -> bool {
      matches!(
        (self, next),
        (Idle, CommitBroadcasted) |
        (CommitBroadcasted, CommitConfirmed) |
        (CommitConfirmed, RevealBroadcasted) |
        (RevealBroadcasted, RevealConfirmed | Failed)
      )
    }
  }
  ```
- Verify transition in `update_broadcast_status()` before updating DB
- Test: "Verify invalid transitions are rejected; only canonical path is allowed"

---

## Attack Narratives (3–6)

### Narrative A: "Backend Restart + Duplicate Signatures = Quorum Inflation"

**Setup:** 5-of-7 multisig (AlpenAdmin). 4 signers have already signed proposal X. 3 more needed.

**Sequence:**

1. Signer E calls `POST /proposals/X/approve` with their signature
2. Backend logs signature in RAM HashMap
3. **Orchestrator pod OOMKilled; restart begins**
4. Kubernetes replaces pod; new process starts with empty HashMap
5. Signer E notices no HTTP response; retries (network timeout)
6. Signer E calls `POST /proposals/X/approve` again (idempotent intent)
7. Backend, now running, has no record of proposal X in RAM → missing (no DB)
8. Signer E still has signature in clipboard; pastes into CLI tool
9. **Signer F (independent, not aware of E's first attempt) also signs and submits**
10. Proposal now has [A, B, C, F, E, E] signatures
11. But deduplication is `(action_id, signer_pubkey)` → E's entry is deduplicated in Postgres unique constraint
12. **Problem: signers see quorum at 5 signatures, but backend truly has 5 unique signers; E's duplicate is silently dropped. Signers may broadcast prematurely if they only count client-side.**

**Why this fails in production:**
- In-memory storage is the root cause; Postgres is never populated
- No idempotency token ties individual signing attempts; retries are treated as new submissions
- Signers cannot distinguish "accepted" from "lost in network"

---

### Narrative B: "Schema Migration Breaks Action ID Determinism"

**Setup:** Proposal X is in production for 2 weeks. Stakeholders have archived signed transactions offline.

**Sequence:**

1. Developer notices `action_hex` column is unbounded TEXT
2. Merges PR: `ALTER TABLE proposals ADD COLUMN action_hex_normalized TEXT GENERATED AS (LOWER(action_hex))`
3. Adds logic to compute ActionId from `action_hex_normalized` instead of raw `action_hex`
4. **Old proposal X with `action_hex = "DEADBEEF"` has `action_id = hash(1, deadbeef)` (old code)**
5. New code loads proposal X, computes `action_id_new = hash(1, deadbeef)` from normalized column → same ID
6. **But stakeholders offline computed signature against original ActionId using old byte order (uppercase)**
7. When signature is replayed (e.g., governance re-vote), new ActionId doesn't match; signature is invalid
8. Entire quorum is lost; governance deadline expires

**Why this fails:**
- No test validating that ActionId computation is stable across migrations
- Implicit assumption that `compute_action_id()` logic never changes
- No backward-compatibility layer

---

### Narrative C: "Signer Public Key Case Collision + Threshold Miscount"

**Setup:** 3-of-5 multisig (SecurityCouncil). Mixed-case pubkeys from hardware wallets.

**Sequence:**

1. Signer A's Trezor returns: `02aAbCdEfAbCdEfAbCdEfAbCdEfAbCdEfAbCdEfAbCdEfAbCdEfAbCdEfAbCdEf`
2. Signer A signs proposal Y; sends signature to backend
3. Backend stores as-is in Postgres
4. **Signer B (using same Trezor firmware) gets the same pubkey returned in lowercase**
5. Backend stores: `02aabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdef`
6. Unique constraint `(action_id, signer_pubkey)` allows both (different strings)
7. **Proposal Y now shows 2 signatures from same signer; threshold incorrectly reads 2 / 5 instead of 1 / 5**
8. Signer C signs; now backend shows 3 / 5
9. **Signer D does NOT sign; but proposal is broadcast because 3 ≥ threshold**
10. Broadcast uses first signature from A (mixed case) + second from B (lowercase) + third from D
11. **Threshold verification fails** (B's signature does not match mixed-case pubkey index in signer set)
12. Bitcoin transaction is broadcast but rejected by ASM

**Why this fails:**
- No normalization of pubkey format at ingress
- Unique constraint doesn't account for case insensitivity in hex encoding
- Threshold counting is naive (distinct count) rather than validated against canonical signer set

---

### Narrative D: "In-Memory Auth Session Reuse After Restart"

**Setup:** Signer A initiates authentication at 14:00.

**Sequence:**

1. Signer A calls `POST /auth/start` → backend generates nonce `0xabc123…`
2. Signer A signs nonce with hardware wallet (takes 2 minutes)
3. Signer A calls `POST /auth/verify` with signature
4. Backend verifies signature; creates auth session `sess_A` in RAM HashMap
5. **At 14:15, orchestrator pod restarts unexpectedly**
6. New process starts; HashMap is empty
7. Signer A is still holding session token `sess_A` in localStorage (frontend)
8. Signer A calls `GET /proposals` with session token `sess_A`
9. Backend looks up `sess_A` in new HashMap → not found → 401 Unauthorized
10. Signer A refreshes browser; returns to auth flow
11. **But if nonce generation uses weak RNG (e.g., timestamp-based), attacker could pre-compute next nonce**
12. Attacker submits signature for `nonce_next` before it's officially issued
13. If verification doesn't strictly check nonce expiry in session, verification succeeds
14. Attacker gains auth session

**Why this fails:**
- Auth state is not persisted; restart invalidates all sessions
- Nonce generation is weak (implied; not documented as cryptographically secure)
- No session revocation list; no way to invalidate old sessions after restart

---

### Narrative E: "Proposal State Ambiguity on Crash — Orphaned Broadcast"

**Setup:** Proposal Z is approved (threshold met). Broadcast is in progress.

**Sequence:**

1. Backend calls `claim_broadcast(Z)` → sets `broadcast_status = CommitBroadcasted`
2. Backend constructs Bitcoin tx; submits to Bitcoin RPC
3. **Bitcoin RPC connection hangs (network partition)**
4. Backend blocks waiting for response
5. **Tauri desktop app (running on same machine for testing) calls Orchestrator simultaneously**
6. RPC times out after 30s; orchestrator panics (unwrap on timeout)
7. Proposal Z is in Postgres with `broadcast_status = CommitBroadcasted` but no `commit_txid`
8. Backend process dies
9. Hours later, backend restarts; loads proposal Z
10. Frontend shows "Broadcast in progress" (because `broadcast_status != Idle`)
11. **Operator is uncertain: was tx submitted? Should I rebroadcast?**
12. Operator manually checks Bitcoin mempool; finds no tx
13. Operator retries broadcast; backend loads Z again, tries to `claim_broadcast()` again
14. **Second `claim_broadcast()` fails (broadcast_status is no longer Idle)** → Conflict error
15. Operator is blocked; must manually clear broadcast state via DB

**Why this fails:**
- No idempotency guarantee on broadcast
- No way to distinguish "broadcast succeeded but status not updated" from "broadcast failed"
- Proposal state machine doesn't have a "retry" state

---

## Evidence Index (Paths)

### Backend Storage & Persistence

- **In-Memory Repo:** `orchestrator-be/src/infrastructure/memory_repo.rs`
- **Postgres Repo:** `orchestrator-be/src/infrastructure/postgres_repo.rs`
- **Main Initialization:** `orchestrator-be/src/main.rs:66-104` (database_url logic)
- **Config:** `orchestrator-be/src/config.rs`
- **State:** `orchestrator-be/src/state.rs` (auth_sessions, challenges in-memory)
- **Migrations:** `orchestrator-be/migrations/` (3 migration files)

### Domain Models

- **Proposal:** `orchestrator-be/src/domain/proposal.rs` (ProposalStatus, BroadcastStatus, ActionId, signatures)
- **Authority:** `orchestrator-be/src/domain/authority.rs`
- **Auth:** `orchestrator-be/src/domain/auth.rs` (PendingAuthChallenge, AuthSession)

### Frontend Local State

- **Session Context:** `desktop-app/src/contexts/wallet-session-context.ts` (React state, no persistence)
- **Auth Session Context:** `desktop-app/src/contexts/auth-session-context.ts`
- **Tauri Bridge:** `desktop-app/src/api/tauri-bridge.ts` (IPC wrapper)

### Architecture & Migration Docs

- **Overview:** `docs/architecture/overview.md` (all layers, but no data persistence narrative)
- **ADR-001:** `docs/architecture/adrs/001-alpen-crate-dependencies.md` (dependency strategy, not data)
- **ADR-002:** `docs/architecture/adrs/002-application-layer-strategy.md`
- **ADR-003:** `docs/architecture/adrs/003-desktop-application-layer-api.md`
- **ADR-004:** `docs/architecture/adrs/004-ci-pipeline-strategy.md`
- **ADR-005:** `docs/architecture/adrs/005-layered-architecture.md`

### Error Handling

- **Error Types:** `orchestrator-be/src/error.rs` (AppError enum, response mapping)
- **Broadcast TX:** `orchestrator-be/src/infrastructure/broadcast_tx.rs` (error propagation)

---

## Smallest Fixes vs. Largest Bets

### Smallest Fixes (Hours → Days)

1. **Require DATABASE_URL (1 hour)**
   - Add startup check: `if database_url.is_none() && env::var("ENVIRONMENT") == "production" { panic!() }`
   - Test: "Verify startup fails without DATABASE_URL in production mode"

2. **Normalize Signer Pubkey Format (2 hours)**
   - Add `.to_lowercase()` in request handlers before any storage
   - Add database constraint: `CHECK (signer_pubkey ~ '^[a-f0-9]{66}$')`
   - Test: "Mixed-case inputs normalize"

3. **Sanitize Broadcast Errors (1 hour)**
   - Map all RPC errors to "Broadcast failed (contact operator)"
   - Store raw error in logs only, not in DB
   - Test: "Verify error response is sanitized"

4. **Add Soft Delete (2 hours)**
   - Add `deleted_at` columns to proposals, proposal_signatures
   - Update queries to filter `WHERE deleted_at IS NULL`
   - Test: "Verify soft-deleted data is not queryable"

5. **Implement State Machine for BroadcastStatus (3 hours)**
   - Add `can_transition_to()` method on BroadcastStatus
   - Verify transition before DB update
   - Test: "Verify invalid transitions are rejected"

### Medium Bets (Days → Week)

6. **Persist Auth Sessions & Challenges to Database (3 days)**
   - Create `auth_challenges` and `auth_sessions` tables
   - Implement archival/cleanup job
   - Test: "Auth state persists across process restart"

7. **Add Event Log (Append-Only Audit Trail) (4 days)**
   - Create `proposal_events` table
   - Log all state transitions
   - Materialize current state from event log
   - Test: "Event log reconstructs proposal history exactly"

8. **Desktop Local Persistence for Drafts (3 days)**
   - Implement Tauri commands: `save_proposal_draft`, `load_proposal_draft`, `delete_proposal_draft`
   - Encrypt drafts with device key
   - Test: "Draft persists; can resume after app crash"

### Largest Bets (Week → Month)

9. **Encryption at Rest (1 week)**
   - Enable pgcrypto; encrypt sensitive columns
   - Migrate existing data
   - Implement key rotation with AWS KMS
   - Test: "Plaintext signer_pubkey never exposed in query plans"

10. **Data Governance & Dictionary (1 week)**
    - Create `docs/data-model.md` with ER diagram, field semantics, constraints
    - Annotate every migration with business narrative, rollback steps, testing
    - Implement schema version verification at startup
    - Test: "Migration version matches code; schema matches documentation"

11. **Idempotency & Replay Protection (2 weeks)**
    - Add `idempotency_key` to signature submissions
    - Store `(action_id, idempotency_key, created_at)` in deduplication table
    - Implement idempotency on broadcast (check if `commit_txid` is already set before resubmitting)
    - Test: "Duplicate requests return cached response; no state change"

---

## What Would Change My Mind (Missing Evidence / Experiments)

1. **Production Deployment Docs**
   - "Show me the deployment guide that mandates DATABASE_URL and shows Postgres schema setup"
   - Current state: none. If docs require Postgres, finding is downgraded to MEDIUM

2. **Encryption-at-Rest Specification**
   - "Show me the security architecture that specifies encryption at rest, key management, and audit of decryption"
   - Current state: no mention. If encryption is planned and documented, finding #7 is downgraded to SUGGESTION

3. **Event Log Specification**
   - "Show me the governance/audit requirements that specify immutable proposal history"
   - Current state: no audit trail. If governance SLA requires forensic reconstruction, this is upgraded to BLOCKING

4. **Idempotency Specification**
   - "Show me the API contract that guarantees idempotent signature submission"
   - Current state: POST /approve is treated as idempotent by operator, but no backend mechanism. If contract specifies idempotency, finding #1 is partially mitigated

5. **Desktop Persistence Strategy**
   - "Show me the spec or test that requires proposal drafts to survive app crash"
   - Current state: no mention. If UX requirement, finding #8 is upgraded to HIGH

6. **Auth Session Persistence Test**
   - "Show me the e2e test that verifies auth session survives backend restart"
   - Current state: e2e tests exist, but no restart scenario. If test passes, finding #4 is downgraded to SUGGESTION

7. **Backup & Disaster Recovery Plan**
   - "Show me the backup strategy: RTO, RPO, test restore procedures, off-site replication"
   - Current state: no mention. If we're in "accept data loss up to 24h", finding #1 rationale changes

---

## Summary

Alpen Multisig's data architecture is **sketch-to-working-prototype** maturity:

- **Blocking risks** (3): In-memory storage by default, no schema governance, flat signature format without append-only semantics
- **High risks** (7): Auth state not durable, pubkey format unspecified, no audit log, no encryption at rest, broadcast error sanitization, accumulating data without retention policy
- **Medium risks** (5): Desktop app has no local persistence, no soft-delete, broadcast status FSM not enforced, signer public key collision risk, session reuse after restart

**Most concerning:** The system **can lose all proposal state on backend restart** and has **no audit trail for governance decisions**. For a multisig coordination system, this violates both durability and compliance expectations.

**Recommended immediate actions (blocking):**
1. Make `DATABASE_URL` required in production
2. Add schema version verification at startup
3. Implement append-only event log for governance audit trail
4. Add `created_at` ordering to signatures; make order deterministic

**These changes are essential before deploying to a live governance environment.**
