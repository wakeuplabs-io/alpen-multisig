# Spec: POC-4 Step 3 — Orchestrator HTTP Handlers

## Objective

Wire the orchestrator's HTTP handlers to the application layer so real HTTP requests flow through to the in-memory repository. This is the minimum needed for the desktop app to talk to a running orchestrator in Step 4.

## Scope

**Included:**
- Wire 5 handlers to application layer functions
- Add `ProposalRepository` to `AppState` (behind `Arc<RwLock<…>>`)
- `authority` passed as a field in the create proposal request body (no auth)
- HTTP integration tests using `axum::test` / `tower::ServiceExt`

**NOT included:**
- Authentication, sessions, middleware
- Signer set validation
- Quorum detection
- Auth handlers (remain `todo!()`)
- Frontend, Tauri, desktop app changes

## Technical Design

### AppState change

```rust
pub struct AppState {
    pub config: Config,
    pub repo: Arc<RwLock<InMemoryProposalRepository>>,
}
```

### Endpoints to wire

| Method | Path | Handler | Delegates to |
|--------|------|---------|-------------|
| `POST` | `/proposals` | `create_proposal` | `proposals::create_update_action` |
| `GET` | `/proposals` | `list_proposals` | `proposals::list_proposals` |
| `GET` | `/proposals/:action_id` | `get_proposal` | `proposals::get_update_action` |
| `POST` | `/proposals/:action_id/signatures` | `submit_signature` | `proposals::approve_action` |
| `GET` | `/proposals/:action_id/signatures` | `list_signatures` | extracts `proposal.signatures` |

### Request/Response types (handlers)

**`POST /proposals`** — `CreateProposalRequest`:
```rust
pub struct CreateProposalRequest {
    pub authority: Authority,
    pub seq_no: u64,
    pub action_hex: String,
    pub signer_pubkey: String,
    pub signature_hex: String,
}
```
Response: `CreateProposalResponse { action_id, proposal }`

**`GET /proposals?status=pending`** — optional query filter.
Response: `ProposalListResponse { proposals: Vec<Proposal> }`

**`GET /proposals/:action_id`**
Response: `Proposal` (directly serialized)

**`POST /proposals/:action_id/signatures`** — `SubmitSignatureRequest`:
```rust
pub struct SubmitSignatureRequest {
    pub signer_pubkey: String,
    pub signature_hex: String,
}
```
Response: `SubmitSignatureResponse { proposal }` (return updated proposal)

**`GET /proposals/:action_id/signatures`**
Response: `SignatureListResponse { signatures: Vec<ProposalSignature> }`

### Production code vs. test helpers

**Production:**
- Modified `AppState` with repo field
- 5 handler functions (wired, no longer `todo!()`)
- Updated `main.rs` to initialize repo in state

**Test helpers:**
- `test_app()` — builds a Router with in-memory repo for integration tests
- Request builder helpers in `#[cfg(test)]`

## Test Cases

All tests are HTTP integration tests (request → router → handler → app layer → in-memory repo → response).

### `create_proposal`
1. **Happy path**: POST valid proposal → 201, returns action_id + proposal
2. **Duplicate rejected**: POST same (action_hex, seq_no) twice → 409 Conflict
3. **Invalid hex**: POST with bad action_hex → 400

### `list_proposals`
4. **Empty list**: GET with no proposals → 200, empty array
5. **Returns all**: Create 2 proposals, GET → 200, 2 items
6. **Filter by status**: GET `?status=pending` → only pending proposals

### `get_proposal`
7. **Happy path**: Create, then GET by action_id → 200, correct proposal
8. **Not found**: GET nonexistent action_id → 404

### `submit_signature`
9. **Happy path**: Create proposal, POST signature → 200, proposal with 2 signatures
10. **Duplicate signer rejected**: POST same signer twice → 409
11. **Nonexistent proposal**: POST signature to unknown action_id → 404

### `list_signatures`
12. **Happy path**: Create proposal + add signature, GET signatures → 200, 2 items
13. **Not found**: GET signatures for unknown action_id → 404

## Module structure

- **`state.rs`** — AppState with config + shared repo. Single responsibility: application-wide shared state.
- **`handlers/proposals.rs`** — HTTP handlers for proposal CRUD. Single responsibility: HTTP ↔ application layer mapping for proposals.
- **`handlers/signatures.rs`** — HTTP handlers for signature operations. Single responsibility: HTTP ↔ application layer mapping for signatures.
- **`handlers/mod.rs`** — Router definition (unchanged structure).

No new modules. Existing handler files get their `todo!()` replaced with real wiring.
