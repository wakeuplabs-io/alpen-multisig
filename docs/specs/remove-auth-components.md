# Spec: Remove Unused Authentication Components and Dead Code

## Objective
Remove unused authentication flow components and all other dead code from `orchestrator-be` and `desktop-app`. The core `Authority` domain concept used by proposals will be preserved.

## Scope
**Included:**
- `orchestrator-be/src/handlers/auth.rs` — handler functions
- `orchestrator-be/src/application/auth.rs` — business logic
- `orchestrator-be/src/middleware/auth.rs` — middleware
- `orchestrator-be/src/main.rs` — auth routes registration
- `orchestrator-be/src/error.rs` — Unauthorized/Forbidden variants
- `orchestrator-be/src/domain/session.rs` — AuthChallenge/SessionInfo types
- `orchestrator-be/src/application/mod.rs` — auth module declaration
- `orchestrator-be/src/handlers/mod.rs` — auth module declaration
- `orchestrator-be/src/middleware/mod.rs` — auth module declaration
- `desktop-app/src-tauri/src/commands/auth.rs` — Tauri commands
- `desktop-app/src-tauri/src/application/auth.rs` — orchestrator client calls
- `desktop-app/src-tauri/src/commands/mod.rs` — auth module declaration
- `desktop-app/src-tauri/src/application/mod.rs` — auth module declaration
- `desktop-app/src/api/auth.ts` — API client functions
- `desktop-app/src/hooks/useAuth.ts` — authentication hook and state

**NOT included:**
- `Authority` enum and related types — used by proposals
- Proposal or signature handling logic
- Quorum detection in proposal domain (planned feature)
- Signer verification in authority domain (planned feature)
- Config in state (used at startup)

**Additional Dead Code Removal:**
Remove all `#[allow(dead_code)]` attributes in orchestrator-be that are marked as "Planned: auth flow" or related to removed auth components:
- `error.rs`: Remove Unauthorized/Forbidden variants and their dead_code attributes
- `domain/session.rs`: Remove entire file (auth-related)
- `handlers/auth.rs`: Already removed with module

## Technical Design
All changes are removals of dead code marked with `#[allow(dead_code)]`.

### orchestrator-be
- Remove auth handler, application, and middleware modules
- Remove auth routes from router
- Remove Unauthorized/Forbidden error variants
- Remove AuthChallenge/SessionInfo domain types

### desktop-app
- Remove auth Tauri commands and application layer
- Remove auth API client functions
- Remove useAuth hook

## Module Structure
No new modules created — only removals.

## Test Cases
No tests to add — this is removal of dead code. Existing tests should continue to pass.