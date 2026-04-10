# Spec: POC-4 Step 1 — Desktop App Proposal Signing Flow

## Objective

Add proposal creation and signing capabilities to the desktop app's application layer, enabling the full propose → sign flow tested against a mocked orchestrator backend. This is the first step of POC-4 (Mini Coordination Flow) as defined in `docs/2-discovery/06-poc4-plan.md`.

The key architectural evolution is introducing a `OrchestratorClient` trait — the first real abstraction per ADR-002 — which enables testing the application layer in isolation with a mock orchestrator.

## Scope

### Included

- `OrchestratorClient` async trait abstracting orchestrator HTTP API
- `HttpOrchestratorClient` real implementation (adapts existing reqwest pattern)
- `MockOrchestratorClient` test-only implementation (in-memory)
- Application layer functions: `create_proposal`, `sign_proposal`, `list_proposals`, `get_proposal`
- Transport DTO types for proposals and signatures
- Unit tests using real `signing.rs` + mock orchestrator client

### NOT included

- Tauri commands (`commands.rs`) — no new commands
- Frontend (React) — no changes
- `state.rs` changes — existing auth state unchanged
- `signing.rs` changes — used as-is
- Orchestrator implementation — that's Step 2
- Auth flow changes — existing auth functions untouched

## Technical Design

### File organization

Per ADR-002, `application.rs` is currently 154 lines. Adding the backend client trait + proposal functions + types will exceed ~300 lines. Split into an `application/` directory:

```
desktop-app/src-tauri/src/
├── application/
│   ├── mod.rs              # Re-exports, auth functions (moved from application.rs)
│   ├── orchestrator_client.rs   # OrchestratorClient trait + HttpOrchestratorClient + transport DTOs
│   └── proposals.rs        # Proposal flow business logic
├── commands.rs             # Unchanged
├── signing.rs              # Unchanged
├── state.rs                # Unchanged
└── main.rs                 # Update: mod application (already points here)
```

### OrchestratorClient trait (`application/orchestrator_client.rs`)

```rust
/// Abstracts HTTP communication with the orchestrator backend.
/// Real implementation uses reqwest; test mock uses in-memory state.
#[async_trait::async_trait]
pub(crate) trait OrchestratorClient: Send + Sync {
    async fn create_proposal(
        &self,
        request: CreateProposalRequest,
    ) -> Result<ProposalResponse, OrchestratorError>;

    async fn list_proposals(
        &self,
        authority: &str,
        status: Option<&str>,
    ) -> Result<Vec<ProposalSummary>, OrchestratorError>;

    async fn get_proposal(
        &self,
        action_id: &str,
    ) -> Result<ProposalDetail, OrchestratorError>;

    async fn submit_signature(
        &self,
        action_id: &str,
        request: SubmitSignatureRequest,
    ) -> Result<SignatureResponse, OrchestratorError>;
}
```

**`OrchestratorError`** — typed error enum:
```rust
#[derive(Debug, thiserror::Error)]
pub(crate) enum OrchestratorError {
    #[error("HTTP request failed: {0}")]
    Request(String),
    #[error("Backend returned error {status}: {message}")]
    Backend { status: u16, message: String },
    #[error("Failed to deserialize response: {0}")]
    Deserialization(String),
}
```

**`HttpOrchestratorClient`** — wraps reqwest, injects bearer token from a shared `Mutex<Option<String>>`:
```rust
pub(crate) struct HttpOrchestratorClient {
    base_url: String,
    session_token: Arc<Mutex<Option<String>>>,
    client: reqwest::Client,
}
```

### Transport DTO types (`application/orchestrator_client.rs`)

These types define the contract between desktop app and orchestrator. The orchestrator (Step 2) must produce compatible JSON.

```rust
/// Request to create a proposal with initial signature.
#[derive(Debug, Serialize)]
pub(crate) struct CreateProposalRequest {
    pub(crate) authority: String,
    pub(crate) seq_no: u64,
    pub(crate) action_hex: String,
    pub(crate) signer_pubkey: String,
    pub(crate) signature_hex: String,
}

/// Response from creating a proposal.
#[derive(Debug, Deserialize)]
pub(crate) struct ProposalResponse {
    pub(crate) action_id: String,
    pub(crate) authority: String,
    pub(crate) seq_no: u64,
    pub(crate) action_hex: String,
    pub(crate) status: String,
    pub(crate) signatures: Vec<SignatureInfo>,
}

/// Summary of a proposal for list views.
#[derive(Debug, Deserialize)]
pub(crate) struct ProposalSummary {
    pub(crate) action_id: String,
    pub(crate) authority: String,
    pub(crate) seq_no: u64,
    pub(crate) status: String,
    pub(crate) signature_count: u32,
    pub(crate) threshold: u32,
}

/// Full proposal detail including all signatures.
#[derive(Debug, Deserialize)]
pub(crate) struct ProposalDetail {
    pub(crate) action_id: String,
    pub(crate) authority: String,
    pub(crate) seq_no: u64,
    pub(crate) action_hex: String,
    pub(crate) status: String,
    pub(crate) signatures: Vec<SignatureInfo>,
    pub(crate) threshold: u32,
}

/// A single signature on a proposal.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct SignatureInfo {
    pub(crate) signer_pubkey: String,
    pub(crate) signature_hex: String,
}

/// Request to submit a signature for an existing proposal.
#[derive(Debug, Serialize)]
pub(crate) struct SubmitSignatureRequest {
    pub(crate) signer_pubkey: String,
    pub(crate) signature_hex: String,
}

/// Response from submitting a signature.
#[derive(Debug, Deserialize)]
pub(crate) struct SignatureResponse {
    pub(crate) quorum_reached: bool,
    pub(crate) signatures_count: u32,
    pub(crate) threshold: u32,
}
```

