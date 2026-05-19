# Action plan — progress tracker

**Base branch:** `develop`  
**Source:** [action-plan-2026-05-14.md](action-plan-2026-05-14.md)  
**Wave 2 PR queue:** **Complete** (A–G merged).  
**Gap review:** [wave2-exit-gap-review.md](wave2-exit-gap-review.md)

---

## Wave 1 (merged)

PR [#134](https://github.com/wakeuplabs-io/alpen-multisig/pull/134). E2E PASS 2026-05-16 (height 594).

---

## Wave 2 — all tracks merged

| Track | PR | Merged | Follow-ups |
|-------|-----|--------|------------|
| A Secrets / IPC | [#136](https://github.com/wakeuplabs-io/alpen-multisig/pull/136) | 2026-05-19 | — |
| B Coordination | [#138](https://github.com/wakeuplabs-io/alpen-multisig/pull/138) | 2026-05-19 | — |
| C Supply chain | [#137](https://github.com/wakeuplabs-io/alpen-multisig/pull/137) | 2026-05-19 | — |
| D Correctness & ops | [#139](https://github.com/wakeuplabs-io/alpen-multisig/pull/139) | 2026-05-19 | [wave2-track-d-followups.md](wave2-track-d-followups.md) |
| E Test floor | [#140](https://github.com/wakeuplabs-io/alpen-multisig/pull/140) | 2026-05-19 | [wave2-track-e-followups.md](wave2-track-e-followups.md) |
| F Docs & signer safety | [#141](https://github.com/wakeuplabs-io/alpen-multisig/pull/141) | 2026-05-19 | [wave2-track-f-followups.md](wave2-track-f-followups.md) |
| G Discovery | [#142](https://github.com/wakeuplabs-io/alpen-multisig/pull/142) | 2026-05-19 | [wave2-track-g-followups.md](wave2-track-g-followups.md) |

### Track F — docs & signer safety (merged slice)

| P-ID | Status | Notes |
|------|--------|-------|
| P-051 | done | Runbook, threat model, signer-safety model + README links |
| P-005, P-006 | follow-up | [wave2-track-f-followups.md](wave2-track-f-followups.md) |
| P-055 | blocked | Gate §4 — legal |

---

## Wave 2 exit (summary)

| Criterion | Met? |
|-----------|------|
| Secret custody (prod) | **Yes** (A) |
| Supply chain CI | **Yes** (C) |
| ADR-006 / no auto-approve | **Yes** (B) |
| RPC timeouts (orchestrator) | **Yes** (D slice) |
| Runbook + threat model + signer safety | **Yes** (F) |
| Tier 0 all closed | **No** — deferred items in [wave2-exit-gap-review.md](wave2-exit-gap-review.md) |
| Full Wave 2 plan (every P-ID) | **No** — track follow-up docs |

**All seven Wave 2 engineering PRs are on `develop`.** Remaining work is follow-up slices, human gates, E2E validation, and Wave 3.

---

## E2E

| When | Result | Height | Manual enactment |
|------|--------|--------|------------------|
| 2026-05-19 Track A worktree | PASS — 3 proposal WDIO | 250 | ASM log OK |
| 2026-05-19 **`develop`** (`782fc12`) | **PASS** — add-signer, co-sign-row1, broadcast-quorum | see post-run mine | **Pending** UI check |

---

## Human decisions

[wave2-human-decisions-pending.md](wave2-human-decisions-pending.md) — §3 US-H5, §4 P-055 still **pending**.

---

## Next steps

1. **E2E** — `/e2e-proposal-flow` on `develop`.  
2. **Gates §3 / §4** — stakeholder decisions.  
3. **Wave 2 sign-off** — review [wave2-exit-gap-review.md](wave2-exit-gap-review.md); ticket deferrals.  
4. **Follow-up PRs** — D / E / F / G backlog docs as needed.  
5. **Wave 3** — kickoff per action plan.
