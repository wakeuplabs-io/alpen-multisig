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

Broadcast operator key and Bitcoin wallet live in **`desktop-app/.env`** only (see `desktop-app/.env.example`; loaded at Tauri startup via a fixed path, not CWD). Decision #2: operator key is process env only; use `ALLOW_DEV_OPERATOR_KEY=1` with the well-known regtest test key. Mnemonic signing IPC is dev/E2E-only (`ALLOW_DEV_MNEMONIC_SIGNING` or debug builds — see `docs/specs/secret-custody-wave2.md` on Track A).

## Incidents

1. **Proposals stuck in broadcast:** Check desktop logs; verify `claim` + `PATCH` coordination. Admin may reset broadcast state (Track D `P-018` when merged).
2. **Auth failures:** Confirm signer is in ASM membership for selected authority; check challenge TTL.
3. **Cross-authority access:** Expect `401` — session is authority-scoped (P-002).

## Logs

Structured tracing on proposal list/get (extend to all handlers in P-029). Correlate with request UUID from desktop bridge when enabled.
