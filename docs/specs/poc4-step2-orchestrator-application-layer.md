# Spec: POC-4 Step 2 — Orchestrator Application Layer

## Objective

Implement the orchestrator's application layer with real business logic (replacing `todo!()` stubs) for proposal CRUD and signature collection. Tested against an in-memory repository. This is Step 2 of POC-4 as defined in `docs/2-discovery/06-poc4-plan.md`.

## Scope

### Included

- `ProposalRepository` trait — persistence abstraction for proposals + signatures
- `InMemoryProposalRepository` — in-memory implementation for tests
- Application layer functions: `create_update_action`, `approve_action`, `get_update_action`, `list_proposals`
- `ActionId` computation: `sha256(seq_no_be_bytes || action_hex_bytes)` — deterministic
- Duplicate `(action, seq_no)` rejection
- Duplicate signer signature rejection (same signer on same proposal)
- Unit tests against in-memory repository

### NOT included

- Authentication / session management — not touched, stays `todo!()`
- SignerSet / signer validation — not implemented
- Quorum detection — not implemented (will be mocked when needed)
- HTTP handlers — not touched (Step 3)
- Middleware — not touched
- Frontend — no changes

## Technical Design

### Domain type changes

**`action_payload`**: Change from `serde_json::Value` to `String` (hex-encoded bytes). The backend is opaque to the action content — it only stores and returns hex bytes. Aligns with desktop app's `action_hex`.

**`Proposal`**: Simplify for POC — remove `Uuid id` (ActionId is the identity), remove timestamps (not needed yet).

Updated domain types:

```rust
/// Deterministic proposal identity: sha256(seq_no_be_bytes || action_hex_bytes).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActionId(pub String);

/// A multisig proposal stored by the coordination backend.
#[derive(Debug, Clone)]
pub struct Proposal {
    pub action_id: ActionId,
    pub seq_no: SeqNo,
    pub authority: Authority,
    pub status: ProposalStatus,
    /// Hex-encoded MultisigAction payload (opaque to backend).
    pub action_hex: String,
    pub signatures: Vec<ProposalSignature>,
}

/// A signature submitted for a proposal.
#[derive(Debug, Clone)]
pub struct ProposalSignature {
    pub signer_pubkey: String,
    pub signature_hex: String,
}
```

### File organization

Per ADR-002, `application.rs` will exceed ~300 lines. Split into `application/` directory:

```
orchestator-be/src/
├── application/
│   ├── mod.rs           # Re-exports + auth stubs (unchanged todo!())
│   ├── proposals.rs     # Business logic: create, approve, get, list
│   └── repository.rs    # ProposalRepository trait + InMemoryProposalRepository
├── domain/              # Unchanged (except Proposal simplification)
├── handlers/            # Unchanged (still todo!() wiring)
└── ...
```

**Single responsibility:**
- `repository.rs` — owns the persistence contract (trait + in-memory impl)
- `proposals.rs` — owns the business logic (CRUD orchestration, ActionId computation, duplicate rejection)
- `mod.rs` — re-exports + auth stubs (untouched)

### ProposalRepository trait (`application/repository.rs`)

```rust
pub(crate) trait ProposalRepository: Send + Sync {
    fn save_proposal(&mut self, proposal: Proposal) -> Result<(), AppError>;
    fn find_by_action_id(&self, action_id: &ActionId) -> Option<&Proposal>;
    fn find_by_action_id_mut(&mut self, action_id: &ActionId) -> Option<&mut Proposal>;
    fn list_by_status(&self, status: Option<ProposalStatus>) -> Vec<&Proposal>;
}
```

`InMemoryProposalRepository` — `HashMap<ActionId, Proposal>`.

### Application functions (`application/proposals.rs`)

