# Wave 1 action plan — progress tracker

Branch: `fix/action-plan-wave1-2026-05-14`  
Source: [action-plan-2026-05-14.md](action-plan-2026-05-14.md)

Manual E2E: `/e2e-proposal-flow` — **final tip validation PASS** at `662e517` (chain height 445, manual enactment confirmed).

| P-ID | Status | Commit | E2E | Notes |
|------|--------|--------|-----|-------|
| P-061 | done | 3981e0c | yes | Route broadcast via orchestrator HTTP |
| P-062 | done | fc3fbb9 | yes | Re-fetch proposal after broadcast |
| P-002 | done | 8178b15 | yes | Authority filter on list/get/broadcast |
| P-001 | done | 31b9f5a | yes | Reject missing/test operator secret key |
| P-015 | done | ceeeb7a | yes | Remove VITE operator key path |
| P-063 | done | 53bcd98 | yes | Pubkey case normalization |
| P-013 | done | 766d685 | yes | Explicit bitcoin network |
| P-014 | done | e51daa7 | yes | HTTPS on orchestrator URL |
| P-035 | done | becf9fe | yes | Threshold drift at broadcast |
| P-020 | done | 1261980 | yes | Broadcast in-flight guard |
| P-004 | done | 809789a | yes | Strict CSP |
| P-008 | done | 50d3d51 | yes | Zod IPC; nullish Option fields (fix atop 16df77b) |
| P-009 | done | 0556b5e | yes | Session bound to authority |
| P-010 | done | 5945d3a | yes | Sign deep-link authority guard |
| P-007 | done | 0825a1a | yes | Freeze sighash at preview |
| P-064 | done | 83399c6 | yes | Full Authority enum parity |
| label-bug | done | 2b6c1d4 | yes | Strata Administrator label |
| P-016 | done | 805c3e1 | yes | DATABASE_URL required in prod |
| P-029 | done | 0e2a34f | yes | Tracing skeleton + /ready |
| P-054 | done | 5bdf8bf | yes | Consolidate Cursor/Claude rules |

Post–Wave 1 fixes on branch (also covered by final E2E): `e6a994d` getrawtransaction confirmations, `662e517` broadcast mining in WDIO.
