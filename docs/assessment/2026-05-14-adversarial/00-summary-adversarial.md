# Alpen Multisig — Adversarial Assessment Rollup (2026-05-14)

## Scope, method, and constraints

Synthesis of **17 axis reports** under `docs/assessment/2026-05-14-adversarial/` produced after a **project update** and a **fresh re-read** of the repo (Rust `orchestrator-be`, Tauri `desktop-app/src-tauri`, React `desktop-app/src`, docs, CI, ADRs, stories). Axis workers used **`composer-2-fast`** after **`claude-opus-4-7-thinking-xhigh`** hit API limits on the first attempt; outputs are still evidence-first and path-cited.

Method: **read-only** code and doc audit. No runtime probes or load tests were executed unless an axis explicitly says otherwise. Every substantive claim below cites **[axis NN]** (the `NN-*-adversarial.md` file in this folder). Where this rollup merges duplicates, multiple axes are listed.

**Correction vs 2026-05-13 rollup:** Axis **04 [axis 04]** confirms **`/api/v1`** is present; the prior “no API versioning” item is **retracted** here [axis 04 intro; contrast 2026-05-13 axis 04 / 00 §Tier 2 #46].

---

## Executive verdict (one paragraph, brutal but specific)

The 2026-05-14 pass **does not improve the production verdict**: the system remains **unsafe to operate as a governance authority** until Tier 0 closes. **New and stronger than the May 13 rollup:** the desktop **`proposals_broadcast`** path **never calls** the backend’s atomic **`claim_broadcast` / `execute_broadcast`** pipeline—local Bitcoin RPC completes commit/reveal while the orchestrator stays **`approved` / `idle`**, the UI can show **“enacted”** from **hard-coded** IPC strings, and another machine can **re-broadcast** the same logical proposal **[axis 04]**, **[axis 02]**, **[axis 01]**. That sits **on top of** the already-critical cluster: **authority-unscoped** `list`/`get`/`prepare`/`execute` broadcast handlers **[axis 01]**, **test-operator-key default** **[axis 01]**, **CSP `null` + plaintext secrets over IPC** **[axis 02]**, **[axis 05]**, **no Zod/runtime validation at the Tauri bridge** **[axis 03]**, **[axis 04]**, **duplicate-signer check case mismatch** (`eq_ignore_ascii_case` for auth vs `==` for dedup) **[axis 04]**, **Tauri `Authority` subset vs backend five variants** (deserialization failures for non–Strata-admin proposals) **[axis 04]**, **in-memory default + sessions/challenges in `HashMap`** **[axis 01]**, **[axis 07]**, **[axis 09]**, **zero frontend tests / weak negative-path coverage** **[axis 10]**, **PRD manual fallback and signer-safety AC still unclosed in stories** **[axis 12]**, **[axis 13]**, **typed `AppError` vs desktop `String` errors** **[axis 14]**, **doc / ADR / SPS provenance gaps** **[axis 15]**, **[axis 16]**, **`.claude/` vs `.cursor/` rule drift** **[axis 17]**. Smallest credible “stop the bleeding” window remains on the order of **weeks**, not days, before honest production claims.

---

## Ranked org-level backlog (merged)

### Tier 0 — Security & signer safety / correctness (BLOCKING)

1. **Desktop broadcast bypasses orchestrator state machine** — `proposals_broadcast` uses local `broadcast_commit_then_reveal` and **does not** invoke `/api/v1/proposals/:id/broadcast`; backend **`claim_broadcast` is dead on the happy path**; **double on-chain governance spend** and **stale dashboard** buckets **[axis 04]**, **[axis 02]**, **[axis 01]**.
2. **Hard-coded `BroadcastResultDto` `"enacted"` / `"reveal_confirmed"`** — UI shows finality **without** persisted orchestrator or chain truth **[axis 04]**, **[axis 02]**.
3. **Case-sensitive duplicate-signer check vs case-insensitive session pubkey** — same signer can appear twice with mixed-case hex **[axis 04]**.
4. **Tauri `Authority` is a subset of backend authorities** — non–Strata-admin proposals **fail deserialization** in the shell despite valid HTTP JSON **[axis 04]**.
5. **Authority scope leakage** — `list_proposals` / `get_proposal` / broadcast prep/exec discard or under-use session authority **[axis 01]**, **[axis 06]**, **[axis 08]**.
6. **Operator secret default + IPC/FE paths** — `OPERATOR_SECRET_KEY_HEX` fallback; operator key on IPC **[axis 01]**, **[axis 02]**, **[axis 05]**, **[axis 14]**.
7. **CSP `null`** — maximum XSS → **`invoke`** blast radius **[axis 02]**, **[axis 05]**.
8. **No runtime validation of IPC / structured errors** — `tauriCall`, `ApiResult` string errors; cannot branch safely **[axis 03]**, **[axis 04]**, **[axis 14]**.
9. **Session / authority / preview–sign hazards** — authority reuse, sighash swap, deep links, wrong **broadcast** authority label (`StrataAdministrator` → “Alpen Administrator”) **[axis 03]**, **[axis 13]**.
10. **Supply chain & release** — unsigned desktop story, weak CI gates (audit/deny/npm), lockfile policy **[axis 05]**.
11. **“Coordination only” vs threshold / validity behavior** — PRD §1 tension with auto-approve / hygiene; needs explicit ADR + SPS archive **[axis 16]**, **[axis 06]**, **[axis 13]**.

### Tier 1 — Durability, idempotency, observability (HIGH)

12. **In-memory repos and auth maps by default** — restart wipes coordination; challenges/sessions unbounded without TTL sweep story **[axis 01]**, **[axis 07]**, **[axis 09]**, **[axis 11]**.
13. **Non-atomic broadcast + missing RPC timeouts** — partial states, stuck proposals, retry ambiguity **[axis 07]**, **[axis 11]**.
14. **Race: duplicate approval / quorum transition** — check-then-act patterns; needs repo-level locking/versioning **[axis 07]**, **[axis 08]**, **[axis 10]**.
15. **`u64` / JSON number precision** — large `seq_no` hypothesis risk **[axis 04]**.
16. **Error / logging / correlation** — generic 500 body, no request ID in JSON, weak on-call story **[axis 11]**, **[axis 05]**.
17. **SSZ / action validation timing** — garbage at create, fail late at broadcast **[axis 06]**, **[axis 08]**, **[axis 14]** (codec comment vs `signing.rs` imports).
18. **BIP-137 / Trezor path fragility** — recovery header normalization **[axis 11]** (if still present; confirm in code when fixing).

### Tier 2 — Maintainability, docs, process (MEDIUM)

19. **`AppState` god-object, anemic `Proposal`, no append-only audit trail** **[axis 06]**, **[axis 08]**, **[axis 09]**.
20. **Diataxis collapse, missing runbooks, no ADR-006-style boundary doc** **[axis 15]**.
21. **No DoR / weak AC on US-H5 manual fallback, HW parity, concurrent cancel** **[axis 13]**, **[axis 12]**.
22. **Discovery evidence gap** (interviews, tabletop offline) **[axis 12]**.
23. **Agent/skill drift and weak auto-trigger metadata** **[axis 17]**.
24. **ADR-001 pinning narrative incomplete** **[axis 16]**.
25. **Mock URL heuristics in production code paths** **[axis 06]**, **[axis 14]**.
26. **Hygiene:** vestigial features, handler thinness inconsistency **[axis 14]**.

---

## Cross-cutting themes (merged)

- **Orchestrator is not the source of truth for broadcast** — Desktop goes around it; backend atomicity and auditability never engage on the happy path **[axis 04]**, **[axis 02]**.
- **“True” UI state is fabricated** — Literal `"enacted"` strings from Tauri, not projections of DB or chain **[axis 04]**, **[axis 02]**.
- **Same defect class, many names** — Authority leak, operator key, in-memory, CSP, and IPC validation appear across **01–05, 08–11, 13–14**; count **merged defects**, not raw bullet counts.
- **Single SSOT broken** — Five authorities in backend vs narrow desktop enum vs two React roles; errors typed in BE, stringly in desktop **[axis 04]**, **[axis 08]**, **[axis 14]**.
- **No golden thread from PRD → story AC → test → metric** for signer safety and fallback **[axis 12]**, **[axis 13]**, **[axis 10]**, **[axis 15]**.
- **Synthesized risk [axis 04 + 07 + 09]:** shipping Postgres + horizontal scale **without** fixing broadcast routing and repo races **increases** blast radius versus today’s accidental serialization.

---

## Disagreements between axes (decision rules)

| Topic | Conflict | Rule |
| --- | --- | --- |
| Backend threshold / approve | **01** (coordination framing) vs **16** (PRD §1 “no threshold checks”) vs **06** (wants early validity) | **Hygiene** in backend is OK; **canonical quorum / SPS-65 validity** is on-chain unless **ADR-006** explicitly documents an advisory carve-out + tests **[axis 16]**, **[axis 06]**, **[axis 01]**. |
| `list_proposals` severity | **06** sometimes MEDIUM vs **01** BLOCKER | **Stronger evidence wins** — handler paths cited in **[axis 01]** → **BLOCKER**. |
| Manual fallback | **15** (missing runbook) vs **12–13** (product/process REJECTED-style gap) | Treat as **Tier 0 product** once org accepts PRD §2 literally; until AC exist, **Tier 1** with explicit **research gate** **[axis 12]**, **[axis 13]**, **[axis 15]**. |
| API versioning | **2026-05-13** “none” vs **2026-05-14 [axis 04]** `/api/v1` | **Retract “no versioning”**; keep **contract drift** (errors, enums, camelCase) as the live risk **[axis 04]**. |

---

## Confidence table (abbrev.)

| Claim | Conf. | Why |
| --- | --- | --- |
| Broadcast bypass + hard-coded DTO | **High** | **[axis 04]** gives end-to-end file:line chain; **[axis 02]** independently cites same commands. |
| Authority read leak + broadcast `_auth` | **High** | **[axis 01]** + **[axis 06]** align on handlers. |
| Operator key default | **High** | **`config.rs`** cited in **01, 05, 14**. |
| CSP null | **High** | **`tauri.conf.json`** in **02, 05**. |
| Case `==` vs `eq_ignore_ascii_case` | **High** | Same function pair in **[axis 04]**. |
| Postgres races amplified | **Med** | Hypothesis until `postgres_repo.rs` locking audited end-to-end **[axis 04 OQ-style]**, **[axis 07]**. |
| BIP-137 gap | **Med** | Confirm current `broadcast_tx` before fix **[axis 11]**. |

---

## What we might still be wrong about

- **Deploy overlays** may forbid missing `DATABASE_URL` / test operator key—code alone does not prove production config **[axis 01]**, **[axis 05]**.
- **SPS-65 / Notion** excerpts still not in-repo; “coordination only” interpretation may shift with Alpen legal-of-record **[axis 16]**.
- **RwLock** may mask races today; **Postgres** path may already use `SELECT FOR UPDATE` in places not re-read this sprint **[axis 07]**.
- **Composer-vs-Opus** depth: some axes note **missing optional skill paths**; no second-pass reviewer cross-check **[axis 01 recovery note]**, **[axis 17]**.

---

## Smallest fixes vs largest bets (org-level)

**Smallest (days):** Route **`broadcastProposal`** through orchestrator **`execute_broadcast`** (or shared library that **must** call it); delete hard-coded status strings—**reload** proposal from server after broadcast; **normalize pubkey case** at ingress + **single dedup rule**; **authority-filter** list/get; **401 not 404** on wrong authority; **strict CSP**; **fail startup** without operator key / DB in prod-shaped profiles; **fix broadcast screen authority label** **[axis 04]**, **[axis 02]**, **[axis 01]**, **[axis 13]**.

**Largest (weeks+):** **Shared `multisig-types` + codegen**; **append-only event log** and **resumable broadcast FSM**; **secrets off IPC** (keychain / sidecar); **signed releases + SCA**; **ADR-006** + **SPS archive**; **DoR + US-H5 AC** + **discovery program** **[axis 14]**, **[axis 08]**, **[axis 09]**, **[axis 05]**, **[axis 16]**, **[axis 12]**, **[axis 13]**, **[axis 15]**.

---

## Suggested sequencing (textual graph)

`P0-fix-broadcast-routing → P0-remove-fake-DTO → P0-case-normalize → P0-authority-filter reads → P0-broadcast-authz → P1-mandatory-DB prod → P1-RPC-timeouts → P1-structured-errors`  
Parallel when possible: **`Track A`** CSP + secrets off IPC **`[02,05]`** || **`Track B`** backend authz + case normalize **`[01,04]`** || **`Track C`** FE Zod + session guards **`[03,04]`**.

---

## Axis index (2026-05-14)

| Axis | File | One-line top signal |
| --- | --- | --- |
| 01 | `01-rust-backend-adversarial.md` | Unscoped list/get + broadcast authz gap + operator key default + in-memory fallback. |
| 02 | `02-rust-tauri-adversarial.md` | Fake broadcast status strings + operator key on IPC + CSP null + broad `invoke` surface. |
| 03 | `03-react-typescript-adversarial.md` | `authorityFromRole` default footgun + typed trust of IPC + string errors. |
| 04 | `04-cross-cutting-drift-adversarial.md` | **Broadcast bypasses backend** + hard-coded DTO + case dedup bug + authority enum subset + error collapse. |
| 05 | `05-platform-cicd-observability-adversarial.md` | No audit/deny story, unsigned release, observability gaps, CSP/build gaps. |
| 06 | `06-application-architecture-adversarial.md` | Handler vs ADR promises; `AppState` coupling; validation timing. |
| 07 | `07-distributed-systems-adversarial.md` | Durability, partial broadcast, races, timeouts, scale assumptions. |
| 08 | `08-domain-ddd-adversarial.md` | Anemic aggregate; context map debt; late protocol validation. |
| 09 | `09-data-engineering-adversarial.md` | Implicit schema; TEXT status; no encryption-at-rest narrative. |
| 10 | `10-testing-strategy-adversarial.md` | In-memory-only HTTP tests; no FE runner; IPC/broadcast negatives thin. |
| 11 | `11-troubleshooting-failure-modes-adversarial.md` | Generic client errors; weak correlation; stuck-state ops pain. |
| 12 | `12-product-discovery-assumptions-adversarial.md` | PRD promises vs story-map deferrals; evidence gaps. |
| 13 | `13-product-owner-requirements-adversarial.md` | US-H5 / DoR / concurrent-sign AC gaps; **wrong broadcast authority label**. |
| 14 | `14-diverge-options-coherence-adversarial.md` | `AppError` vs `String`; codec invariant violated; duplicate `Authority`; mocks. |
| 15 | `15-documentation-diataxis-adversarial.md` | Runbooks, boundary ADR, Diataxis collapse. |
| 16 | `16-research-sources-adversarial.md` | PRD vs code on threshold; weak SPS pinning in-repo. |
| 17 | `17-agent-spec-quality-adversarial.md` | Rule fork `.claude` / `.cursor`; optional audits not enforced. |

---

*End of rollup. For meta-review of these documents (quality of the assessment set), generate a separate `99-meta-review.md` if desired.*
