# Troubleshooting & failure modes — adversarial axis (read-only review)

**Audit date:** 2026-05-14  
**Lens:** Incident response, logs, diagnosability, operator/signer UX under failure

---

## Scope

**In scope:** How failures surface through HTTP responses, structured logging, layers (`TraceLayer`, `tracing_subscriber`), desktop bridge error normalization, Bitcoin RPC failures, Postgres errors. **Out of scope:** Chain-level finality/debug (Bitcoin/Strata node ops runbooks unless reflected in repo behavior).

---

## Top findings (ranked)

1. **Internal faults collapse to opaque `"internal error"` JSON for clients** while detail only hits server logs (`orchestrator-be/src/error.rs`, `IntoResponse` for `AppError::Internal`). Operators must **correlate** user reports with timestamps and hope log level captures context.
2. **No first-class request/correlation id in error JSON** visible in `error.rs`; reliance on Axum `TraceLayer` default span output for HTTP tracing (`orchestrator-be/src/main.rs`). Distributed tracing across desktop + orchestrator + bitcoind is **not evidently standardized** in-repo.
3. **`DATABASE_URL` missing → silent durability degradation** logged at `warn` only (`orchestrator-be/src/main.rs`). On-call confusion: healthy `/health`, **empty proposals after restart**.
4. **Bitcoin RPC wiring exists in startup** (`HttpBitcoinRpcClient::new` in `main.rs`); RPC misconfig failures likely surface as broadcast/prepare errors — diagnosing **whether fault is credential, wallet name, or chain** depends on wrapping error discipline in command handlers (not fully audited here).
5. **Desktop `invoke` normalization** (`desktop-app/src/api/tauri-bridge.ts`) abstracts errors — advantageous for UX, **risky if it swallows discriminators** operators need when triaging orchestrator vs local signing faults.

---

## Attack narratives

1. **Masked root cause:** A signer screenshots `"internal error"`; support has no request id → log grep by wall clock across pods fails — incident extends.
2. **Warn fatigue:** Routine missing-`DATABASE_URL` warning scrolled past in centralized logs looks like benign noise until proposals vanish post-deploy.
3. **Broadcast stuck:** `broadcast_error` persists in DB (`orchestrator-be/migrations/20260507120000_add_broadcast_fields.sql`) — if UI hides raw error, signer believes chain stuck when issue is RPC auth.
4. **Auth/session expiry confusion:** Middleware returns `401`/`403` equivalents — without stable machine-readable codes, frontend may label all as “wallet disconnected.”
5. **Concurrent operator double-click:** Postgres `Conflict` responses on broadcast claim (`postgres_repo.rs` `claim_broadcast`) — if surfaced generically as conflict, signer reads “proposal broken” vs “already broadcasting.”

---

## Evidence index

| Topic | Path |
|--------|------|
| Error mapping & internal logging | `orchestrator-be/src/error.rs` |
| Subscriber + HTTP trace layers | `orchestrator-be/src/main.rs` |
| Postgres contention messaging | `orchestrator-be/src/infrastructure/postgres_repo.rs` (`claim_broadcast`, unique violations → `Conflict`) |
| Frontend invoke wrapper | `desktop-app/src/api/tauri-bridge.ts` |
| Broadcast diagnostics column | `orchestrator-be/migrations/20260507120000_add_broadcast_fields.sql` |
| Operational expectations narrative | `docs/architecture/overview.md` |

---

## Smallest vs largest bets

| Size | Bet |
|------|-----|
| **Smallest** | Add `request_id` to JSON error bodies (propagate from middleware) and document “how to grep logs” with one example in ops notes. |
| **Largest** | OpenTelemetry end-to-end (desktop spans optional), error taxonomy enums stable across HTTP + IPC, and user-facing surfaced **safe** diagnostics separate from privileged logs. |

---

## What would change my mind

- Evidence of **structured JSON logging** (`tracing-subscriber` `json`) enabled in staging with sampled traces tying `request_id` to `action_id` **without leaking secrets**.
- Screenshots showing **distinct UI copy** for: RPC fault, quorum incomplete, unauthorized, conflict/broadcast races.
- Runbooks committed next to **known 500 hotspots** listing which `AppError` variants map operational actions.
