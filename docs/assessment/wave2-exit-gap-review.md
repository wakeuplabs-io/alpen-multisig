# Wave 2 — exit gap review

**As of:** `develop` after PRs [#136](https://github.com/wakeuplabs-io/alpen-multisig/pull/136)–[#140](https://github.com/wakeuplabs-io/alpen-multisig/pull/140), [#142](https://github.com/wakeuplabs-io/alpen-multisig/pull/142).  
**Source exit criteria:** [action-plan-2026-05-14.md §Wave 2](action-plan-2026-05-14.md#wave-2--correctness-supply-chain-operations-weeks-36).

Only **Track F** (#141) remains an open engineering PR.

---

## Exit criteria checklist

| Criterion | Status | Evidence / gap |
|-----------|--------|----------------|
| No mnemonic/key over IPC (production) | **Met** | Track A, [secret-custody-wave2.md](../specs/secret-custody-wave2.md) |
| `cargo deny` / `cargo audit` / `npm audit` block CI | **Met** | Track C |
| No auto-Approve; ADR-006 + threshold at claim | **Met** | Track B, [ADR-006](../architecture/adrs/006-backend-coordination-boundary.md) |
| All RPC calls in `tokio::time::timeout` + structured errors | **Partial** | Orchestrator ASM/BTC 30s (D); desktop Tauri RPC not wrapped; retries/backoff open |
| Every Tier 0 BLOCKER from 2026-05-14 closed | **Partial** | See table below |
| Ops runbook + threat model + signer-safety model linked from README | **Open** | In PR #141; merge F |

---

## Tracks merged vs plan

| Track | PR | Plan intent | Merged reality |
|-------|-----|-------------|----------------|
| A | #136 | P-001, P-003, P-040, P-033 | **Complete** for Wave 2 slice |
| B | #138 | ADR-006, P-012, P-026, P-025, P-037, P-028 | **Complete** |
| C | #137 | P-011 MVP | **Complete** |
| D | #139 | Full correctness & ops row | **Slice:** P-027 orchestrator only → [follow-ups](wave2-track-d-followups.md) |
| E | #140 | P-032 + P-008 + US-H5 E2E | **Slice:** auth P-008 + ipc-schemas tests; no WDIO US-H5 → [follow-ups](wave2-track-e-followups.md) |
| G | #142 | P-053 discovery start | **Slice:** plans only → [follow-ups](wave2-track-g-followups.md) |
| F | #141 | P-051, P-005, P-006, P-055 | **Not merged** — P-055 blocked on legal §4 |

---

## Tier 0 / high-leverage gaps (still open on `develop`)

| P-ID | Topic | Owner | Notes |
|------|--------|-------|-------|
| P-004 | Tauri CSP / capabilities | FE/Tauri | CSP enabled in `tauri.conf.json`; capabilities (P-040 prod set) done in A |
| P-005 | On-device verification UX | F | Not merged |
| P-006 | HW vs software payload parity | F + G usability | Blocked on product + G sessions |
| P-008 | IPC Zod everywhere | FE | Auth + proposals done (E); signing/orch/asm IPC open |
| P-017–P-019, P-023, P-029 | Ops / correctness | BE | D follow-ups |
| P-032 | Test floor axis-10 | QA | E slice only; rest in [wave2-track-e-followups.md](wave2-track-e-followups.md) |
| P-055 | SPS archive in repo | Docs | **Gate §4** |
| US-H5 | Manual fallback | Product + QA | **Gate §3**; not in e2e-webdriver by policy |

Wave 1 partials carried into Wave 2 (see [action-plan-progress.md](action-plan-progress.md) Wave 1 table): P-020, P-063, P-064, etc.

---

## Human gates (must close for full Wave 2 scope)

| # | Topic | Blocks |
|---|--------|--------|
| §3 | US-H5 Slice-0 vs deferred | Fallback implementation + P-032/US-H5 tests |
| §4 | P-055 legal | SPS excerpts in `docs/specs/sps-reference/` |

---

## Recommended close-out sequence

1. **Merge F (#141)** — satisfies runbook / threat model / signer-safety exit line (minus P-055 import).
2. **Resolve §3 and §4** with Alpen stakeholders.
3. **Run `/e2e-proposal-flow` on `develop`** — proposal WDIO + manual enactment (gate for release confidence).
4. **Phase 2 PRs** (optional before declaring Wave 2 “closed”):
   - D: [wave2-track-d-followups.md](wave2-track-d-followups.md)
   - E: [wave2-track-e-followups.md](wave2-track-e-followups.md)
   - Product: [wave2-track-g-followups.md](wave2-track-g-followups.md)
5. **Wave 2 sign-off meeting** — accept partial Tier 0 deferrals to Wave 3 or open tickets with acceptance criteria.
6. **Wave 3 planning** — `multisig-types`, event log (P-031), US-H5 walking skeleton (P-052).

---

## E2E gate

| Check | Status |
|-------|--------|
| WDIO connect testids on `develop` | Done (`f8d724e`) |
| Full proposal flow on current `develop` | **Pending** |
| Manual enactment confirmation | **Pending** |
