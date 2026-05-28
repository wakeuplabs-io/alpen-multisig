# Backend operations runbook (P-051)

## Health

| Endpoint | Meaning |
|----------|---------|
| `GET /health` | Process up |
| `GET /ready` | Bitcoin RPC reachable (orchestrator does not broadcast) |

## Environment

| Variable | Required | Notes |
|----------|----------|-------|
| `ORCHESTRATOR_PROFILE=production` | Prod | Requires `DATABASE_URL` |
| `DATABASE_URL` | Prod | Postgres for durable proposals |
| `STRATA_ADMIN_STATE_RPC_URL` | Yes | ASM runner JSON-RPC |
| `BITCOIN_RPC_*` | Yes | Used by `/ready` only |

Broadcast configuration and the commit/reveal internal key live in **`desktop-app/.env`** only (see `desktop-app/.env.example`; loaded at Tauri startup via a fixed path, not CWD). The commit/reveal internal key is derived from `ADMIN_WALLET_REGTEST_MNEMONIC` at `m/86'/0'/73'/2/0` — no separate operator key env var is used (Phase 3.5+). Guard `ALLOW_DEV_MNEMONIC_SIGNING=1` is required in regtest. Mnemonic signing IPC is dev/E2E-only (`ALLOW_DEV_MNEMONIC_SIGNING` or debug builds — see `docs/specs/secret-custody-wave2.md`).

## Incidents

1. **Proposals stuck in broadcast:** Check desktop logs; verify `claim` + `PATCH` coordination. Admin may reset broadcast state (Track D `P-018` when merged).
2. **Auth failures:** Confirm signer is in ASM membership for selected authority; check challenge TTL.
3. **Cross-authority access:** Expect `401` — session is authority-scoped (P-002).

## Logs

Structured tracing on proposal list/get (extend to all handlers in P-029). Correlate with request UUID from desktop bridge when enabled.
