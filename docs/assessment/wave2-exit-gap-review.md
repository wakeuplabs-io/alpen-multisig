# Wave 2 — exit gap review

**As of:** `develop` @ `2afe0d3` — **all Wave 2 track PRs merged** ([#136](https://github.com/wakeuplabs-io/alpen-multisig/pull/136)–[#142](https://github.com/wakeuplabs-io/alpen-multisig/pull/142)).  
**Source:** [action-plan-2026-05-14.md §Wave 2](action-plan-2026-05-14.md#wave-2--correctness-supply-chain-operations-weeks-36).

The parallel **engineering PR queue is complete**. Wave 2 is not fully closed against every P-ID in the plan — several tracks landed as **slices** with documented follow-ups.

---

## Exit criteria checklist

| Criterion | Status | Evidence |
|-----------|--------|----------|
| No mnemonic/key over IPC (production) | **Met** | Track A — [secret-custody-wave2.md](../specs/secret-custody-wave2.md) |
| `cargo deny` / `cargo audit` / `npm audit` block CI | **Met** | Track C |
| No auto-Approve; ADR-006 + threshold at claim | **Met** | Track B — [ADR-006](../architecture/adrs/006-backend-coordination-boundary.md) |
| All RPC calls in `tokio::time::timeout` + structured errors | **Partial** | Orchestrator 30s (D); desktop + retries → [wave2-track-d-followups.md](wave2-track-d-followups.md) |
| Every Tier 0 BLOCKER from 2026-05-14 closed | **Partial** | See tier table below |
| Ops runbook + threat model + signer-safety linked from README | **Met** | Track F — [README.md](../../README.md) |

---

## Tracks merged vs full plan

| Track | PR | Merged slice | Follow-up doc |
|-------|-----|--------------|---------------|
| A | #136 | P-001, P-003, P-040, P-033 | — |
| B | #138 | ADR-006, P-012, P-026, P-025, P-037, P-028 | — |
| C | #137 | P-011 | — |
| D | #139 | P-027 orchestrator | [wave2-track-d-followups.md](wave2-track-d-followups.md) |
| E | #140 | P-008 auth; ipc-schemas tests | [wave2-track-e-followups.md](wave2-track-e-followups.md) |
| F | #141 | P-051 docs | [wave2-track-f-followups.md](wave2-track-f-followups.md) |
| G | #142 | P-053 plans | [wave2-track-g-followups.md](wave2-track-g-followups.md) |

---

## Still open (post–Wave 2 PR queue)

### Human gates

| # | Topic | Doc |
|---|--------|-----|
| §3 | US-H5 manual-fallback scope | [wave2-human-decisions-pending.md](wave2-human-decisions-pending.md) |
| §4 | P-055 SPS in-repo | Same |

### P-ID backlog (by follow-up doc)

| Doc | P-IDs |
|-----|-------|
| [wave2-track-d-followups.md](wave2-track-d-followups.md) | P-017, P-018, P-019, P-023, P-029, P-027 remainder |
| [wave2-track-e-followups.md](wave2-track-e-followups.md) | P-008 remainder, P-032 axis-10, US-H5 tests |
| [wave2-track-f-followups.md](wave2-track-f-followups.md) | P-005, P-006, P-055 |
| [wave2-track-g-followups.md](wave2-track-g-followups.md) | P-053 execution (interviews, usability) |

### Validation

| Item | Status |
|------|--------|
| `/e2e-proposal-flow` on `develop` (2026-05-19) | **PASS** — 3 WDIO specs; chain mined to 548 |
| Manual enactment sign-off | **Done** (2026-05-19) — 3 Strata Administrator signers incl. `03dd6d7…427c` |

---

## Recommended close-out

1. **Stakeholder** resolution of §3 and §4.  
2. **Sign-off** meeting: accept slice deferrals → GitHub issues / Wave 3.  
3. Start **Wave 3** ([action plan §Wave 3](action-plan-2026-05-14.md#wave-3--architectural-hardening--governance-integrity-weeks-712)).
