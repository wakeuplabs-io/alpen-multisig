# ADR-002: Application Layer Strategy

**Status:** Accepted
**Date:** 2026-04-09
**Context:** The project has two applications (orchestrator backend and desktop app) with growing business logic currently inlined in handlers/commands. We need a minimal abstraction layer that enables testability without over-engineering upfront.

## Decision

### Architecture approach

Both applications adopt a **minimal application layer** that separates business logic from framework concerns (Axum handlers, Tauri commands). The architecture starts intentionally thin and evolves toward clean architecture as complexity demands it.

### Current structure (Phase 1 — minimal)

Each application adds a single `application.rs` file between the framework layer and the domain:

**Orchestrator Backend:**

```
orchestator-be/src/
├── main.rs
├── config.rs
├── state.rs
├── error.rs
├── domain/            # Pure types (already exists)
├── application.rs     # Business logic: auth, proposals, signatures
├── handlers/          # Axum HTTP layer (thin, delegates to application)
└── middleware/
```

**Desktop App (Tauri):**

```
desktop-app/src-tauri/src/
├── main.rs
├── state.rs
├── commands.rs        # #[tauri::command] functions (thin, delegates to application)
├── application.rs     # Business logic: backend calls, session management
└── signing.rs         # Signing library (already exists, standalone)
```

### Rules for the application layer

1. **Handlers/commands are thin** — parse input, call application, format output. No business logic.
2. **`application.rs` owns the logic** — validation, orchestration, state transitions, error decisions.
3. **No traits or ports yet** — application functions use concrete types directly (Postgres pool, reqwest client, etc.).
4. **Testable by design** — application functions receive their dependencies as parameters, not via globals. This enables testing without mocks by passing test doubles later.

### Evolution path (when to grow)

| Trigger | Action |
|---------|--------|
| `application.rs` exceeds ~300 lines | Split into `application/` directory with one file per domain area (e.g., `auth.rs`, `proposals.rs`, `signatures.rs`) |
| Need to test business logic without Postgres/HTTP | Extract traits (ports) for repositories and external services |
| Signing logic needed by multiple crates | Extract `crates/multisig-core/` shared crate |
| Payout Admin diverges significantly from other authorities | Consider bounded context separation |

### What we explicitly avoid

- Generic command/query bus or mediator patterns
- Event sourcing or domain events
- Formal DDD tactical patterns (aggregate roots, value objects as distinct types)
- Premature trait abstractions — traits are introduced when testing or swapping demands them, not before

## Consequences

1. **Positive:** Handlers stay thin from day one, making it easy to swap frameworks or add new interfaces later.
2. **Positive:** Business logic is testable in isolation once traits are introduced — the function signatures are already designed for it.
3. **Positive:** Low ceremony — one file per app, no boilerplate.
4. **Risk:** Without traits, early unit tests may need a running database. Mitigated by prioritizing integration tests initially and extracting traits when the testing cost becomes apparent.
5. **Risk:** The single `application.rs` file may grow quickly. Mitigated by the clear trigger (300 lines) to split into a directory.
