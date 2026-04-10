# Spec: POC-4 Step 1 — Desktop App Application Layer

## Objective

Add proposal coordination capabilities to the desktop app's application layer, enabling the full propose → approve flow tested against a mocked orchestrator. This is Step 1 of POC-4 (Mini Coordination Flow) as defined in `docs/2-discovery/06-poc4-plan.md`.

See [ADR-003](../architecture/adrs/003-desktop-application-layer-api.md) for the full API design rationale, PRD terminology analysis, and evolution path.

## Scope

### Included

- `OrchestratorClient` async trait abstracting orchestrator HTTP API
- `HttpOrchestratorClient` real implementation (reqwest + bearer token)
- `MockOrchestratorClient` test-only implementation (in-memory)
- Application layer functions aligned with PRD `MultisigBackend`
- Domain types owned by the application layer (not transport DTOs)
- Unit tests using real `signing.rs` + mock orchestrator client

### NOT included

- Tauri commands — no new commands (Step 3+)
- Frontend — no changes
- `signing.rs` — used as-is
- Orchestrator implementation — Step 2
- `seq_no` auto-resolution — requires orchestrator `get_last_seqno` (future slice)
- `MultisigAction` as input type — accepts `action_hex` for now (see ADR-003 evolution path)

## Design Decisions

### Naming (see ADR-003)

The application layer uses **proposal** as its coordination concept, aligned with the backend PRD's "Proposal Semantics" section. The function names mirror the PRD code sketch's `MultisigBackend` trait.

### Key principles

1. **Authority is session-scoped** — never passed as parameter, resolved from the authenticated session by the orchestrator
2. **Signing is external** — the application layer never sees private keys; it receives pre-signed `Signature { signer_pubkey, signature_hex }`
3. **Domain types, not DTOs** — consumers see `Proposal`, `ApprovalResult`, etc., not orchestrator transport types
4. **PRD-aligned API** — function names match `MultisigBackend`: `create_update_action`, `approve_action`, `get_update_action`

### POC-4 compromises (documented in ADR-003)

| What | POC-4 | Production target |
|------|-------|-------------------|
| Action input | `action_hex: &str` | `action: &MultisigAction` |
| Sequence number | `seq_no: u64` (explicit) | Auto-resolved via `get_last_seqno` |
| Authority | Implicit (session) | Implicit (session) |
| Signing | External (pre-signed) | External (HW wallet) |

## Technical Design

### File organization

```
desktop-app/src-tauri/src/
├── application/
│   ├── mod.rs                  # Auth functions (existing) + submodule declarations
│   ├── orchestrator_client.rs  # Trait + HTTP impl + transport DTOs (orchestrator's contract)
│   └── proposals.rs            # Application layer entry point + domain types
├── commands.rs                 # Unchanged
├── signing.rs                  # Unchanged
├── state.rs                    # Unchanged
└── main.rs
```

**Single responsibility per file:**
- `orchestrator_client.rs` — owns the transport contract with the orchestrator (DTOs, trait, HTTP impl)
- `proposals.rs` — owns the business API for consumers (domain types, orchestration functions, DTO→domain mapping)

### Application layer API (`proposals.rs`)

```rust
// Domain types
pub(crate) struct Signature { signer_pubkey, signature_hex }
pub(crate) struct Proposal { action_id, authority, seq_no, action_hex, status, signatures, threshold }
pub(crate) struct ProposalSummary { action_id, authority, seq_no, status, signature_count, threshold }
pub(crate) struct ApprovalResult { quorum_reached, signatures_count, threshold }

// PRD-aligned functions
create_update_action(client, action_hex, seq_no, &signature) -> Proposal
approve_action(client, action_id, &signature) -> ApprovalResult
get_update_action(client, action_id) -> Proposal
list_proposals(client, status?) -> Vec<ProposalSummary>
```

### OrchestratorClient trait (`orchestrator_client.rs`)

```rust
#[async_trait]
pub(crate) trait OrchestratorClient: Send + Sync {
    async fn create_proposal(&self, request: CreateProposalRequest) -> Result<ProposalResponse, OrchestratorError>;
    async fn list_proposals(&self, status: Option<&str>) -> Result<Vec<ProposalSummary>, OrchestratorError>;
    async fn get_proposal(&self, action_id: &str) -> Result<ProposalDetail, OrchestratorError>;
    async fn submit_signature(&self, action_id: &str, request: SubmitSignatureRequest) -> Result<SignatureResponse, OrchestratorError>;
}
```

Authority is **not** a parameter — the orchestrator resolves it from the bearer token (session-scoped).

### Flow diagram

```
Consumer (Tauri command / CLI / test)
    │
    │  1. Build MultisigAction (using Alpen crates)
    │  2. Serialize to action_hex
    │  3. Compute sighash (signing::compute_sighash)
    │  4. Sign externally (HW wallet / signing::sign_sighash)
    │
    ▼
proposals::create_update_action(client, action_hex, seq_no, &signature)
    │
    │  Maps domain Signature → transport CreateProposalRequest
    │
    ▼
OrchestratorClient::create_proposal(request)
    │
    │  HTTP POST /proposals (bearer token injected)
    │
    ▼
Orchestrator Backend
    │  Computes ActionId = hash(action, seq_no)
    │  Stores action + first signature
    │  Returns ProposalResponse
    │
    ▼
proposals::create_update_action
    │  Maps transport ProposalResponse → domain Proposal
    │
    ▼
Consumer receives Proposal { action_id, status, signatures, ... }
```

### Production code vs. test helpers

**Production** (exposed to consumers):
- `proposals.rs`: domain types + orchestration functions
- `orchestrator_client.rs`: trait + HTTP impl + transport DTOs

**Test helpers** (`#[cfg(test)]` only):
- `MockOrchestratorClient` — records calls, returns canned responses
- `sign_action()` — simulates external signing using `signing.rs`
- Key generation and demo action builders

## Test Cases

### Happy path
1. **`test_create_update_action`** — Create with valid pre-signed data → orchestrator receives correct request, returns domain Proposal
2. **`test_approve_action`** — Approve existing proposal → orchestrator receives signature, returns ApprovalResult
3. **`test_get_update_action`** — Fetch by action_id → returns domain Proposal with correct fields
4. **`test_list_proposals`** — List with status filter → returns domain ProposalSummary list

### Round-trip consistency
5. **`test_create_then_get_consistent`** — Create → get → authority, seq_no match

### Signing correctness
6. **`test_signature_is_verifiable`** — Create → extract signature from mock → verify with `signing::verify_threshold` → valid

### Error propagation
7. **`test_create_backend_error_propagates`** — Orchestrator error → `ProposalError::Orchestrator`
8. **`test_approve_backend_error_propagates`** — Orchestrator error on approve → `ProposalError::Orchestrator`

## Module structure

All new code lives in `desktop-app/src-tauri/src/application/`. No shared crate extraction needed yet.

**Dependency direction:**
```
proposals.rs (business logic)
    → depends on orchestrator_client.rs (trait + DTOs)
        → HttpOrchestratorClient depends on reqwest (infrastructure)
```

Business logic depends on abstractions (the trait), never the reverse. Transport DTOs live with their contract owner (the orchestrator client module).

### Dependencies added

```toml
async-trait = "0.1"
thiserror = "2"
```
