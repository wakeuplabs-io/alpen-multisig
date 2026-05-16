# Wave 1 action plan — progress tracker

Branch: `fix/action-plan-wave1-2026-05-14` (PR [#134](https://github.com/wakeuplabs-io/alpen-multisig/pull/134))  
Base: `develop`  
Source: [action-plan-2026-05-14.md](action-plan-2026-05-14.md)  
Execution record: [action-plan-2026-05-14.md §5.1](action-plan-2026-05-14.md#51-wave-1--execution-record-2026-05-16)

## Summary

| Metric | Value |
|--------|--------|
| Planned P-IDs + label | 20 + 1 |
| Commits (bootstrap + issues) | 21 (`213ce09` … `5bdf8bf`) |
| Post–Wave 1 fix commits | 7 (`6b0b9d4` … `a74c817`) |
| Branch tip | `a74c817` |
| E2E gate | Single `/e2e-proposal-flow` PASS at tip (chain height 445; manual enactment **yes**) |
| Per-commit E2E (planned) | Not run — one tip validation only |

## P-ID status

| P-ID | Status | Commit | E2E | Notes |
|------|--------|--------|-----|-------|
| P-061 | done | 3981e0c | tip | Orchestrator `prepare` + `broadcast` HTTP; no local commit/reveal |
| P-062 | done | fc3fbb9 | tip | Re-fetch proposal; UI shows persisted status |
| P-002 | done | 8178b15 | tip | Authority filter; 401 on mismatch; integration test |
| P-001 | done | 31b9f5a | tip | No default operator key; reject `0x00…01` unless allow-test flag |
| P-015 | done | ceeeb7a | tip | Removed `VITE_OPERATOR_*` from broadcast FE path |
| P-063 | partial | 53bcd98 | tip | Lowercase at ingress; **no** Postgres `CHECK` (in-memory dev) |
| P-013 | done | 766d685 | tip | `BITCOIN_NETWORK` required (no port heuristic) |
| P-014 | done | e51daa7 | tip | HTTPS on orchestrator URL (localhost exception) |
| P-035 | done | becf9fe | tip | `ensure_threshold_snapshot_current` before broadcast |
| P-020 | partial | 1261980 | tip | In-flight dedupe + disabled button; **no** `Idempotency-Key` header |
| P-004 | partial | 809789a | tip | Strict CSP in `tauri.conf.json`; **no** Tauri 2 capabilities per window |
| P-008 | partial | 50d3d51 | tip | Zod on proposal/broadcast IPC; **not** wallet/orchestrator auth IPC |
| P-009 | done | 0556b5e | tip | Session authority must match selected role |
| P-010 | done | 5945d3a | tip | Sign deep-link refuses wrong authority |
| P-007 | done | 0825a1a | tip | Freeze sighash at preview; re-preview on edit |
| P-064 | partial | 83399c6 | tip | Five variants + serde tests in Tauri; **no** shared crate (Wave 3) |
| label-bug | done | 2b6c1d4 | tip | `Strata Administrator` via `authority-label.ts` |
| P-016 | done | 805c3e1 | tip | `ORCHESTRATOR_PROFILE=production` requires `DATABASE_URL` |
| P-029 | partial | 0e2a34f | tip | `/ready` + tracing on list/get only; **no** FE request UUID; `/ready` = BTC RPC only |
| P-054 | done | 5bdf8bf | tip | `description:` on `.claude/rules/*` (parity with `.cursor/rules`) |

**E2E column:** `tip` = covered by final `/e2e-proposal-flow` on branch tip, not a dedicated run after that commit.

## Post–Wave 1 commits (same branch)

| Commit | Scope |
|--------|--------|
| `6b0b9d4` | Prettier `use-broadcast-proposal` hook |
| `50d3d51` | P-008 follow-up: Zod `nullish` for proposal optional fields |
| `4e1b4e7` | Tracker: P-008 commit SHA |
| `4cad4d1` | Tracker: backfill + rustfmt |
| `e6a994d` | **Broadcast fix:** `getrawtransaction` for confirm polling (reveal not wallet-owned) |
| `662e517` | **E2E:** `mineWhileWaitingForBroadcastDone`; `mine-blocks.sh` env sourcing |
| `a74c817` | Nullish broadcast txids in Zod; WDIO 600s timeout; tracker E2E yes |

## Wave 1 exit criteria (§5)

| Criterion | Met? |
|-----------|------|
| Broadcast only via orchestrator | Yes |
| No `"enacted"` literal in `BroadcastResultDto` | Yes |
| Cross-authority 401 (integration test) | Yes |
| Prod: operator key + `DATABASE_URL`; reject test key | Yes (dev needs manual env — see gaps) |
| CSP in `tauri.conf.json` | Yes |
| Every IPC return through Zod | **Partial** (proposal/broadcast only) |
| Skeleton ADR-006 | **No** |

## Gaps → Wave 2 / follow-up

- **ADR-006** skeleton document (only unmet §5 exit criterion).
- **`autotest/start-stack.sh`:** does not export `OPERATOR_SECRET_KEY_HEX`, `BITCOIN_NETWORK`, etc. — documented in `~/.cursor/commands/e2e-proposal-flow.md` only.
- **Regtest broadcast:** orchestrator blocks on commit/reveal confirmations; E2E must mine blocks during wait (`662e517`).
- **Tier 0 still open:** P-003, P-005, P-006, P-011, P-012, … (see Wave 2 in action plan).

## Commit order (bootstrap + 20 issues)

```
213ce09 docs: add Wave 1 action-plan progress tracker
3981e0c fix(desktop): route broadcast via orchestrator (P-061)
fc3fbb9 fix(desktop): use persisted broadcast status (P-062)
8178b15 fix(be): filter proposals by session authority (P-002)
31b9f5a fix(be): reject missing/test operator secret key (P-001)
ceeeb7a fix(desktop): remove VITE operator key path (P-015)
53bcd98 fix(be): normalize pubkey case for dedup (P-063)
766d685 fix(be): require explicit BITCOIN_NETWORK (P-013)
e51daa7 fix(tauri): enforce HTTPS on orchestrator URL (P-014)
becf9fe fix(be): refuse broadcast on threshold drift (P-035)
1261980 fix(desktop): broadcast in-flight guard (P-020)
809789a fix(desktop): enable strict CSP (P-004)
16df77b fix(desktop): Zod validate Tauri IPC responses (P-008)
0556b5e fix(desktop): bind session to selected authority (P-009)
5945d3a fix(desktop): guard sign deep-link by authority (P-010)
0825a1a fix(desktop): freeze sighash at preview (P-007)
83399c6 fix(tauri): full Authority enum parity (P-064)
2b6c1d4 fix(desktop): correct broadcast authority label
805c3e1 fix(be): require DATABASE_URL in prod (P-016)
0e2a34f fix(be): tracing skeleton and /ready (P-029)
5bdf8bf chore: consolidate Cursor/Claude rules (P-054)
```
