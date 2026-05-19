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
| D | `wave2/track-d-correctness-ops` | [#139](https://github.com/wakeuplabs-io/alpen-multisig/pull/139) | **Merged** (P-027 slice; see [follow-ups](wave2-track-d-followups.md)) |
| E | `wave2/track-e-test-floor` | [#140](https://github.com/wakeuplabs-io/alpen-multisig/pull/140) | Open — rebase onto `develop` |
| F | `wave2/track-f-docs-signer-safety` | [#141](https://github.com/wakeuplabs-io/alpen-multisig/pull/141) | Open — rebase onto `develop` |
| G | `wave2/track-g-discovery` | [#142](https://github.com/wakeuplabs-io/alpen-multisig/pull/142) | Open — rebase onto `develop` |

### Track A — secrets, IPC, broadcast crypto (merged)

| P-ID | Status | Notes |
|------|--------|-------|
| P-001, P-033, P-003, P-040 | done | See PR #136; [secret-custody-wave2.md](../specs/secret-custody-wave2.md) |

### Track B — coordination boundary (merged)

| P-ID | Status | Notes |
|------|--------|-------|
| ADR-006, P-012, P-026, P-025, P-037, P-028 | done | [ADR-006](../architecture/adrs/006-backend-coordination-boundary.md) |

### Track C — supply chain (merged)

| P-ID | Status | Notes |
|------|--------|-------|
| P-011 | done | Lockfile, audit/deny, gitleaks, CI ipc-schemas |

### Track D — correctness & ops

| P-ID | Status | Notes |
|------|--------|-------|
| P-027 (orchestrator) | **done** | PR #139 — 30s timeout on ASM + Bitcoin RPC (`rpc_timeout.rs`) |
| P-017, P-018, P-019, P-023, P-029 | **follow-up** | [wave2-track-d-followups.md](wave2-track-d-followups.md) |
| P-027 (desktop / retries) | **follow-up** | Tauri ASM/BTC calls; backoff/circuit breaker |

### Track E — test floor (open)

| P-ID | Status | Notes |
|------|--------|-------|
| P-008, P-032 | partial | PR #140 |
| US-H5 E2E | blocked | Human gate §3 |

### Track F — docs & signer safety (open)

| P-ID | Status | Notes |
|------|--------|-------|
| P-051 | in PR | Runbook, threat model, signer-safety model |
| P-005, P-006 | pending | PR #141 |
| P-055 | blocked | Human gate §4 |

### Track G — discovery (open)

| P-ID | Status | Notes |
|------|--------|-------|
| P-053 | partial | Gate log + interview plan — PR #142 |

---

## Wave 2 exit criteria (snapshot)

| Criterion | Status |
|-----------|--------|
| No mnemonic/key over IPC (prod) | **Done** (Track A) |
| cargo deny / audit / npm audit in CI | **Done** (Track C) |
| Auto-Approve removed or ADR-006 + test | **Done** (Track B) |
| RPC timeouts (orchestrator) | **Done** (Track D P-027) |
| RPC timeouts (desktop + retries) | **Open** — follow-up |
| All Tier 0 BLOCKERS closed | **Partial** — E, F, D follow-ups |
| Ops runbook + threat model + signer safety | **In PR #141** |

---

## E2E (Wave 2)

| When | Result | Height | Manual enactment |
|------|--------|--------|------------------|
| 2026-05-19 Track A branch | **PASS** — three WDIO proposal specs | 250 | ASM log OK; UI third signer optional |
| Post–Track D `develop` | **Pending** | — | After E2E testid fix + `/e2e-proposal-flow` on `develop` |

---

## Human decisions

See [wave2-human-decisions-pending.md](wave2-human-decisions-pending.md).

| # | Topic | Status |
|---|--------|--------|
| 1 | P-012 / ADR-006 | **Resolved** |
| 2 | Secret custody | **Resolved** |
| 3 | US-H5 manual-fallback scope | **Pending** — blocks Track E |
| 4 | P-055 SPS excerpts | **Pending** — blocks Track F |

---

## Next steps

1. **Merge G → E → F** (rebased on `develop` after #139).
2. **Human gates** §3 and §4 before full Track E / P-055 scope.
3. **E2E on `develop`** — `/e2e-proposal-flow` after WDIO connect testids land.
4. **Track D phase 2** — [wave2-track-d-followups.md](wave2-track-d-followups.md).
5. **Wave 2 exit review** → Wave 3 planning.
