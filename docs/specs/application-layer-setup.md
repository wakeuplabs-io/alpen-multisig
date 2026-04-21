# Spec: Application Layer Setup

## Objective

Introduce a minimal `application.rs` in both the orchestrator backend and the desktop app (Tauri) to separate business logic from framework concerns. This establishes the layer boundary defined in [ADR-002](../architecture/adrs/002-application-layer-strategy.md) without changing behavior — all existing `todo!()` stubs remain as `todo!()` in the new location.

## Scope

### Included

- **Backend:** Create `orchestator-be/src/application.rs` with public functions matching each handler. Update handlers to delegate to application functions.
- **Desktop:** Extract `AppState` to `src/state.rs`, create `src/application.rs` with logic extracted from `main.rs`, create `src/commands.rs` with thin `#[tauri::command]` wrappers, reduce `main.rs` to Tauri setup only.
- Update `mod` declarations in both apps.

### NOT included

- New features or business logic implementation
- New dependencies
- Traits, ports, or abstractions
- Changes to `signing.rs`
- Changes to `domain/`
- Changes to frontend (React)
- Database or persistence layer

## Technical Design

### Orchestrator Backend

**New file: `src/application.rs`**

```rust
pub(crate) fn get_challenge(pubkey: &str, authority: &str) -> crate::error::Result<AuthChallenge> {
    todo!()
}

pub(crate) fn create_session(payload: CreateSessionInput) -> crate::error::Result<Session> {
    todo!()
}

pub(crate) fn delete_session(session_id: &str) -> crate::error::Result<()> {
    todo!()
}

pub(crate) fn list_proposals(authority: Authority, status: Option<ProposalStatus>) -> crate::error::Result<Vec<Proposal>> {
    todo!()
}

pub(crate) fn create_proposal(authority: Authority, seq_no: SeqNo, action_payload: serde_json::Value) -> crate::error::Result<Proposal> {
    todo!()
}

pub(crate) fn get_proposal(action_id: &str) -> crate::error::Result<Proposal> {
    todo!()
}

pub(crate) fn submit_signature(action_id: &str, signer_pubkey: &str, signature: &str) -> crate::error::Result<SubmitSignatureResult> {
    todo!()
}

pub(crate) fn list_signatures(action_id: &str) -> crate::error::Result<Vec<ProposalSignature>> {
    todo!()
}
```

Functions use domain types (`Authority`, `Proposal`, `ProposalStatus`, `SeqNo`, `ProposalSignature`) from `crate::domain`. Input/result structs specific to the application layer (e.g., `CreateSessionInput`, `SubmitSignatureResult`) are defined in `application.rs`.

**Handler changes:** Each handler calls the corresponding application function and maps the result. The handler request/response DTOs stay in `handlers/`. Example:

```rust
// handlers/auth.rs
pub async fn get_challenge(
    State(_state): State<AppState>,
    Query(params): Query<ChallengeQuery>,
) -> Result<Json<ChallengeResponse>> {
    let challenge = crate::application::get_challenge(&params.signer_pubkey, &params.authority)?;
    Ok(Json(ChallengeResponse {
        nonce: challenge.nonce,
        expires_at: challenge.expires_at.to_rfc3339(),
    }))
}
```

**`main.rs` change:** Add `mod application;` declaration. No other changes.

### Desktop App (Tauri)

**New file: `src/state.rs`**

Extract `AppState` from `main.rs`:

```rust
pub(crate) struct AppState {
    pub(crate) session_token: Mutex<Option<String>>,
    pub(crate) backend_url: String,
}
```

**New file: `src/application.rs`**

Move the actual reqwest/Mutex logic from the current Tauri commands:

```rust
pub(crate) async fn fetch_challenge(backend_url: &str, pubkey: &str, authority: &str) -> Result<AuthChallenge, String> {
    // reqwest GET /auth/challenge — logic currently in main.rs get_challenge
}

pub(crate) async fn create_session(
    backend_url: &str,
    session_token: &Mutex<Option<String>>,
    payload: CreateSessionPayload,
) -> Result<SessionInfo, String> {
    // reqwest POST /auth/session + store token — logic currently in main.rs create_session
}

pub(crate) async fn delete_session(
    backend_url: &str,
    session_token: &Mutex<Option<String>>,
) -> Result<(), String> {
    // reqwest DELETE /auth/session — logic currently in main.rs delete_session
}

pub(crate) async fn fetch_proposals(
    backend_url: &str,
    session_token: &Mutex<Option<String>>,
    status: Option<String>,
) -> Result<serde_json::Value, String> {
    // reqwest GET /proposals — logic currently in main.rs list_proposals
}
```

Types used by the application layer (`AuthChallenge`, `BackendSession`, `SessionInfo`, `CreateSessionPayload`) move from `main.rs` to `application.rs`.

**New file: `src/commands.rs`**

Thin `#[tauri::command]` wrappers that extract `State<AppState>` and delegate:

```rust
#[tauri::command]
pub(crate) async fn get_challenge(
    state: State<'_, AppState>,
    pubkey: String,
    authority: String,
) -> Result<AuthChallenge, String> {
    crate::application::fetch_challenge(&state.backend_url, &pubkey, &authority).await
}
```

**`main.rs` reduces to:**

```rust
mod application;
mod commands;
mod signing;
mod state;

fn main() {
    let backend_url = std::env::var("BACKEND_URL").unwrap_or_else(|_| "...".to_string());

    tauri::Builder::default()
        .manage(state::AppState { ... })
        .invoke_handler(tauri::generate_handler![
            commands::get_challenge,
            commands::create_session,
            commands::delete_session,
            commands::list_proposals,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Production code vs. test helpers

All changes are production code (structural refactor). No new test helpers needed. Existing `signing.rs` tests remain unchanged.

## Test Cases

This is a structural refactor with no behavioral changes. Testing focuses on compilation and existing test preservation:

1. `cargo build` — entire workspace compiles
2. `cargo test -p orchestator-be` — passes (no tests exist yet, but compilation must succeed)
3. `cargo test` in desktop-app/src-tauri — signing tests (13) still pass
4. `cargo clippy` — no new warnings
5. `cargo fmt --check` — properly formatted
6. `cd desktop-app && npm run build` — frontend still builds

## Module structure

```
orchestator-be/src/
├── main.rs              # Axum setup (adds: mod application)
├── config.rs
├── state.rs
├── error.rs
├── domain/              # Unchanged
├── application.rs       # NEW: business logic stubs
├── handlers/            # Updated: delegate to application
└── middleware/

desktop-app/src-tauri/src/
├── main.rs              # Reduced to Tauri setup only
├── state.rs             # NEW: AppState extracted from main.rs
├── commands.rs          # NEW: thin #[tauri::command] wrappers
├── application.rs       # NEW: reqwest/session logic from main.rs
└── signing.rs           # Unchanged
```
