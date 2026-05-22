# Adversarial assessment: agent-spec quality (rules + skills)

**Date:** 2026-05-14  
**Repo:** `alpen-multisig`  
**Assessor stance:** hostile-but-fair — assumes contributors optimize for speed, tools disagree, and specs rot unless mechanically enforced.

---

## 1. Scope

**In scope (read for this note):**

| Area | Inventory |
|------|-----------|
| `AGENTS.md` | Single root onboarding doc for AI-assisted work |
| `.claude/rules/` | 4 Markdown rules: `typescript-standards`, `react-frontend-patterns`, `rust-backend-standards`, `backend-api-conventions` |
| `.cursor/rules/` | 5 MDC rules: same four domains plus `general.mdc` (`alwaysApply: true`) |
| `.claude/skills/` | 8 skills (`SKILL.md` each): `react-code-audit`, `rust-code-audit`, `react-ui-screen-implementation`, `sdd`, `spec-compliance-audit`, `sprint-board`, `rust-specialist`, `use-case-testing-plan` |

**Explicitly out of scope:** application source (`orchestrator-be/`, `desktop-app/src/`, protocol crates), CI workflows, runtime configuration.

**Method:** static comparison of paired artifacts, cross-reference to `AGENTS.md`, and line-count / section presence checks on skills (aggregate ~837 lines across 8 skills; largest single skill `sdd` at 204 lines).

---

## 2. Top findings

1. **Forked rule corpora (`.claude` vs `.cursor`) are not equivalent.** The TypeScript rule in `.claude` carries additional bullets absent from `.cursor` (boundary modules, typed hook contracts, feature-local model paths — `.claude/rules/typescript-standards.md` L18–21 vs `.cursor/rules/typescript-standards.mdc` ending after L22 without those lines). The React rule is worse: `.cursor/rules/react-frontend-patterns.mdc` omits whole sections that exist in `.claude/rules/react-frontend-patterns.md` — **Architecture by Domain** (L8–15), **Styling** (L26–29), **Separation of Responsibilities** (L31–36), and **What Goes Where** (L38–44). An agent or human editing only one stack systematically inherits different architecture constraints.

2. **`AGENTS.md` describes Claude Code rule loading, not Cursor.** It states rules live in `.claude/rules/` and auto-load by `paths` frontmatter (`AGENTS.md` L73–80). Cursor’s active constraints are primarily `.cursor/rules/*.mdc` with `globs` / `alwaysApply`. A Cursor-native session can believe it is “following AGENTS” while never seeing the fuller `.claude` variants.

3. **High-impact automation skills bake in environment assumptions.** `sdd/SKILL.md` instructs reading `.claude/rules/` (L19), mandates `rust-specialist` under `.claude/skills/` (L20–21, L92–93, L203–204), assumes `gh` + a fixed reviewer login (`juandahl`) for PR creation (`sdd` L169–191). `sprint-board` pins org project URL and numeric project id (`sprint-board` L7–21). Any rename, permission change, or tool absence fails silently or produces wrong artifacts — the spec does not define graceful degradation.

4. **`disable-model-invocation: true` on `sdd` and `sprint-board`** (`sdd` L4; `sprint-board` L4) reduces accidental autonomous runs but also means orchestrators must explicitly attach these skills — easy to “forget” in hybrid workflows, yielding inconsistent SDD enforcement.

5. **Multisig-critical posture is unevenly encoded.** Signer safety and coordination-only backend rules appear in `AGENTS.md` / `.cursor/rules/general.mdc` as bullets, and partially in `backend-api-conventions` / audit skills — but `rust-specialist` is generic idiomatic Rust (`rust-specialist` L7–80 sampled) without tying errors, logging, or types back to SPS/ASM invariants. Compliance skills anchor PRDs (`spec-compliance-audit` L12–14; `react-code-audit` L12–15) — good — yet nothing forces those skills to run before merges.

---

## 3. Attack narratives (3–6)

### A. “Split-brain frontend architecture”

**Attacker:** Maintainer updates `.claude/rules/react-frontend-patterns.md` with a new mandatory folder convention. No one mirrors `.cursor/rules/react-frontend-patterns.mdc`.

**Outcome:** Cursor-assisted edits violate domain-folder discipline while Claude Code sessions obey it. Reviews argue past each other; regressions land because “the rule said so” depends on which editor injected which file.

**Why it succeeds:** No single SSOT file; diff-shaped drift between stacks.

---

### B. “SDD phase gate bypass”

**Attacker:** Contributor invokes a generic coding agent without attaching `sdd` (blocked by `disable-model-invocation`) or skips Phase 2 user confirmation (`sdd` L68–68: explicit wait for user).

**Outcome:** Implementation lands without spec contract, without mandatory verification gates (`sdd` L133–149), or with PR metadata missing spec linkage (`sdd` L183–186).

**Why it succeeds:** Process enforcement is narrative (“never skipping”) not mechanical; no repo hook ties branches to `docs/specs/<slug>.md`.

