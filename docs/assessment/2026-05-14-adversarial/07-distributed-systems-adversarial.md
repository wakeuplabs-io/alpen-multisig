# Distributed systems — Adversarial Assessment (re-audit)

**Date:** 2026-05-14  
**Lens:** System designer (durability, concurrency, idempotency, partial failure, recovery)  
**Method:** Read-only review of coordination service behavior and persistence adapters.

---

## Scope & threat model

**What we are trying to break:**

- **Durability illusions:** Operators believe proposals survive process restarts; in-memory mode or partial Postgres outages violate that belief without loud failure modes.
- **Concurrent writers:** Two signers or two broadcasters interleave operations; repository must preserve invariants (threshold, duplicate signatures, single broadcast flight).
- **Idempotency:** Retried HTTP calls after timeouts must not double-spend governance actions or strand UTXOs.
- **Partial failure recovery:** Bitcoin commit succeeds but reveal fails (or reverse); state machine must be resumable and observable.

**In scope:** `orchestrator-be/src/infrastructure/{memory_repo,postgres_repo}.rs`, `application/proposals.rs` (broadcast path), `main.rs` (repo selection), `handlers/proposals.rs` (entrypoints).

---

## Top findings (ranked by severity)

### BLOCKER: D1 — Silent fallback to in-memory persistence when `DATABASE_URL` is unset

**Risk:** All proposal state is **volatile**; restarts wipe coordination history; backups are meaningless; “we run Postgres” runbooks may be wrong in one environment.

**Evidence:** `orchestrator-be/src/main.rs` — if `database_url` is `None`, log `warn!` and `InMemoryProposalRepository::new()`.

**Failure scenario:** Staging omits env var; team demos successfully; production-like load test loses every proposal on deploy — discovered only after incident.

**Smallest fix:** Fail closed in non-dev profiles unless `ALLOW_INMEMORY_REPO=1`; or require explicit `STORAGE_BACKEND=memory|postgres`.

**Largest bet:** Enforced migration path: boot blocks if schema version ≠ expected; automated backup verification job.

---

### CRITICAL: D2 — Broadcast concurrency: `claim_broadcast` is the correct primitive, but HTTP surface still allows unauthorized callers to trigger expensive or state-changing flows if bearer is compromised

**Risk:** From a **systems** view, the scarcest resource is **Bitcoin UTXO space + feerate**; any principal that can call broadcast endpoints can trigger chain actions without authority binding at handler level (see also architecture axis). Idempotency of *who may initiate* is undefined.

**Evidence:** `orchestrator-be/src/application/proposals.rs` — `broadcast_commit_then_reveal` documents atomic claim; `repo.claim_broadcast(action_id)` before `do_broadcast`. Handlers: `prepare_broadcast` / `execute_broadcast` use `_auth` only (`handlers/proposals.rs`).

**Failure scenario:** Leaked token from low-privilege signer machine triggers broadcast on high-value proposal IDs enumerated from global list endpoint — amplifies blast radius beyond typical RBAC expectations.

**Smallest fix:** Bind broadcast initiation to authority + optional operator role; align list/get filters.

**Largest bet:** Outbox pattern + worker queue for broadcasts; single writer lease via DB row lock.

---

### HIGH: D3 — Partial failure after `claim_broadcast`: error path marks `Failed` but automated retry semantics are undefined for operators

**Risk:** Client retries `POST /broadcast` after timeout; `claim_broadcast` may already be non-`Idle` — caller sees `Conflict`; ops playbooks unclear whether to resume manually via RPC or patch DB.

**Evidence:** `broadcast_commit_then_reveal` — on `Err`, `update_broadcast_status(..., Failed, ..., Some(&e.to_string()))`. `memory_repo` / `postgres_repo` — `claim_broadcast` returns `Conflict` if not `Idle` (`memory_repo.rs` ~90–93).

**Failure scenario:** Network partition during `send_to_address`; client retries; human operator assumes “failed == safe to retry immediately” without inspecting chain — potential double commit attempts depending on RPC idempotency (not shown in-repo).

**Smallest fix:** Document **exactly-one** operator procedure: inspect `broadcast_status`, chain txids, then `PATCH` or support `resume_broadcast` command; add integration tests for timeout + retry.

**Largest bet:** Sagas with durable step log + automatic safe resume.

---

### HIGH: D4 — `InMemoryProposalRepository` uses `RwLock` over `HashMap`; `add_signature` is not an atomic quorum transition

