# Wave 2 Track D — follow-up backlog

PR [#139](https://github.com/wakeuplabs-io/alpen-multisig/pull/139) merged **P-027 (orchestrator only)**: 30s `tokio::time::timeout` on ASM and Bitcoin JSON-RPC via `orchestrator-be/src/infrastructure/rpc_timeout.rs`.

Remaining Track D items from [action-plan-2026-05-14.md](action-plan-2026-05-14.md) — schedule as separate PR(s) on `develop` (no human gate unless noted).

| P-ID | Summary | Status | PR |
|------|---------|--------|----|
| **P-027** (desktop) | 30s timeout on Tauri ASM/Bitcoin clients via `rpc_timeout::rpc_client()` | **done** (Wave 3) | [#155](https://github.com/wakeuplabs-io/alpen-multisig/pull/155) |
| **P-019** | Dedup check in `add_signature` under write lock | **done** (Wave 3) | [#153](https://github.com/wakeuplabs-io/alpen-multisig/pull/153) |
| **P-023** | `errorCode` discriminant on API + IPC (`ApiResult`, Tauri bridge) | **done** (Wave 3) | [#155](https://github.com/wakeuplabs-io/alpen-multisig/pull/155) |
| **P-029** | `X-Request-Id` in bridge; `#[tracing::instrument]` on approve/patch/claim/broadcast | **done** (Wave 3) | [#156](https://github.com/wakeuplabs-io/alpen-multisig/pull/156) |
| **P-027** (remainder) | Retries with jitter; circuit breaker per dependency | **deferred** → Wave 4 | — |
| **P-017** | Session/challenge TTL; rate limit (`tower-governor`) | **deferred** → Wave 4 | — |
| **P-018** | Admin `/reset-broadcast`; resumable broadcast FSM | **deferred** → Wave 4 | — |

**Wave 2 exit line** met: desktop P-027 + P-029 skeleton done in Wave 3.

**Wave 2 PR queue:** All tracks A–G merged. Phase 2 ops work listed here — see [wave2-exit-gap-review.md](wave2-exit-gap-review.md).
