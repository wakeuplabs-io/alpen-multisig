# Action plan — progress tracker

**Base branch:** `develop`  
**Source:** [action-plan-2026-05-14.md](action-plan-2026-05-14.md)  
**Wave 2:** **Closed** (sign-off 2026-05-19; A–G merged on `develop`).  
**Gap review:** [wave2-exit-gap-review.md](wave2-exit-gap-review.md)  
**Active wave:** Wave 3 — [action-plan-2026-05-14.md](action-plan-2026-05-14.md) §Wave 3.

---

## Wave 1 (merged)

PR [#134](https://github.com/wakeuplabs-io/alpen-multisig/pull/134). E2E PASS 2026-05-16 (height 594).

---

## Wave 2 — closed

Sign-off 2026-05-19. Slice deferrals: `wave2-track-*-followups.md`. **`develop → main` deferred** until Wave 3 + assessment action plan complete.

### Tracks merged

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
| P-055 | deferred | Post-merge docs PR — gate §4 resolved; awaits legal OK for in-repo excerpts |

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

**Wave 2 complete on `develop`.** Human gates §1–§4 resolved. Active work: Wave 3 + follow-up slices on `develop`; `main` updates after Wave 3 close-out.

---

## E2E

| When | Result | Height | Manual enactment |
|------|--------|--------|------------------|
| 2026-05-19 Track A worktree | PASS — 3 proposal WDIO | 250 | ASM log OK |
| 2026-05-19 **`develop`** (`30ca94f`) | **PASS** — add-signer, co-sign-row1, broadcast-quorum | 548 | **Yes** — 3 Strata Administrator signers incl. `03dd6d7…427c` |

---

## Human decisions

[wave2-human-decisions-pending.md](wave2-human-decisions-pending.md) — **all gates resolved** (§3 US-H5 → Wave 3; §4 P-055 → post-merge docs + legal OK).

---

## Next steps (post–Wave 2 close)

1. **Wave 3** — execute on `develop` per [action-plan-2026-05-14.md](action-plan-2026-05-14.md) §Wave 3 (US-H5 / PRD §2.3).  
2. **Follow-up slices** — D / E / F / G backlog as PRs to `develop`.  
3. **P-055 docs** — `sps-reference/` after Alpen legal OK (gate §4).  
4. **develop → main** — after Wave 3 + assessment action plan complete.  
5. **Optional** — GitHub issues from follow-up docs for tracking.

---

## Wave 3 — Stabilization

**Playbook:** [wave3-stabilization-execution-playbook.md](wave3-stabilization-execution-playbook.md)  
**Rule:** one P-ID → one commit; one PR → one or more P-IDs; open draft PR before first implementation commit.

### PR rows

| PR | Branch | Title | P-IDs | Status | PR link |
|----|--------|-------|-------|--------|---------|
| W3-0 | `wave3/w3-0-playbook` | Playbook on develop | DOC-PLAYBOOK | merged | [#151](https://github.com/wakeuplabs-io/alpen-multisig/pull/151) |
| W3-1 | `wave3/w3-1-ipc-zod` | IPC Zod remainder | P-008 | merged | [#152](https://github.com/wakeuplabs-io/alpen-multisig/pull/152) |
| W3-2 | `wave3/w3-2-coordination` | Coordination correctness | P-019, P-032 (race) | merged | [#153](https://github.com/wakeuplabs-io/alpen-multisig/pull/153) |
| W3-3 | `wave3/w3-3-test-floor` | Happy-path test floor | P-032 (floor) | merged | [#154](https://github.com/wakeuplabs-io/alpen-multisig/pull/154) |
| W3-4 | `wave3/w3-4-timeout-errors` | Timeout + typed errors | P-027, P-023 | merged | [#155](https://github.com/wakeuplabs-io/alpen-multisig/pull/155) |
| W3-5 | `wave3/w3-5-correlation` | Correlation slice | P-029 | in_progress | [#156](https://github.com/wakeuplabs-io/alpen-multisig/pull/156) |
| W3-6 | `wave3/w3-6-wallet-pubkey` | Wallet pubkey binding | P-039 | pending | — |
| W3-7 | `wave3/w3-7-hygiene` | Codebase hygiene | HYG-POC, P-036 [, P-057] | pending | — |
| W3-8 | `wave3/w3-8-docs` | Action plan close-out | DOC-W3 | pending | — |

### P-ID rows

| P-ID | Status | PR | Notes |
|------|--------|----|-------|
| DOC-PLAYBOOK | done | W3-0 | Playbook + tracker section |
| P-008 | done | W3-1 | Zod at `tauriCall` for signing, orchestrator-auth, asm-state, action-builder |
| P-019 | done | W3-2 | Duplicate check in `add_signature` under write lock |
| P-032 (race) | done | W3-2 | Integration test concurrent duplicate approve |
| P-032 (floor) | done | W3-3 | Claim/broadcast conflict guards + `e2e_propose_sign` |
| P-027 | done | W3-4 | ~30s timeout on Tauri ASM/Bitcoin RPC in broadcast path |
| P-023 | done | W3-4 | `errorCode` on happy-path orchestrator APIs + Tauri/bridge |
| P-029 | done | W3-5 | `X-Request-Id` in bridge; `tracing` on approve/patch/claim/broadcast |
| P-039 | pending | W3-6 | Reject when `wallet.publicKeyHex !== signature.publicKeyHex` |
| HYG-POC | pending | W3-7 | Remove POC naming from product code + active docs |
| P-036 | pending | W3-7 | Centralize `REVEAL_TX_VBYTES`, `COMMIT_DUST_SATS` |
| P-057 | pending | W3-7 | Vestigial Tauri feature / unused config (optional) |
| DOC-W3 | pending | W3-8 | Rewrite Wave 3 + Future appendix + satellite doc touch-ups |
