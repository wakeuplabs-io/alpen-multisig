# Action plan — progress tracker

**Base branch:** `develop`  
**Source:** [action-plan-2026-05-14.md](action-plan-2026-05-14.md)  
**Wave 2 gap review:** [wave2-exit-gap-review.md](wave2-exit-gap-review.md)

---

## Wave 1 (merged)

PR [#134](https://github.com/wakeuplabs-io/alpen-multisig/pull/134). E2E PASS 2026-05-16 (height 594). Broadcast: P-066 + [proposal-broadcast-commit-reveal.md](../specs/proposal-broadcast-commit-reveal.md).

---

## Wave 2 — tracks

| Track | PR | Status |
|-------|-----|--------|
| A Secrets / IPC | [#136](https://github.com/wakeuplabs-io/alpen-multisig/pull/136) | **Merged** |
| B Coordination | [#138](https://github.com/wakeuplabs-io/alpen-multisig/pull/138) | **Merged** |
| C Supply chain | [#137](https://github.com/wakeuplabs-io/alpen-multisig/pull/137) | **Merged** |
| D Correctness & ops | [#139](https://github.com/wakeuplabs-io/alpen-multisig/pull/139) | **Merged** — [follow-ups](wave2-track-d-followups.md) |
| E Test floor | [#140](https://github.com/wakeuplabs-io/alpen-multisig/pull/140) | **Merged** — [follow-ups](wave2-track-e-followups.md) |
| G Discovery | [#142](https://github.com/wakeuplabs-io/alpen-multisig/pull/142) | **Merged** — [follow-ups](wave2-track-g-followups.md) |
| F Docs & signer safety | [#141](https://github.com/wakeuplabs-io/alpen-multisig/pull/141) | **Open** — rebase + merge |

### Track E — test floor (merged slice)

| P-ID | Status | Notes |
|------|--------|-------|
| P-008 (auth IPC) | done | PR #140 — Zod on all `auth_*` commands |
| P-008 (other IPC) | follow-up | signing, orchestrator-auth, asm-state — [wave2-track-e-followups.md](wave2-track-e-followups.md) |
| P-032 | partial | ipc-schemas tests; axis-10 inventory not in e2e-webdriver |
| US-H5 | gated | §3; no WDIO matrix in #140 by design |

---

## Wave 2 exit (summary)

| Criterion | Met? |
|-----------|------|
| Secret custody (prod) | Yes (A) |
| Supply chain CI | Yes (C) |
| ADR-006 / no auto-approve | Yes (B) |
| RPC timeouts | Partial (orchestrator D) |
| Tier 0 all closed | **No** — see [wave2-exit-gap-review.md](wave2-exit-gap-review.md) |
| Runbook + threat model + signer safety | **PR #141** |

**Engineering PRs left:** F only. **Product/legal:** gates §3, §4.

---

## E2E

| When | Result |
|------|--------|
| 2026-05-19 Track A worktree | PASS — 3 proposal WDIO specs; height 250 |
| Current `develop` | **Pending** — `/e2e-proposal-flow` |

---

## Human decisions

[wave2-human-decisions-pending.md](wave2-human-decisions-pending.md) — §3 US-H5 pending; §4 P-055 pending.

---

## Next steps

1. Merge **#141** (F).  
2. Resolve **§3 / §4**.  
3. **E2E** on `develop`.  
4. Wave 2 sign-off vs [wave2-exit-gap-review.md](wave2-exit-gap-review.md) deferrals.  
5. Wave 3 kickoff.
