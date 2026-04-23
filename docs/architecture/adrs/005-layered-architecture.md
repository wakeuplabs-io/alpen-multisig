# ADR-005: Layered Architecture for Backend and Desktop App

**Status:** Implemented
**Date:** 2026-04-11
**Context:** The orchestrator backend and desktop Tauri app have grown organically during POC phases. Both already have partial layering (domain/, application/, handlers/commands), but boundaries are blurry — domain types live inside application modules, repository implementations sit next to business logic, and the desktop app lacks an explicit domain layer. Before building production features (Postgres persistence, full auth, HWI integration), we need clear module boundaries to avoid coupling that becomes expensive to untangle later.

## Decision

Adopt a **lightweight layered architecture** with 3–4 layers per app. Dependencies flow strictly downward. No hexagonal ports/adapters ceremony, no Clean Architecture use-case classes — only the abstractions the project actually needs today.

### Alternatives considered

| Architecture | Verdict | Reason |
|---|---|---|
| Clean Architecture (Uncle Bob) | Rejected | Use-case interactors, input/output ports, and presenter layers add ceremony without value for a CRUD coordination service with a narrow domain |
| Hexagonal (Ports & Adapters) | Partially adopted | The repository trait pattern (one port, one adapter) is valuable for swapping in-memory → Postgres. Full port/adapter separation is overhead for a single-DB, single-API service |
| Vertical Slices | Rejected | The domain is narrow (proposals + auth across 5 authorities). Slicing by feature would fragment code that shares 80% of its types |
| Alpen-style crate-per-module | Reference only | Designed for 144-crate workspaces. Our scope (2 apps, bounded domain) doesn't justify the crate overhead |

### Orchestrator Backend (`orchestrator-be`)

```
orchestrator-be/src/
├── main.rs                    # Bootstrap: config, router, middleware, server
├── config.rs                  # Env-based configuration
├── error.rs                   # AppError enum → HTTP response mapping
├── state.rs                   # AppState: config + repos + services
│
├── domain/                    # Layer 1: Pure types, no external dependencies
│   ├── mod.rs
│   ├── authority.rs           # Authority enum, SignerPubkey, SignerSet
│   ├── proposal.rs            # Proposal, ActionId, SeqNo, ProposalStatus
│   ├── session.rs             # Session, AuthChallenge, ephemeral key types
│   └── signature.rs           # Signature types, SignerIndex
│
├── application/               # Layer 2: Business logic, orchestrates domain + traits
│   ├── mod.rs
│   ├── proposals.rs           # create, approve, list, get, cancel, expire
│   ├── auth.rs                # challenge generation, session creation/validation
│   └── traits.rs              # ProposalRepository, SessionRepository (trait definitions)
│
├── infrastructure/            # Layer 3a: Concrete implementations of traits
│   ├── mod.rs
│   ├── memory_repo.rs         # InMemoryProposalRepository (dev/test)
│   └── postgres_repo.rs       # PostgresProposalRepository (future)
│
├── handlers/                  # Layer 3b: HTTP boundary (Axum extractors → application)
│   ├── mod.rs
│   ├── auth.rs                # GET /auth/challenge, POST /auth/session
│   └── proposals.rs           # CRUD + approve endpoints
│
└── middleware/                 # Cross-cutting: auth extractor, request logging
    ├── mod.rs
    └── auth.rs                # Bearer token → AuthenticatedSession extractor
```

**Key rules:**
- `domain/` has zero dependencies on application, infrastructure, or framework crates (no Axum, no sqlx, no reqwest).
- `application/` depends on `domain/` and defines traits. It never imports concrete implementations.
- `infrastructure/` implements the traits defined in `application/`. It depends on `domain/` for types and on external crates (sqlx, etc.) for implementation.
- `handlers/` depends on `application/` and `domain/`. Handlers are thin: parse request → call application → format response.
- `main.rs` is the composition root: it wires concrete implementations into application services and passes them to handlers via `AppState`.

