# Wave 1 action plan — progress tracker

Branch: `fix/action-plan-wave1-2026-05-14` (PR [#134](https://github.com/wakeuplabs-io/alpen-multisig/pull/134))  
Base: `develop`  
Source: [action-plan-2026-05-14.md](action-plan-2026-05-14.md)  
Execution record: [§5.1](action-plan-2026-05-14.md#51-wave-1--execution-record-2026-05-16)  
Broadcast boundary (SSOT): [§2.1 + §5.2](action-plan-2026-05-14.md#21-broadcast-boundary--ssot-reconciles-prd-discovery-assessments)

## Summary

| Metric | Value |
|--------|--------|
| Planned P-IDs + label | 20 + 1 |
| Wave 1 commits (bootstrap + issues) | 21 (`213ce09` … `5bdf8bf`) |
| Post–Wave 1 commits | 7 (`6b0b9d4` … `a74c817`) |
| P-066 correction | Committed on branch (see tip after commit) |
| E2E gate (2026-05-16) | **PASS** — add-signer, co-sign-row1, broadcast-quorum; manual enactment **yes** (chain height 594) |
| ADR-006 skeleton | Not started (Wave 2) |

## P-ID status

| P-ID | Status | Commit | E2E | Notes |
|------|--------|--------|-----|-------|
| P-061 | superseded | 3981e0c (reverted in P-066) | — | Do not use server `POST …/broadcast` execute path |
| **P-066** | done | (this commit) | yes | Tauri: local commit/reveal; BE: `claim` + `PATCH` only |
| P-062 | done | fc3fbb9 | tip* | Re-fetch proposal; persisted status in UI |
| P-002 | done | 8178b15 | tip* | Authority on list/get + broadcast coordination |
| P-001 | partial | 31b9f5a | tip* | BE rejects test key; **operator key now desktop-only** for broadcast |
| P-015 | done | ceeeb7a | tip* | No `VITE_OPERATOR_*` |
| P-063 | partial | 53bcd98 | tip* | Lowercase ingress; no Postgres `CHECK` |
| P-013 | done | 766d685 | tip* | `BITCOIN_NETWORK` required on orchestrator |
| P-014 | done | e51daa7 | tip* | HTTPS orchestrator URL |
| P-035 | done | becf9fe | tip* | Threshold snapshot before claim |
| P-020 | partial | 1261980 | tip* | In-flight guard; no `Idempotency-Key` |
| P-004 | partial | 809789a | tip* | Strict CSP; no Tauri capabilities |
| P-008 | partial | 50d3d51 | tip* | Zod on proposal/broadcast IPC |
| P-009 | done | 0556b5e | tip* | Session authority binding |
| P-010 | done | 5945d3a | tip* | Sign deep-link authority guard |
| P-007 | done | 0825a1a | tip* | Sighash freeze at preview |
| P-064 | partial | 83399c6 | tip* | Tauri enum parity; shared crate deferred |
| label-bug | done | 2b6c1d4 | tip* | Authority label on broadcast screen |
| P-016 | done | 805c3e1 | tip* | `DATABASE_URL` in prod profile |
| P-029 | partial | 0e2a34f | tip* | `/ready` BTC RPC; partial tracing |
| P-054 | done | 5bdf8bf | tip* | `.claude/rules` descriptions |

\* `tip` = last E2E at `a74c817` before P-066; not valid until re-run with desktop broadcast env.

## P-066 deliverables (architectural correction)

| Area | Change |
|------|--------|
| Orchestrator | Removed `POST …/broadcast/prepare`, `POST …/broadcast` (execute), `broadcast_tx.rs`; added `POST …/broadcast/claim`, `PATCH …/broadcast` |
| Desktop Tauri | `broadcast_env.rs`; local `prepare_broadcast_bundle` / `broadcast_commit_then_reveal`; coordination callbacks |
| Docs | `docs/specs/proposal-broadcast-commit-reveal.md`, `docs/architecture/overview.md`, action plan §2.1 / §5.2 |
| Config | `OPERATOR_SECRET_KEY_HEX` in `desktop-app/.env.example`; removed from orchestrator prod path |

## Wave 1 exit criteria

| Criterion | Met? |
|-----------|------|
| Desktop-owned broadcast + coordinator metadata | **Yes** (P-066 + E2E 2026-05-16) |
| No `"enacted"` literal in `BroadcastResultDto` | Yes |
| Cross-authority 401 | Yes |
| Prod: `DATABASE_URL` on orchestrator; operator key for broadcast on desktop | Yes (P-001 split) |
| CSP in `tauri.conf.json` | Yes |
| Every IPC return through Zod | Partial |
| ADR-006 skeleton | No |

## Gaps → Wave 2

- **ADR-006** — broadcast boundary + P-012 threshold policy.
- **P-001** — desktop `broadcast_env` must reject test key (mirror BE rules).
- **E2E** — `/e2e-proposal-flow` passed 2026-05-16 with desktop broadcast env in `desktop-app/.env`.
- **Manual fallback test** — broadcast with orchestrator down after signatures (US-H5 matrix).
- **Tier 0 open:** P-003, P-005, P-006, P-011, P-012, …

## Commit order (reference)

Wave 1 issue commits: `213ce09` … `5bdf8bf` (see [action-plan-2026-05-14.md §5.1](action-plan-2026-05-14.md#51-wave-1--execution-record-2026-05-16)).

