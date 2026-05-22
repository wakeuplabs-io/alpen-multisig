# Application Architecture — Adversarial Assessment

**Date:** 2026-05-13  
**Auditor:** Atlas (Solution Architecture Reviewer)  
**Scope:** Backend (`orchestrator-be/`), Desktop (`desktop-app/src-tauri/`), E2E tests, ADRs (001–005), workspace boundaries, layer separation, protocol alignment

---

## Scope & Threat Model

**What we're trying to break:**

1. Layer boundaries hold under signer/authority isolation load
2. Backend truly operates as coordination-only, not re-implementing protocol rules
3. Desktop-app signer authority remains isolated from UI logic and backend coupling
4. Workspace members (backend, desktop, e2e-tests) are not creating hidden dependency tangles
5. ADR-declared architecture actually matches code
6. The system scales to "multiple schemes, roles, hash variants" without shotgun changes
7. Failure in one subsystem (backend crash, misbehaving signer) doesn't silently corrupt state

**Assumed architecture:** 
- Domain ← Application ← Handlers/Commands
- Infrastructure implements traits, never depended-on by application
- Backend is stateless coordination (except proposal storage)
- Desktop is decoupled from orchestrator API contract via OrchestratorClient trait
- Protocol enforcement happens in ASM, never re-invented in Rust
- Five isolated authorities (Alpen Admin, Strata Admin, Sequencer Manager, Security Council, Payout Admin)

---

## Top Findings

### ⛔ **BLOCKING & HIGH Issues**

