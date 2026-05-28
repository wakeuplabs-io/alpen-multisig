# Secret custody — Wave 2 Slice-0 (Decision #2)

**Status:** Implemented (Track A)  
**Gate log:** [wave2-human-decisions-pending.md](../assessment/wave2-human-decisions-pending.md) §2  
**Owners:** Alpen security + Wakeup platform

## Policy

In production, the React webview must never pass a full mnemonic or operator hex to Tauri. The operator key loads from process env at startup (`broadcast_env`). Mnemonic-over-IPC is allowed only for dev/E2E behind an explicit flag and/or debug builds.

## Commit/reveal internal key (broadcast)

| Item | Behavior |
|------|----------|
| Source | Derived from `ADMIN_WALLET_REGTEST_MNEMONIC` at `m/86'/0'/73'/2/0` in Tauri process env at startup (Phase 3.5+). No separate operator key env var. |
| Webview | Must not supply signing material over IPC |
| Orchestrator | Does not derive or hold the commit/reveal key |
| Guard | `ALLOW_DEV_MNEMONIC_SIGNING=1` required in regtest; replaces `ALLOW_DEV_OPERATOR_KEY` (retired in Phase 3.5) |

Implementation: `desktop-app/src-tauri/src/infrastructure/broadcast_env.rs`.

## Multisig signer material (mnemonic / software signing)

| Surface | Production / release | Dev / E2E |
|---------|----------------------|-----------|
| `sign_with_mnemonic_path` | Not registered; command guard fails closed | Debug build and/or `ALLOW_DEV_MNEMONIC_SIGNING=1` |
| `list_mnemonic_addresses` | Same | Same |
| `sign_action_sighash` (raw key hex) | Same | Same |
| Trezor commands | Always available | Always available |

Implementation:

- `infrastructure/dev_secrets.rs` — env + debug profile gate
- `commands/invoke.rs` — production handler set omits dev signing commands (P-040)
- `commands/signing.rs` — `ensure_dev_mnemonic_signing_allowed()` on dev commands

## Deferred (Wave 3)

OS keychain, HSM, and secrets manager for operator or signer storage — ops/runbook; not required to close Wave 2 on this decision.

## Verification

```bash
cargo test -p desktop-app dev_secrets
cargo test -p desktop-app broadcast_env
```

Release profile without `ALLOW_DEV_MNEMONIC_SIGNING`: dev signing commands are not registered.