---

### C. “TypeScript boundary leakage”

**Attacker:** Team standardizes on Cursor; `.cursor/rules/typescript-standards.mdc` is treated as canonical.

**Outcome:** Weaker guidance on transport vs view-model separation and hook contracts (missing bullets per §2 finding #1). Raw DTO shapes propagate into screens; multisig authority bugs become more likely despite spirit of PRDs.

**Why it succeeds:** Shorter `.mdc` reads faster and wins attention; stricter `.claude` copy is orphaned.

---

### D. **“Sprint board phantom work”**

**Attacker:** `sprint-board` skill runs with stale project constants or expired `gh` auth.

**Outcome:** Wrong project receives issues, or CLI errors loop; skill text says “stop and ask” on ambiguity (`sprint-board` L31–38) but real failures are often **external** (403, renamed board) — not enumerated.

**Why it succeeds:** Hard-coded IDs (`wakeuplabs-io`, project `4`) without validation steps or health-check phase.

---

### E. **“Audits that never fire”**

**Attacker:** Shipping pressure; no agent invokes `react-code-audit`, `rust-code-audit`, or `spec-compliance-audit`.

**Outcome:** PRDs remain nominal sources of truth while implementation diverges; skills are well-written checklists but optional ornaments.

**Why it succeeds:** Skills lack triggers in CI and lack mandatory invocation wiring in `AGENTS.md` beyond human diligence.

---

### F. **“Reviewer roulette”**

**Attacker:** `gh pr edit … --add-reviewer juandahl` (`sdd` L188–191) after org change.

**Outcome:** PRs stall or violate current review policy; SDD doc becomes misinformation.

**Why it succeeds:** Personal identifier embedded in automation spec without parameterization or “verify current reviewers” step.

---

## 4. Evidence

| Claim | Evidence |
|-------|----------|
| `AGENTS.md` frames Claude Code + `.claude/rules` | `AGENTS.md` L1–4, L73–80 |
| Global Cursor conventions mirror Key Conventions | `.cursor/rules/general.mdc` L1–16 vs `AGENTS.md` L61–71 |
| TypeScript rule divergence | `.claude/rules/typescript-standards.md` L18–21 absent in `.cursor/rules/typescript-standards.mdc` (file ends at different bullet set) |
| React rule: Cursor missing architecture/styling sections | `.claude/rules/react-frontend-patterns.md` L8–44 vs `.cursor/rules/react-frontend-patterns.mdc` L7–39 (no Architecture by Domain / Styling / Separation / What Goes Where) |
| SDD locks paths to `.claude` tree | `sdd/SKILL.md` L19–21, L92–93, L203–204 |
| SDD hard-codes GitHub reviewer | `sdd/SKILL.md` L188–191 |
| SDD disables model invocation | `sdd/SKILL.md` L4 |
| Sprint board pins GitHub project | `sprint-board/SKILL.md` L7–21 |
| Compliance skills mandate PRD reads | `spec-compliance-audit/SKILL.md` L10–24; `react-code-audit/SKILL.md` L10–26 |
| Skill size distribution | `wc -l`: `react-code-audit` 66; `sdd` 204; total 837 across 8 skills (shell enumeration 2026-05-14) |

---

## 5. Smallest vs largest bets

**Smallest bets (low cost, high clarity)**

- Add a one-line **cross-stack parity note** to `AGENTS.md`: Cursor loads `.cursor/rules`; Claude Code loads `.claude/rules`; editors must diff paired files when changing conventions.
- Normalize **TypeScript** and **React** `.mdc` files to include every bullet/section present in `.claude` counterparts (or formally declare Cursor the subset with rationale).

**Largest bets (structural)**

- Introduce a **single generated artifact** (script or CI check) that asserts `.claude/rules/*` and `.cursor/rules/*` semantic parity for paired topics — fail CI on drift.
- Replace hard-coded **GitHub reviewer / project IDs** in skills with placeholders + explicit “resolve from team SSOT” steps, or move constants to `docs/` YAML consumed by both humans and agents.
- Wire **audit skills** into a documented release gate (e.g., required PR checklist section naming which skill outputs attach) — still human-enforced, but visible.

---

## 6. What would change my mind

- **Measured parity:** Automated diff report showing React + TS rules byte-aligned (modulo frontmatter) on every PR touching either stack.
- **Invocation telemetry:** Evidence that SDD phases actually run (spec file created before implementation commits; CI referencing spec path).
- **Outcome data:** Post-mortems where drift caused a defect — currently hypothetical; absent data keeps severity at “process risk” not “proven incident.”
- **Cursor/Claude convergence:** Official maintainer policy (“`.cursor` is generated from `.claude` nightly”) — would downgrade fork-brain narrative substantially.

---

## Appendix: quick inventory counts

- `.claude/skills`: 8 files  
- `.claude/rules`: 4 files  
- `.cursor/rules`: 5 files (includes `general.mdc`)