#### 1. **Backend skips threshold verification — creates accept/reject vulnerability**
- **Severity:** BLOCKING
- **Location:** `orchestrator-be/src/application/proposals.rs` (lines 66–94, `approve_action()`); `handlers/proposals.rs` (lines 68–95, `create_proposal()`)
- **Issue:** The backend accepts and stores signatures without verifying that the submitted sighash matches the canonical SPS-65 computation. The code only stores `signer_pubkey` and `signature_hex` as opaque strings. If a signer submits a signature for the *wrong* action or wrong seq_no (off-by-one attack, typo, malicious substitution), the backend accepts it. The signature is only verified later during broadcast (`broadcast_tx.rs:build_signed_payload_bytes()`) or in e2e tests. 
  - In `approve_action()`, line 88–94: duplicate signer check exists, but there is no sighash validation.
  - In `create_proposal()`, line 78–92: handler calls `threshold_for_authority()` to check quorum, but never validates that the signer actually signed *this* action.
  - **Consequence:** A malicious signer can submit a signature for `(seq_no=5, action=PayoutUpdate)` to a proposal for `(seq_no=5, action=RollupUpgrade)` if both happen to use the same action_hex accidentally. The backend stores it. Only when broadcast is attempted does the signature fail verification, at which point the proposal is blocked with a cryptic error.
  - **ADR drift:** ADR-005 and overview.md state "Allowed: Hygiene checks (malformed input, duplicate signatures, structural consistency). Forbidden: Re-implementing SPS-65 logic." This is correct — but the backend also omits the one check it *should* do: verify the signer pubkey is in the canonical signer set for this authority (it delegates to ASM but doesn't validate locally before accepting the proposal).

---

#### 2. **AppState aggregates unrelated concerns — god-object pattern blocks scalability**
- **Severity:** HIGH
- **Location:** `orchestrator-be/src/state.rs` (lines 1–56)
- **Issue:** AppState conflates six distinct responsibilities:
  ```rust
  pub struct AppState {
    pub repo: Arc<dyn ProposalRepository>,          // Persistence
    pub asm_rpc_url: Arc<String>,                   // ASM config
    pub challenges: Arc<RwLock<HashMap<...>>>,     // Auth state
    pub sessions: Arc<RwLock<HashMap<...>>>,       // Auth state
    pub auth_challenge_ttl_ms: u64,                 // Auth config
    pub auth_session_ttl_ms: u64,                   // Auth config
    pub btc_client: Arc<dyn BitcoinRpcClient>,      // Bitcoin integration
    pub operator_keypair: Arc<UntweakedKeypair>,    // Broadcast auth
    pub confirm_poll_interval_ms: u64,              // Broadcast config
    pub confirm_timeout_ms: u64,                    // Broadcast config
    pub bitcoin_magic_bytes: MagicBytes,            // Network config
    pub bitcoin_network: Network,                   // Network config
  }
  ```
  - **Why this breaks:** When a second scheme/network/role is added, AppState becomes a bag of context. Example: adding `network_id: NetworkId` to support testnet + mainnet requires passing network ID through every handler, or creating a separate AppState per network, or wrapping AppState in a map. All are inefficient.
  - **Scaling failure:** If Payout Admin requires a different broadcast pipeline (quorum-locked UTXOs instead of operator keypair), you can't add `payout_utxo_provider: Arc<dyn PayoutUtxoProvider>` without breaking all other handlers that don't need it.
  - **Current consequence:** Low impact now (5 authorities, single Bitcoin network), but a sign that "composition root" discipline is absent.

---

#### 3. **Action validation deferred to broadcast/network — creates stale-proposal trap**
- **Severity:** HIGH
- **Location:** `orchestrator-be/src/application/proposals.rs` (lines 28–64, `create_update_action()`) — accepts `action_hex` without SSZ decode/validation; validation only happens in `broadcast_tx::build_signed_payload_bytes()` (lines 34–38) and `compute_sighash_for_proposal()` (lines 422–431).
- **Issue:** A user calls `POST /proposals` with `action_hex="deadbeef"` (invalid SSZ). The backend accepts it, stores the proposal in state, and the proposal sits in `Pending` for 7 days. When a signer tries to broadcast on day 3, the SSZ decode fails with `"invalid SSZ action: InvalidSszError"`, the proposal is marked `Failed`, and the 7-day window is wasted.
  - **Why it breaks:** The action should be validated at the boundary (create time), not at the end of the pipeline (broadcast time). Early validation provides rapid feedback and prevents wasting signer time.
  - **Consequence:** Signers see cryptic "broadcast setup error" messages instead of rejecting the proposal at submission time with "action is not valid SSZ".
  - **Scaling trap:** If action types proliferate (e.g., RollupUpgrade, SequencerKeyRotation, EmergencyDefcon, PayoutBlock), each handler will repeat the same "try to decode, fail cryptically" pattern.

---

#### 4. **Desktop app infrastructure imports Strata crates across multiple layers — codec isolation not enforced**
- **Severity:** HIGH
- **Location:** `desktop-app/src-tauri/src/infrastructure/action_codec.rs` is the declared boundary, but:
  - `application/proposals.rs` (line 13): imports `use strata_asm_txs_admin::actions::MultisigAction`
  - `application/broadcast_request.rs` (if exists): likely imports broadcast types
  - `commands/proposals.rs` (implied): may pass MultisigAction to frontend, coupling frontend to Strata wire format
- **Issue:** ADR-003 and ADR-005 state that Strata crates are isolated to `infrastructure/action_codec.rs`. The code shows they've leaked into application layer. This means:
  - A change to `strata-asm-txs-admin` (e.g., renaming `MultisigAction` to `AdminAction`) breaks `application/proposals.rs` directly, not just the codec.
  - The application layer is now coupled to Alpen crate versions, not just to domain types.
  - Frontend/commands can accidentally receive `MultisigAction` instead of client domain types, leaking protocol coupling to UI.
- **ADR drift:** ADR-003, line 215: "All other layers talk in client-owned domain types (`Authority`, `Action`, `MultisigUpdate`, `CompressedPubKey`). A codec test asserts byte-level borsh compatibility." In practice, lines 11–15 of `application/proposals.rs` show application layer directly consuming Strata crates.

---

#### 5. **Mock RPC bypasses — testing-to-production coupling in ASM role membership checks**
- **Severity:** HIGH
- **Location:** `orchestrator-be/src/infrastructure/asm_role_membership.rs` (lines 17–19, 44–46, 63–65) — `mock_membership()`, `mock_ordered_keys()`, `mock_last_seqno()` functions are called before real RPC in production code.
- **Issue:** 
  ```rust
  pub(crate) async fn is_signer_member_for_authority(...) -> Result<bool, AppError> {
    if let Some(is_member) = mock_membership(rpc_url, authority, signer_pubkey) {  // <- Production code checks mock!
      return Ok(is_member);
    }
    // Only then calls real RPC...
  }
  ```
  Line 58: `// TODO: decouple mock from implementation`
  - **Why it breaks:** If the environment variable or RPC URL is accidentally set to a mock pattern (e.g., `ASM_RPC_URL="mock://..."`), the backend silently uses mock signer sets instead of the real ASM state. A signer not in the real set can forge approvals locally and the backend accepts them.
  - **Scaling failure:** For each new authority added, a corresponding mock path must be added to avoid real RPC calls during testing. This is never going to stay in sync as authorities grow.
  - **Comment in code:** Line 58 already flags the issue — "TODO: decouple mock from implementation" — suggesting this is known friction.

---

### 📊 **MEDIUM Issues**

#### 6. **No replay protection check — backend accepts out-of-sequence proposals without validation**
- **Severity:** MEDIUM
- **Location:** `orchestrator-be/src/application/proposals.rs` (lines 30–64, `create_update_action()`) — accepts any `seq_no` without checking if it's greater than `last_seqno_for_authority()`.
- **Issue:** A signer could submit a proposal with `seq_no=1` even though the authority's `last_seqno=100`. The backend stores it. When broadcast is attempted, the ASM rejects it with "seqno too old". The proposal sits marked `Failed`.
  - **Why:** The backend should validate `seq_no > last_seqno` before storing. This is not ASM logic (which also checks), but basic hygiene that saves wasted proposals.
  - **Consequence:** Signers burn time waiting for a broadcast that will always fail.
  - **Mitigating factor:** `get_next_seq_no()` (line 97–105) provides the correct next seq_no, so a well-behaved client won't hit this. But malicious or buggy signers will.

---

#### 7. **Backend doesn't enforce authority isolation — handler accepts any session authority**
- **Severity:** MEDIUM
- **Location:** `orchestrator-be/src/handlers/proposals.rs` (line 110): `pub async fn list_proposals(..., _auth: AuthenticatedSession, ...)` — the underscore prefix shows the `auth` parameter is not used.
- **Issue:** `list_proposals()` accepts an authenticated session but ignores it. Any signer can list all proposals for all authorities, leaking proposal existence (though not signatures, since Proposal struct stores them).
  - **Why:** Authorization should either:
    1. Filter by `auth.authority` (return only proposals for this signer's authority), or
    2. Explicitly allow cross-authority listing if that's intentional.
  - **Mitigating factor:** The overview.md says "Signers of that authority only [see pending/quorum met]", but `list_proposals()` isn't enforcing this.

---

#### 8. **Broadcast state machine is frontend-aware but backend doesn't guarantee atomicity**
- **Severity:** MEDIUM
- **Location:** `orchestrator-be/src/domain/proposal.rs` (lines 16–40, `BroadcastStatus` enum); `handlers/proposals.rs` (lines 24–38, `PrepareBroadcastResponse` and `BroadcastResponse`).
- **Issue:** The broadcast flow is:
  1. `prepare_broadcast()` (no state change) → returns `(commit_address, commit_amount_sats, estimated_fee_sats)`
  2. Signer funds the commit address (offchain step)
  3. `broadcast()` → state transitions to `CommitBroadcasted`
  But if the signer funds the commit address and the backend crashes before `broadcast()` is called, the committed UTXOs are orphaned. The backend doesn't record "prepare was called" or "commit was funded" — it only knows `CommitBroadcasted` or `Idle`.
  - **Consequence:** A signer can fund multiple commit addresses and retry broadcasts. This is annoying but not a security issue (UTXOs are recoverable). But it shows the state machine isn't designed for the offline-survivability contract.

---

#### 9. **No configuration validation — hardcoded values will break under network/scheme changes**
- **Severity:** MEDIUM
- **Location:** 
  - `orchestrator-be/src/application/proposals.rs` (line 143): `const REVEAL_TX_VBYTES: u64 = 350;`
  - `desktop-app/src-tauri/src/application/proposals.rs` (line 47): `const REVEAL_TX_VBYTES: u64 = 350;` (duplicated)
  - `orchestrator-be/src/application/proposals.rs` (line 140): `const COMMIT_DUST_SATS: u64 = 1500;`
  - `strata_l1_txfmt::MagicBytes` (in state.rs line 23): Magic bytes for SPS-50 are baked into AppState but not configurable per network.
- **Issue:** If testnet uses a different magic byte or requires different dust/fee estimates, these hardcoded values break. They should be:
  - Loaded from config (via `config.rs`)
  - Validated at startup
  - Not duplicated between backend and desktop app
  - **Current consequence:** Any testnet deployment requires a code change (recompile with new `const` values).

---

#### 10. **Workspace members duplicate domain types — no shared abstraction prevents divergence**
- **Severity:** MEDIUM
- **Location:**
  - `orchestrator-be/src/domain/authority.rs` (Authority enum)
  - `desktop-app/src-tauri/src/domain/authority.rs` (Authority enum — redefined)
  - No shared `multisig-types` crate
- **Issue:** Both backend and desktop define their own Authority enum. They're identical now, but if one is changed (e.g., adding a variant for a new role), the other won't be updated automatically. E2E tests will break cryptically ("enum variant not found").
  - **Why:** ADR-005 explicitly says "if backend and desktop domain types diverge significantly, we may extract a shared `multisig-types` crate. Not needed yet." This is currently being *relied* on via convention, not enforcement.
  - **Consequence:** Adding a sixth authority (e.g., `OperatorCouncil`) requires synchronized changes across 3 files.

---

### 📋 **LOW Issues**

#### 11. **No structured logging of authority context — audit trail is invisible**
- **Severity:** LOW
- **Location:** `orchestrator-be/src/main.rs` (no tracing setup for authority/action_id); `handlers/proposals.rs` (no span context).
- **Issue:** When debugging why a signer's proposal failed, there's no structured log entry linking `(authority, action_id, signer_pubkey, timestamp)`. Errors are logged at the handler level but not correlated by action.
  - **Mitigating factor:** Low impact for now (in-memory state is easy to inspect), but becomes critical when Postgres is added and data is distributed across nodes.

---

#### 12. **Action codec test doesn't validate wire-format compatibility with current Strata main**
- **Severity:** LOW
- **Location:** `desktop-app/src-tauri/src/infrastructure/action_codec.rs` (line 198+, test `test_encode_matches_direct_strata_ssz`) — tests that desktop encoding matches a local SSZ round-trip, but doesn't verify compatibility with current `alpenlabs/asm#main`.
- **Issue:** The test passes with the pinned rev (`a8559d3`), but if upstream changes (e.g., renames a field), the test won't catch it until the pin is updated and `cargo test` is run.
  - **Mitigating factor:** Low risk because ADR-001 requires explicit update procedure (lines 92–98), which includes running this test. But no CI-time check that we're still compatible with the latest upstream.

---

## Attack Narratives

### 📍 **Narrative 1: Bad signature leaks past backend, wastes broadcast window**

**Attacker goal:** Introduce noise / waste signer time.

**Attack sequence:**
1. Signer A submits `POST /proposals` with a valid signature for Action X at seq_no=5.
2. Signer B submits `POST /proposals/:action_id/approve` with an *invalid* signature (corrupted hex, wrong key, wrong sighash).
3. Backend stores it without validation (Finding #1).
4. When quorum is reached and broadcast is attempted, `build_signed_payload_bytes()` tries to recover the public key from Sig B's corrupt signature. Signature verification fails.
5. Proposal is marked `Failed` after 1–2 minutes of polling. Signer B has no way to know the signature was bad until broadcast.
6. The 7-day window is now ticking. When Signer B re-signs correctly, they have to manually increment seq_no (because `get_next_seq_no()` now returns seq_no=6 due to the failed proposal), potentially skipping actions.

**Impact:** Repeated back-and-forth, wasted time, possible seq_no gaps that alarm operators.

---

### 📍 **Narrative 2: Mock RPC in production — signer set mismatch goes undetected**

**Attacker goal:** Forge approvals for an action they're not authorized to sign.

**Attack sequence:**
1. Backend is deployed to staging with `ASM_RPC_URL="mock://localhost"` (accidental environment variable).
2. Attacker (not in the real ASM signer set for StrataAdmin) calls `POST /proposals` with an action.
3. Backend calls `is_signer_member_for_authority()`. The mock matches the attacker's pubkey and returns `Ok(true)` (Finding #5).
4. Proposal is created and stored. Quorum is reached.
5. During broadcast, `asm_role_membership::ordered_keys_for_authority()` is called again, also hits the mock, and returns the attacker's key in the canonical set.
6. Broadcast succeeds. Signer submits the tx to Bitcoin.
7. Bitcoin mempool accepts it (structure is valid). But when the tx reaches the ASM, the real signer set (different from the mocked one) rejects the signatures.
8. No trace of the forgery is visible in the backend logs because the mock was silent.

**Impact:** Broken governance flow, manual intervention required, loss of trust in the system.

---

### 📍 **Narrative 3: Maintainer adds a new role, forgets to update domain types in one crate**

**Attacker goal:** N/A — this is an operational failure.

**Attack sequence:**
1. Team decides to add `OperatorCouncil` as a sixth authority.
2. Maintainer updates `orchestrator-be/src/domain/authority.rs` to add the variant.
3. Maintainer forgets to update `desktop-app/src-tauri/src/domain/authority.rs`.
4. E2E test tries to list proposals for OperatorCouncil (new variant in backend), passes the domain type to desktop-app layer.
5. Serde deserialization fails because desktop app doesn't recognize the variant.
6. Test fails with "unknown variant". Maintainer debugs for 30 minutes before realizing both files need updating.

**Impact:** Development friction, delayed release.

---

### 📍 **Narrative 4: God-object AppState blocks sidecar addition**

**Attacker goal:** N/A — this is an architectural scaling failure.

**Attack sequence:**
1. Product decides to add real-time proposal notifications via WebSocket sidecar (separate process, same backend).
2. Sidecar needs access to `repo` and `asm_rpc_url` from AppState.
3. Currently, AppState is created in `main.rs` and passed to handlers. Extracting `repo` + config into a separate struct is possible but requires refactoring all handlers.
4. No clear "config" struct exists — magic values are mixed with state. Sidecar would also need the same `bitcoin_magic_bytes`, `operator_keypair`, etc.
5. Maintainer ends up duplicating AppState initialization in the sidecar, creating two sources of truth.

**Impact:** Code duplication, divergence risk, hard to add new integrations.

---

### 📍 **Narrative 5: Action validation pushed to broadcast — stale proposals pile up**

**Attacker goal:** Degrade system availability through accumulated failed proposals.

**Attack sequence:**
1. User calls `POST /proposals` with `action_hex="zzzz"` (invalid SSZ) due to a bug in their client.
2. Backend stores it (no validation, Finding #3).
3. Proposal sits in `Pending` state. Other signers can see it but can't approve it (they don't know the action is malformed).
4. 7 days later, it expires.
5. During those 7 days, new proposals for the same authority accumulate. If the UI shows "3 pending proposals", the user has to manually skip the malformed one to find the real action.

**Impact:** User confusion, reduced clarity on which proposals are actionable.

---

### 📍 **Narrative 6: Broadcast state incomplete — orphaned UTXOs after crash**

**Attacker goal:** N/A — this is a failure mode.

**Attack sequence:**
1. Signer calls `POST /proposals/:id/prepare_broadcast`. Gets back `commit_address="bcrt1..."` and `commit_amount_sats=1500`.
2. Signer funds the commit address offchain and returns.
3. Signer calls `POST /proposals/:id/broadcast`.
4. Backend crashes after transitioning to `CommitBroadcasted` but before actually submitting the commit tx to Bitcoin.
5. Signer retries broadcast. Backend sees status is `CommitBroadcasted` (not `Idle`), returns conflict error.
6. Signer manually resets proposal state (if they can) or contacts an operator.
7. UTXOs remain locked in the commit address.

**Impact:** Manual recovery required, operational burden.

---

## Evidence Index

### Backend Architecture
- `orchestrator-be/src/state.rs`: AppState god-object (Finding #2)
- `orchestrator-be/src/application/proposals.rs`:
  - Lines 28–64: No SSZ validation on create (Finding #3)
  - Lines 66–94: No sighash verification on approve (Finding #1)
  - Line 140, 143: Hardcoded vbytes/dust (Finding #9)
  - Lines 422–431: Deferred sighash computation (Finding #3)
- `orchestrator-be/src/handlers/proposals.rs`:
  - Lines 68–95: No validation in handler (Finding #1)
  - Line 110: `_auth` underscore prefix (Finding #7)
- `orchestrator-be/src/infrastructure/asm_role_membership.rs`:
  - Lines 17–19, 44–46, 63–65: Mock bypasses (Finding #5)
  - Line 58: TODO comment acknowledging coupling (Finding #5)
- `orchestrator-be/src/domain/proposal.rs`:
  - Lines 16–40: BroadcastStatus enum (Finding #8)

### Desktop App Coupling
- `desktop-app/src-tauri/src/application/proposals.rs`:
  - Lines 13: Strata import in application layer (Finding #4)
  - Line 47: Duplicate hardcoded vbytes (Finding #9)
- `desktop-app/src-tauri/src/infrastructure/action_codec.rs`:
  - Lines 169–210: Test doesn't validate against upstream main (Finding #12)

### Domain Type Duplication
- `orchestrator-be/src/domain/authority.rs`: Backend authority enum
- `desktop-app/src-tauri/src/domain/authority.rs`: Desktop authority enum (redefined, Finding #10)

### ADRs vs Code
- ADR-005, Section "Desktop App": "All other layers talk in client-owned domain types" — violated by Finding #4
- ADR-005, Section "Key rules": Infrastructure "never on application" — arch diagram shows this enforced but code shows Strata leakage
- ADR-001, lines 50–51: "SSZ codec handles this, other layers use client types" — code shows `application/proposals.rs` imports Strata directly
- ADR-003, line 215: "infrastructure/action_codec.rs is the single module that imports Strata crates" — violated by Finding #4

### Config & Hardcoding
- `orchestrator-be/src/main.rs`: No validation of `bitcoin_magic_bytes` or `MagicBytes` at startup
- `Cargo.toml`: Alpen crate pins are centralized (ADR-001 compliance ✓), but network-specific config is not

---

## Smallest Fixes vs Largest Bets

### **Small Fixes (1–2 PR, < 100 LOC impact)**

1. **Add SSZ validation at boundary** (Finding #3)
   - In `handlers/proposals.rs::create_proposal()`, before calling `proposals::create_update_action()`, decode `action_hex` to `MultisigAction` and reject if invalid.
   - ~15 lines, immediate feedback to signers.

2. **Remove mock bypasses** (Finding #5)
   - Delete `mock_membership()`, `mock_ordered_keys()`, `mock_last_seqno()` from `asm_role_membership.rs`.
   - Move mocking to a separate test fixture or use a test-only flag (not RPC URL pattern matching).
   - ~30 lines deleted, prevents production coupling.

3. **Enforce authority in list_proposals** (Finding #7)
   - Filter proposals by `auth.authority` instead of accepting `_auth`.
   - ~5 lines changed.

4. **Dedup vbytes constant** (Finding #9)
   - Move `REVEAL_TX_VBYTES` and `COMMIT_DUST_SATS` to `config.rs`.
   - Load from env or config file.
   - Remove duplicates from desktop app.
   - ~20 lines.

### **Medium Fixes (1–2 PRs, 100–500 LOC impact)**

5. **Add seq_no validation** (Finding #6)
   - In `create_update_action()`, call `last_seqno_for_authority()` and validate `seq_no > last_seqno`.
   - Also validate `seq_no <= last_seqno + max_gap` (ASM already does this, but early rejection is hygiene).
   - ~25 lines.

6. **Uncouple Strata imports from application layer** (Finding #4)
   - Move `MultisigAction` usage to `infrastructure/action_codec.rs` only.
   - Define a client-owned `Action` domain type in `domain/action.rs` that doesn't import Strata.
   - `application/proposals.rs` works with `Action`, not `MultisigAction`.
   - ~80 lines of refactoring.

7. **Extract config struct from AppState** (Finding #2)
   - Create `struct OrchestratorConfig` with `magic_bytes`, `bitcoin_network`, `confirm_timeout_ms`, etc.
   - `AppState` holds `Arc<OrchestratorConfig>` instead of scattering fields.
   - ~50 lines, clarifies intent.

### **Largest Bets (full feature work, blocks scalability)**

8. **Shared `multisig-types` crate** (Finding #10)
   - Extract `Authority`, `ActionId`, `Proposal`, `ProposalSignature` into a shared crate.
   - Backend and desktop import from the same source.
   - E2E tests validate in one place.
   - ~200 LOC, high organizational value, blocks new authorities from diverging.

9. **Introduce signer verification service** (Finding #1 + #5)
   - Create `infrastructure/signer_verifier.rs` that:
     - Loads canonical signer set from ASM at startup (not per-request)
     - Validates signer authorization at proposal create time, not broadcast time
     - Prevents mock bypasses via dependency injection (test uses mock, prod uses real RPC)
   - ~150 lines, unblocks authority isolation enforcement.

10. **Revise broadcast state machine** (Finding #8)
    - Add `ProposalBroadcastState` that tracks `{Prepared, CommitFunded, CommitBroadcasted, ...}`
    - Signer calls `prepare_broadcast()` → state = `Prepared`
    - After funding, signer calls `confirm_commit_funded()` → state = `CommitFunded` (optional, improves UX)
    - Backend doesn't allow broadcast if state is not `CommitFunded` or `Prepared`
    - Offline survivability: if backend crashes, signer can re-call `confirm_funded()` to resume
    - ~200 lines, significant UX improvement.

---

## What Would Change My Mind

### **Evidence That Would Reduce Risk**

1. **Backend adds sighash validation at create time** — code sample showing:
   ```rust
   // In create_proposal handler:
   let sighash = compute_sighash_for_proposal(seq_no, action_hex)?;
   signing::verify_threshold(&canonical_keys, threshold, &[sig_hex], &sighash)?;
   // Only then call proposals::create_update_action()
   ```
   This would elevate Finding #1 from BLOCKING to LOW.

2. **AppState is split into Config + State** — code showing:
   ```rust
   struct OrchestratorConfig { ... }
   struct AppState { config: Arc<Config>, repo: Arc<dyn ProposalRepository> }
   ```
   Would address Finding #2 completely.

3. **Mock is deleted and replaced with dependency injection** — no more runtime RPC URL pattern matching. Test passes in a mock client; prod passes in the real one. Would eliminate Finding #5.

4. **Desktop app defines its own Action domain type**, not imported from Strata. Would address Finding #4.

5. **Shared `multisig-types` crate is introduced**, used by backend, desktop, and e2e-tests for Authority, ActionId, etc. Would eliminate Finding #10 and reduce divergence risk.

6. **Integration test verifies that a malformed action_hex is rejected at create time**, not at broadcast time. Would confirm Finding #3 is fixed.

7. **Authorization check in list_proposals filters by authority**, and an integration test confirms that Signer A cannot list proposals for Authority B. Would verify Finding #7.

### **Experiments / Measurements**

8. **Performance test:** Measure latency of a full proposal → broadcast cycle with 10 proposals in parallel. If AppState contention (Finding #2) causes >100ms P95 latency increase, it's real. 

9. **Failure injection:** Restart backend mid-broadcast and verify that the signer can recover without manual intervention (Finding #8).

10. **Seq_no stress test:** Submit proposals with out-of-order seq_nos and verify backend rejects them early (Finding #6).

---

## Summary

**Verdict: Architecture has high-impact gaps; code does NOT fully match declared design.**

**Critical path items blocking production readiness:**
- Remove signer validation from application layer and add it to the boundary (Finding #1)
- Uncouple Strata imports from application layer (Finding #4)
- Remove mock RPC bypasses (Finding #5)
- Validate actions at create time, not broadcast (Finding #3)

**Scaling risks:**
- AppState god-object will become a maintenance liability beyond 5 authorities / 2 networks
- Duplicated domain types will cause divergence bugs as roles/schemes multiply
- Hardcoded values require recompilation for testnet/mainnet switching

**Mitigating factors:**
- E2E tests are comprehensive and catch integration bugs
- Broadcast layer has safety checks (signature verification, sighash validation)
- ADRs are well-intentioned and documented — implementation gaps are localized

**Recommendation: Treat Findings #1, #4, #5 as pre-production blockers. Address Findings #2, #3, #6 before adding a second scheme or network. Findings #7–12 are tech debt; prioritize after finding P0 issues.**
