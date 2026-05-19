# Wave 2 Track D — follow-up backlog

PR [#139](https://github.com/wakeuplabs-io/alpen-multisig/pull/139) merged **P-027 (orchestrator only)**: 30s `tokio::time::timeout` on ASM and Bitcoin JSON-RPC via `orchestrator-be/src/infrastructure/rpc_timeout.rs`.

Remaining Track D items from [action-plan-2026-05-14.md](action-plan-2026-05-14.md) — schedule as separate PR(s) on `develop` (no human gate unless noted).

| P-ID | Summary | Suggested scope |
|------|---------|-----------------|
| **P-027** (remainder) | Retries with jitter; circuit breaker per dependency | `orchestrator-be` + `desktop-app/src-tauri` external RPC |
| **P-027** (desktop) | Same timeout wrapper on Tauri ASM/Bitcoin clients | Mirror `rpc_timeout` or shared crate |
| **P-017** | Session/challenge TTL; rate limit (`tower-governor`) | `orchestrator-be` auth store |
| **P-018** | Admin `/reset-broadcast`; timeout already partial | Handler + docs; resumable FSM → Wave 3 |
| **P-019** | Dedup under lock; `version: u64` optimistic locking | `orchestrator-be` proposals repo |
| **P-023** | `errorCode` discriminant on API + IPC | BE `ApiResult`, desktop Zod/Tauri errors |
| **P-029** | `#[tracing::instrument]` on all handlers; request UUID in bridge; `/ready` Postgres+ASM+BTC | BE + `desktop-app` |

**Wave 2 exit line** (“every RPC call wrapped in timeout with structured error metadata”) needs at least **desktop P-027** + **P-029** skeleton before calling Track D complete.

**Priority suggestion:** P-029 skeleton (ops visibility) → P-019 (race under Postgres) → P-017 → P-023 → P-027 retries → P-018 reset endpoint.

**Wave 2 tracks merged before phase 2:** A, B, C, D (P-027 slice), G (P-053 plans). Open engineering tracks: **E**, **F** — see [action-plan-progress.md](action-plan-progress.md).
