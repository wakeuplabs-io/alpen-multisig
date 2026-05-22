# Review of Assessment Documents (Meta-Review)

**Date:** 2026-05-13  
**Scope:** Quality review of the 18 adversarial assessment documents in `docs/assessment/2026-05-13-adversarial/`. NOT a second repo audit — claims here trace back to specific axis findings.  
**Convention:** Inline citations `[NN]` reference axis files `NN-*-adversarial.md`. Findings synthesized by this meta-review that no single axis named are flagged "Synthesized — see [N, M]".

---

## Inputs used

All 18 inputs present and read in full. No missing files.

| File | Read | Notes |
|---|---|---|
| `00-summary-adversarial.md` | ✅ | Already consolidates Tier 0/1/2 with cross-axis citations and a confidence table |
| `01-rust-backend-adversarial.md` | ✅ | F1–F12, evidence index, attack narratives |
| `02-rust-tauri-adversarial.md` | ✅ | D1–D11, attack narratives A–E |
| `03-react-typescript-adversarial.md` | ✅ | D1–D12, scenarios 1–6 |
| `04-cross-cutting-drift-adversarial.md` | ✅ | 10 findings, 6 narratives |
| `05-platform-cicd-observability-adversarial.md` | ✅ | 5 Blockers + 5 High + Mediums, 6 narratives |
| `06-application-architecture-adversarial.md` | ✅ | 12 findings, ADR drift, 6 narratives |
| `07-distributed-systems-adversarial.md` | ✅ | 11 findings, 5 narratives, RPC dependency table |
| `08-domain-ddd-adversarial.md` | ✅ | DDD/aggregate/event findings, 5 narratives |
| `09-data-engineering-adversarial.md` | ✅ | 12 findings, 5 narratives |
| `10-testing-strategy-adversarial.md` | ✅ | B1–B3, H1–H3, M1–M2, L1–L2; clear test gaps |
| `11-troubleshooting-failure-modes-adversarial.md` | ✅ | 10 findings, 5 incident-style narratives |
| `12-product-discovery-assumptions-adversarial.md` | ✅ | 9 findings, 5 narratives; "REJECTED" verdict |
| `13-product-owner-requirements-adversarial.md` | ✅ | 14 findings, 6 narratives; "REJECTED_PENDING_REVISIONS" |
| `14-diverge-options-coherence-adversarial.md` | ✅ | 11 findings, 6 narratives |
| `15-documentation-diataxis-adversarial.md` | ✅ | 10 findings, 5 narratives |
| `16-research-sources-adversarial.md` | ✅ | 5 findings, 5 narratives; SPS-65 source-chain audit |
| `17-agent-spec-quality-adversarial.md` | ✅ | 10 findings, 6 narratives; `.claude/` vs `.cursor/` drift |

---

## Executive summary

