# Signer safety model (P-051)

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

When orchestrator is unavailable after signatures: export payloads, broadcast via any Bitcoin RPC, reconcile metadata when coordinator returns. Scope: [wave2-human-decisions-pending.md](../assessment/wave2-human-decisions-pending.md).