**Migration from current structure:**
1. Extract proposal domain types from `application/proposals.rs` into `domain/proposal.rs`.
2. Move `application/repository.rs` → split into `application/traits.rs` (trait definition) + `infrastructure/memory_repo.rs` (implementation).
3. Promote auth stubs from `application/mod.rs` to `application/auth.rs`.

### Desktop App — Tauri (`desktop-app/src-tauri`)

```
desktop-app/src-tauri/src/
├── main.rs                    # Tauri bootstrap, registers commands
├── lib.rs                     # Public exports for e2e-tests
├── state.rs                   # AppState (session token, backend_url)
│
├── domain/                    # Layer 1: Pure client-side domain types
│   ├── mod.rs
│   ├── authority.rs           # Authority enum, signer types
│   ├── proposal.rs            # Proposal, ActionId, ProposalStatus, QuorumStatus
│   └── session.rs             # SessionInfo, auth-related types
│
├── commands/                  # Layer 3: Tauri IPC boundary (#[tauri::command])
│   ├── mod.rs
│   ├── auth.rs                # get_challenge, create_session, delete_session
│   └── proposals.rs           # list_proposals, create_proposal, approve_action
│
├── application/               # Layer 2: Business logic
│   ├── mod.rs
│   ├── auth.rs                # Auth flow logic
│   └── proposals.rs           # Proposal operations via OrchestratorClient trait
│
├── infrastructure/            # Layer 3: Concrete implementations
│   ├── mod.rs
│   └── orchestrator_client.rs # HttpOrchestratorClient (reqwest)
│
└── signing.rs                 # Standalone library: crypto ops (no layers, pure functions)
```

**Key rules:**
- `domain/` holds types that represent the client's view of the model. These may overlap with backend domain types but are independently owned — the desktop app is a separate deployable.
- `commands/` are the equivalent of backend `handlers/`: thin IPC boundary that delegates to `application/`.
- `infrastructure/orchestrator_client.rs` implements the `OrchestratorClient` trait. The trait definition lives in `application/` (or a shared `traits.rs`).
- `signing.rs` remains a standalone, Tauri-decoupled module. It has no layer — it's a pure crypto library consumed by `application/` and by `e2e-tests`.

**Migration from current structure:**
1. Create `domain/` and extract types currently embedded in `application/proposals.rs` and `state.rs`.
2. Split `commands.rs` → `commands/auth.rs` + `commands/proposals.rs`.
3. Move `application/orchestrator_client.rs` → `infrastructure/orchestrator_client.rs`, keep the trait in `application/`.

### Dependency direction (both apps)

```
handlers / commands
        │
        ▼
   application
     │      │
     ▼      ▼
 domain   infrastructure
     ▲         │
     └─────────┘
```

`infrastructure` depends on `domain` for types but never on `application` logic. `application` defines traits that `infrastructure` implements. The composition root (`main.rs`) wires them together.

### What this ADR does NOT cover

- **React frontend architecture** — will be addressed in a separate ADR when UI implementation begins.
- **Shared crate extraction** — if backend and desktop domain types diverge significantly, we may extract a shared `multisig-types` crate. Not needed yet.
- **Persistence schema** — Postgres table design will be a separate ADR.

## Consequences

- **Positive:** Clear boundaries make Postgres migration straightforward (implement a new struct behind the existing trait). HWI integration gets a clean home in `infrastructure/`. New team members can navigate the codebase by layer.
- **Positive:** Minimal refactoring from current structure — we're sharpening existing boundaries, not rewriting.
- **Negative:** Some initial churn moving types between modules. All existing tests must be updated for new import paths.
- **Accepted trade-off:** The repository trait is the only formal abstraction boundary. Everything else is enforced by module visibility (`pub(crate)`) and convention, not by trait indirection. This is intentional — more abstractions can be added when proven necessary, but premature abstractions are harder to remove.