```rust
/// Compute ActionId = sha256(seq_no_be_bytes || action_hex_bytes).
pub(crate) fn compute_action_id(seq_no: SeqNo, action_hex: &str) -> Result<ActionId, AppError>

/// Create a new proposal with first signature. Rejects duplicate ActionId.
pub(crate) fn create_update_action(
    repo: &mut dyn ProposalRepository,
    authority: Authority,
    seq_no: SeqNo,
    action_hex: &str,
    signer_pubkey: &str,
    signature_hex: &str,
) -> Result<Proposal, AppError>

/// Add a signature to an existing proposal. Rejects duplicate signer.
pub(crate) fn approve_action(
    repo: &mut dyn ProposalRepository,
    action_id: &ActionId,
    signer_pubkey: &str,
    signature_hex: &str,
) -> Result<Proposal, AppError>

/// Get proposal by ActionId.
pub(crate) fn get_update_action(
    repo: &dyn ProposalRepository,
    action_id: &ActionId,
) -> Result<Proposal, AppError>

/// List proposals, optionally filtered by status.
pub(crate) fn list_proposals(
    repo: &dyn ProposalRepository,
    status: Option<ProposalStatus>,
) -> Vec<Proposal>
```

Note: `authority` is passed explicitly in Step 2 since there's no auth. In production, it will come from the authenticated session.

### ActionId computation

```
ActionId = hex(sha256(seq_no_be_bytes(8) || hex_decode(action_hex)))
```

This is deterministic — same `(action, seq_no)` always produces the same ActionId. Uses `sha2` (already in Cargo.toml).

### Error mapping

Reuse existing `AppError`:
- Duplicate ActionId → `AppError::Conflict("proposal already exists")`
- Duplicate signer → `AppError::Conflict("signer already signed")`
- Proposal not found → `AppError::NotFound`
- Invalid action_hex (bad hex) → `AppError::BadRequest("invalid action hex")`

### Production code vs. test helpers

**Production:**
- `ProposalRepository` trait + `InMemoryProposalRepository` (in-memory is production for POC, Postgres later)
- `compute_action_id`, `create_update_action`, `approve_action`, `get_update_action`, `list_proposals`

**Test helpers** (`#[cfg(test)]` only):
- None expected — `InMemoryProposalRepository` is the test double AND the POC implementation

## Test Cases

### Happy path
1. **`test_create_update_action`** — Create with valid data → returns proposal with correct ActionId, action_hex, first signature
2. **`test_approve_action`** — Create then approve → proposal has 2 signatures
3. **`test_get_update_action`** — Create then get → returns consistent data
4. **`test_list_proposals`** — Create multiple → list returns all
5. **`test_list_proposals_with_status_filter`** — Create multiple with different status → filter works

### ActionId determinism
6. **`test_action_id_is_deterministic`** — Same (action_hex, seq_no) → same ActionId
7. **`test_action_id_differs_by_seq_no`** — Same action, different seq_no → different ActionId
8. **`test_action_id_differs_by_action`** — Different action, same seq_no → different ActionId

### Duplicate rejection
9. **`test_create_duplicate_action_rejected`** — Create same (action, seq_no) twice → Conflict error
10. **`test_approve_duplicate_signer_rejected`** — Same signer signs twice → Conflict error

### Error cases
11. **`test_approve_nonexistent_proposal`** — Approve unknown action_id → NotFound
12. **`test_get_nonexistent_proposal`** — Get unknown action_id → NotFound
13. **`test_create_invalid_action_hex`** — Non-hex action → BadRequest

## Module structure

```
orchestator-be/src/application/
├── mod.rs           — Re-exports public API + unchanged auth stubs
├── proposals.rs     — Business logic for proposal CRUD (single responsibility: orchestration)
└── repository.rs    — ProposalRepository trait + InMemoryProposalRepository (single responsibility: persistence contract)
```

**Dependency direction:**
```
proposals.rs (business logic)
    → depends on repository.rs (ProposalRepository trait)
    → depends on domain/ (Proposal, ActionId, Authority, etc.)
```

Business logic depends on abstractions (trait), never the reverse.
