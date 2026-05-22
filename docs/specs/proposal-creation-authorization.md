# Spec: Proposal Creation Authorization by Canonical Signer Set

## Objective

Enforce that proposal creation is allowed only for authenticated signers that belong to the canonical signer set of the selected authority, as required by the PRD/backend guidelines.

This spec closes the current gap where `authority` is client-provided in the proposal creation request (POC simplification) and moves the source of truth to the authenticated session scope.

## Scope

### Included

- Authorization rule for `create proposal`: signer membership in canonical signer set of session authority.
- Session-scoped authority for write operations (`create proposal`, and same pattern for other write endpoints).
- Removal of caller-controlled authority from the create proposal payload.
- Unified backend behavior for unauthorized/non-signer access (no proposal existence leak).
- Backend tests (application + handler/integration) for membership and authority isolation.
- Desktop app adjustments to stop sending authority in create proposal requests.

### NOT included

- Protocol-level validity checks from SPS-65 (sequence validity, threshold correctness, replay protection).
- Quorum detection and lifecycle transitions beyond existing behavior.
- New proposal types or UI product flows for composing actions.
- Full signer-set synchronization architecture (this spec defines interface/contract and checks, not chain indexer implementation details).

## Requirements Alignment

### PRD / backend requirements this spec enforces

- Only addresses in the canonical signer set for a selected authority can create proposals.
- Non-signers must be treated as non-signers and denied access.
- Authorization decisions must use canonical signer sets derived from current onchain ASM state.
- Session must be authority-scoped and bounded; write operations must honor that scope.

## Technical Design

### 1) Authorization model

For `POST /proposals`, backend must evaluate all of the following:

1. Request has a valid authenticated session.
2. Session is bound to exactly one authority.
3. Session signer public key belongs to the canonical signer set for that authority.
4. Operation executes in that authority scope only.

If any check fails, request is rejected as unauthorized.

### 2) API contract changes

#### Current (POC)

`CreateProposalRequest` includes `authority`.

#### Target

`CreateProposalRequest` no longer includes `authority`; authority is derived from session context.

```rust
pub struct CreateProposalRequest {
    pub seq_no: u64,
    pub action_hex: String,
    pub signer_pubkey: String,
    pub signature_hex: String,
}
```

Notes:
- `signer_pubkey` in body is preserved in this slice to minimize surface change.
- Backend must verify consistency between body signer and authenticated session signer identity; mismatch is unauthorized.

### 3) Backend layering changes

#### Handlers (`orchestrator-be/src/handlers/proposals.rs`)

- Resolve session context via auth middleware/extractor (authority + signer identity).
- Do not read authority from request body.
- Call application service with `session_authority` and `session_signer_pubkey`.

#### Application (`orchestrator-be/src/application/proposals.rs`)

Extend create path with explicit auth context:

```rust
pub(crate) fn create_update_action_authorized(
    repo: &mut dyn ProposalRepository,
    signer_set_repo: &dyn SignerSetRepository,
    session_authority: Authority,
    session_signer_pubkey: &str,
    seq_no: SeqNo,
    action_hex: &str,
    sig: &ProposalSignature,
) -> Result<Proposal, AppError>
```

Checks in order:

1. `sig.signer_pubkey == session_signer_pubkey`
2. `session_signer_pubkey` is member of canonical signer set for `session_authority`
3. Existing hygiene/business checks (valid hex, duplicate action ID, etc.)

#### New repository contract (application trait)

```rust
pub(crate) trait SignerSetRepository: Send + Sync {
    fn is_signer_for_authority(&self, authority: Authority, signer_pubkey: &str) -> Result<bool, AppError>;
}
```

Implementation can be in-memory/mock for tests now; production can be backed by ASM-derived state.

### 4) Error and response behavior

To reduce information leaks:

- Unauthorized, invalid session, signer mismatch, and non-member access all return the same generic unauthorized response (`401` or `403`, choose one globally and keep consistent).
- Do not include reason strings that reveal signer-set membership details.

### 5) Desktop app contract updates

#### Rust Tauri app-layer (`desktop-app/src-tauri/src/application/orchestrator_client.rs`)

