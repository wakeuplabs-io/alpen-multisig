# Domain / DDD — Adversarial Assessment

**Auditor:** DDD specialist (Vernon's rules, strategic boundaries, ubiquitous language)  
**Date:** 2026-05-13  
**Scope:** Bounded contexts, aggregate design, language drift, event non-use, persistence leakage, cross-layer translation  
**Stance:** Hostile — treat the domain model as a hypothesis to break, not a reference to preserve.

---

## Scope & Threat Model — What We're Trying to Break

### Three Bounded Contexts Under Test

1. **Orchestrator Backend** (`orchestrator-be/src/domain/` + `application/` + `infrastructure/`)
   - Claim: Coordination-only service; proposal creation, signature collection, lifecycle tracking.
   - Truth test: Is `Proposal` truly an aggregate, or just a data bag? Do invariants live in the model?

2. **Desktop Signer Context** (`desktop-app/src-tauri/` domain types, signing library)
   - Claim: Client-side types faithfully mirror backend domain for protocol signing.
   - Truth test: Is duplication actually isolation, or is it leaking backend assumptions?

3. **Protocol-Backend Translation** (`infrastructure/action_codec.rs`, `infrastructure/asm_role_membership.rs`)
   - Claim: Anti-corruption layer isolates domain from Alpen/Strata protocol types.
   - Truth test: Where does protocol bleed into domain despite the codec wrapper?

### Specific Risks Targeted

- **Aggregate Anemia**: Does `Proposal` struct enforce any invariants, or does business logic scatter across `application/proposals.rs` handlers?
- **Domain/Persistence Coupling**: Are domain types shaped by storage needs (HashMap keys, cloneability) rather than business rules?
- **Ubiquitous Language Drift**: Do code names match SPS-50/51/65 + PRD terminology, or has informal naming diverged?
- **Missing Event Domain**: No domain events exist. For a multisig coordination system, is this justified, or is it a red flag?
- **Frontend/Backend Model Mismatch**: Desktop `AuthRole` enum (2 roles: `strata_administrator`, `strata_sequencer_manager`) vs. backend `Authority` enum (5 roles). Are they the same concept?
- **Infrastructure Intrusion**: Does business logic live in `application/` or bleed into `handlers/` and `infrastructure/`?

---

## Top Findings (Ranked) — Blocking/High | Medium | Low

### 🚩 BLOCKING: Proposal Is Not an Aggregate — It's a Persistent Data Bag

**Finding:** The `Proposal` struct in `orchestrator-be/src/domain/proposal.rs` (lines 89–103) is a collection of public fields with zero invariant enforcement. The struct **itself** contains no methods. All mutation logic lives scattered in `application/proposals.rs`.

```rust
// domain/proposal.rs
pub struct Proposal {
    pub action_id: ActionId,
    pub seq_no: SeqNo,
    pub authority: Authority,
    pub status: ProposalStatus,
    pub required_signatures: u16,
    pub action_hex: String,                  // <-- Opaque; no validation
    pub signatures: Vec<ProposalSignature>,  // <-- Public! Can be mutated by anyone with &mut
    pub broadcast_status: BroadcastStatus,
    pub commit_txid: Option<String>,
    pub reveal_txid: Option<String>,
    pub broadcast_error: Option<String>,
}
```

**Invariants That Should Live Here But Don't:**

1. **Signature immutability after Approved state**: Once status == `Approved`, signatures **should never change**. The repo enforces this through the `update_broadcast_status` contract, not through the aggregate.  
   **Risk:** If a caller gets `&mut proposal` directly, they can add signatures to an Approved proposal, violating the state machine.

2. **Quorum threshold consistency**: `required_signatures` must not exceed the true canonical threshold for the authority. Code queries ASM at handler time (`asm_role_membership::threshold_for_authority`), but the aggregate cannot validate this invariant without a dependency.

3. **Status-to-Broadcast mapping**: Certain status transitions should forbid broadcast state changes:
   - `Pending` → only allow `Idle` → `CommitBroadcasted`
   - `Approved` → allow `CommitBroadcasted` → `CommitConfirmed` → `RevealBroadcasted` → `RevealConfirmed`
   - `Expired/Canceled` → broadcast must be `Idle` or `Failed`

   Code checks these at handler boundaries (`require_approved` in `proposals.rs:154`), not in the aggregate.

4. **Action payload immutability**: `action_hex` is public and never validated as SSZ-decodeable. It can contain garbage. The backend just passes it through; actual validation happens during broadcast (`broadcast_tx.rs:35–38`).

**Why This Violates Vernon's Rule #1:**

> *Model true invariants in consistency boundaries. Only include elements that MUST be consistent within the same transaction.*

The `Proposal` struct includes all seven fields, but they don't all share a single invariant. A signer set update (`MultisigUpdate` action) with invalid `action_hex` and an incoherent `status`+`broadcast_status` combination can coexist in memory. The invariant should be: *"A Proposal in Approved state with completed-reveal broadcast has an immutable action_hex and a concrete reveal_txid"* — that cluster belongs in the aggregate. Everything else (pending signatures, idle broadcast state) belongs in a separate aggregate or value object.

**Proof:** The in-memory repo's `add_signature` method (line 60–63) blindly appends a signature to `proposal.signatures` via `proposal.signatures.push()`. There is no check whether the proposal is already Approved. This works only because the application layer (`proposals.rs:87–94`) prevents the call from reaching the repo. But the aggregate is defenseless.

**Production Risk:** When Postgres persistence is added, multiple concurrent HTTP handlers might load the same Proposal, mutate it, and attempt to save. Without aggregate-level checks, race conditions will corrupt the state machine.

---

### 🔴 HIGH: Ubiquitous Language Drift — Five Authorities, Two Enums; Desktop Doesn't Know Them

**Finding:** The backend defines five authorities (`orchestrator-be/src/domain/authority.rs:6–11`):

```rust
pub enum Authority {
    AlpenAdmin,
    StrataAdmin,
    SequencerManager,
    SecurityCouncil,
    PayoutAdmin,
}
```

The desktop app's TypeScript frontend defines two (`desktop-app/src/types/auth-role.ts`):

```typescript
export enum AuthRole {
	StrataAdministrator = 'strata_administrator',
	StrataSequencerManager = 'strata_sequencer_manager',
}
```

**The Term Means Two Things:**

- In backend code, "authority" is a domain concept: a multisig governance role with its own signer set, threshold, and sequence number.
- In frontend code, "auth role" or "authority" (when used) conflates two meanings:
  1. An authentication/session role (which multisig are you claiming to sign for?).
  2. A protocol role on-chain (Alpen Admin has a specific signer set in the ASM state).

**Proof of Drift:** The story map (`docs/3-stories/story-map.md:7–18`) names five actors: *Alpen Admin Signer, Strata Admin Signer, Sequencer Manager Signer, Security Council Signer, Payout Admin Signer*. The PRD and acceptance tests assume all five. But the desktop frontend is coded to only offer two radio buttons. If a user is an Alpen Admin signer, they cannot use this app today—there is no UI to select that authority.

**Consequence for Invariant Enforcement:**

The backend's `SessionContext` struct (line 22–25 in `application/proposals.rs`) binds every operation to a single authority:

```rust
pub(crate) struct SessionContext<'a> {
    pub authority: Authority,
    pub signer_pubkey: &'a str,
}
```

This is correct DDD—one context per authority. But if the frontend and backend enums disagree on what authorities exist, **the contract is broken**. The backend can receive a request claiming `StrataAdmin` authority, but the frontend has no way to present that choice to a user who should be able to make it.

**Where It Breaks:**

1. **Incomplete Authority Mapping** (`infrastructure/asm_role_membership.rs:109–116`):
   ```rust
   fn authority_to_role(authority: Authority) -> Result<Role, String> {
       match authority {
           Authority::StrataAdmin => Ok(Role::StrataAdministrator),
           Authority::SequencerManager => Ok(Role::StrataSequencerManager),
           _ => Err(format!("authority `{authority:?}` is not mapped to ASM role authorization yet")),
       }
   }
   ```
   Three of the five authorities are unmapped. A backend handler that receives `AlpenAdmin` authority will fail with a cryptic "not mapped" error.

2. **Frontend Proposal Creation Block**:  
   If a Payout Admin signer launches the desktop app, there is no UI path to authenticate as Payout Admin, create a payout proposal, and sign it. They hit an invisible wall.

**Ubiquitous Language Violation:** "Authority" should mean the same thing across all layers. Today:
- Backend domain: Authority = one of five multisig roles.
- Frontend: AuthRole = one of two roles (and only when code is loaded).
- Database (planned): Authority will be whatever Postgres column name is chosen — likely `authority_id` or `role`, decoupling the term further.

**Production Risk:** When a user or auditor asks "why can't I sign as Payout Admin?" the answer will be scattered across five files with different spellings. Maintenance burden skyrockets.

---

### 🔴 HIGH: No Domain Events — Justified Absence or Missing Safety Signal?

**Finding:** The backend has no domain events. Commands (`create_update_action`, `approve_action`, `broadcast_commit_then_reveal`) mutate proposals directly and return the modified state. No events are emitted.

**Proof:**
- `application/proposals.rs`: 7 public async functions; zero return events.
- `domain/proposal.rs`: 5 status enum values, zero event types.
- `handlers/proposals.rs`: JSON responses echo the mutated proposal; no event collection or dispatch.

**Where Events Might Matter:**

1. **Coordination Among Signers**: When one signer approves a proposal, other signers should be notified. Today, the backend has no publish mechanism. Desktop app polling is the only option—inefficient and late.

2. **State Machine History**: The `ProposalStatus` and `BroadcastStatus` enums track the current state, but there's no immutable ledger of transitions:
   - When did the proposal move from Pending to Approved?
   - Who was the last signer to approve it?
   - Did a cancel happen, and was it race-free with a broadcast attempt?

   A production system needs this audit trail. Without events, you must query the database at every state-change point.

3. **Eventual Consistency**: ASM state and orchestrator state can diverge. If Bitcoin network confirms the reveal tx, but the orchestrator crashes before marking it as Enacted, a restart must detect the divergence. Events would enable a reconciliation service to subscribe to on-chain events and emit `OnChainEventDetected` domain events that the orchestrator listens for. Today, there's no such mechanism.

**Is the Absence Justified?**

**Yes, partially.** The orchestrator is explicitly positioned as a coordination layer, not an event-sourced system. The spec document `docs/architecture/overview.md:72` says: *"Backend is coordination only: proposal creation, signature collection, lifecycle tracking—never re-implement protocol validity rules."* This implies:
- No need for complex event publishing (that's Bitcoin's job).
- No need for temporal queries (signers are live and poll).
- No need for multiple views (there's only the proposal list).

**But it's incomplete.** The absence of events prevents:
- **Cross-signer notification** (violation of signer UX safety principle from `AGENTS.md`: "Explicit confirmation steps, authority context, high-signal errors").
- **Integration with payout flows** (planned Slice 4 in story map). Payout Admin depends on knowing when a governance proposal enacted.
- **Offline survivability testing**. If the backend is down, signers can still aggregate signatures manually—but with no event log, how do they know they've reached quorum? Today, they must poll the backend or manually count sigs. A signer-local event log would help.

**Verdict:** Not a blocker, but a design debt. Document it explicitly in an ADR so future developers don't accidentally re-implement event handling three times.

---

### 🔴 HIGH: Frontend/Backend Model Duplication — Faithful Mirror or Divergent Copy?

**Finding:** The desktop app has its own domain types (`src-tauri/src/domain/{authority, action, proposal}.rs`), separate from the backend. The readme (`docs/architecture/overview.md:213–217`) claims:

> *Strata crate isolation: infrastructure/action_codec.rs is the single module in the desktop app that imports strata_asm_params. All other layers talk in client-owned domain types.*

**Reality Check:**

The desktop `Authority` enum (`src-tauri/src/domain/authority.rs`) is unknown because the file wasn't listed in our glob. Let me assume it mirrors the backend for now. But the React frontend's `AuthRole` enum (2 roles) doesn't mirror either of them.

**Duplication Pattern:**

| Concept | Backend Type | Desktop Type | React Type |
|---------|---|---|---|
| Authority | 5-variant Enum | Unknown | 2-variant Enum |
| Proposal | Struct w/ Proposal, signatures, status | Unknown | Unknown |
| Signature | ProposalSignature struct | Unknown | Unknown |

**Problem:** If the backend adds `AlpenAdmin` support and emits it in a GET /proposals response, the React frontend will fail to render the authority dropdown. The TypeScript types won't have the variant.

**Worst Case:** Desktop infrastructure layer (`orchestrator_client.rs`) handles HTTP serialization/deserialization. If backend Authority serializes as `alpen_admin` but React types only know `strata_administrator` and `strata_sequencer_manager`, the JSON parse will reject unknown variants.

**Root Cause:** The ADR-005 (`docs/architecture/adrs/005-layered-architecture.md:100–104`) allows duplication by design:

> *Desktop app is a separate deployable. Domain types may overlap but are independently owned.*

This is correct DDD. But it requires **careful synchronization** between the two domains. The current setup has no sync mechanism:
- No shared types crate.
- No schema registry or code generation.
- No tests that verify backend responses deserialize on desktop.

**Production Risk:** A backend engineer adds a new authority to support Payout Admin. They update backend domain, handlers, and tests. Desktop tests (if they exist) still pass because they mock the backend. Six months later, a Payout Admin user logs in, the desktop backend client receives an Approved proposal with `authority: PayoutAdmin`, and the React component crashes on undefined enum variant.

---

### 🟡 MEDIUM: Proposal "Repository" Is Transaction-Less; Race Conditions Possible

**Finding:** The `ProposalRepository` trait (line 10–46 in `application/traits.rs`) defines eight async methods, each independently transactional:

```rust
pub(crate) trait ProposalRepository: Send + Sync {
    async fn save_proposal(&self, proposal: Proposal) -> Result<(), AppError>;
    async fn add_signature(&self, action_id: &ActionId, signer_pubkey: &str, sig_hex: &str) -> Result<Option<Proposal>, AppError>;
    async fn claim_broadcast(&self, action_id: &ActionId) -> Result<Proposal, AppError>;
    async fn update_broadcast_status(&self, action_id: &ActionId, status: BroadcastStatus, ...) -> Result<Option<Proposal>, AppError>;
    // ...
}
```

Each method is independently atomic. But **the business operation** `approve_action` (line 69–119 in `proposals.rs`) calls multiple methods:

```rust
pub(crate) async fn approve_action(...) -> Result<Proposal, AppError> {
    // 1. Load
    let existing = repo.find_by_action_id(action_id).await?;
    let proposal = existing.ok_or(AppError::NotFound)?;
    
    // 2. Check not already signed
    let already_signed = proposal.signatures.iter().any(|s| s.signer_pubkey == sig.signer_pubkey);
    if already_signed {
        return Err(AppError::Conflict("signer already signed".to_string()));
    }
    
    // 3. Add signature (atomic)
    let updated = repo.add_signature(action_id, &sig.signer_pubkey, &sig.signature_hex).await?;
    
    // 4. Check quorum and transition to Approved if reached (separate atomic call)
    if proposal.status == ProposalStatus::Pending && proposal.signatures.len() >= proposal.required_signatures as usize {
        let approved = repo.update_broadcast_status(action_id, ..., Some(ProposalStatus::Approved), ...).await?;
        return approved.ok_or(AppError::NotFound);
    }
}
```

**Race Condition:** Between steps 1 (load) and 3 (add), another handler might:
- Add the same signature (step 2 check fails because loaded state was stale).
- Or advance the proposal to Approved, and step 4 will then try to add a signature to an Approved proposal (which should be forbidden).

In-memory repo is safe because it holds a global `RwLock`. But Postgres repo (planned) will fail silently:

1. Handler A loads proposal with 1 signature, required = 2.
2. Handler B loads the same proposal.
3. Handler B adds signature → now 2 signatures → transitions to Approved.
4. Handler A checks step 1 state (stale): `signatures.len() = 1`, still < 2, so it doesn't transition.
5. A calls `add_signature` again—this appends Handler A's sig to already-Approved proposal.
6. Result: Approved proposal with 3 signatures, violating the contract.

**Why It Matters for DDD:**

The `Proposal` aggregate is supposed to enforce that signatures-after-Approved is impossible. But the repository's granular transactions prevent the aggregate from protecting this invariant. The business logic in `approve_action` tries to enforce it with a stale load-and-check pattern, which is inherently race-y.

**Fix:** Either:
- Add a `check_and_add_signature_atomic` method to the repo that rolls back if the proposal state changed.
- Move the invariant into the aggregate root with a method like `proposal.add_signature_if_not_approved(sig)` that the repo calls atomically.

---

### 🟡 MEDIUM: "Proposal" Overloaded — Conflates Offchain Coordination State + Onchain Broadcast State

**Finding:** The `Proposal` struct contains two independent state machines:

1. **Offchain coordination** (`status`): Pending → Approved → Enacted | Expired | Canceled.
2. **Onchain broadcast** (`broadcast_status`): Idle → CommitBroadcasted → CommitConfirmed → RevealBroadcasted → RevealConfirmed | Failed.

These have **different lifecycles, actors, and invariants**. Mixing them violates Vernon's Rule #1 (one invariant per aggregate).

**The Problem:**

An Approved proposal might have `broadcast_status = Idle` (not yet broadcast) or `broadcast_status = CommitConfirmed` (waiting for reveal) or `broadcast_status = RevealConfirmed` (done, waiting for ASM enactment).

The `ProposalStatus::Enacted` transition depends on `broadcast_status = RevealConfirmed` **and** some offchain marker (commit/reveal txids set). But there's no method on the aggregate to validate this invariant—it's enforced only by the application logic that calls `update_broadcast_status`.

Code at line 409–416 in `proposals.rs`:

```rust
repo.update_broadcast_status(
    action_id,
    BroadcastStatus::RevealConfirmed,
    Some(ProposalStatus::Enacted),       // <-- Only *here* is Enacted used
    Some(&commit_txid),
    Some(&reveal_txid),
    None,
)
```

**Design Smell:** If these two concepts are truly separate, split the aggregate:

- **ProposalAggregate** (offchain): action_id, seq_no, authority, status, required_signatures, action_hex, signatures.
- **BroadcastAggregate** (onchain coordination): proposal_id, broadcast_status, commit_txid, reveal_txid.

But that would require significant refactoring. For now, document the coupling and enforce it via code review.

---

### 🟡 MEDIUM: Protocol Type Leakage in Application Layer

**Finding:** The `application/proposals.rs` module imports and uses `strata_l1_txfmt::MagicBytes` and `bitcoin::Network` as direct parameters (line 194, 241, 321):

```rust
pub(crate) async fn prepare_broadcast_bundle(
    repo: &dyn ProposalRepository,
    btc_client: &dyn BitcoinRpcClient,
    asm_rpc_url: &str,
    operator_keypair: &UntweakedKeypair,     // <-- Bitcoin type
    network: Network,                         // <-- Bitcoin type
    action_id: &ActionId,
) -> Result<BroadcastBundle, AppError> { ... }
```

These are infrastructure types, not domain concepts. The application layer should not import them directly.

**Why It Matters:**

If Strata protocol changes `MagicBytes` representation or Bitcoin's `Network` enum changes, the application layer breaks. The application layer should be stable and independent of external crate versions.

**Proper Pattern:**

Define domain-level abstractions in `domain/`:

```rust
pub struct BroadcastConfig {
    pub magic_bytes: Vec<u8>,     // <-- domain-neutral representation
    pub network_type: NetworkType, // custom enum
}
```

Translate in the handler or infrastructure layer, not in the application.

---

### 🟡 MEDIUM: SessionContext in Application Layer Lacks Explicit Binding

**Finding:** `SessionContext` (line 22–25 in `application/proposals.rs`) is defined in the application layer, not domain:

```rust
pub(crate) struct SessionContext<'a> {
    pub authority: Authority,
    pub signer_pubkey: &'a str,
}
```

This ties the application layer to session concerns (authentication, authority scoping). But `SessionContext` is a domain concept—it represents the authenticated principal and their scope.

**Why It Matters:**

Domain logic should never know about HTTP sessions. If a cron job or batch process needs to approve a proposal on behalf of an authority (e.g., automated payout validation), it can't use `create_update_action` because it requires a `SessionContext`.

**Fix:** Move `SessionContext` to `domain/` (or create a `domain/session.rs` that defines the concept). Make it clear that the domain cares about *who is authorizing this* and *for which authority*, independent of HTTP.

---

### 🟢 LOW: Authority Mapping Incomplete (Three of Five Unmapped)

**Finding:** Line 113 in `infrastructure/asm_role_membership.rs`:

```rust
_ => Err(format!("authority `{authority:?}` is not mapped to ASM role authorization yet"))
```

Three authorities (AlpenAdmin, SecurityCouncil, PayoutAdmin) are not mapped. If a request arrives with one of these, the handler fails with a 400 BadRequest.

**Risk:** Low for now (story map Slice 0 only uses StrataAdmin). But this is a ticking bomb. When Slice 2 (all authorities) is implemented, this unmapped code path will silently break.

**Proof Test:** Add a test that calls `authority_to_role` with all five authorities and asserts all pass:

```rust
#[test]
fn all_five_authorities_must_map() {
    assert!(authority_to_role(Authority::AlpenAdmin).is_ok());
    assert!(authority_to_role(Authority::StrataAdmin).is_ok());
    assert!(authority_to_role(Authority::SequencerManager).is_ok());
    assert!(authority_to_role(Authority::SecurityCouncil).is_ok());
    assert!(authority_to_role(Authority::PayoutAdmin).is_ok());
}
```

This test will fail on the last three until they're mapped.

---

## Attack Narratives: How This Fails in Production

### Narrative 1: The Race Condition That Broke Approval

**Scenario:** Two signers, Alper and Balter, both click "Approve" for the same proposal within 100ms.

1. Alper's handler loads proposal (2 sigs collected, 3 required, status=Pending).
2. Balter's handler loads the same proposal (stale: also sees 2 sigs, 3 required, Pending).
3. Alper's handler adds Alper's sig (now 3 collected).
4. Application logic detects quorum: transitions to Approved.
5. Balter's handler adds Balter's sig (but now the proposal is Approved!).
6. Balter's application logic re-checks: `proposal.signatures.len() >= required_signatures` (now 4 >= 3, true), so it tries to transition again.
7. Repo's `update_broadcast_status` call succeeds (idempotent), returns Approved.
8. Later, an auditor counts signatures: 4 sigs on a 3-of-5 multisig. Something is very wrong.

**Why It Happened:** The aggregate didn't enforce the invariant; the application layer tried via a stale load. With Postgres (future), the stale check will be even more obvious—milliseconds of clock skew amplify the race.

**Production Impact:** Not a security breach (multisig signatures don't matter if one is invalid anyway), but it's a data integrity bug that auditors will flag.

---

### Narrative 2: The Desktop Signer Who Couldn't Sign

**Scenario:** A user is promoted to Alpen Admin. They launch the multisig desktop app to sign a governance action.

1. Wallet connects, address selected.
2. Authority dropdown shows: StrataAdministrator, StrataSequencerManager.
3. User looks for "Alpen Administrator"—not there.
4. User thinks the app is broken and closes it. Opens a support ticket: "Why can't I sign?"

**Why It Happened:** Frontend types and backend types diverged silently. No compile error; no test failure. Backend supports 5 authorities; frontend supports 2.

**Production Impact:** User friction. Support costs. If the user is a critical signer and they miss a deadline, governance delays.

---

### Narrative 3: The Proposal Status Machine That Nobody Understood

**Scenario:** An auditor reviews code to understand the proposal lifecycle.

1. They read `ProposalStatus` enum: 5 states (Pending, Approved, Enacted, Canceled, Expired).
2. They see those states mentioned in the architecture overview's state diagram (which is correct).
3. They read the code and notice `BroadcastStatus` with 6 states (Idle, CommitBroadcasted, etc.).
4. They ask: "Why two state machines? What's the invariant between them?"
5. Developer: "Um, well, when status is Approved, broadcast_status should be one of {CommitBroadcasted, CommitConfirmed, RevealBroadcasted, RevealConfirmed}, and when Enacted, it should be RevealConfirmed..."
6. Auditor: "Is that enforced in the code?"
7. Developer: "Well, no, it's enforced by the application layer logic."
8. Auditor: "Can you point me to the test that validates this invariant?"
9. Developer: "...there isn't one."

**Production Impact:** Maintainability debt. Onboarding friction. Subtle bugs that only show up under production load.

---

### Narrative 4: The Payout Admin Update That Nobody Mapped

**Scenario:** Slice 4 (Payout Admin) ships. A backend engineer adds support for the PayoutAdmin authority.

1. Updates `domain/authority.rs`: adds `PayoutAdmin` variant.
2. Updates handlers to accept PayoutAdmin sessions.
3. Forgets to update `asm_role_membership.rs` mapping.
4. Tests pass (mock endorsement) because the mapping is only used on real RPC calls.
5. Deploys to staging. Real Payout Admin signer tries to create a proposal: gets 500 Internal Error (bad request: "authority not mapped").
6. Payout Admin escalates to support; engineer frantically adds the mapping.

**Why It Happened:** The `authority_to_role` function is not tested with all five authorities. No failing test means the unmapped state is invisible.

**Production Impact:** Silent failures until tested in production. Delayed feature rollout.

---

### Narrative 5: The Offshore Backup That Revealed a Model Error

**Scenario:** The orchestrator backend crashes and needs recovery from database backups.

1. Restore Postgres snapshot from 12 hours ago.
2. Some Approved proposals have `broadcast_status = RevealConfirmed` and `commit_txid` and `reveal_txid` set, but `status = Approved` (not Enacted).
3. The re-started backend's reconciliation logic should detect that the reveal tx is on-chain, bump the status to Enacted, and notify signers.
4. But there's no such reconciliation logic. The backend has no knowledge of on-chain state.
5. The proposal hangs in Approved state forever.
6. Signers are confused: "Why is the governance action not active?"

**Why It Happened:** No domain events means no audit trail. No reconciliation event listeners means no recovery logic. The state machine is only defined by forward progress, not backward consistency checks.

**Production Impact:** Operational risk. Manual intervention required to fix state.

---

## Evidence Index (Paths)

| Finding | File | Lines | Evidence Type |
|---------|------|-------|---|
| Proposal lacks invariants, all-public fields | `orchestrator-be/src/domain/proposal.rs` | 89–103 | struct definition |
| `add_signature` blindly appends | `infrastructure/memory_repo.rs` | 60–63 | method impl |
| No invariant enforcement on status+broadcast | `application/proposals.rs` | 102–116 | state transition logic |
| Five authorities in backend | `domain/authority.rs` | 6–11 | enum definition |
| Two roles in frontend | `desktop-app/src/types/auth-role.ts` | 1–4 | enum definition |
| Authority mapping incomplete | `infrastructure/asm_role_membership.rs` | 109–116 | error path |
| No domain events | `application/proposals.rs` | entire file | absence |
| Bitcoin types in application layer | `application/proposals.rs` | 194, 241, 321 | function signatures |
| SessionContext in application | `application/proposals.rs` | 22–25 | struct definition |
| Race condition in approve logic | `application/proposals.rs` | 69–119 | stale-load pattern |
| Dual state machines in Proposal | `domain/proposal.rs` | 62–72, 18–26 | enum definitions |

---

## Smallest Fixes vs. Largest Bets

### Smallest Fixes (Quick Wins)

1. **Test for all five authorities in mapping** (30 min)
   - Add test in `asm_role_membership.rs:290–296` to assert all authorities map or fail fast.
   - Prevents Narrative 4 (Payout Admin update silent failure).

2. **Document the dual state machines** (1 hr)
   - Add an ADR explaining why `status` and `broadcast_status` are coupled.
   - Update code comments to explain invariants.
   - Prevents Narrative 3 (auditor confusion).

3. **Add a Proposal::can_add_signature() validator** (2 hr)
   - Move the "already signed" check into the aggregate.
   - Doesn't fix the race condition but makes the intent clear.

### Medium Fixes (Tactical)

4. **Make Proposal fields private and add getters** (4 hr)
   - Force mutations through explicit methods (`proposal.add_signature(...)` not `proposal.signatures.push(...)`).
   - Aggregate root becomes defensible.

5. **Extract Postgres test for stale-load races** (6 hr)
   - Write a test that spawns two concurrent HTTP handlers and simulates the race.
   - Confirm the race exists, then add a fix (check-and-add-atomic method in repo trait).

6. **Sync frontend and backend authority enums** (8 hr)
   - Add all five authorities to React `AuthRole` enum.
   - Add UI tests to verify the dropdown renders all five.
   - Add e2e test that creates a proposal for each authority.

### Largest Bets (Structural Refactor)

7. **Split Proposal into ProposalAggregate + BroadcastAggregate** (24 hr)
   - Separate the two state machines explicitly.
   - Requires repo trait redesign.
   - Worth doing before Postgres persistence is added; much harder after.

8. **Add domain events** (16 hr)
   - Define `ProposalEvent` enum (ProposalCreated, SignatureAdded, QuorumMet, BroadcastStarted, etc.).
   - Collect events in each aggregate operation.
   - Publish events to a simple in-process event bus.
   - Enables reconciliation logic and cross-signer notification.

9. **Migrate SessionContext to domain layer** (6 hr)
   - Move to `domain/session.rs`.
   - Make authentication orthogonal to domain objects.
   - Enables batch operations and testing without HTTP context.

---

## What Would Change My Mind (Missing Evidence / Experiments)

### Missing Evidence That Would Downgrade HIGH Issues to MEDIUM

1. **Full Postgres integration test** showing the `approve_action` race condition is impossible.
   - If the Postgres repo uses a single check-and-add-signature transaction, the race vanishes.
   - Currently unimplemented; can't confirm safety.

2. **E2E test spanning all five authorities** from frontend to backend.
   - If React can render and create proposals for Alpen Admin, Security Council, and Payout Admin, the frontend/backend model mismatch is only partial.
   - Today, only two authorities are tested; can't confirm coverage.

3. **Integration test with real Bitcoin network** showing the BroadcastAggregate state machine is enforced.
   - If the test confirms that a proposal in Enacted state always has reveal_txid set and on-chain confirmation, the coupling is actually safe.
   - Currently no such test; assumptions are unvalidated.

### Missing Evidence That Would Justify Event Absence

1. **Documented decision (ADR)** explaining why events were not added.
   - An explicit choice (with trade-offs) is better than an implicit one.
   - Missing today.

2. **Reconciliation job** that syncs orchestrator state with Bitcoin / ASM on-chain state.
   - If such a job exists, the lack of events becomes less risky.
   - Roadmap doesn't mention it; can't find code.

3. **Cross-signer notification service** (even async polling) that detects quorum and notifies signers.
   - Would mitigate the "signers miss the approval" risk.
   - Not mentioned in specs; can't find code.

### Experiments to Run

1. **Load test**: Spawn 100 concurrent approval requests for a 2-of-100 multisig. Count signatures. Expect exactly 2; flag if > 2.
2. **Schema validation**: Serialize/deserialize a backend Proposal as JSON on React. Expect round-trip equality; flag if parse fails.
3. **Authority coverage**: For each of the five authorities, create a proposal (via API mock if needed). Expect success for all five; fail fast on unmapped authority.

---

## Final Verdict

### Summary

The domain model **has the **shape** of proper DDD** (distinct layers, trait abstraction for repos, some separation of concerns), but it **lacks the **substance**—invariants are not enforced at the aggregate level, state machines are underspecified, and the language is not unified across frontend/backend.**

### Verdict: **REVISIONS REQUIRED**

**Why not APPROVED:**

1. **Proposal aggregate is indefensible** (BLOCKING). Race conditions are possible; invariants live in application layer, not aggregate. Violates Vernon's Rule #1.
2. **Ubiquitous language is fractured** (HIGH). Five authorities in backend, two in frontend, three unmapped. Synchronization risk is high.
3. **Missing event model** (HIGH, though justified). Prevents reconciliation and cross-signer coordination. Document the choice or add events.

**Why not REJECTED:**

1. Layering is present and mostly consistent (domain, application, infrastructure, handlers).
2. Repository pattern enables testing and Postgres migration.
3. The issues are correctable; no architectural dead-end.

### Path Forward

1. **Immediate (Sprint):**
   - Make Proposal fields private; add getters.
   - Add failing test for all five authorities in mapping.
   - Document the dual state machines in an ADR.

2. **Near-term (2 weeks):**
   - Sync frontend and backend authority enums (story map Slice 2 blocker).
   - Implement stale-load race test; confirm it fails with planned Postgres repo.
   - Add fix: atomic check-and-add-signature in repo trait.

3. **Medium-term (before Postgres ship):**
   - Split Proposal into ProposalAggregate + BroadcastAggregate.
   - Move SessionContext to domain layer.
   - Define domain events (even if not implemented yet).

---

**Audit completed:** 2026-05-13 12:58 UTC-3
