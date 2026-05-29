# Threat model (P-051) — summary

## Assets

- Signer private keys (HW wallet, mnemonic dev path, operator key for broadcast)
- Session bearer tokens (authority-scoped)
- Proposal `action_hex` and collected signatures

## Trust boundaries

```mermaid
flowchart LR
  React[React webview]
  Tauri[Tauri Rust]
  Orch[Orchestrator]
  BTC[Bitcoin RPC]
  ASM[ASM RPC]

  React -->|IPC no secrets in prod| Tauri
  Tauri -->|HTTPS| Orch
  Tauri -->|RPC| BTC
  Orch -->|RPC| ASM
```

## Top risks (Wave 2 mitigations)

| Risk | Mitigation |
|------|------------|
| Malicious backend returns wrong proposal | P-005 hash verify (Track F) |
| Well-known test mnemonic in production | `ALLOW_DEV_MNEMONIC_SIGNING` guard (Phase 3.5+); commit/reveal key from login session at `m/86'/0'/73'/2/0` (Phase 3.7c; env mnemonic removed) |
| Mnemonic / raw key exfil via XSS | P-003 + P-040: dev signing IPC off in release (Decision #2; Track A) |
| Supply-chain compromise | P-011 audit/deny/lockfile |
| Cross-authority data leak | P-002 session + proposal scope |
| Coordinator/UI desync on broadcast | P-066 desktop execute + PATCH metadata |

## Out of scope (Wave 3+)

- Signed releases all platforms (P-011 full)
- Shared types codegen (P-043)
- Event-sourced audit log (P-031)
