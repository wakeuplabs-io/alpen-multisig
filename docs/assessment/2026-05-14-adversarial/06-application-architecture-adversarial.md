# Application architecture — Adversarial Assessment (re-audit)

**Date:** 2026-05-14  
**Lens:** Solution architect (boundaries, composition root, ADR alignment, `AppState` cohesion)  
**Method:** Read-only; evidence paths cite current modules and ADRs.

---

## Scope & threat model

**What we are trying to break:**

- **Layering guarantees:** Application logic, HTTP adapters, and infrastructure leak into each other until changes require wide refactors.
- **Composition root clarity:** `AppState` becomes a “god bag” holding unrelated concerns; tests cannot substitute behavior cleanly.
- **Cross-cutting auth:** Session and authority context are inconsistently enforced at the HTTP boundary — “authenticated but not authorized” handlers.
- **ADR drift:** ADR-002 (minimal application layer) and ADR-005 (layered backend + desktop) disagree with living code in naming, middleware placement, or stated rules.

**In scope:** `orchestrator-be/src/state.rs`, `handlers/`, `application/`, `domain/`, `main.rs`, ADR-002, ADR-005; brief cross-check to desktop layout per ADR-005.

**Out of scope:** Full UI architecture (React) except where IPC/desktop layering is cited by ADR.

---

## Top findings (ranked by severity)

### BLOCKER: A1 — HTTP handlers discard session authority for list, get, and broadcast routes while still requiring a bearer token

**Risk:** Call pattern is “auth n gate” without **authorization** tied to multisig authority or membership — any valid signer session may exercise endpoints that reveal or mutate proposals outside their authority (details overlap security review; architectural issue is **boundary contract violation**: handlers bypass `SessionContext` pattern used elsewhere).

**Evidence:** `orchestrator-be/src/handlers/proposals.rs` — `list_proposals` and `get_proposal` take `_auth: AuthenticatedSession` but call `proposals::list_proposals` / `get_update_action` with **no** `auth.authority` filter; `prepare_broadcast` and `execute_broadcast` likewise use `_auth` only to satisfy extractor. Compare to `create_proposal` / `approve_action`, which build `SessionContext { authority, signer_pubkey }`.

**Failure scenario:** Team adds “admin” tooling assuming bearer == scoped user; architecture encourages copy-paste of `_auth` when adding endpoints — systemic Authorization bypass class.

**Smallest fix:** Every mutating and confidentiality-sensitive handler passes `authority` into application layer; repository `list_by_status` gains optional authority filter; broadcast paths verify session authority matches proposal.

**Largest bet:** Central policy layer (e.g., `authorize(&session, &resource) -> Result`) shared by handlers.

**Disconfirming probe:** Grep `_auth: AuthenticatedSession` in `proposals.rs` — multiple occurrences with underscore.

---

### CRITICAL: A2 — `AppState` mixes durable coordination, Bitcoin RPC, signer secrets, and ephemeral in-process auth maps

**Risk:** High cohesion **across stability classes** (durable proposals vs RAM-only auth challenges/sessions vs `operator_keypair`) makes lifecycle and security reviews harder; scaling-out story for auth state is absent while proposal repo may be Postgres.

**Evidence:** `orchestrator-be/src/state.rs` — fields: `repo`, `asm_rpc_url`, `btc_client`, `operator_keypair`, `challenges: Arc<RwLock<HashMap<...>>>`, `sessions: Arc<RwLock<HashMap<...>>>`, TTL knobs, broadcast tuning, `bitcoin_magic_bytes`, `bitcoin_network`.

**Failure scenario:** Horizontal scaling or rolling restart invalidates all sessions; operators mistake “stateless API” for “safe to duplicate behind LB” without sticky sessions or external session store — architectural trap.

**Smallest fix:** Extract `AuthState` service trait; document “single-instance or external session store required” in architecture SSOT.

**Largest bet:** Redis-backed sessions + sticky-less API design; split read models for list vs detail.

---

### HIGH: A3 — ADR-005 diagram references `middleware/auth.rs`; handler tests live inside `handlers/mod.rs` — discoverability / drift risk

**Risk:** New contributors cannot find auth middleware from ADR path; tests co-located with `router` increase merge conflicts and obscure application-level invariants.

**Evidence:** `docs/architecture/adrs/005-layered-architecture.md` — lists `middleware/auth.rs`; repository uses `handlers/auth_session.rs` and `#[cfg(test)] mod tests` in `handlers/mod.rs` (large fixture block).

