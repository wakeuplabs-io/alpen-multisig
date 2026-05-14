# Rust Backend (orchestrator-be) — Adversarial Assessment

## Scope & threat model (what we're trying to break)

- **Authority isolation**: Can an authenticated signer for authority A enumerate or mutate proposals belonging to authority B (`backend-api-conventions.md`, AGENTS.md)?
- **Coordination-only boundary**: Does the Axum stack re-derive ASM/SPS canonical validity (forbidden), or restrict itself to hygiene, lifecycle, and I/O (`AGENTS.md`, rust-code-audit skill)?
- **Signer safety**: High-signal, unambiguous errors; no covert channels that leak proposal existence to the wrong principals.
- **Availability & misuse of dev defaults**: In-memory repos, deterministic operator keys, permissive CORS — do they silently ship into a hostile-network deployment?
- **Lifecycle races**: Concurrent approve vs broadcast paths; atomicity of claim vs long async broadcast (`application/proposals.rs`).
- **RPP smells (L1–L6 skim)**: Stale docs vs behavior, misplaced concerns, duplication, leaky abstractions (`application/proposals.rs` header vs real auth in handlers).

## Top findings (ranked) — Blocking/High | Medium | Low

### Blocking / High

1. **`list_proposals` / `get_proposal` ignore session authority — global proposal visibility.** Handlers bind `AuthenticatedSession` but discard it (`_auth`) while listing or fetching (`orchestrator-be/src/handlers/proposals.rs`). Application `list_proposals` calls `ProposalRepository::list_by_status` with only an optional status — no authority filter (`orchestrator-be/src/application/proposals.rs`). Postgres repo lists all rows with `WHERE status = $1` or no authority predicate (`orchestrator-be/src/infrastructure/postgres_repo.rs`, `list_by_status`). **Violates** “bind each session to exactly one multisig authority” and isolation expectations in `.claude/rules/backend-api-conventions.md`.

2. **Prepare / execute broadcast do not bind caller authority to proposal authority.** `_auth` is unused on `prepare_broadcast` and `execute_broadcast` (`orchestrator-be/src/handlers/proposals.rs`). Any bearer with any valid multisig session could drive broadcast for another authority’s proposal if they learn `action_id`. Defense-in-depth failure even if UX never shows foreign IDs.

3. **Deterministic fallback operator secret in config.** `OPERATOR_SECRET_KEY_HEX` defaults to `…0001` hex (`orchestrator-be/src/config.rs`). A missing env in non-dev deployment yields a predictable operator Taproot path / reveal semantics — catastrophic if ever exposed outside pure local dev.

4. **`get_proposal` returns `NotFound` (HTTP 404) before authority check.** Domain fetch is unscoped; wrong-authority callers may distinguish missing vs forbidden depending on layering (approve uses authority check; read path does not). **`backend-api-conventions.md`** requires non-signers not infer existence via status differential; cross-authority authenticated users are similarly sensitive.

### Medium

5. **`DATABASE_URL` absent → full in-memory proposal store.** Logged warning only (`orchestrator-be/src/main.rs`). Restart erases coordination state — undermines reliance on orchestrator history for manual fallback narratives.

6. **CORS `allow_origin(Any)` + permissive headers** (`orchestrator-be/src/main.rs`). For browser-hosted clients this expands CSRF/session-use surface if cookies were ever introduced; paired with bearer tokens in headers it is mainly a posture smell unless a malicious origin is scripted against a victim’s browser.

7. **`auth_challenge` storage without TTL sweep.** Challenges live in `Arc<RwLock<HashMap<…>>>` (`orchestrator-be/src/handlers/auth.rs`, `state.rs`): verify path rejects expired IDs, but map entries without verify can accumulate (DoS / memory).

8. **Post-broadcast concurrency window.** `claim_broadcast` is atomic, but `approve_action`/`add_signature` can still mutate proposals while a long `do_broadcast` runs (`application/proposals.rs`) — observable inconsistency between stored signatures and broadcasted artifact if model allows (threshold edge cases).