- Remove `authority` from `CreateProposalRequest`.

#### Rust proposal application layer (`desktop-app/src-tauri/src/application/proposals.rs`)

- `create_update_action` no longer accepts `authority` argument for HTTP request construction.
- Authority is implied by authenticated backend session.

#### Frontend (`desktop-app/src/api/...`)

- Any proposal creation API wrapper stops sending `authority`.
- UI authority selector remains for auth/session selection, not per-request override.

## Production code vs. test helpers

### Production

- Auth-aware proposal creation handler path.
- `SignerSetRepository` trait and concrete implementation adapter.
- Session-scoped authority extraction and signer consistency checks.
- Desktop client DTO changes (no authority in create request).

### Test helpers

- In-memory/mock signer-set repository with fixture memberships.
- Test fixtures for authorized signer, cross-authority signer, and non-signer cases.

## Test Cases

### Backend application tests

1. **Authorized signer can create proposal**  
   Session signer is member of authority signer set -> success.
2. **Non-member signer rejected**  
   Session signer not in signer set -> unauthorized.
3. **Signer mismatch rejected**  
   Body signer != session signer -> unauthorized.
4. **Cross-authority signer rejected**  
   Signer belongs to another authority only -> unauthorized.
5. **Existing duplicate behavior preserved**  
   Authorized create duplicate `(seq_no, action_hex)` -> conflict.

### Backend handler/integration tests

6. **Create without auth session rejected**
7. **Create with expired/invalid session rejected**
8. **Create with valid session but non-member signer rejected**
9. **Create with valid member signer succeeds and stores authority from session scope**
10. **Unauthorized responses are uniform (no membership detail leakage)**

### Desktop app tests

11. **Create request payload no longer includes authority**
12. **Create flow still succeeds with authenticated session**
13. **Backend unauthorized errors propagate with high-signal but generic messaging**

## Rollout Notes

1. Land backend authz checks and test coverage first.
2. Then update desktop request DTO and app layer call sites.
3. Keep a short compatibility window only if necessary; prefer atomic backend+desktop update in same PR to avoid mixed contracts.

## Module structure

Expected touched modules:

- `orchestrator-be/src/handlers/proposals.rs`
- `orchestrator-be/src/application/proposals.rs`
- `orchestrator-be/src/application/traits.rs` (or equivalent location for `SignerSetRepository`)
- `orchestrator-be/src/infrastructure/*` (signer-set adapter)
- `desktop-app/src-tauri/src/application/orchestrator_client.rs`
- `desktop-app/src-tauri/src/application/proposals.rs`
- `desktop-app/src/api/*` (proposal creation request shape)

## Implementation status

### Completed

- Orchestrator now exposes auth endpoints (`POST /auth/challenge`, `POST /auth/verify`, `POST /auth/logout`) and issues short-lived bearer tokens after challenge verification + signer-set membership checks.
- `POST /proposals`, `GET /proposals`, `GET /proposals/:action_id`, and `POST /proposals/:action_id/approve` now require `Authorization: Bearer <token>`.
- Backend create authorization enforces:
  - session signer equals request signer
  - session signer is in canonical signer set for session authority
- Backend approve authorization now enforces:
  - session signer equals request signer
  - session signer is in canonical signer set for session authority
  - proposal authority matches session authority
- Unauthorized cases now map to a uniform `401 unauthorized` response.
- `CreateProposalRequest` no longer includes `authority` in backend and desktop app-layer contracts.
- `SignerSetRepository` now validates membership with `is_signer_for_authority(authority, signer)`.
- Desktop HTTP orchestrator client now requires bearer tokens for auth-required calls and includes orchestrator challenge/verify/logout methods.
- Tests updated and passing:
  - backend unit/integration
  - desktop unit tests
  - focused e2e propose/approve flow

### Deferred / follow-up

- Frontend API wrapper (`desktop-app/src/api/*`) proposal call-site integration is still pending because proposal creation UI flow is not yet wired in this branch.
- `SignerSetRepository` is still backed by in-memory fixtures in this slice. Production hardening still requires a canonical ASM-backed signer-set adapter in orchestrator-be.

