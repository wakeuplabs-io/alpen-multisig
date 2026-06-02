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

Broadcast configuration lives in **`desktop-app/.env`** only (see `desktop-app/.env.example`; loaded at Tauri startup via a fixed path, not CWD). As of R1.1, **no env flag gates broadcast** — signing capability is decided per-signer by network (`allowed_on`: mnemonic on regtest/testnet only, hardware on any network). The mnemonic-signing IPC remains dev/E2E-only, exposed in release builds via `ALLOW_DEV_MNEMONIC_SIGNING` and automatically in debug builds (P-040 — see `docs/specs/secret-custody-wave2.md`).

**Phase 3.7 (3.7c):** when logged in (Palabras), the wallet panel, commit **funding**, and the SPS-50 commit/reveal **internal key** (`m/86'/0'/73'/2/0`) all follow the login session. `ADMIN_WALLET_REGTEST_MNEMONIC` is **removed** — do not set it in `.env`. Broadcast and wallet IPC require a mnemonic login (`wallet_session_init`).

## Incidents

1. **Proposals stuck in broadcast:** Check desktop logs; verify `claim` + `PATCH` coordination. Admin may reset broadcast state (Track D `P-018` when merged).
2. **Auth failures:** Confirm signer is in ASM membership for selected authority; check challenge TTL.
3. **Cross-authority access:** Expect `401` — session is authority-scoped (P-002).

## Logs

Structured tracing on proposal list/get (extend to all handlers in P-029). Correlate with request UUID from desktop bridge when enabled.
