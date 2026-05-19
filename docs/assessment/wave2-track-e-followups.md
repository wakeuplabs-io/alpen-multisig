# Wave 2 Track E — follow-up backlog

PR [#140](https://github.com/wakeuplabs-io/alpen-multisig/pull/140) merged onto `develop`:

| P-ID | Delivered |
|------|-----------|
| **P-008** | Zod on `auth_start_challenge`, `auth_complete`, `auth_get_session` (`authRoleSchema` = `z.nativeEnum(AuthRole)`) |
| **P-008** | Proposal/broadcast IPC already validated (Wave 1 + ipc-schemas tests) |
| **Docs** | E2E README — Decision #2 dev flags (`ALLOW_DEV_OPERATOR_KEY`, `ALLOW_DEV_MNEMONIC_SIGNING`) |

**Explicitly out of #140:** WDIO negative / US-H5 matrix in `e2e-webdriver` (manual curated specs only).

## P-008 remainder

Add Zod (or equivalent validation) at `tauriCall` for IPC surfaces still unchecked:

| Module | Commands (examples) |
|--------|---------------------|
| `signing.ts` | `decode_action_hex`, `compute_sighash`, `verify_threshold` |
| `orchestrator-auth.ts` | `orchestrator_auth_*` |
| `asm-state.ts` | `get_multisig_config` |
| `action-builder.ts` | `build_admin_multisig_update_hex` |

## P-032 remainder (axis-10 inventory)

Per [action-plan-2026-05-14.md](action-plan-2026-05-14.md) Track E — **not** via new `e2e-webdriver` specs unless manually promoted:

| Item | Suggested home |
|------|----------------|
| Broadcast negative paths | Integration tests (`orchestrator-be`) or dedicated test crate |
| Concurrent approval races | Load/integration tests (pairs with P-019) |
| Extra frontend smoke | Curated WDIO only when stable |

## US-H5 manual fallback

**Blocked:** [wave2-human-decisions-pending.md](wave2-human-decisions-pending.md) §3.

After decision: implement outside default `e2e-webdriver` package or add a **single** manually maintained spec; tabletop script in [wave2-p053-interview-plan.md](../2-discovery/wave2-p053-interview-plan.md) §4.
