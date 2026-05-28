# Wave 2 Track E — follow-up backlog

PR [#140](https://github.com/wakeuplabs-io/alpen-multisig/pull/140) merged (Wave 2 PR queue complete with [#141](https://github.com/wakeuplabs-io/alpen-multisig/pull/141)):

| P-ID | Delivered |
|------|-----------|
| **P-008** | Zod on `auth_start_challenge`, `auth_complete`, `auth_get_session` (`authRoleSchema` = `z.nativeEnum(AuthRole)`) |
| **P-008** | Proposal/broadcast IPC already validated (Wave 1 + ipc-schemas tests) |
| **Docs** | E2E README — Decision #2 dev flags (`ALLOW_DEV_MNEMONIC_SIGNING`; `ALLOW_DEV_OPERATOR_KEY` retired in Phase 3.5) |

**Explicitly out of #140:** WDIO negative / US-H5 matrix in `e2e-webdriver` (manual curated specs only).

## P-008 remainder — **done** (Wave 3, W3-1 [#152](https://github.com/wakeuplabs-io/alpen-multisig/pull/152))

Zod schemas added at `tauriCall` boundary for all remaining IPC surfaces:

| Module | Status |
|--------|--------|
| `signing.ts` — `decode_action_hex`, `compute_sighash`, `verify_threshold`, `sign_sighash_mock` | done |
| `orchestrator-auth.ts` — challenge + session schemas | done |
| `asm-state.ts` — `get_multisig_config`, authority memberships | done |
| `action-builder.ts` — `build_admin_multisig_update_hex` | done |

## P-032 remainder — **done** (Wave 3, W3-2/W3-3 [#153](https://github.com/wakeuplabs-io/alpen-multisig/pull/153) / [#154](https://github.com/wakeuplabs-io/alpen-multisig/pull/154))

| Item | Delivered |
|------|-----------|
| Concurrent approval race (dedup under write lock) | P-019 + integration test W3-2 |
| Claim when pending, broadcast conflict guards | Handler tests W3-3 |
| `e2e_propose_sign` extended to quorum → approved | W3-3 e2e-tests |

## US-H5 manual fallback

**Deferred to Wave 3 / Slice 5** — [wave2-human-decisions-pending.md](wave2-human-decisions-pending.md) §3 (resolved 2026-05-19). Not required for Wave 2 sign-off or develop → main.

When implemented: outside default `e2e-webdriver` unless manually promoted; align with P-052 and PRD §2.3.
