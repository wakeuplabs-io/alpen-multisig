# Spec: POC-4 Step 4 — Desktop ↔ Orchestrator Integration

## Objective

Align the desktop app's orchestrator client with the real orchestrator HTTP API so they can talk to each other. Validate with an integration test that starts a real orchestrator and runs the flow from the desktop's `HttpOrchestratorClient`.

## Scope

**Included:**
- Align desktop DTOs to match orchestrator's actual JSON responses
- Simplify: one `Proposal` type everywhere, no wrappers, no summaries
- Remove auth (no bearer token, no session token)
- Fix URLs (`/approve` instead of `/signatures`)
- Orchestrator `create_proposal` returns `Proposal` directly (no wrapper)
- Integration test: create → get → approve → get (verify 2 signatures)
- Update mock + existing desktop tests to match new contract

**NOT included:**
- `list_proposals` (not needed for integration test)
- Quorum detection / threshold
- Auth / sessions
- UI / Tauri commands

## Technical Design

### Orchestrator changes (minimal)

`create_proposal` handler returns `Json<Proposal>` with status 201 instead of `Json<CreateProposalResponse>`.

### Desktop changes

**Simplified trait:**
```rust
#[async_trait::async_trait]
pub(crate) trait OrchestratorClient: Send + Sync {
    async fn create_proposal(&self, request: CreateProposalRequest) -> Result<Proposal, OrchestratorError>;
    async fn get_proposal(&self, action_id: &str) -> Result<Proposal, OrchestratorError>;
    async fn approve_action(&self, action_id: &str, request: ApproveActionRequest) -> Result<Proposal, OrchestratorError>;
}
```

**Simplified DTOs (match orchestrator JSON exactly):**
```rust
// Requests
struct CreateProposalRequest { authority, seq_no, action_hex, signer_pubkey, signature_hex }
struct ApproveActionRequest { signer_pubkey, signature_hex }

// Response — reused from orchestrator's Proposal serialization
struct Proposal { action_id, seq_no, authority, status, action_hex, signatures }
struct ProposalSignature { signer_pubkey, signature_hex }
```

**Removed:** `ProposalResponse`, `ProposalDetail`, `ProposalSummary`, `SignatureResponse`, `ApprovalResult`, `SignatureInfo`, bearer token logic.

**HttpOrchestratorClient:** No auth. Just `base_url` + `reqwest::Client`.

### Integration test

Lives in desktop-app's test module. Starts a real orchestrator (tokio::spawn + axum listener on random port), then:

1. Signer A: `create_proposal` with action_hex + signature → get back Proposal with 1 sig
2. `get_proposal` → verify same data
3. Signer B: `approve_action` → get back Proposal with 2 sigs
4. `get_proposal` → verify 2 signatures present

### Production code vs. test helpers

**Production:** Simplified `OrchestratorClient` trait, DTOs, `HttpOrchestratorClient`, updated `proposals.rs` functions.

**Test helpers:** `start_test_server()` (spawns orchestrator on random port), mock client (updated).

## Test Cases

1. **Integration: create → get → approve → get** (real HTTP, real orchestrator)
2. **Existing unit tests updated** to new contract (mock-based, already exist)

## Module structure

No new modules. Existing files modified:
- `orchestrator_client.rs` — simplified trait + DTOs + HTTP impl
- `proposals.rs` — updated to use simplified types, approve returns Proposal
- `orchestator-be/src/handlers/proposals.rs` — create returns Proposal directly