**Risk:** Under concurrent `approve` requests, interleavings exist where signature list grows but status transition to `Approved` races (mitigated partly by serialized lock per map, but **Postgres vs memory** behavioral parity must hold). Poisoned lock maps to `AppError::Internal` — callers may not distinguish transient vs permanent.

**Evidence:** `memory_repo.rs` — `add_signature` pushes to vec under write lock; `application/proposals.rs` then may call `update_broadcast_status` separately for approval promotion.

**Failure scenario:** Fuzz tests or parallel clients hit different error surfaces between adapters — flaky e2e only on Postgres.

**Smallest fix:** Property tests for concurrent approvals; single SQL transaction in Postgres adapter for “add sig + maybe approve”.

**Largest bet:** Serializable isolation level + formal linearizability argument.

---

### MEDIUM: D5 — No graceful shutdown; in-flight broadcast may be cut mid-flight

**Risk:** Orchestrator loses responses to clients; Bitcoin RPC may have submitted tx but DB not updated — **observability gap** couples with recovery ambiguity (D3).

**Evidence:** `main.rs` — `axum::serve(listener, app).await` without shutdown future.

**Smallest fix:** `with_graceful_shutdown` on Ctrl+C/SIGTERM; configurable drain timeout.

**Largest bet:** Durable workflow engine for long broadcasts.

---

### MEDIUM: D6 — Idempotent create relies on `ActionId` uniqueness; clients must not randomize `action_hex`

**Risk:** Correct for deterministic IDs, but **client bug** that mutates whitespace/casing in hex while “meaning same action” bypasses dedup — sociotechnical idempotency contract.

**Evidence:** `domain/proposal.rs` — `ActionId` from `seq_no` + `action_hex`; `save_proposal` conflicts on existing ID (`memory_repo` / `postgres_repo`).

**Smallest fix:** Canonicalize `action_hex` (lowercase, no `0x`) at handler boundary with explicit error message.

**Largest bet:** Content-addressed store with hash of decoded SSZ bytes, not raw hex string.

---

## Attack narratives

1. **Ops forgets `DATABASE_URL`:** Production namespace missing env → silent memory mode. **Outcome:** restart deletes pending proposals; signers see empty dashboard; incident response lacks metrics because `/health` green.

2. **Double-click broadcast:** User retries after gateway timeout; first request committed tx, second hits `Conflict` on `claim_broadcast`. **Outcome:** operator confusion, possible manual “fix” that retries unsafe RPC calls.

3. **Parallel approvers at threshold boundary:** Last two signatures arrive together. **Outcome:** behavior depends on repo adapter transaction boundaries — needs proof both adapters promote to `Approved` exactly once.

---

## Evidence index (paths)

| Area | Path |
|------|------|
| Repo selection / warning | `orchestrator-be/src/main.rs` |
| Broadcast pipeline + claim | `orchestrator-be/src/application/proposals.rs` |
| In-memory concurrency | `orchestrator-be/src/infrastructure/memory_repo.rs` |
| Postgres conflicts | `orchestrator-be/src/infrastructure/postgres_repo.rs` |
| Handler entry (retries) | `orchestrator-be/src/handlers/proposals.rs` |
| Action ID computation | `orchestrator-be/src/domain/proposal.rs` |

---

## Smallest fixes vs largest bets

| Finding | Smallest fix | Largest bet |
|---------|--------------|-------------|
| D1 | Fail closed without explicit memory flag | Automated durability CI suite |
| D2 | Authz on broadcast + scoped listing | Queued broadcast worker + lease |
| D3 | Runbook + `resume` API + tests | Saga orchestrator |
| D4 | Txn-scoped approve in Postgres | Linearizability proofs |
| D5 | Graceful shutdown | Workflow durability |
| D6 | Hex canonicalization | SSZ-hash id |

---

## What would change my mind

- **D1:** Deployment platform enforces `DATABASE_URL` presence via admission (webhook) for all non-dev clusters — cite manifest policy.
- **D3:** Bitcoin RPC layer proves idempotent `sendrawtransaction` behavior for this client usage — with logs demonstrating safe retries.

---

## Conclusion

**What works:** `claim_broadcast` encodes a **single-flight** intent for the broadcast state machine — the right primitive for UTXO-spending sequences. **Idempotent proposal creation** via `ActionId` conflict is sound if inputs are canonical.

**Systemic gaps:** **Silent in-memory fallback** is the strongest durability foot-gun; **shutdown and retry stories** for long Bitcoin operations are under-specified; **handler-level authorization** gaps amplify misuse of chain-affecting endpoints. Harden persistence defaults and document **exactly-once operator semantics** before labeling the service production-grade under chaos scenarios.