**Smallest fix:** Update ADR file tree to match repo; move route tests to `tests/` or `handlers/tests.rs` per project convention.

**Largest bet:** Contract tests generated from OpenAPI for every route + auth matrix.

---

### HIGH: A4 — Composition root encodes network inference (`bitcoin_network` from RPC URL substring)

**Risk:** Configuration knowledge is split between `config.rs` and `main.rs` “magic ports” — surprising behavior when using proxies, non-standard ports, or dual-stack URLs; harder to test matrix of networks.

**Evidence:** `orchestrator-be/src/main.rs` — `if config.bitcoin_rpc_url.contains("18443")` → `Regtest`, etc.

**Smallest fix:** Explicit `BITCOIN_NETWORK` env or structured URL parser with override.

**Largest bet:** Config crate with validated types, snapshot tests for config examples.

---

### MEDIUM: A5 — ADR-002 explicitly avoids formal patterns later embraced by ADR-005 (layer directories, traits)

**Risk:** Onboarding documents contradict each other — “no traits yet” vs repository already on `ProposalRepository` trait and `application/` directory.

**Evidence:** `docs/architecture/adrs/002-application-layer-strategy.md` — Phase 1 `application.rs`, “No traits or ports yet”; `docs/architecture/adrs/005-layered-architecture.md` — full `application/` + `traits.rs` layout. Current code matches ADR-005, not Phase-1 ADR-002 snapshot.

**Failure scenario:** Security review cites wrong ADR; reviewers under-test repository traits because ADR says they do not exist.

**Smallest fix:** Mark ADR-002 superseded by ADR-005 or add “Current state (2026)” banner to ADR-002.

**Largest bet:** Single architecture SSOT page generating diagrams from repo structure.

---

## Attack narratives

1. **The “thin handler” regression:** Engineer adds `GET /proposals/export` by copying `list_proposals`; uses `_auth`. **Outcome:** Export remains global-scope even if list is later fixed — pattern spreads via example code.

2. **The “scale out Friday” deploy:** DevOps adds second replica behind LB; sessions in `RwLock` maps diverge. **Outcome:** intermittent 401/404 on verify; architecture gave no single-instance guardrail in type system.

3. **The “ADR archaeology” audit:** External auditor reads ADR-002 only, concludes no repository ports. **Outcome:** false assurance; delayed findings on Postgres vs memory parity.

---

## Evidence index (paths)

| Area | Path |
|------|------|
| App composition / state shape | `orchestrator-be/src/state.rs` |
| Composition root | `orchestrator-be/src/main.rs` |
| HTTP routes + health | `orchestrator-be/src/handlers/mod.rs` |
| Proposal handlers / auth usage | `orchestrator-be/src/handlers/proposals.rs` |
| Application services | `orchestrator-be/src/application/proposals.rs` |
| Persistence port | `orchestrator-be/src/application/traits.rs` |
| Layered architecture ADR | `docs/architecture/adrs/005-layered-architecture.md` |
| Earlier application ADR | `docs/architecture/adrs/002-application-layer-strategy.md` |

---

## Smallest fixes vs largest bets

| Finding | Smallest fix | Largest bet |
|---------|--------------|-------------|
| A1 | Pass `authority` through handlers; repo filters | Policy engine + audit log per decision |
| A2 | Document single-instance; extract auth service | External session store; split services |
| A3 | ADR tree sync; relocate tests | OpenAPI-driven conformance tests |
| A4 | Explicit network config | Validated config crate |
| A5 | Supersedence note on ADR-002 | Generated SSOT docs |

---

## What would change my mind

- **A1:** Evidence that list/get/broadcast are intentionally **global** for a closed operator network and product docs supersede PRD isolation — with threat model signed by stakeholders (then downgrade to “documented trade-off”).
- **A2:** Deploy manifests explicitly requiring **max replica = 1** for orchestrator with enforced admission webhook.

---

## Conclusion

**Architecture strengths:** Clear `domain` / `application` / `infrastructure` split matches ADR-005; `ProposalRepository` trait preserves swapability; `SessionContext` exists for some flows.

**Structural gap:** **HTTP boundary consistency** is broken where `_auth` appears — the most severe architectural issue because it undermines the stated layering rule “handlers are thin **and** enforce access context.” Combine with **`AppState` mixing ephemeral auth and durable coordination** for a maturity picture that is **not yet multi-instance or enterprise-audit ready**. Resolve ADR-002 vs ADR-005 for contributor truth.
