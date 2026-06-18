# Signer safety model (P-051)

**SSOT (security — signer UX):** Pair with [`security/threat-model.md`](../security/threat-model.md) for assets, trust boundaries, and top risks. Threat model = what can go wrong; this doc = what the signer must verify. Read both; do not duplicate.

## Principles

1. **Explicit authority context** — Every sign/broadcast screen shows which multisig authority is active.
2. **Verify before sign** — User must confirm sighash hex matches intent (Trezor shows 32-byte hash only; UI names authority — P-006).
3. **No secrets in webview (production)** — Operator key loads from Tauri env at startup only; mnemonic/raw-key IPC is dev/E2E-only (Wave 2 Decision #2; implementation: Track A `docs/specs/secret-custody-wave2.md`).
4. **Persisted broadcast truth** — UI reads coordinator `proposal_status` / `broadcast_status` after broadcast (P-062).

## Signer checklist

- [ ] Selected role matches proposal `authority`
- [ ] Preview sighash frozen (P-007) before Trezor sign
- [ ] After broadcast, UI shows persisted txids from coordinator

## Manual fallback (US-H5)

**Partial.** The `/manual` route and `proposals_broadcast_manual` IPC let a signer broadcast commit/reveal from locally aggregated signatures when the orchestrator is unreachable. Portable export of `actionHex` + signature list and coordinator reconciliation when the backend returns remain open — see [manual-execution-flow.md](./manual-execution-flow.md) and [deferred-backlog.md](../assessment/deferred-backlog.md#us-h5--manual-coordinator-down-fallback).
