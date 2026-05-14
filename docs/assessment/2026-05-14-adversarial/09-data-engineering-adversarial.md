# Data engineering — adversarial axis (read-only review)

**Audit date:** 2026-05-14  
**Lens:** Persistence, schema, PII, governance (data engineer)

---

## Scope

**In scope:** Orchestrator coordination data (proposals, signatures, broadcast metadata), SQL migrations, Postgres vs in-memory implementations, fields that identify signers or governance actions, retention/ops implications implied by code. **Out of scope:** On-chain ASM truth (source of truth remains protocol), pure UI state, Alpen crate internals.

**Threat stance:** Treat the backend as a governance-adjacent datastore: integrity, durability, and least-privilege logging matter as much as correctness.

---

## Top findings (ranked)

1. **Production foot-gun: Postgres is optional; default path is ephemeral in-memory storage.** If `DATABASE_URL` is unset, the server logs a warning and uses `InMemoryProposalRepository`, so restarts erase all coordination state (`orchestrator-be/src/main.rs`, lines 66–104).
2. **Schema governance is thin: no repo-level data dictionary; authority/status/broadcast enums are constrained in Rust/SQL inconsistently.** DB columns are `TEXT` for `authority`, `status`, `broadcast_status` with application-side parsing (`orchestrator-be/migrations/20260501000000_create_proposals_tables.sql`; `20260507120000_add_broadcast_fields.sql`; `orchestrator-be/src/infrastructure/postgres_repo.rs` `authority_from_db` / `status_from_db`).
3. **Unbounded/coarse textual payloads:** `action_hex`, pubkey, and signature blobs are stored as `TEXT` without documented max lengths or hashing strategy for oversized governance actions (`orchestrator-be/migrations/20260501000000_create_proposals_tables.sql`).
4. **Operational data (`broadcast_error`) can carry sensitive diagnostics** if propagated from Bitcoin RPC or node errors into the proposals row (`orchestrator-be/migrations/20260507120000_add_broadcast_fields.sql`; `postgres_repo.rs` `update_broadcast_status`).
5. **PII analogue — signer linkage:** Compressed pubkeys plus signatures fingerprint signers across time; absent a documented retention/erasure policy, the DB effectively stores a longitudinal governance activity graph (`proposal_signatures` table in `20260501000000_create_proposals_tables.sql`).

---

## Attack narratives

1. **Restart amnesia:** Ops deploys without `DATABASE_URL`; pod restarts after partial quorum; signers recreate proposals with same logical intent but divergent client state → duplicate coordination threads and confused broadcast attempts.
2. **Enum drift:** A manual SQL patch writes `Authority = 'StrataAdmin'` (wrong casing) → rows become unreadable or map to `Internal` errors, blocking reads for affected proposals (`postgres_repo.rs` `authority_from_db`).
3. **Log + DB correlation:** Support enables verbose HTTP tracing; `TraceLayer` plus error paths log enough to tie `action_id` to signer pubkeys in log aggregators — governance metadata exposed without access to DB (`orchestrator-be/src/main.rs` `TraceLayer::new_for_http()`; `error.rs` internal logging).
4. **Broadcast error injection:** A malicious or compromised RPC endpoint returns an error string crafted to maximize row size or exploit downstream JSON consumers if errors are ever mirrored to clients without sanitization (`broadcast_error` column).
5. **Migration ordering / partial apply:** A failed migration mid-deploy leaves `required_signatures` default inconsistency relative to app expectations (`20260505101000_add_required_signatures_to_proposals.sql` adds column then drops default — verify all envs applied both steps).

---

## Evidence index

| Topic | Path |
|--------|------|
| DB bootstrap & in-memory fallback | `orchestrator-be/src/main.rs` |
| Initial schema | `orchestrator-be/migrations/20260501000000_create_proposals_tables.sql` |
| Required signatures migration | `orchestrator-be/migrations/20260505101000_add_required_signatures_to_proposals.sql` |
| Broadcast columns | `orchestrator-be/migrations/20260507120000_add_broadcast_fields.sql` |
| Postgres repository & enum mapping | `orchestrator-be/src/infrastructure/postgres_repo.rs` |
| Layered persistence intent | `docs/architecture/adrs/005-layered-architecture.md` |
| High-level data picture | `docs/architecture/overview.md` |

---

## Smallest vs largest bets

| Size | Bet |
|------|-----|
| **Smallest** | Require `DATABASE_URL` when `ENVIRONMENT=production` (fail fast) and document max recommended `action_hex` length in a one-page data note. |
| **Largest** | Full data-governance pack: ER diagram, retention policy, column-level classification (signer identifiers vs payloads), migration rollback playbooks, and automated schema drift checks between migrations and `SELECT_PROPOSAL_COLS`. |

---

## What would change my mind

- Evidence that **all** production-like deploys set `DATABASE_URL` and run migrations in CI/CD with blocking gates.
- A **checked-in data contract** (OpenAPI or SQL comments + generated types) proving Rust and migrations stay aligned.
- **Redacted logging policy** proving signer pubkeys / digests never land in plaintext logs at `info` or below in production configs.