- **Convergence is strong on Tier 0.** All 18 axes independently surface 5 themes: operator-key default, authority leakage, in-memory durability, CSP-off + plaintext IPC secrets, and payload divergence on the hardware wallet. Severity is high-confidence.
- **Synthesizer `[00]` did the heavy lifting** — Tier 0/1/2 buckets, confidence table, and contradiction list. This meta-review accepts most of that ordering and adds traceable IDs (P-###) and ownership.
- **Documentation quality is high but rubric inconsistent.** Each axis has scope/threat model, ranked findings, attack narratives, evidence index, smallest-fix-vs-largest-bet, and "what would change my mind" — but severity vocabulary (BLOCKING vs CRITICAL vs BLOCKER, B1–B3 vs F1–F12 vs P-#-style) varies per axis.
- **Effort / change-risk classifications are missing.** No axis tags items S/M/L or low/med/high change-risk; this meta-review supplies them.
- **Fact / hypothesis / open-question taxonomy is implicit.** Axis 04 marks `HYPOTHESIS`; others don't. Several Tier 0 items are code-read severity, not measured (no probe was run) — flagged explicitly in `[00] §What we might still be wrong about`.
- **Two real contradictions need a decision rule** (see "Contradictions" below): backend's role in threshold/sighash validation, and `list_proposals` authority severity. The latter is already reconciled in `[00]` (BLOCKER wins on evidence).
- **Cross-cutting risk that no single axis fully owns:** the *combination* of in-memory state + no rate limit + no observability + no idempotency + in-memory broadcast claim becomes catastrophic the moment Postgres replaces the global `RwLock` (Synthesized — see [04, 07, 08, 09]).
- **Process gaps to fix.** Findings are duplicated (operator key surfaces in 6 axes; authority leak in 4). De-dup conventions, severity calibration, and a parallelization map for re-runs are absent.
- **The agent/skill drift `[17]` is the only finding with a plausible "this is intentional" defense.** `.cursor/rules/` may be Cursor IDE artifacts; this should be confirmed before forcing a delete.
- **Production-readiness window: 4–6 weeks for Tier 0 closure, ~1 quarter for full hardening** — consistent with `[00]`'s estimate.

---

## Document quality review

### Strengths (what worked)

- **Common structure across axes.** Scope/threat model → ranked findings → attack narratives → evidence index → smallest fix vs largest bet → what would change my mind. Reviewers can navigate in seconds.
- **High evidence density.** Most Blocker/High findings cite `file.rs:lineNN–MM` with verbatim code blocks. Strongest examples: `[01]`, `[02]`, `[03]`, `[06]`, `[09]`.
- **Adversarial "what would change my mind" sections.** `[01]`, `[02]`, `[03]`, `[09]`, `[11]`, `[14]`, `[16]` explicitly say "if X is true, I'd downgrade Y". This is the single best practice in the set; it must be required on every axis.
- **Attack narratives are concrete.** `[01]` Narrative 2 (test operator key), `[07]` Narrative 4 (LB + in-memory sessions), `[11]` Narrative 1 (Sev-2 broadcast stuck) are end-to-end and reproducible.
- **`[00]` is a model rollup.** Confidence table, contradiction table, disagreement-between-axes table, and a 2/6/12-week sequencing. Most of this meta-review accepts its ordering.
- **`[12]` and `[13]` are the only axes that take a stand-or-reject verdict** ("REJECTED_PENDING_REVISIONS"). That's appropriate for discovery/PO gates.
- **`[16]` is uniquely valuable** — it audits not the code but the *claim chain*. Treat it as a template for future audits.

### Weaknesses (process + writing issues)

- **Severity vocabulary is inconsistent.** `BLOCKER`, `BLOCKING`, `CRITICAL`, `🔴`, `Blocker` all appear. No single legend.
- **No effort or change-risk classification.** Reader has to invent S/M/L.
- **Fact / hypothesis / open-question tags are inconsistent.** Only `[04]` and `[12]` mark HYPOTHESIS explicitly.
- **Duplicate findings inflate the perceived count.** Operator-key default appears in [01, 02, 05, 11, 14, 16]; in-memory storage in [01, 05, 07, 09, 11]; authority leak in [01, 03, 06, 08]. Without `[00]` consolidating, the backlog looks like 200+ items.
- **No required citation format.** Some axes use `file.rs:NN–MM`; others use `(implied; not fully read)`. The latter is unacceptable for Blocker claims (`[02]` D8/D11, `[09]` #8, `[15]` ADR cross-refs) and silently lowers confidence.
- **Attack narratives sometimes drift speculative.** `[04]` Narrative 5 (status enum rollout breaking expirations) is plausible but unevidenced; `[12]` Narrative 3 (signer fatigue at scale) is hypothesis.
- **Disconfirming probes proposed but not executed.** Every axis lists probes (`[01]` lists 5); none were run in this engagement. `[00]` explicitly flags this.
- **`[10]` references B1/H1/M1 IDs that don't tie back to a global ID scheme** — no way to know that `[10]` B1 ≅ `[07]` #4/#5 without manual matching.
- **`[15]` and `[17]` cite files they didn't fully read** (`.claude/skills/sdd/SKILL.md` line numbers in `[17]`; `docs/3-stories/README.md` in `[13]`). Plausible but reduces evidence weight.
- **No axis has a Haiku/cheap reviewer pass.** `[00]` is the only consolidation; reviewer-reviewer pairs would catch the contradictions earlier.
- **`[16]` is the strongest evidence chain, but its central claim depends on a Notion document not in the repo.** This is itself a finding (P-055), but it weakens the contradiction in §"Disagreements".

### Contradictions / duplicates to reconcile

| Axes in conflict | What they disagree on | Conflict in one sentence | Decision rule |
|---|---|---|---|
| `[01]` F11 (LOW, by design) vs `[06]` #1 (BLOCKING) vs `[16]` #1 (BLOCKING — backend over-reaches) | Should backend validate signatures / detect quorum? | Three positions: do nothing, fully validate, or "you're already doing too much". | **Rule:** Hygiene checks (compact ECDSA format, 64-byte length, lowercase hex) belong in backend. Canonical SPS-65 sighash validity and threshold enforcement belong on-chain. The current auto-Approve-on-threshold transition is over-reach — either remove it or ADR-006 must explicitly carve out "advisory quorum detection" with explicit threshold-resync requirement. `[16]` wins on evidence chain (PRD §1 cited verbatim). |
| `[01]` F1 (BLOCKER) vs `[06]` #7 (MEDIUM) | Severity of `list_proposals` authority leak | Cross-authority enumeration is either critical-confidentiality or polish. | **Rule:** Evidence-based — `[01]` cites exact handler, missing filter, and PRD §3.2. Severity = BLOCKER. (Already chosen in `[00]`.) |
| `[01]` F5 (HIGH) vs `[05]`/`[07]`/`[09]` (BLOCKING/CRITICAL) | In-memory durability severity | Same finding, four severities. | **Rule:** Majority + PRD §2.3 invariant → BLOCKING. `[01]` understates. |
| `[01]` F4 (`parking_lot`) vs `[02]` D2 (`zeroize`) | RwLock fix | Both correct, address different threats. | **Rule:** Both required; treat as two sub-fixes under one P-###. |
| `[12]` & `[13]` (manual fallback unfeasible) vs `[15]` #1 (just a missing ops doc) | Severity of missing offline fallback | UX-blocking risk vs documentation gap. | **Rule:** Once user research lands and proves doc isn't enough, escalate to Tier 0. Until then, treat as Tier 1 with research-gate. |
| ADR-005 ("authority extraction not needed yet") vs `[04]`/`[06]`/`[08]`/`[14]` | Urgency of shared `multisig-types` crate | "yet" is doing work; 4 axes say "before next slice". | **Rule:** ADRs are revisable. The 4-axis convergence overrides ADR-005's deferral; require shared crate before Slice 2 (PayoutAdmin). |
| `[17]` (`.cursor/rules/` is drift) vs unaddressed possibility (`.cursor/rules/` is IDE artifact) | Whether `.cursor/rules/` should be deleted | Could be intentional caching by Cursor IDE itself. | **Rule:** Verify Cursor IDE loading semantics with a 30-min experiment before deleting. If IDE writes those files automatically, document instead of deleting. |

**Duplicates already merged in `[00]` (this meta-review accepts):** operator key (6 axes → P-001); in-memory storage (5 axes → P-016); authority leak (4 axes → P-002); IPC type drift (3 axes → P-022); CSP-off + supply chain (3 axes → P-004 + P-011); broadcast atomicity (4 axes → P-018); HW-wallet payload divergence (5 axes → P-006).

---

## Consolidated improvement backlog (PRODUCT/REPO)

Ordered by severity, then effort (S before L). Where a finding appears in multiple axes, it is listed once with all axis sources cited.

### Blockers (P-001 to P-015)

- **ID**: P-001
- **Title**: Operator secret key defaults to publicly-known test value `0x00...01`
- **Severity**: Blocker
- **Axis source(s)**: `[01]` F2, `[02]` D4, `[05]` BLOCKER-003, `[11]` #8, `[14]` H4, `[16]` #5
- **Evidence**: `orchestrator-be/src/config.rs:56–61`; frontend leak path `desktop-app/src/vite-env.d.ts:9`; IPC arg `desktop-app/src-tauri/src/commands/proposals.rs:81,169`
- **User impact / risk**: Attacker who observes the commit tx can forge a competing reveal with the test key and steal the commit UTXO; same key potentially leaks via sourcemaps.
- **Proposed change**: Remove `unwrap_or_else` for `OPERATOR_SECRET_KEY_HEX`; refuse startup if env var missing or equals the literal test value; remove `VITE_OPERATOR_SECRET_KEY_HEX` and IPC arg; load operator key only inside Rust at startup.
- **Effort**: S
- **Change risk**: low
- **Verification**: Test `test_startup_fails_without_operator_key`; test `test_startup_rejects_test_key_value`; grep CI gate confirming `VITE_OPERATOR_SECRET_KEY_HEX` is unused.
- **Suggested owner**: BE + TAURI

- **ID**: P-002
- **Title**: Authority-scope leakage across read + broadcast endpoints (`_auth` discarded)
- **Severity**: Blocker
- **Axis source(s)**: `[01]` F1/F3/F9, `[03]` D2, `[06]` #7, `[08]`
- **Evidence**: `orchestrator-be/src/handlers/proposals.rs:108–126,156–212`; `src/application/traits.rs:26`; `src/application/proposals.rs:132–137`
- **User impact / risk**: Any authenticated signer enumerates and reads every other authority's proposals; violates PRD §3.2.
- **Proposed change**: Add `authority` filter to `list_by_status` and `find_by_action_id`; return 401 (not 404) on authority mismatch in `get_proposal`; check `auth.authority == proposal.authority` in `prepare_broadcast` and `execute_broadcast`.
- **Effort**: S
- **Change risk**: low
- **Verification**: Integration test: Strata-Admin token cannot list/get/broadcast Alpen-Admin proposals.
- **Suggested owner**: BE

- **ID**: P-003
- **Title**: Plaintext secrets across the Tauri IPC boundary; no `zeroize`
- **Severity**: Blocker
- **Axis source(s)**: `[02]` D1/D2/D4, `[05]` BLOCKER-003/MEDIUM-002
- **Evidence**: `desktop-app/src-tauri/src/commands/signing.rs:22–27,43–55`; `commands/proposals.rs:74–88`; `application/orchestrator_auth.rs:1–20,48–64`
- **User impact / risk**: Compromised React dep or XSS exfiltrates mnemonics, operator keys, bearer tokens in clear; no memory zeroization on drop.
- **Proposed change**: Never accept raw mnemonics/secrets from webview — accept derivation indices only and load from OS keychain; wrap sensitive fields in `ZeroizeOnDrop`.
- **Effort**: M
- **Change risk**: med
- **Verification**: IPC-fuzzer test confirms no plaintext key flows to commands; manual memory-dump test on a built binary.
- **Suggested owner**: TAURI

- **ID**: P-004
- **Title**: Tauri CSP disabled (`"csp": null`)
- **Severity**: Blocker
- **Axis source(s)**: `[02]` D3, `[05]` BLOCKER-005
- **Evidence**: `desktop-app/src-tauri/tauri.conf.json:21–23`
- **User impact / risk**: Any XSS or compromised npm dep runs with full `__TAURI__.invoke()` access including signing/broadcast.
- **Proposed change**: Set `"default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self' <orchestrator-url>"`; add SRI for bundled JS; configure Tauri 2 capabilities per window.
- **Effort**: S
- **Change risk**: low
- **Verification**: Test that injected `<script>` cannot reach Tauri invoke; smoke-test all flows after CSP enable.
- **Suggested owner**: TAURI + FE

- **ID**: P-005
- **Title**: No client-side validation that returned proposal matches what user submitted (MITM / malicious backend)
- **Severity**: Blocker
- **Axis source(s)**: `[02]` D5, `[03]` D1/D6
- **Evidence**: `desktop-app/src-tauri/src/infrastructure/orchestrator_client.rs:103–113`; `application/proposals.rs:60–101`
- **User impact / risk**: Compromised backend can substitute a different action_hex; signer broadcasts wrong proposal with operator key.
- **Proposed change**: Before signing/broadcast, hash the user's submitted action and compare against returned proposal; re-display + require explicit confirmation. Largest bet: signed-proposal envelopes.
- **Effort**: M
- **Change risk**: med
- **Verification**: Test with proxy returning altered proposal — broadcast must refuse.
- **Suggested owner**: TAURI + FE

- **ID**: P-006
- **Title**: Hardware wallet payload divergence — signer cannot verify authority on-device
- **Severity**: Blocker
- **Axis source(s)**: `[02]` D5, `[03]` D5, `[12]` (digest-test), `[13]` #2, `[15]` #3, `[16]`
- **Evidence**: `desktop-app/src/domain/sign-proposal/components/sign-proposal-view.tsx:43–133`; no `docs/specs/sps65-signing-visualization.md`
- **User impact / risk**: Trezor only shows 32-byte sighash; signer cannot verify authority/action semantics; classic multisig payload-divergence footgun.
- **Proposed change**: Short-term: prominent UI warning naming the authority + freeze-preview gate (see P-007). Long-term: protocol-level addition of authority/tag bytes inside signed payload + Trezor firmware integration; write `docs/specs/sps65-signing-visualization.md`.
- **Effort**: L
- **Change risk**: med
- **Verification**: User test (3 signers, real Trezor): can they spot a swapped authority in <1min? Spec acceptance test.
- **Suggested owner**: FE + ARCH (protocol coord with Alpen)

- **ID**: P-007
- **Title**: Sighash swap between preview and sign in `create-proposal-form.tsx`
- **Severity**: Blocker
- **Axis source(s)**: `[03]` D4, `[13]` #2
- **Evidence**: `desktop-app/src/domain/create-proposal/components/create-proposal-form.tsx:124–138`; `hooks/use-create-proposal.ts:110–145`
- **User impact / risk**: Edited form values recompute a different sighash without re-displaying — signer approves an unintended proposal.
- **Proposed change**: Freeze form values at preview; deep-compare on submit; force re-preview if anything changed.
- **Effort**: S
- **Change risk**: low
- **Verification**: Component test: edit-after-preview-then-submit triggers re-preview gate.
- **Suggested owner**: FE

- **ID**: P-008
- **Title**: No runtime validation of IPC results at the Tauri bridge
- **Severity**: Blocker
- **Axis source(s)**: `[03]` D1, `[04]` BLOCKER-1/2, `[10]` B3
- **Evidence**: `desktop-app/src/api/tauri-bridge.ts:11–17`
- **User impact / risk**: Backend (or compromised IPC) can return unknown status variants, fabricated signatures, mis-cased enums; UI renders them as valid.
- **Proposed change**: Add Zod schemas at `tauriCall` for `Proposal`, `ProposalStatus`, `BroadcastStatus`, `AuthSession`. Largest bet: codegen TS types from Rust serde (or shared `multisig-types` crate); CI contract tests.
- **Effort**: M
- **Change risk**: low
- **Verification**: Unit test: bridge rejects unknown enum variants; e2e contract test on every IPC command.
- **Suggested owner**: FE + TAURI

- **ID**: P-009
- **Title**: Session token has no authority binding; cross-authority token reuse
- **Severity**: Blocker
- **Axis source(s)**: `[03]` D2/D3, `[04]` BLOCKER-1, `[13]` #4
- **Evidence**: `desktop-app/src/api/orchestrator-auth.ts:13–18`; `contexts/session-provider.tsx:34–62`; `contexts/auth-session-provider.tsx:35–41` (no await on `authLogout`)
- **User impact / risk**: Stale session reused for wrong authority; race window during role switch.
- **Proposed change**: Validate `session.authority === authorityFromRole(selectedRole)` before reuse; `await authLogout()` before re-auth. Largest bet: signed JWT carrying authority claim, short access + refresh tokens.
- **Effort**: S
- **Change risk**: low
- **Verification**: Test: switching roles purges old token before new auth; cross-authority `listProposals` after role-switch returns 401.
- **Suggested owner**: FE + BE

- **ID**: P-010
- **Title**: Deep-link `/proposals/:actionId/sign` bypasses authority context
- **Severity**: Blocker
- **Axis source(s)**: `[03]` D6, `[13]` #4
- **Evidence**: `desktop-app/src/screens/sign-poc-screen.tsx`; `App.tsx:57–63`
- **User impact / risk**: Attacker-supplied link makes the app prompt to sign a proposal belonging to a different authority.
- **Proposed change**: Refuse to render sign screen unless `proposal.authority === authorityFromRole(selectedRole)`. Largest bet: require navigation provenance from dashboard.
- **Effort**: S
- **Change risk**: low
- **Verification**: Route guard test for authority mismatch.
- **Suggested owner**: FE

- **ID**: P-011
- **Title**: No release signing, no SCA, no `package-lock.json`, unsigned git-rev pinned Alpen crates
- **Severity**: Blocker
- **Axis source(s)**: `[05]` BLOCKER-001/004 + HIGH-001, `[02]` D3
- **Evidence**: `.github/workflows/ci.yml` (no release job, no `cargo audit`/`cargo deny`); `Cargo.toml:10–21` (git rev pins); `desktop-app/package.json:14–41` (no lockfile committed); ADR-004:57
- **User impact / risk**: A compromised transitive dep is shipped to signers as an unsigned binary.
- **Proposed change**: Commit `package-lock.json`; `npm ci`; add `cargo audit` + `cargo deny` (do not skip pre-release warnings via allowlist); `detect-secrets` pre-commit. Larger: signed `release.yml`, Apple/Authenticode signing, PGP checksum manifest, Tauri updater verification, PRD NF-3 multi-employee binary signing.
- **Effort**: L
- **Change risk**: med
- **Verification**: CI fails on missing lockfile or new audit finding; release artifact carries verifiable signature.
- **Suggested owner**: PLATFORM

- **ID**: P-012
- **Title**: Backend re-implements protocol validity rules — auto-Approve on threshold; stale `required_signatures` snapshot
- **Severity**: Blocker
- **Axis source(s)**: `[16]` #1/#2, `[06]` #1, `[13]` #5
- **Evidence**: `orchestrator-be/src/application/proposals.rs:103–104` + test at 557–580
- **User impact / risk**: Backend marks Approved → user broadcasts → ASM rejects because on-chain threshold changed. Split-brain governance.
- **Proposed change**: Two paths — (a) remove auto-transition and let on-chain be the source of quorum truth; or (b) write ADR-006 explicitly carving out "advisory" quorum detection + threshold-resync test before broadcast. Either way, add forbidden-import lint on `strata-crypto`/`strata-asm-params` in `orchestrator-be/Cargo.toml`.
- **Effort**: M
- **Change risk**: high
- **Verification**: Test: threshold changes mid-proposal — broadcast refuses or warns. ADR-006 reviewed by Alpen.
- **Suggested owner**: ARCH + BE

- **ID**: P-013
- **Title**: Default network is `regtest`
- **Severity**: Blocker
- **Axis source(s)**: `[02]` D6
- **Evidence**: `desktop-app/src-tauri/src/commands/proposals.rs:158`
- **User impact / risk**: A frontend bug or missing param silently broadcasts on the wrong network with the operator key.
- **Proposed change**: Remove default; require explicit `bitcoin`/`testnet`/`signet`/`regtest` and fail otherwise.
- **Effort**: S
- **Change risk**: low
- **Verification**: Test: omitting `network` returns error.
- **Suggested owner**: TAURI

- **ID**: P-014
- **Title**: Bearer token transported over user-supplied `base_url`; no HTTPS enforcement
- **Severity**: Blocker
- **Axis source(s)**: `[02]` D7
- **Evidence**: `desktop-app/src-tauri/src/infrastructure/orchestrator_client.rs:30–41`; `commands/proposals.rs:190`
- **User impact / risk**: Anyone passing `http://…` ships the bearer in clear.
- **Proposed change**: Reject non-`https://` in `build_client` (allow `http://localhost` only in dev). Largest bet: cert pinning.
- **Effort**: S
- **Change risk**: low
- **Verification**: Test: `http://` external URL rejected.
- **Suggested owner**: TAURI

- **ID**: P-015
- **Title**: Frontend `VITE_OPERATOR_SECRET_KEY_HEX` env path can leak via sourcemaps
- **Severity**: Blocker
- **Axis source(s)**: `[05]` BLOCKER-003/MEDIUM-002, `[02]` D4
- **Evidence**: `desktop-app/src/vite-env.d.ts:9`; `desktop-app/src/domain/broadcast-proposal/hooks/use-broadcast-proposal.ts:20`
- **User impact / risk**: Operator key in frontend bundle / sourcemaps / Sentry breadcrumbs.
- **Proposed change**: Delete the env var path; load operator key only in Rust at startup; strip sourcemaps from production builds.
- **Effort**: S
- **Change risk**: low
- **Verification**: Bundle audit (`source-map-explorer`) confirms no operator key reference.
- **Suggested owner**: FE + PLATFORM

### High (P-016 to P-040)

- **ID**: P-016
- **Title**: In-memory storage is silent default when `DATABASE_URL` unset
- **Severity**: High
- **Axis source(s)**: `[01]` F5, `[05]` BLOCKER-002, `[07]` #1, `[09]` #1, `[11]` #4
- **Evidence**: `orchestrator-be/src/main.rs:90–104`; `infrastructure/memory_repo.rs`
- **User impact / risk**: Crash/restart wipes all in-flight proposals; PRD §2.3 offline-fallback is violated because signers don't know what they were signing.
- **Proposed change**: Fail startup if `DATABASE_URL` missing in production mode. Larger: durable append-only event log (see P-031).
- **Effort**: S
- **Change risk**: low
- **Verification**: Test: omit `DATABASE_URL` in `ENVIRONMENT=production` — startup fails.
- **Suggested owner**: BE

- **ID**: P-017
- **Title**: Auth challenges/sessions in `Arc<RwLock<HashMap>>` — no persistence, no TTL cleanup, no rate limit, no distributed store
- **Severity**: High
- **Axis source(s)**: `[01]` F4/F7, `[02]` D2, `[07]` #3/#6, `[09]` #4, `[11]` #7
- **Evidence**: `orchestrator-be/src/state.rs:15–16`; `handlers/auth.rs:67–71,94–97,142–146,164–167`
- **User impact / risk**: Unbounded memory under challenge spam; sessions vanish on restart; multi-instance load balancing fails.
- **Proposed change**: TTL cleanup task + `tower-governor` rate limit + `parking_lot::RwLock` (no poison). Larger: Postgres- or Redis-backed sessions.
- **Effort**: M
- **Change risk**: low
- **Verification**: Load test: 1000 challenges/s for 60s — memory stable. Restart test: sessions survive.
- **Suggested owner**: BE

- **ID**: P-018
- **Title**: Broadcast is non-atomic; partial state corrupts proposals; no admin reset
- **Severity**: High
- **Axis source(s)**: `[01]` F8, `[06]` #8, `[07]` #2/#10, `[11]` #4
- **Evidence**: `orchestrator-be/src/application/proposals.rs:234–289` (no transaction wrap, no RPC timeout); no `/reset-broadcast` endpoint
- **User impact / risk**: Panic or RPC timeout strands proposal with commit UTXO on-chain and `broadcast_status` desynced.
- **Proposed change**: `tokio::time::timeout` on every BTC/ASM RPC call; add admin reset endpoint. Larger: durable resumable broadcast state machine + Postgres advisory lock.
- **Effort**: M
- **Change risk**: med
- **Verification**: Chaos test: kill backend after `claim_broadcast`, verify recovery procedure.
- **Suggested owner**: BE

- **ID**: P-019
- **Title**: Duplicate-signer race + non-linearized quorum transition
- **Severity**: High
- **Axis source(s)**: `[07]` #4/#5, `[08]` (race), `[10]` B1
- **Evidence**: `orchestrator-be/src/application/proposals.rs:87–119`; `infrastructure/memory_repo.rs:47–65,96–97`
- **User impact / risk**: Two retries inflate signatures; quorum transitions twice; on-chain ASM rejects "insufficient unique signers".
- **Proposed change**: Move duplicate detection inside `add_signature` under write lock; return `(proposal, quorum_reached)` and own transition in repo. Larger: optimistic locking with `version: u64` CAS.
- **Effort**: M
- **Change risk**: med
- **Verification**: Concurrent test (axis-10 B1) — assert exactly N unique sigs after N concurrent requests from same signer.
- **Suggested owner**: BE

- **ID**: P-020
- **Title**: Tauri broadcast has no idempotency / in-flight dedupe / UI button disable
- **Severity**: High
- **Axis source(s)**: `[02]` D10, `[04]` BLOCKER-5, `[07]` #8, `[10]` H1
- **Evidence**: `desktop-app/src-tauri/src/application/proposals.rs:109–229`
- **User impact / risk**: Double-click or retry-after-timeout double-broadcasts; "input already spent" downstream.
- **Proposed change**: Disable Send button while broadcasting; cache action_ids of in-flight broadcasts in Tauri. Larger: `Idempotency-Key` header end-to-end; backend stores `(action_id, idempotency_key) → (commit_txid, reveal_txid)`.
- **Effort**: S
- **Change risk**: low
- **Verification**: Test: double-invoke `proposals_broadcast` returns same result, no double on-chain tx.
- **Suggested owner**: FE + TAURI + BE

- **ID**: P-021
- **Title**: `u64 seq_no` precision loss in JavaScript
- **Severity**: High
- **Axis source(s)**: `[04]` HIGH-4
- **Evidence**: `desktop-app/src/domain/create-proposal/hooks/use-create-proposal.ts:114`
- **User impact / risk**: `seq_no > 2^53−1` is rounded; backend rejects as different proposal.
- **Proposed change**: Serialize `seq_no` as string in JSON; parse via `BigInt` in TS. Larger: codegen-checked numeric types.
- **Effort**: S
- **Change risk**: low
- **Verification**: Test: round-trip `2^60` seq_no.
- **Suggested owner**: BE + FE

- **ID**: P-022
- **Title**: Status/Authority enums travel as opaque strings; no TS enum guard; serialization mismatch (camelCase vs snake_case vs React `strata_administrator`)
- **Severity**: High
- **Axis source(s)**: `[04]` BLOCKER-1/2, `[06]` #10, `[08]`, `[14]` (auth enum dup)
- **Evidence**: `desktop-app/src-tauri/src/domain/proposal.rs:8–21`; `desktop-app/src/types/auth-role.ts:1–4`; `commands/proposals.rs:11–100`
- **User impact / risk**: Backend ships new variant → UI silently mis-renders; 400/401 with no obvious cause.
- **Proposed change**: Branded TS unions for `ProposalStatus`, `BroadcastStatus`, `Authority`; Zod parse at the bridge (pairs with P-008). Larger: shared `multisig-types` codegen (pairs with P-043).
- **Effort**: M
- **Change risk**: med
- **Verification**: Round-trip serde test; e2e for each authority.
- **Suggested owner**: BE + FE

- **ID**: P-023
- **Title**: Error model collapses to `error: string` across the bridge
- **Severity**: High
- **Axis source(s)**: `[04]` BLOCKER-3, `[14]` BLOCKING (desktop error type), `[11]` #2/#3
- **Evidence**: `orchestrator-be/src/error.rs:10–45`; `desktop-app/src-tauri/src/application/orchestrator_client.rs:10–18`; `desktop-app/src/types/index.ts:3`; `desktop-app/src/api/tauri-bridge.ts:11–17`
- **User impact / risk**: UI cannot tell retry-safe from non-idempotent, "device disconnected" from "session expired".
- **Proposed change**: Add `errorCode` discriminant to `ApiResult`; thread HTTP status and Tauri error category through. Larger: typed `DesktopError` enum mirroring `AppError`; per-error UI recovery copy.
- **Effort**: M
- **Change risk**: low
- **Verification**: Test: 409 vs 503 produce different UI behaviour.
- **Suggested owner**: BE + TAURI + FE

- **ID**: P-024
- **Title**: Signer pubkey case-sensitivity mismatch between backend and Tauri
- **Severity**: High
- **Axis source(s)**: `[04]` MEDIUM-6, `[09]` #5
- **Evidence**: `orchestrator-be/src/application/proposals.rs:38–42` (`eq_ignore_ascii_case`) vs `desktop-app/src-tauri/src/application/proposals.rs:87–90` (`==`)
- **User impact / risk**: Mixed-case Trezor pubkey passes one duplicate check and fails the other; quorum inflated.
- **Proposed change**: Normalize hex pubkeys to lowercase at every ingress; DB `CHECK (signer_pubkey ~ '^[a-f0-9]{66}$')`.
- **Effort**: S
- **Change risk**: low
- **Verification**: Test: mixed-case pubkey deduplicated on both sides.
- **Suggested owner**: BE + TAURI

- **ID**: P-025
- **Title**: Mock RPC via URL pattern matching is wired into production code paths
- **Severity**: High
- **Axis source(s)**: `[06]` #5, `[14]` (mock injection), `[08]`
- **Evidence**: `orchestrator-be/src/infrastructure/asm_role_membership.rs:17–19,44–46,63–65,125–170`; same anti-pattern in desktop, hardcoded to StrataAdmin only
- **User impact / risk**: Typo'd env containing "mock" or "localhost" silently authorizes attacker.
- **Proposed change**: Dependency-injected `AsmStateRpc` trait; mocks only in `#[cfg(test)]`.
- **Effort**: M
- **Change risk**: low
- **Verification**: Lint or test forbidding mock paths outside `cfg(test)`.
- **Suggested owner**: BE + TAURI

- **ID**: P-026
- **Title**: No SSZ validation at create-proposal boundary
- **Severity**: High
- **Axis source(s)**: `[06]` #3, `[14]`
- **Evidence**: `orchestrator-be/src/application/proposals.rs:28–64`
- **User impact / risk**: Garbage `action_hex` is stored and only fails at broadcast time — wasted 7-day expiry.
- **Proposed change**: SSZ-decode at create-handler ingress; reject early.
- **Effort**: S
- **Change risk**: low
- **Verification**: Test: malformed `action_hex` returns 400 at POST `/proposals`.
- **Suggested owner**: BE

- **ID**: P-027
- **Title**: No timeouts/retries/backoff on Bitcoin RPC or ASM RPC
- **Severity**: High
- **Axis source(s)**: `[07]` #10, `[11]` narrative 5
- **Evidence**: `orchestrator-be/src/application/proposals.rs:218–219`; `infrastructure/bitcoin_rpc.rs`
- **User impact / risk**: Hung RPC pegs Tokio workers and cascades to service-wide outage.
- **Proposed change**: `tokio::time::timeout` on every external call; structured retries with jitter. Larger: circuit breaker per dependency.
- **Effort**: S
- **Change risk**: low
- **Verification**: Test: mock RPC hangs 60s → request returns 504 in 10s.
- **Suggested owner**: BE

- **ID**: P-028
- **Title**: Strata crates leak from `infrastructure/action_codec.rs` into application layer
- **Severity**: High
- **Axis source(s)**: `[06]` #4, `[14]` (SSZ codec)
- **Evidence**: `desktop-app/src-tauri/src/application/proposals.rs:13` imports `MultisigAction`; `infrastructure/signing.rs:57` calls `MultisigAction::from_ssz_bytes` outside codec
- **User impact / risk**: Application layer breaks directly on upstream Alpen changes; mocking requires duplicate work.
- **Proposed change**: Route all SSZ decode through `action_codec`; enforce via clippy/`deny` lint that only `action_codec.rs` may import `strata_asm_*`/`strata_crypto`.
- **Effort**: M
- **Change risk**: med
- **Verification**: CI lint that fails on forbidden imports.
- **Suggested owner**: TAURI

- **ID**: P-029
- **Title**: No structured logging, no request/correlation IDs, no `/ready` probe, no metrics
- **Severity**: High
- **Axis source(s)**: `[04]` MEDIUM-7, `[05]` MEDIUM-001, `[11]` #1/#3/#10
- **Evidence**: `orchestrator-be/src/main.rs:25–30`; `error.rs:34–39`; `desktop-app/src/api/tauri-bridge.ts:11–17`
- **User impact / risk**: Sev-2 incident takes 15–30 min just to find the right log line.
- **Proposed change**: `#[tracing::instrument]` on every handler with `action_id`/`authority`/`seq_no`; generate request UUID in `tauri-bridge.ts` and surface in error toasts; `/ready` checks Postgres+BTC+ASM. Larger: structured JSON logs + Prometheus + SLOs + runbook.
- **Effort**: M
- **Change risk**: low
- **Verification**: One incident drill — find a request from toast → backend log in <1min.
- **Suggested owner**: BE + FE + PLATFORM

- **ID**: P-030
- **Title**: No HTTP-layer auth-challenge replay defense; only Tauri-layer tested
- **Severity**: High
- **Axis source(s)**: `[09]` #4, `[10]` M1
- **Evidence**: `orchestrator-be/src/handlers/auth.rs`; replay test exists only in Tauri layer
- **User impact / risk**: Replayed `(challenge_id, signature)` at HTTP layer accepted.
- **Proposed change**: Persist consumed-challenge IDs with TTL in Postgres; add backend test for replayed `challenge_id`. Larger: stateless JWTs + nonce ledger.
- **Effort**: S
- **Change risk**: low
- **Verification**: Backend test for replay at HTTP layer.
- **Suggested owner**: BE

- **ID**: P-031
- **Title**: No append-only event/audit log; auditors cannot reconstruct who signed when
- **Severity**: High
- **Axis source(s)**: `[08]`, `[09]` #6, `[11]` #4
- **Evidence**: `proposals` table; `update_broadcast_status` overwrites in place
- **User impact / risk**: Indispensable governance audit trail does not exist.
- **Proposed change**: `proposal_events` table append-only with `event_type` + `data jsonb` + `created_at`. Larger: event sourcing as source of truth with materialized read views (pairs with P-018 resumable FSM).
- **Effort**: L
- **Change risk**: med
- **Verification**: Test: event log replays current state exactly.
- **Suggested owner**: BE

- **ID**: P-032
- **Title**: No frontend tests; no Tauri IPC contract tests; no concurrent-approval test; no broadcast error-path test; signature verification only happy-path
- **Severity**: High
- **Axis source(s)**: `[10]` B1/B2/B3 + H1/H2/H3
- **Evidence**: `find desktop-app/src -name '*.test.*'` empty; no `#[tauri::test]` exists
- **User impact / risk**: Most Tier 0/1 risks cannot be caught by CI.
- **Proposed change**: Add the 3+6+5+6 tests enumerated in axis-10. Larger: e2e harness covering all 5 authorities + real ASM + hardware-wallet smoke.
- **Effort**: L
- **Change risk**: low
- **Verification**: CI coverage on the listed test names.
- **Suggested owner**: BE + FE + TAURI

- **ID**: P-033
- **Title**: BIP-137 recovery-id normalization missing in `broadcast_tx::build_signed_payload_bytes`
- **Severity**: High
- **Axis source(s)**: `[11]` narrative 3
- **Evidence**: `orchestrator-be/src/infrastructure/broadcast_tx.rs:108–115`
- **User impact / risk**: Trezor `signMessage` 65-byte format (header 27–42) misparsed as `recid||r||s`; surfaces as opaque "signature invalid".
- **Proposed change**: Import or implement `normalize_recovery_id` from `strata-crypto`; map to specific error code.
- **Effort**: S
- **Change risk**: med
- **Verification**: Test with real Trezor BSM 65-byte signature.
- **Suggested owner**: BE

- **ID**: P-034
- **Title**: No proposal expiry enforcement
- **Severity**: High
- **Axis source(s)**: `[07]` #11, `[13]` #10
- **Evidence**: `domain/proposal.rs:62–73` (`Expired` exists but never transitioned to)
- **User impact / risk**: A 9-day-old proposal can still be signed and broadcast, only to be rejected on-chain.
- **Proposed change**: Check `now > expires_at` in `approve_action` + broadcast paths. Larger: background expiry job.
- **Effort**: S
- **Change risk**: low
- **Verification**: Test: 8-day-old proposal cannot be approved.
- **Suggested owner**: BE

- **ID**: P-035
- **Title**: Threshold snapshot vs on-chain change mid-proposal
- **Severity**: High
- **Axis source(s)**: `[13]` #5
- **Evidence**: `application/proposals.rs:52` stores `required_signatures` at create-time only
- **User impact / risk**: UI counter and auto-Approved transition can both be wrong in opposite directions. Pairs with P-012.
- **Proposed change**: Refuse to broadcast if backend `required_signatures` doesn't match a freshly-fetched ASM threshold (or document explicitly via ADR-006 that snapshot is binding).
- **Effort**: M
- **Change risk**: med
- **Verification**: Test: threshold changes during pending — broadcast warns/refuses.
- **Suggested owner**: BE + ARCH

- **ID**: P-036
- **Title**: Hardcoded constants duplicated and role-73-only
- **Severity**: High
- **Axis source(s)**: `[02]` D8, `[06]` #9, `[14]` H4
- **Evidence**: `REVEAL_TX_VBYTES = 350`, `COMMIT_DUST_SATS = 1500`, magic bytes `0x414c504e`, derivation path `m/86'/0'/73'/0/*` duplicated backend↔desktop; role 73 is StrataAdmin-only
- **User impact / risk**: Signers on other roles derive wrong key.
- **Proposed change**: Centralize in config; fetch role at auth and pass into `list_mnemonic_addresses`. Larger: shared `multisig-types` crate (pairs with P-043).
- **Effort**: S
- **Change risk**: low
- **Verification**: Test: role-N signer derives role-N path.
- **Suggested owner**: BE + TAURI

- **ID**: P-037
- **Title**: Authority mapping incomplete in `asm_role_membership::authority_to_role` (3 of 5 unmapped)
- **Severity**: High
- **Axis source(s)**: `[06]`, `[08]` LOW, `[16]` H4
- **Evidence**: `infrastructure/asm_role_membership.rs:109–116`
- **User impact / risk**: A user selecting AlpenAdmin/SecurityCouncil/PayoutAdmin gets a generic 400 with no context.
- **Proposed change**: Test that fails until all 5 are mapped; `#[doc]` on blocked variants linking to `docs/2-discovery/08-alpen-crate-prd-coverage.md`. Larger: feature-gate unsupported authorities in UI.
- **Effort**: S
- **Change risk**: low
- **Verification**: `test_all_five_authorities_must_map` passes.
- **Suggested owner**: BE

- **ID**: P-038
- **Title**: No signature golden-test against SPS-65 for non-StrataAdmin authorities
- **Severity**: High
- **Axis source(s)**: `[10]` H3, `[14]`, `[16]` H1
- **Evidence**: `e2e-tests/tests/e2e_admin_commit_reveal.rs` covers Strata-Admin only
- **User impact / risk**: `compute_sighash` correctness across 5×N grid is unverified.
- **Proposed change**: Parameterized e2e test over 5 authorities × N update types with golden hex from SPS-65.
- **Effort**: M
- **Change risk**: low
- **Verification**: New test grid passes.
- **Suggested owner**: BE

- **ID**: P-039
- **Title**: Wallet address not validated against connected wallet in `session-provider.tsx`
- **Severity**: High
- **Axis source(s)**: `[03]` D10
- **Evidence**: `desktop-app/src/contexts/session-provider.tsx:51–58`
- **User impact / risk**: Signer can sign with a different Trezor account than the UI advertises.
- **Proposed change**: Compare `signature.publicKeyHex` to `wallet.publicKeyHex` before submission.
- **Effort**: S
- **Change risk**: low
- **Verification**: Test: account-switch mid-flow rejected.
- **Suggested owner**: FE

- **ID**: P-040
- **Title**: Tauri commands lack a capability / OCAP model
- **Severity**: High
- **Axis source(s)**: `[02]` D11, `[05]` BLOCKER-005
- **Evidence**: `desktop-app/src-tauri/src/main.rs:9–36`
- **User impact / risk**: Malicious frontend can call `proposals_broadcast` before auth completes.
- **Proposed change**: Gate high-risk commands on `get_session()` + role check. Larger: Tauri 2 capabilities per window.
- **Effort**: M
- **Change risk**: low
- **Verification**: Test: pre-auth broadcast invocation returns "insufficient role".
- **Suggested owner**: TAURI

### Medium (P-041 to P-057)

- **ID**: P-041
- **Title**: `AppState` is a god-object aggregating 12 unrelated concerns
- **Severity**: Medium
- **Axis source(s)**: `[06]` #2
- **Evidence**: `orchestrator-be/src/state.rs:1–56`
- **User impact / risk**: Adding a second network/scheme requires touching every handler.
- **Proposed change**: Extract `OrchestratorConfig` struct from `AppState`.
- **Effort**: M
- **Change risk**: med
- **Verification**: Refactor passes existing tests.
- **Suggested owner**: BE + ARCH

- **ID**: P-042
- **Title**: `Proposal` is an anemic data bag; invariants in `application/`, not aggregate
- **Severity**: Medium
- **Axis source(s)**: `[08]` BLOCKING, `[09]` #12
- **Evidence**: `orchestrator-be/src/domain/proposal.rs:89–103`; `infrastructure/memory_repo.rs:60–63`
- **User impact / risk**: Races become inevitable once Postgres replaces the global `RwLock` (pairs with P-019). Synthesized — see [07, 08, 09].
- **Proposed change**: Private fields + `add_signature_if_pending` method. Larger: split into `ProposalAggregate` + `BroadcastAggregate`.
- **Effort**: L
- **Change risk**: high
- **Verification**: Aggregate-level unit tests for invariants.
- **Suggested owner**: BE + ARCH

- **ID**: P-043
- **Title**: Authority duplication & ubiquitous-language drift; "Strata→Alpen Administrator" label bug
- **Severity**: Medium
- **Axis source(s)**: `[04]` BLOCKER-1, `[06]` #10, `[08]`, `[13]` #4, `[14]`
- **Evidence**: 5 backend variants vs 5 desktop variants vs 2 React variants; `broadcast-proposal-screen.tsx:17` mislabels
- **Proposed change**: Fix label bug now (S); align React enum strings with backend wire format (S); add round-trip serde test (S). Larger: shared `multisig-types` crate consumed by backend + desktop + e2e.
- **Effort**: L
- **Change risk**: med
- **Verification**: Round-trip serde test passes for all 5 authorities; e2e renders all 5.
- **Suggested owner**: BE + TAURI + FE + ARCH

- **ID**: P-044
- **Title**: Bitcoin/Strata types in application layer
- **Severity**: Medium
- **Axis source(s)**: `[06]` #4, `[08]`
- **Evidence**: `prepare_broadcast_bundle(operator_keypair: &UntweakedKeypair, network: Network, …)`
- **Proposed change**: Define domain-level abstractions in `domain/`; translate in handler/infrastructure.
- **Effort**: M
- **Change risk**: med
- **Verification**: Lint forbids `bitcoin::*` import in `application/`.
- **Suggested owner**: BE

- **ID**: P-045
- **Title**: `SessionContext` lives in `application/`, not `domain/`
- **Severity**: Medium
- **Axis source(s)**: `[08]`
- **Evidence**: `application/proposals.rs:22–25`
- **Proposed change**: Move to `domain/session.rs`.
- **Effort**: S
- **Change risk**: low
- **Verification**: Cron/batch task uses `SessionContext` without HTTP.
- **Suggested owner**: BE

- **ID**: P-046
- **Title**: No API versioning (no `/api/v1` prefix, no version header)
- **Severity**: Medium
- **Axis source(s)**: `[04]` MEDIUM-9
- **Evidence**: backend routes have no version segment
- **Proposed change**: Add `/api/v1` prefix and `Accept: application/json; version=1`.
- **Effort**: S
- **Change risk**: low
- **Verification**: Existing tests + new version-mismatch test.
- **Suggested owner**: BE

- **ID**: P-047
- **Title**: No data retention, soft delete, or FSM enforcement on `BroadcastStatus`
- **Severity**: Medium
- **Axis source(s)**: `[09]` #9/#11/#12
- **Evidence**: `update_broadcast_status` accepts any transition; no `deleted_at`
- **Proposed change**: `can_transition_to` predicate; `deleted_at` columns; archive job.
- **Effort**: M
- **Change risk**: low
- **Verification**: Test: invalid FSM transition rejected.
- **Suggested owner**: BE

- **ID**: P-048
- **Title**: No encryption at rest; broadcast errors leak RPC URLs/credentials
- **Severity**: Medium
- **Axis source(s)**: `[09]` #7/#10
- **Evidence**: `proposal_signatures.signer_pubkey TEXT`; `broadcast_error` stored verbatim
- **Proposed change**: `pgcrypto` for `signer_pubkey`/`signature_hex`; sanitize broadcast errors before persistence/exposure.
- **Effort**: M
- **Change risk**: med
- **Verification**: Audit test: error strings contain no IP/credential.
- **Suggested owner**: BE + PLATFORM

- **ID**: P-049
- **Title**: No desktop local persistence (drafts vanish on crash)
- **Severity**: Medium
- **Axis source(s)**: `[09]` #8
- **Evidence**: `desktop-app/src/contexts/wallet-session-context.ts` (RAM only)
- **Proposed change**: Tauri-side `save_proposal_draft` / `load_proposal_draft` to encrypted local file.
- **Effort**: M
- **Change risk**: low
- **Verification**: Test: kill app mid-form, restart, draft restored.
- **Suggested owner**: TAURI + FE

- **ID**: P-050
- **Title**: DIVIO/Diataxis collapse in `README.md` and `AGENTS.md`
- **Severity**: Medium
- **Axis source(s)**: `[15]` #6
- **Evidence**: README mixes tutorial + how-to + reference + explanation
- **Proposed change**: README as 5-line tutorial; AGENTS.md as reference with "when to use".
- **Effort**: S
- **Change risk**: low
- **Verification**: Onboarding test: new engineer makes `cargo test` pass in <30min unaided.
- **Suggested owner**: DOCS

- **ID**: P-051
- **Title**: Critical missing docs — ops runbook, ADR-006 coordination boundary, signer-safety model, threat model, capability matrix, build/release reproducibility, testing strategy, superseded markers
- **Severity**: Medium
- **Axis source(s)**: `[15]` #1/#2/#3/#4/#5/#7/#8/#9/#10
- **Evidence**: no files under `docs/architecture/` match the names above
- **Proposed change**: Create the 7 listed docs; flag superseded discovery docs with frontmatter.
- **Effort**: L
- **Change risk**: low
- **Verification**: Doc-review walkthrough.
- **Suggested owner**: DOCS + ARCH + PLATFORM

- **ID**: P-052
- **Title**: `docs/3-stories/` lacks DoR; missing signer-rotation and offline-fallback stories
- **Severity**: Medium
- **Axis source(s)**: `[13]` #7/#8, narratives 5–6, `[12]`
- **Evidence**: story map has no DoR section; `US-H5` deferred to Slice 5
- **Proposed change**: 8-item DoR checklist; story-by-story audit; rewrite Slice 0 to include offline fallback as walking-skeleton invariant; add `US-E_ROTATE`.
- **Effort**: L
- **Change risk**: low
- **Verification**: Every story passes DoR.
- **Suggested owner**: ARCH (PO role) + DOCS

- **ID**: P-053
- **Title**: Zero user discovery — assumptions not validated with real signers
- **Severity**: Medium
- **Axis source(s)**: `[12]` (all)
- **Evidence**: no interview artifacts in `docs/2-discovery/`
- **Proposed change**: 5–8 signer interviews; digest-verification usability test; manual-fallback tabletop sim.
- **Effort**: L
- **Change risk**: low (informational)
- **Verification**: Findings published; backlog re-prioritized.
- **Suggested owner**: ARCH (PO role)

- **ID**: P-054
- **Title**: Rule/skill stack drift between `.claude/rules/` and `.cursor/rules/`; missing `description:` on 6 skills
- **Severity**: Medium
- **Axis source(s)**: `[17]`
- **Evidence**: 27-line gap in `react-frontend-patterns.mdc`; `.cursor/rules/general.mdc` duplicates AGENTS.md
- **Proposed change**: First, verify whether `.cursor/rules/` is a Cursor IDE artifact (P-A-018). If safe, delete `.cursor/rules/` or mirror to `.claude/rules/`; add `description:` fields; add lint that fails on duplicate guidance.
- **Effort**: S
- **Change risk**: low
- **Verification**: Diff = 0 between intended-canonical sources.
- **Suggested owner**: DOCS + ARCH

- **ID**: P-055
- **Title**: SPS-65 cited as source-of-truth but source not in repo; backend↔SPS contradiction unverifiable
- **Severity**: Medium
- **Axis source(s)**: `[16]` #2/#5
- **Evidence**: no `docs/specs/sps-reference/` excerpts; no code comment cites a specific SPS-65 section
- **Proposed change**: Archive SPS-50/51/65 excerpts; add section-id comments in `signing.rs`, `proposals.rs`, `action_codec.rs`. Pairs with P-012/P-051.
- **Effort**: M
- **Change risk**: low
- **Verification**: Each Tier 0 rule cites a specific SPS section.
- **Suggested owner**: ARCH + DOCS

- **ID**: P-056
- **Title**: Crate-pinning rationale incomplete (ADR-001 — rev vs. tag)
- **Severity**: Medium
- **Axis source(s)**: `[16]` #4
- **Evidence**: ADR-001 mixes pin styles with no trade-off or convergence signal
- **Proposed change**: Expand ADR with explicit trade-off table, convergence signal definition, and rollback plan.
- **Effort**: S
- **Change risk**: low
- **Verification**: ADR review.
- **Suggested owner**: ARCH

- **ID**: P-057
- **Title**: Vestigial Tauri `custom-protocol` feature flag; deprecated GraphQL in `sprint-board` skill; unused config fields
- **Severity**: Low
- **Axis source(s)**: `[14]` LOW, `[17]` LOW
- **Evidence**: `desktop-app/src-tauri/Cargo.toml:46`; `sprint-board/SKILL.md:136–143`
- **Proposed change**: Delete or document.
- **Effort**: S
- **Change risk**: low
- **Verification**: CI builds clean.
- **Suggested owner**: TAURI + DOCS

---

## Consolidated improvement backlog (ASSESSMENT PROCESS)

- **ID**: A-001
- **Title**: Standardize per-finding template (severity, evidence path:line, F/H/OQ tag, effort S/M/L, change-risk, owner)
- **Severity**: High
- **Axis source(s)**: meta (all axes)
- **Evidence**: severity vocab inconsistent; only `[04]`/`[12]` mark HYPOTHESIS; no effort/owner anywhere
- **User impact / risk**: Reader has to invent classifications; rollup work re-done every time.
- **Proposed change**: Create `docs/assessment/_template/axis-template.md` with required fields and a checklist; reviewers reject axes missing fields.
- **Effort**: S
- **Change risk**: low
- **Verification**: Next assessment uses template; rollup is a `jq` away.
- **Suggested owner**: PLATFORM (assessment ops)

- **ID**: A-002
- **Title**: Enforce evidence-bar: every Blocker/High must cite `file:line` and an excerpt
- **Severity**: High
- **Axis source(s)**: `[02]` D8/D11 (implied), `[09]` #8, `[15]` (ADR cross-refs not read)
- **Proposed change**: CI lint or reviewer pass that fails axes containing "(implied)" or "not fully read" on Blocker/High items.
- **Effort**: S
- **Change risk**: low
- **Verification**: Re-run rejects axes that don't comply.
- **Suggested owner**: PLATFORM

- **ID**: A-003
- **Title**: Fact / Hypothesis / Open-Question tag set on every finding
- **Severity**: High
- **Axis source(s)**: meta
- **Proposed change**: One-line tag at the top of every finding: `Fact:` (verified in code), `Hypothesis:` (plausible, no probe run), `OpenQuestion:` (need external confirmation).
- **Effort**: S
- **Change risk**: low
- **Verification**: Synthesizer can grep tags to weight confidence.
- **Suggested owner**: ARCH

- **ID**: A-004
- **Title**: Effort & change-risk classification embedded in finding template
- **Severity**: High
- **Axis source(s)**: meta
- **Proposed change**: Required fields `effort: S|M|L`, `change_risk: low|med|high`. Tooling lints presence.
- **Effort**: S
- **Change risk**: low
- **Verification**: Backlog import script reads them directly.
- **Suggested owner**: ARCH + PLATFORM

- **ID**: A-005
- **Title**: Severity-calibration rules when axes disagree
- **Severity**: High
- **Axis source(s)**: `[00]` "Disagreements between axes" section
- **Proposed change**: Decision rule documented: evidence chain wins; PRD-cited items beat code-only items; security/signer-safety items can only be downgraded with a documented disconfirming probe.
- **Effort**: S
- **Change risk**: low
- **Verification**: Reconciliation table on every assessment.
- **Suggested owner**: ARCH

- **ID**: A-006
- **Title**: De-duplication convention: single P-### per defect with multi-axis citation
- **Severity**: High
- **Axis source(s)**: meta
- **Proposed change**: Synthesizer maintains an axis-source field per backlog item. Adversarial axes are encouraged to keep their own finding IDs; consolidation maps `[NN] F#` → `P-###` in a registry.
- **Effort**: S
- **Change risk**: low
- **Verification**: Backlog has no duplicate semantic items.
- **Suggested owner**: ARCH

- **ID**: A-007
- **Title**: Citation discipline for synthesizer — flag synthesized claims
- **Severity**: Medium
- **Axis source(s)**: `[00]` (already does this)
- **Proposed change**: Adopt `Synthesized — see [N, M]` marker (as in `[00]`) for any claim no single axis owns; reject unsynthesized novel claims.
- **Effort**: S
- **Change risk**: low
- **Verification**: Reviewer pass for unflagged synthesis.
- **Suggested owner**: ARCH

- **ID**: A-008
- **Title**: When to use `/nw-review` vs Task-based reviewers (and when neither)
- **Severity**: Medium
- **Axis source(s)**: meta (process)
- **Proposed change**: Document decision rule: `/nw-review` only when valid nWave artifacts (roadmap.json / SSOT design) + required params exist; otherwise use Task-based generic reviewers (e.g., `nw-*-reviewer`). For assessment passes, use specialized adversarial reviewers (e.g., `nw-*-reviewer` per axis).
- **Effort**: S
- **Change risk**: low
- **Verification**: Decision tree in `docs/assessment/README.md`.
- **Suggested owner**: ARCH

- **ID**: A-009
- **Title**: Parallelization rules + reviewer-axis mapping
- **Severity**: Medium
- **Axis source(s)**: meta
- **Proposed change**: Document which axes can run truly independently (1–11 are code-axes; 12–14 are PO/research; 15–17 are docs/agent). Run code axes in parallel; PO/discovery axes after code axes (so they can cite them).
- **Effort**: S
- **Change risk**: low
- **Verification**: Re-run completes in <duration target.
- **Suggested owner**: PLATFORM

- **ID**: A-010
- **Title**: Repeatability — scripted assessment launcher
- **Severity**: Medium
- **Axis source(s)**: meta
- **Proposed change**: `scripts/assessment/launch.sh <date>` that spawns the 17 axes with identical prompts, identical input set; writes outputs to `docs/assessment/<date>-adversarial/NN-*.md`. Stores prompt fingerprints alongside.
- **Effort**: M
- **Change risk**: low
- **Verification**: A future date re-run yields a comparable corpus.
- **Suggested owner**: PLATFORM

- **ID**: A-011
- **Title**: Guardrails to keep signing/security findings from being diluted
- **Severity**: High
- **Axis source(s)**: meta — risk that hygiene refactors crowd out Tier 0
- **Proposed change**: Synthesizer always publishes a "Signer Safety Sub-Rollup" separately from the general backlog; Tier 0 must list signer-safety items first.
- **Effort**: S
- **Change risk**: low
- **Verification**: Reviewer confirms separation on every meta-review.
- **Suggested owner**: ARCH

- **ID**: A-012
- **Title**: Disconfirming-probe expectations — every Blocker proposes and runs a probe when feasible
- **Severity**: High
- **Axis source(s)**: `[00]` "What we might still be wrong about", `[01]` Experiments, `[02]` Experiments
- **Proposed change**: Axis template requires `disconfirming_probe:` section with status `proposed | run-passed | run-failed | not-feasible`. CI flags Blocker findings with `not-feasible` for ARCH review.
- **Effort**: M
- **Change risk**: low
- **Verification**: Subsequent assessment shows probes actually executed.
- **Suggested owner**: ARCH

- **ID**: A-013
- **Title**: Reviewer pair model (Sonnet writer + Haiku reviewer per axis)
- **Severity**: Medium
- **Axis source(s)**: meta
- **Proposed change**: Each axis output reviewed by paired `nw-*-reviewer` (Haiku) before synthesizer consumes. Reviewer enforces template + evidence-bar.
- **Effort**: M
- **Change risk**: low
- **Verification**: Reviewer logs attached to each axis.
- **Suggested owner**: PLATFORM

- **ID**: A-014
- **Title**: Required-sections enforcement
- **Severity**: Low
- **Axis source(s)**: meta
- **Proposed change**: Required headings: scope/threat model, top findings (with rubric fields), attack narratives (3–6), evidence index (paths), smallest fixes vs largest bets, what would change my mind. Lint axes without all six.
- **Effort**: S
- **Change risk**: low
- **Verification**: Lint passes on every axis.
- **Suggested owner**: PLATFORM

- **ID**: A-015
- **Title**: Cross-axis contradiction reconciliation step required in summary
- **Severity**: Medium
- **Axis source(s)**: `[00]` already does this; should be required
- **Proposed change**: Summary axis must include a "Disagreements" table with decision rule for each row. This meta-review formalizes that.
- **Effort**: S
- **Change risk**: low
- **Verification**: Section present on every summary.
- **Suggested owner**: ARCH

- **ID**: A-016
- **Title**: Stable finding IDs across axes — map original `[NN] F#` to `P-###`
- **Severity**: Medium
- **Axis source(s)**: `[10]` B1 / `[07]` #4-#5 / `[08]` "race" all describe the same defect with different IDs
- **Proposed change**: Synthesizer maintains a registry mapping `(axis, original_id) → P-###`. Saved alongside `99-meta-review.md`.
- **Effort**: S
- **Change risk**: low
- **Verification**: Registry exists and is referenced.
- **Suggested owner**: ARCH

- **ID**: A-017
- **Title**: Numeric severity score + automatic top-N rollup
- **Severity**: Low
- **Axis source(s)**: meta
- **Proposed change**: Each finding gets a 0–10 score = `severity_weight × evidence_weight × user_impact_weight`. Top-N rollup is mechanical.
- **Effort**: M
- **Change risk**: low
- **Verification**: Top-20 derived without manual judgement.
- **Suggested owner**: PLATFORM

- **ID**: A-018
- **Title**: Confirm Cursor IDE loading semantics before deleting `.cursor/rules/`
- **Severity**: Medium
- **Axis source(s)**: `[17]` (its own "what would change my mind")
- **Proposed change**: 30-min experiment: edit a file in Cursor, check whether `.cursor/rules/` is overwritten by IDE; document finding in `docs/architecture/adrs/`.
- **Effort**: S
- **Change risk**: low
- **Verification**: ADR records the IDE behaviour; P-054 unblocked or revised.
- **Suggested owner**: PLATFORM

- **ID**: A-019
- **Title**: Living confidence table — downgrade findings when evidence collected
- **Severity**: Low
- **Axis source(s)**: `[00]` confidence table is a one-shot
- **Proposed change**: Persist confidence table to a file updated after each probe run; meta-review references the latest.
- **Effort**: M
- **Change risk**: low
- **Verification**: Confidence file changes between assessment dates.
- **Suggested owner**: ARCH

---

## Implementation plan (phased)

### Phase 0 (0–3 days): safety/correctness hotfixes + highest-confidence blockers

- **Scope**: P-001, P-002, P-004, P-007, P-008 (initial schema), P-009, P-010, P-013, P-014, P-015, P-016, P-024, P-026, P-027, P-034, P-043 (label-bug-fix subset), P-054 (after A-018 experiment), A-018.
- **Entry criteria**: Tier 0 backlog reviewed; owners assigned; `cargo test` passes on `main`.
- **Exit criteria**: All listed P-### items have merged PRs; CI gates added for `cargo audit`, `package-lock.json`, no-test-key startup, lowercase pubkey constraint; manual smoke test of all flows after CSP enable.
- **Milestones**: end-of-day 1 — operator-key + default-network fixes; end-of-day 2 — Zod IPC validation skeleton + authority filter on list/get; end-of-day 3 — CSP + label bug + DATABASE_URL mandatory.
- **Dependencies**: P-008 (Zod) unblocks P-022 testing; P-002 unblocks P-019/P-035 testing.
- **Rollback/mitigation**: CSP enable can break flows — keep a one-line feature flag for emergency disable in dev; secret-fail-fast can break local dev — provide a `.env.example` with documented test key clearly labeled `LOCAL DEV ONLY`.

### Phase 1 (1–2 weeks): reliability, maintainability, top architectural payoffs

- **Scope**: P-003, P-008 (full Zod), P-011 (MVP: lockfile + SCA), P-017, P-018, P-019, P-020, P-021, P-022, P-023, P-025, P-028, P-029, P-030, P-032 (axis-10 named tests), P-033, P-035, P-036, P-037, P-038, P-039, P-040, P-046, P-047, P-049, P-050, P-055 (skeleton), P-056. ADR-006 written even if not implemented; ADR-007 for sighash boundary.
- **Entry criteria**: Phase 0 PRs merged; structured logging skeleton + `/ready` deployed; `multisig-types` crate proposal accepted.
- **Exit criteria**: Concurrent-approval, broadcast error-path, signature negative-path, IPC contract, and authority-isolation tests are green; backend ops runbook v1 written; manual-fallback tabletop sim scheduled.
- **Milestones**: week 1 — Tier-1 backend correctness (P-017, P-018, P-019, P-027 full, P-030); week 1.5 — IPC contract + frontend test scaffold; week 2 — ADR-006/007 reviewed by Alpen.
- **Dependencies**: P-019 must precede P-042 (aggregate refactor); P-022 needs P-008.
- **Rollback/mitigation**: P-012/P-035 changes carry the highest change risk (touches core proposal lifecycle) — ship behind a feature flag; require Alpen sign-off on ADR-006 before flipping. P-028 (lint) may surface unrelated import sites — fix in same PR or skip with allow attribute and follow-up ticket.

### Phase 2 (2–6 weeks): larger refactors / platform work / deeper test hardening

- **Scope**: P-005 (signed envelopes), P-006 (visualization spec + UI work), P-011 (full signed release pipeline), P-031, P-032 (e2e all 5 authorities + ASM real-state integration), P-041, P-042, P-043 (shared `multisig-types` crate + codegen), P-044, P-045, P-047 (FSM enforcement), P-048, P-052 (DoR + `US-E_ROTATE` + offline-fallback spec), P-053 (5 signer interviews + digest-verification + tabletop), P-055 (full archive + section comments), Phase-2 portions of P-051.
- **Entry criteria**: Phase 1 exit met; ADR-006/007 accepted by Alpen.
- **Exit criteria**: Shared `multisig-types` crate consumed by backend + desktop + e2e; durable event log + replay-on-startup; resumable broadcast FSM; signed release pipeline producing verifiable Linux/macOS/Windows binaries; user research findings published.
- **Milestones**: week 3 — `multisig-types` crate ships; week 4 — durable event log + resumable broadcast; week 5 — user research wrap; week 6 — full e2e green for all 5 authorities.
- **Dependencies**: P-031 unblocks P-018 resumable path; P-042 needs P-019; P-006 needs P-053 evidence to design correct UX.
- **Rollback/mitigation**: P-043 (shared crate) is a high-blast-radius refactor — sequence carefully with CI green-build gate; keep both copies live for one release cycle. Release signing (P-011) may fail Apple notarization on first attempt — schedule a buffer.

### Phase 3 (later): opportunistic cleanups and doc debt

- **Scope**: P-051 (remaining docs: threat model, incident playbook, capability matrix, build/release, testing strategy), P-057, P-053 follow-up (institutional signer interviews), key rotation procedure, hardware-wallet-only operator path, agent/skill stack consolidation finalization, A-013 reviewer-pair model rollout, A-017 numeric scoring.
- **Entry criteria**: Phase 2 exit met; production-readiness gate cleared for Tier 0/1 items.
- **Exit criteria**: Documentation stack passes onboarding test (<30 min unaided to `cargo test`); capability matrix living; signed-release verification documented for end-users.
- **Milestones**: rolling; tied to feature ship cadence.
- **Dependencies**: minimal; most of these are documentation or hygiene that compound with code maturity.
- **Rollback/mitigation**: low risk; opportunistic.

---

## Sequencing graph (textual)

**Foundations (Track A — BE):** P-001 → P-002 → P-016 → P-017 → P-018 → P-019 → P-031 → P-042  
**Boundary correctness (Track A):** P-026 → P-012 → P-035 → P-038  
**Observability (Track A || B):** P-029 → P-027 → P-030  
**IPC / type safety (Track B — TAURI/FE):** P-008 → P-022 → P-023 → P-021 → P-020  
**Auth + session (Track B):** P-009 → P-039 → P-040 → P-014  
**Tauri secrets (Track B):** P-003 → P-015 → P-013  
**Frontend UX (Track C — FE):** P-007 → P-010 → P-006 (needs P-053)  
**Supply chain (Track D — PLATFORM):** P-011 → P-004  
**Storage hardening (Track A, after P-016/P-017):** P-024 → P-046 → P-047 → P-048 → P-049  
**Architecture (Track A, after P-019/P-031):** P-041 → P-044 → P-045 → P-028 → P-025  
**Shared types (Track A + B + C — ARCH):** P-043 (depends on P-022 done) → unblocks future authorities  
**Research/discovery (Track E — ARCH-PO):** P-052 → P-053 → P-006 → P-051  
**Docs/agents (Track F — DOCS):** A-018 → P-054 → P-050 → P-051 → P-055 → P-056

**Parallelizable tracks**:
- **Track A (BE)** || **Track B (TAURI+FE shared)** || **Track D (PLATFORM)** in Phase 0
- **Track A** || **Track B** || **Track C** || **Track D** || **Track E** || **Track F** in Phase 1 (lower coupling)
- Phase 2 serializes Track A around `multisig-types` extraction; other tracks remain parallel.

**Critical-path single-thread items** (cannot parallelize): P-012/P-035 (Alpen sign-off on ADR-006 gates Phase-2 ASM-related work); P-006 (depends on P-053 user research); P-043 (touches backend + tauri + react simultaneously, needs one-shot landing).

---

## Top 20 execution queue

1. P-001 — Remove operator-key default; fail startup if missing or equals test key. *(S, low, BE+TAURI)*
2. P-002 — Authority filter on `list_proposals` / `get_proposal` / broadcast handlers; 401 on mismatch. *(S, low, BE)*
3. P-004 — Set strict CSP in `tauri.conf.json`. *(S, low, TAURI+FE)*
4. P-013 — Remove `regtest` default; require explicit `network`. *(S, low, TAURI)*
5. P-014 — Reject non-`https://` in `build_client` (allow `http://localhost` only in dev). *(S, low, TAURI)*
6. P-015 — Delete `VITE_OPERATOR_SECRET_KEY_HEX` path; strip sourcemaps in prod. *(S, low, FE+PLATFORM)*
7. P-016 — Fail backend startup if `DATABASE_URL` missing in production. *(S, low, BE)*
8. P-007 — Freeze form values at preview; force re-preview on change. *(S, low, FE)*
9. P-010 — Refuse sign screen render unless `proposal.authority === authorityFromRole(selectedRole)`. *(S, low, FE)*
10. P-009 — Validate session authority on reuse; `await authLogout()` before re-auth; fix `Strata→Alpen Administrator` label bug. *(S, low, FE+BE)*
11. P-008 — Add Zod schemas at the Tauri bridge for `Proposal`/`ProposalStatus`/`BroadcastStatus`/`AuthSession`. *(M, low, FE+TAURI)*
12. P-011 — Commit `package-lock.json`, `npm ci`, `cargo audit` + `cargo deny` in CI. *(L, med, PLATFORM)*
13. P-024 — Normalize signer pubkey to lowercase on every ingress; DB `CHECK` constraint. *(S, low, BE+TAURI)*
14. P-026 — SSZ-decode at create-handler ingress; reject early. *(S, low, BE)*
15. P-027 — `tokio::time::timeout` wrap every BTC/ASM RPC call. *(S, low, BE)*
16. P-029 — `#[tracing::instrument]` on handlers; request UUID in tauri-bridge; `/ready` checks Postgres + RPCs. *(M, low, BE+FE+PLATFORM)*
17. P-019 — Move duplicate-signer check into repo write lock; repo owns quorum transition. *(M, med, BE)*
18. P-020 — Disable Send button while broadcasting; cache in-flight action_ids in Tauri. *(S, low, FE+TAURI+BE)*
19. P-033 — BIP-137 recovery-id normalization in `broadcast_tx`. *(S, med, BE)*
20. P-012 — Decide threshold-check policy (remove auto-Approve, OR write ADR-006 with explicit advisory carve-out + threshold-resync test). *(M, high, ARCH+BE)*

---

## Open questions / missing evidence

The following items need data before promoting hypothesis → fact, or before downgrading findings. Each names a specific command/owner.

- **OQ-1 — Production deploy script.** Does the deploy pipeline actually fail without `OPERATOR_SECRET_KEY_HEX` / `DATABASE_URL`? *(Run: review k8s/systemd unit; owner: PLATFORM. If yes, P-001/P-016 severity drops to High.)*
- **OQ-2 — SPS-65 source.** Get a local archive of the relevant SPS-65 sections (threshold checks, sighash payload, expiry semantics). *(Owner: ARCH; coordinate with Alpen. Required to close P-012/P-055 with confidence.)*
- **OQ-3 — `.cursor/rules/` ownership.** Is `.cursor/rules/` written by Cursor IDE or hand-maintained? *(Owner: PLATFORM, 30-min experiment per A-018. Outcome resolves P-054 direction.)*
- **OQ-4 — Postgres race.** Does the Postgres path use `SELECT FOR UPDATE` or an advisory lock? *(Owner: BE; read `infrastructure/postgres_repo.rs` end-to-end. If yes, P-019 race window narrows.)*
- **OQ-5 — `u64` precision in practice.** Are real-world `seq_no` values plausibly above `2^53`? *(Owner: ARCH; check Alpen seqno cadence. If always small, P-021 can be deferred.)*
- **OQ-6 — Manual fallback.** Tabletop sim with 3 signers, backend deliberately offline. *(Owner: ARCH-PO. Outcome promotes/demotes P-053 / P-052; may escalate manual-fallback to Tier 0.)*
- **OQ-7 — Digest verification UX.** Test with 3 non-developer signers on a real Trezor. *(Owner: ARCH-PO. Required to design P-006 visualization.)*
- **OQ-8 — Authority count.** Confirm whether React 2-variant `AuthRole` is a Slice-0 placeholder. *(Owner: FE; `git log -- desktop-app/src/types/auth-role.ts`.)*
- **OQ-9 — HW wallet path.** Audit `hw_wallet/trezor.rs` and `hw_wallet/ledger.rs` for derivation-path, BIP-137 handling. *(Owner: TAURI. Pairs with P-033.)*
- **OQ-10 — Existing E2E coverage of authority isolation.** Does `alpen-multisig-e2e-tests` already test cross-authority denial? *(Owner: BE; one grep. If yes, P-002 verification is partly written.)*

HYPOTHESIS-tagged items in inputs awaiting fact/discard:
- `[04]` Narrative 5 (status enum rollout breaks expiry awareness) — H. Promote with a real-world rollout postmortem.
- `[04]` HIGH-4 `u64` precision — H. See OQ-5.
- `[12]` "32-byte digest verification at scale" — H. See OQ-7.
- `[12]` "manual fallback is feasible" — H. See OQ-6.
- `[17]` `.cursor/rules/` is "drift" not "IDE artifact" — H. See OQ-3.
- `[10]` race-condition severity — H (theoretical until load test). See OQ-4.

---

## Citation discipline

This meta-review uses `[NN]` to refer to axis files `NN-*-adversarial.md`. Specific findings inherit the file's evidence chain. Where this meta-review introduces a claim no single axis names, it is flagged `Synthesized — see [N, M]`. No new technical claims have been introduced; every P-### maps back to one or more axis findings.

**Synthesized claims used in this document (all flagged inline):**
- "In-process `RwLock` accidentally serializes everything → races become worse with Postgres" — Synthesized — see [04, 07, 08, 09].
- "CSP-off + npm `^`-ranges + no SCA + unsigned releases + plaintext IPC = textbook supply-chain attack surface" — Synthesized — see [02, 05, 17].
