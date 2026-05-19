# Action plan — progress tracker

**Base branch:** `develop`  
**Source:** [action-plan-2026-05-14.md](action-plan-2026-05-14.md)

---

## Wave 1 (merged)

Branch: `fix/action-plan-wave1-2026-05-14` → **merged** via PR [#134](https://github.com/wakeuplabs-io/alpen-multisig/pull/134).

E2E PASS 2026-05-16 (chain height 594, manual enactment yes). Broadcast boundary: P-066 + [proposal-broadcast-commit-reveal.md](../specs/proposal-broadcast-commit-reveal.md).

---

## Wave 2 — parallel tracks

| Track | Branch | PR | Status |
|-------|--------|-----|--------|
| A | `wave2/track-a-secrets-ipc` | [#136](https://github.com/wakeuplabs-io/alpen-multisig/pull/136) | **Merged** |
| B | `wave2/track-b-coordination-boundary` | [#138](https://github.com/wakeuplabs-io/alpen-multisig/pull/138) | **Merged** |
| C | `wave2/track-c-supply-chain` | [#137](https://github.com/wakeuplabs-io/alpen-multisig/pull/137) | **Merged** |
| D | `wave2/track-d-correctness-ops` | [#139](https://github.com/wakeuplabs-io/alpen-multisig/pull/139) | **Merged** — [follow-ups](wave2-track-d-followups.md) |
| G | `wave2/track-g-discovery` | [#142](https://github.com/wakeuplabs-io/alpen-multisig/pull/142) | **Merged** — [follow-ups](wave2-track-g-followups.md) |
| E | `wave2/track-e-test-floor` | [#140](https://github.com/wakeuplabs-io/alpen-multisig/pull/140) | Open — rebase onto `develop` |
| F | `wave2/track-f-docs-signer-safety` | [#141](https://github.com/wakeuplabs-io/alpen-multisig/pull/141) | Open — rebase onto `develop` |

### Track A — secrets, IPC, broadcast crypto (merged)

| P-ID | Status | Notes |
|------|--------|-------|
| P-001, P-033, P-003, P-040 | done | PR #136; [secret-custody-wave2.md](../specs/secret-custody-wave2.md) |

### Track B — coordination boundary (merged)

| P-ID | Status | Notes |
|------|--------|-------|
| ADR-006, P-012, P-026, P-025, P-037, P-028 | done | [ADR-006](../architecture/adrs/006-backend-coordination-boundary.md) |

### Track C — supply chain (merged)

| P-ID | Status | Notes |
|------|--------|-------|
| P-011 | done | Lockfile, audit/deny, gitleaks, CI ipc-schemas |

### Track D — correctness & ops (merged slice)

| P-ID | Status | Notes |
|------|--------|-------|
| P-027 (orchestrator) | done | PR #139 — `rpc_timeout.rs` |
| P-017, P-018, P-019, P-023, P-029, P-027 remainder | follow-up | [wave2-track-d-followups.md](wave2-track-d-followups.md) |

### Track G — discovery (merged slice)

| P-ID | Status | Notes |
|------|--------|-------|
| P-053 (plans) | done | PR #142 — interview + digest usability protocols |
| P-053 (execution) | follow-up | [wave2-track-g-followups.md](wave2-track-g-followups.md) — run sessions, findings doc |

### Track E — test floor (open)

| P-ID | Status | Notes |
|------|--------|-------|
| P-008, P-032 | partial | PR #140 |
| US-H5 E2E | blocked | Human gate §3 — see G tabletop plan after decision |

### Track F — docs & signer safety (open)

| P-ID | Status | Notes |
|------|--------|-------|
| P-051 | in PR | Runbook, threat model, signer-safety model |
| P-005, P-006 | pending | PR #141; P-006 informed by G usability outcomes |
| P-055 | blocked | Human gate §4 |

---

## Wave 2 exit criteria (snapshot)

| Criterion | Status |
|-----------|--------|
| No mnemonic/key over IPC (prod) | **Done** (A) |
| cargo deny / audit / npm audit in CI | **Done** (C) |
| Auto-Approve removed or ADR-006 + test | **Done** (B) |
| RPC timeouts (orchestrator) | **Done** (D) |
| RPC timeouts (desktop + retries) | Open — D follow-up |
| P-053 discovery **started** | **Done** (G plans); execution open |
| All Tier 0 BLOCKERS closed | Partial — E, F, D follow-ups |
| Ops runbook + threat model + signer safety | In PR #141 |

---

## E2E (Wave 2)

| When | Result | Height | Manual enactment |
|------|--------|--------|------------------|
| 2026-05-19 Track A branch | **PASS** — three WDIO proposal specs | 250 | ASM log OK |
| Post–A–G merge `develop` | **Pending** | — | `/e2e-proposal-flow` on current `develop` (testids on `f8d724e`) |

---

## Human decisions

[wave2-human-decisions-pending.md](wave2-human-decisions-pending.md)

| # | Topic | Status |
|---|--------|--------|
| 1 | P-012 / ADR-006 | **Resolved** |
| 2 | Secret custody | **Resolved** |
| 3 | US-H5 manual-fallback scope | **Pending** — blocks Track E matrix |
| 4 | P-055 SPS excerpts | **Pending** — blocks Track F import |

---

## Next steps

1. **Rebase and merge E, F** onto `develop` (A–D, G done).
2. **Resolve gates §3 and §4** with stakeholders.
3. **E2E** — `/e2e-proposal-flow` on `develop`.
4. **Execute** — [wave2-track-g-followups.md](wave2-track-g-followups.md) (interviews/usability); [wave2-track-d-followups.md](wave2-track-d-followups.md) (ops P-IDs).
5. **Wave 2 exit review** → Wave 3.