### Production functions (`application/proposals.rs`)

```rust
/// Create a proposal: compute sighash, sign it, send to orchestrator with first signature.
pub(crate) async fn create_proposal(
    client: &dyn OrchestratorClient,
    secret_key_hex: &str,
    authority: &str,
    seq_no: u64,
    action_hex: &str,
) -> Result<ProposalResponse, ProposalError>

/// Sign an existing proposal: compute sighash, sign it, submit signature to orchestrator.
pub(crate) async fn sign_proposal(
    client: &dyn OrchestratorClient,
    secret_key_hex: &str,
    action_id: &str,
    action_hex: &str,
    seq_no: u64,
) -> Result<SignatureResponse, ProposalError>

/// List proposals for an authority, optionally filtered by status.
pub(crate) async fn list_proposals(
    client: &dyn OrchestratorClient,
    authority: &str,
    status: Option<&str>,
) -> Result<Vec<ProposalSummary>, ProposalError>

/// Get full details of a specific proposal.
pub(crate) async fn get_proposal(
    client: &dyn OrchestratorClient,
    action_id: &str,
) -> Result<ProposalDetail, ProposalError>
```

**`ProposalError`** — typed error enum:
```rust
#[derive(Debug, thiserror::Error)]
pub(crate) enum ProposalError {
    #[error("Signing failed: {0}")]
    Signing(String),
    #[error("Orchestrator error: {0}")]
    Client(#[from] OrchestratorError),
}
```

### Flow diagrams

**Create proposal:**
```
create_proposal(client, secret_key, authority, seq_no, action_hex)
    │
    ├── signing::compute_sighash(seq_no, action_hex)  → sighash_hex
    ├── signing::sign_sighash(secret_key, sighash_hex) → (pubkey, signature)
    └── client.create_proposal(CreateProposalRequest { authority, seq_no, action_hex, pubkey, signature })
            → ProposalResponse
```

**Sign proposal:**
```
sign_proposal(client, secret_key, action_id, action_hex, seq_no)
    │
    ├── signing::compute_sighash(seq_no, action_hex)  → sighash_hex
    ├── signing::sign_sighash(secret_key, sighash_hex) → (pubkey, signature)
    └── client.submit_signature(action_id, SubmitSignatureRequest { pubkey, signature })
            → SignatureResponse
```

### Production code vs. test helpers

**Production functions** (exposed to consumers):
- `create_proposal`, `sign_proposal`, `list_proposals`, `get_proposal` — called by Tauri commands in future steps
- `OrchestratorClient` trait + `HttpOrchestratorClient` — production orchestrator communication
- All DTO types — shared contract with orchestrator

**Test helpers** (`#[cfg(test)]` only):
- `MockOrchestratorClient` — in-memory orchestrator mock that records calls and returns canned responses
- Key generation helpers (reuse pattern from `signing.rs` tests)
- Demo action builder (reuse from `signing.rs` tests)

## Test Cases

### Happy path
1. **`test_create_proposal_computes_sighash_signs_and_sends`** — Create proposal with valid action + key → mock receives correct sighash, signature, and pubkey
2. **`test_sign_proposal_computes_sighash_signs_and_submits`** — Sign existing proposal → mock receives correct signature submission
3. **`test_list_proposals_returns_filtered_results`** — List proposals with status filter → returns mock's canned proposals
4. **`test_get_proposal_returns_detail`** — Get proposal by action_id → returns mock's canned detail

### Round-trip consistency
5. **`test_create_then_get_proposal_data_consistent`** — Create proposal → get proposal → action_hex, seq_no, authority match

### Signing correctness
6. **`test_create_proposal_signature_is_verifiable`** — Create proposal → extract signature from mock's received request → verify with `signing::verify_threshold` → valid

### Error cases
7. **`test_create_proposal_invalid_action_hex_fails`** — Invalid action hex → `ProposalError::Signing`
8. **`test_create_proposal_invalid_secret_key_fails`** — Invalid secret key → `ProposalError::Signing`
9. **`test_create_proposal_backend_error_propagates`** — Mock returns error → `ProposalError::Client`
10. **`test_sign_proposal_backend_error_propagates`** — Mock returns error on submit → `ProposalError::Client`

## Module structure

All new code lives in `desktop-app/src-tauri/src/application/`. No shared crate extraction needed yet — `signing.rs` is standalone in the desktop app, and the orchestrator doesn't need these types (it defines its own domain types). Shared crate extraction is triggered when signing logic is needed by `e2e-tests` (Slice 2+).

### Dependency: `async-trait`

The `OrchestratorClient` trait uses async methods. Add `async-trait` to `desktop-app/src-tauri/Cargo.toml`:
```toml
async-trait = "0.1"
```