9. **RPP / hygiene:** Header in `application/proposals.rs` still claims “No authentication…” while handlers enforce sessions — **stale ubiquitous language** risks future edits in the wrong layer.

### Low

10. **`AppError::NotFound` message is `"not found"` vs others** (`error.rs`) — minor consistency for logs; acceptable.

11. **Bitcoin network inferred from RPC URL substring** (`main.rs`) — brittleness vs explicit config.

## Attack narratives (3–6)

1. **Cross-tenant list.** Attacker obtains any valid bearer (Strata signer). Calls `GET /api/v1/proposals`. Receives every authority’s pending/approved proposals from the coordination DB.

2. **Broadcast hijack.** Same bearer learns an Alpen-admin `action_id` (sidebar leak, pasted URL, insider). Calls `prepare`/`broadcast` endpoints; handlers never compare `auth.authority` to `proposal.authority`.

3. **Prod foot-gun operator key.** Deploy omitting `OPERATOR_SECRET_KEY_HEX`; chain observers who assume test key parity game commit/reveal ordering (risk model depends on whether that key/path is economically reachable — treat as HIGH until proven unreachable).

4. **Existence oracle.** Caller probes `GET /proposals/:id` across guessed IDs; 404 payload/timing leaks differ from forbidden responses once authority filtering is partially fixed unless errors are unified.

5. **Challenge flooding.** Anonymous `POST /auth/challenge` allocates entries until memory pressure (`auth.rs`), amplifying infra fragility without rate limits.

## Evidence index (paths)

| Area | Paths |
|------|-------|
| Unscoped read handlers | `orchestrator-be/src/handlers/proposals.rs` |
| List application | `orchestrator-be/src/application/proposals.rs` |
| Postgres list | `orchestrator-be/src/infrastructure/postgres_repo.rs` (`list_by_status`) |
| In-memory list | `orchestrator-be/src/infrastructure/memory_repo.rs` (`list_by_status`) |
| Operator key default | `orchestrator-be/src/config.rs` |
| Startup / CORS | `orchestrator-be/src/main.rs` |
| Auth challenges / sessions | `orchestrator-be/src/handlers/auth.rs`, `orchestrator-be/src/handlers/auth_session.rs`, `orchestrator-be/src/state.rs` |
| Error mapping | `orchestrator-be/src/error.rs` |
| Broadcast pipeline | `orchestrator-be/src/application/proposals.rs` |

## Smallest fixes vs largest bets (be explicit)

**Smallest**

- Thread `authority` from `AuthenticatedSession` into `list_proposals` / `get_update_action`; add `WHERE authority = $n` (Postgres + trait + memory impl).
- In `prepare_broadcast` / `execute_broadcast`, load proposal and `ensure_eq!(proposal.authority, auth.authority)` → `Unauthorized`.
- Fail fast on unset `OPERATOR_SECRET_KEY_HEX` in non-test builds, or refuse known test constant.
- Periodic or on-read GC for stale challenge entries.

**Largest bets**

- Durable Postgres **required** for non-dev; migrate auth sessions/challenges too (eliminate RwLock maps).
- Request-level audit log keyed by `{authority, action_id}` with anomaly detection on cross-authority attempts.
- Optimistic versioning on proposals for whole broadcast-critical sections.

## What would change my mind (missing evidence / experiments)

- Prove **staging/prod Helm** always sets `DATABASE_URL`, `OPERATOR_SECRET_KEY_HEX`, and stricter CORS — code alone does not.
- Integration test matrix: dual authority fixtures; assert list/get/broadcast forbidden across boundary.
- Load test challenge endpoint for memory bound; confirm OOM curve.
- Formal product decision: is “authenticated but wrong authority” on `GET` required to be indistinguishable from “missing” everywhere (404 vs 401) per PRD wording — need exact AC quote and expected status codes.
