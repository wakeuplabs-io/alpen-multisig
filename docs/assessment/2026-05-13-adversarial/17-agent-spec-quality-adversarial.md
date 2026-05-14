# Agent / Skill / Rule Definitions — Adversarial Assessment

## Scope & threat model (what we're trying to break)

This audit attacks maintainability and misleading-guidance risks in agent specifications across this repository.

**Scope:**
- Skill loading, triggering criteria, and applicability (8 project skills in `.claude/skills/`)
- Rule clarity and consistency between `.claude/rules/` (4 files, `.md`) and `.cursor/rules/` (5 files, `.mdc`)
- AGENTS.md and CLAUDE.md as the single source of truth for architecture/conventions
- Central guidance documents and frontmatter (`paths:`, `globs:`, `description`, `disable-model-invocation`)
- Conflict detection: rules vs rules, skills vs rules, stale guidance vs current layout

**Threat model:**
1. **Silent violations**: rules that agents routinely ignore because they're unclear or conflicting
2. **Misleading skills**: skills with weak triggering criteria that fire at wrong moments or miss their intended use
3. **Stale guidance**: rules that describe old file structure or outdated patterns (e.g., references to moved directories)
4. **Conflict cascades**: two rules saying opposite things → agents have no way to choose → inconsistent output
5. **Safety risks**: rules that weaken signer safety, authority isolation, or repo integrity (e.g., "always commit", "auto-apply")
6. **Duplication**: near-identical rules/skills that create confusion about which to use

---

## Top findings (ranked) — Blocking/High | Medium | Low

### BLOCKING: Rule duplication and inconsistency between `.claude/rules/` and `.cursor/rules/`

**Evidence:**
- `.claude/rules/typescript-standards.md` (26 lines) vs `.cursor/rules/typescript-standards.mdc` (23 lines) — differ by **4 critical lines**
- `.claude/rules/react-frontend-patterns.md` (68 lines) vs `.cursor/rules/react-frontend-patterns.mdc` (39 lines) — differ by **27 lines** (39% of content)
  - `.claude` version includes "Architecture by Domain", "Styling", "Separation of Responsibilities", "What Goes Where"
  - `.cursor` version omits these sections entirely
- **Frontend path layout not specified in `.cursor` rules** (`.cursor/rules/react-frontend-patterns.mdc` has no mention of `domain/<feature>/`, `screens/`, `components/` directory structure)
- Frontmatter format differs: `.claude` uses `paths:` (YAML list), `.cursor` uses `globs:` (string)
- `.cursor` version adds `alwaysApply: false` and `description:` fields not present in `.claude`

**Risk:**
- Agents loading `.cursor/rules/` for TypeScript get weaker guidance on DTO/view-model separation (4 lines missing from `.claude` version)
- Agents loading `.cursor/rules/react-frontend-patterns.mdc` **do not see the directory structure** ("Architecture by Domain" section completely omitted)
- Frontend developers could place feature components in `src/components/` instead of `src/domain/<feature>/components/` because the `.cursor` version doesn't mandate the structure
- If Cursor IDE loads `.cursor/rules/` and agents use `.claude/rules/`, the agent's understanding of "where to put code" differs from the editor's IDE rules

### HIGH: Skill triggering criteria are vague or missing

**Evidence:**

1. **`sdd/SKILL.md` has `disable-model-invocation: true`** but no `description` field is used to auto-trigger
   - Phase 1 reads `.claude/skills/rust-specialist/SKILL.md` but never reads it at the agent's invocation
   - The skill is never automatically triggered; a user must explicitly invoke it
   - **How it fails:** Agent answers "how to implement this feature?" in chat without being asked to follow SDD; user has to know to explicitly read the skill first

2. **`rust-specialist/SKILL.md` has `paths: "**/*.rs"`** (only path frontmatter, no `description` field)
   - No clear trigger phrase in the loaded agent transcripts that says "read this skill when editing Rust"
   - **How it fails:** New agent doesn't know when to load the skill; if Cursor IDE loads by glob, it works; if agent uses semantic search, skill may not surface

3. **`react-ui-screen-implementation/SKILL.md` claims to follow `.claude/rules/react-frontend-patterns.md`** but that file contains directory rules that `.cursor/rules/react-frontend-patterns.mdc` omits
   - "Use when building or refactoring route screens, feature UI, hooks, and screen flows in desktop-app/src"
   - No explicit trigger phrase in the skill; relies on user saying "implement screens"
   - **How it fails:** User says "build a component" vs "implement a screen" — same task, but different trigger → may or may not invoke skill

4. **`spec-compliance-audit/SKILL.md` and `rust-code-audit/SKILL.md` both reference PRDs** but no skill triggers on "audit my code" or "check compliance"
   - Manual invocation only; no auto-discovery when a PR is opened
   - **How it fails:** Feature ships without being audited; user has to remember to read these skills

### HIGH: Skill `rust-specialist` embedded in `.claude/skills/` but also referenced as workspace rule

**Evidence:**
- `.claude/skills/rust-specialist/SKILL.md` line 4 sets `paths: "**/*.rs"`
- AGENTS.md line 76 says "Rules in `.claude/rules/` auto-load based on `paths` frontmatter"
- AGENTS.md lines 79-80 lists which rules load for which paths
- **But:** `.claude/skills/rust-specialist/SKILL.md` is a **skill**, not a rule; unclear if it auto-loads or must be explicitly read
- Agents reading `.claude/rules/rust-backend-standards.md` (lines 79) get different guidance than agents reading `.claude/skills/rust-specialist/SKILL.md` (lines 9-18)

**Risk:**
- Agent uses `.claude/rules/rust-backend-standards.md` (no `.unwrap()` rule explicitly stated)
- Different agent uses `.claude/skills/rust-specialist/SKILL.md` (line 16: "No `.unwrap()` in production code")
- Both are correct, but mixed application across agents creates inconsistency

### HIGH: `sdd/SKILL.md` prescribes skipping stages and gating but provides no override mechanism

**Evidence:**
- Lines 8, 23, 99, 150, 165: "Gate: do not proceed without meeting it"
- Lines 196-198: "Do not skip ahead" / "Spec is the contract"
- **But:** If a user says "skip phase 5" or "I already have a spec", the skill has no branch for that
- Phase 2 says "Show the spec to the user and wait for confirmation" (line 68) — blocks even if spec exists
- **How it fails in production:** User has an existing spec, asks SDD to implement it; SDD regenerates the spec anyway, wastes time, spec diverges from user intent

### MEDIUM: Stale guidance in `.claude/rules/react-frontend-patterns.md`

**Evidence:**
- Line 10: "Keep `desktop-app/src/screens/` as route roots only"
- Line 28: "Use Tailwind CSS utility classes as the default styling approach"
- Line 39: Domain hooks should own "DTO-to-view-model mapping"
- **But:** No reference to how to apply Alpen branding — `.claude/skills/react-ui-screen-implementation/SKILL.md` claims "Use Alpen branding" (line 3) and references `branding/` (line 13) as "Required Source"
- **Conflict:** Rules say nothing about branding; skill says read branding docs; agent may not know branding is a requirement

### MEDIUM: `use-case-testing-plan/SKILL.md` and `spec-compliance-audit/SKILL.md` both reference PRDs but don't specify which PRD

**Evidence:**
- `use-case-testing-plan/SKILL.md` lines 13-14: "read and anchor conclusions to: `docs/0-prd/01-multisig-ui.md`, `docs/0-prd/02-multisig-backend.md`"
- `spec-compliance-audit/SKILL.md` lines 13-14: identical requirement
- `react-code-audit/SKILL.md` lines 13-14: identical requirement
- `rust-code-audit/SKILL.md` lines 13-14: identical requirement
- **Near-duplicate:** Four skills have identical PRD loading boilerplate

### MEDIUM: `.cursor/rules/general.mdc` is a duplicate of AGENTS.md "Key Conventions" section

**Evidence:**
- `.cursor/rules/general.mdc` lines 2-16: exact copy of AGENTS.md lines 61-71
- Same frontmatter: `alwaysApply: true` (applied to all file edits)
- Single source of truth violated: changes to conventions must be made in **two places** (AGENTS.md + `.cursor/rules/general.mdc`)
- **Risk:** Developer updates AGENTS.md but forgets `.cursor/rules/general.mdc` → IDE rules and agent rules drift

### MEDIUM: `sprint-board/SKILL.md` has `disable-model-invocation: true` and is never triggered

**Evidence:**
- Lines 4, disabled so agent cannot invoke itself
- "Input: `$ARGUMENTS`" (line 11) — expects arguments at runtime
- No description or trigger phrase for auto-discovery
- **How it fails:** User reads `.claude/commands/ship.md` and doesn't know about `sprint-board` skill; creates PRs without adding items to the Sprint Board

### MEDIUM: Missing skill for "I want to refactor code safely"

**Evidence:**
- No skill for refactoring with property-based testing or mutation testing
- `.claude/skills/` contains audit skills but no "refactor guidance" skill
- AGENTS.md doesn't reference a refactoring methodology
- **Risk:** Developer refactors, claims "no behavior change" but has no systematic way to verify with mutation testing

### LOW: `.claude/skills/sprint-board/SKILL.md` uses deprecated GraphQL mutation syntax

**Evidence:**
- Lines 136-143: uses `updateProjectV2DraftIssue` GraphQL mutation
- This is a real GitHub mutation but depends on exact project/draft-issue structure
- If project schema changes or GitHub deprecates the mutation, skill breaks silently
- **Not blocking** because it's a tool integration, not core agent guidance

---

## Attack narratives (3–6): "How this fails in production / for a signer / for maintainers"

### Attack 1: Frontend developer reads wrong rule, ships bad architecture

**Scenario:**
1. Developer opens VSCode with Cursor IDE, edits `desktop-app/src/my-feature/Button.tsx`
2. Cursor IDE loads `.cursor/rules/react-frontend-patterns.mdc` (because glob matches)
3. Developer reads `.cursor` version, which **does not mention** "Architecture by Domain" or the directory structure
4. Developer creates `src/domain/my-feature/components/Button.tsx` but adds business logic (API call, state mutation) inside the component
5. Reviewer reads `.claude/rules/react-frontend-patterns.md` (from AGENTS.md), which says "presentational: receive prepared props" (line 20)
6. **Conflict:** Developer built to `.cursor` guidance (weak on separation), reviewer expects `.claude` guidance (strict separation)
7. PR comments are confusing; developer feels attacked

### Attack 2: Agent generates Rust code with `.unwrap()`, rules disagree on when it's allowed

**Scenario:**
1. Agent loads `.claude/skills/rust-specialist/SKILL.md`, sees "No `.unwrap()` in production code" (line 16)
2. Agent generates Rust handler with typed `Result` error propagation using `?`
3. Reviewer checks against `.claude/rules/rust-backend-standards.md`, which does **not explicitly forbid `.unwrap()`** (line 18 says "return typed errors" but doesn't cite the skill's stricter rule)
4. Another PR by a different agent loads only `.claude/rules/rust-backend-standards.md`, generates code with `.unwrap()` in a test helper
5. CI passes; inconsistency spreads
6. **Signer safety impact:** If `.unwrap()` is in production error path and a signer experiences an edge case, panic → app crash → lost signature opportunity

### Attack 3: User invokes SDD with existing spec, SDD regenerates spec anyway, divergence occurs

**Scenario:**
1. User has a well-reviewed spec at `docs/specs/my-feature.md`
2. User reads `.claude/skills/sdd/SKILL.md` Phase 2, wants to skip to implementation
3. **Skill has no override branch** for "I have a spec already"; Phase 2 always generates a spec
4. Agent overwrites existing spec with a new version that subtly differs
5. Implementation tests against new spec; code passes tests but doesn't match original business intent
6. **Authority/signer impact:** Signer expects one behavior; implementation delivers another

### Attack 4: Rule duplication causes agent confusion about "where to put code"

**Scenario:**
1. Agent A (Cursor IDE, using `.cursor/rules/react-frontend-patterns.mdc`) builds a modal
2. Agent B (Claude Code, reading `.claude/rules/react-frontend-patterns.md`) reviews the code
3. Agent A placed the modal's state in `src/domain/my-feature/hooks/` (per `.cursor` which omits the "Separation of Responsibilities" section)
4. Agent B says "state should be in hook but must be clearly separated from presentational logic" (per `.claude` which specifies "Domain hooks own state, async effects, validation, and DTO-to-view-model mapping")
5. Agent A says "but my IDE rule doesn't mention DTO-to-view-model separation" and resists the feedback
6. **Maintainability impact:** Two agents building the same codebase in two slightly different styles; future developers see inconsistent patterns

### Attack 5: Skill never triggers because user doesn't know about it

**Scenario:**
1. User ships a feature without running compliance audit
2. User writes: "I'm shipping my feature, can you review for spec compliance?"
3. Agent doesn't recognize the intent as "run spec-compliance-audit"
4. Agent does a general code review instead
5. Spec compliance gaps are not caught until production
6. **Risk:** Signer safety or protocol alignment issue ships

### Attack 6: Frontmatter mismatch causes rule to load in wrong context

**Scenario:**
1. `.claude/rules/typescript-standards.md` has `paths: ["desktop-app/src/**/*.{ts,tsx}"]` (YAML list)
2. `.cursor/rules/typescript-standards.mdc` has `globs: "desktop-app/src/**/*.{ts,tsx}"` (string)
3. Agent parsing `.claude` version treats `paths` as explicit file paths to load
4. IDE parsing `.cursor` version treats `globs` as glob patterns
5. **File matching differs:** YAML list might be interpreted as individual filenames, not patterns
6. One system loads the rule; the other doesn't
7. **Result:** inconsistent guidance depending on which system loaded the rule

---

## Evidence index (paths)

### Critical conflicts:
- `.claude/rules/typescript-standards.md` vs `.cursor/rules/typescript-standards.mdc` — **4 lines missing in `.cursor`**
- `.claude/rules/react-frontend-patterns.md` vs `.cursor/rules/react-frontend-patterns.mdc` — **27 lines missing in `.cursor` (39% of content, including entire "Architecture by Domain" section)**
- `.cursor/rules/general.mdc` vs AGENTS.md lines 61-71 — **exact duplicate, violates single source of truth**

### Stale/unclear guidance:
- `.claude/rules/react-frontend-patterns.md` line 28: mentions Tailwind but no branding guidance
- `.claude/skills/react-ui-screen-implementation/SKILL.md` line 3: claims to use branding but not referenced in rules
- `.claude/skills/sdd/SKILL.md` line 20: references "`.claude/skills/rust-specialist/SKILL.md`" (skill-to-skill dependency)

### Weak triggering criteria:
- `.claude/skills/sdd/SKILL.md` — `disable-model-invocation: true`, no `description` field
- `.claude/skills/sprint-board/SKILL.md` — `disable-model-invocation: true`, no `description` field
- `.claude/skills/rust-specialist/SKILL.md` — `paths: "**/*.rs"` (no glob pattern in frontmatter; frontmatter format unclear)
- `react-ui-screen-implementation/SKILL.md` — no explicit trigger phrase in name or description

### Duplicate guidance:
- Four audit skills reference identical PRD paths: `use-case-testing-plan`, `spec-compliance-audit`, `react-code-audit`, `rust-code-audit`
- `.cursor/rules/general.mdc` copies AGENTS.md Key Conventions verbatim

### Missing guidance:
- No skill for "safely refactor code" with mutation testing verification
- No skill for "when to read a specific skill" (meta-guidance)
- `.cursor/rules/` does not include architecture-by-domain guidance despite being applied to frontend edits

---

## Smallest fixes vs largest bets (be explicit)

### Smallest fixes (1-3 lines each, highest impact per diff):

1. **Align `.cursor/rules/` with `.claude/rules/` by deletion or unification**
   - Option A: Delete `.cursor/rules/` and rely on AGENTS.md to load rules
   - Option B: Mirror `.cursor/rules/` to match `.claude/rules/` content exactly
   - Option C: Keep `.cursor/rules/` as cached/compiled copies and document that `.claude/rules/` is the source of truth
   - **Recommended:** Option A (delete `.cursor/rules/`, let AGENTS.md be the source of truth via Cursor's rule-loading mechanism)
   - **Diff:** Remove 5 files from `.cursor/rules/`; update AGENTS.md to say "Rules are auto-loaded from `.claude/rules/` by Cursor IDE"

2. **Remove `.cursor/rules/general.mdc` (duplicate of AGENTS.md)**
   - Delete the file entirely; AGENTS.md Key Conventions is the single source of truth
   - **Diff:** 17 lines deleted

3. **Add `description:` field to `sdd/SKILL.md` and `sprint-board/SKILL.md`**
   - Enables auto-discovery and better explains when to use the skill
   - Example: `description: "Spec-Driven Development: generates a detailed spec, implements with TDD, runs exhaustive verification, and creates a PR."`
   - **Diff:** 1 line per skill

4. **Update `.cursor/rules/react-frontend-patterns.mdc` to include "Architecture by Domain" section**
   - Copy lines 10-15 from `.claude/rules/react-frontend-patterns.md`
   - **Diff:** 8 lines added

5. **Update `.cursor/rules/typescript-standards.mdc` to include DTO/view-model separation guidance**
   - Copy lines 18-21 from `.claude/rules/typescript-standards.md`
   - **Diff:** 4 lines added

### Medium fixes (architectural):

6. **Consolidate audit skill boilerplate by creating a shared `audit-preamble/` skill or macro**
   - `use-case-testing-plan`, `spec-compliance-audit`, `react-code-audit`, `rust-code-audit` all start with identical PRD-loading instructions
   - Option: Create a small include or document that audit skills reference: "First, read the PRDs (see `audit-preamble/SKILL.md`)"
   - **Diff:** Remove 3 lines from each of 4 skills; add 1 reference line to each; create 1 new skill file

7. **Add meta-guidance skill: "When to use which skill?"**
   - Create `.claude/skills/skill-selection/SKILL.md` that maps user intents to skills
   - Example: "User says 'audit my code' → load `react-code-audit` or `rust-code-audit`; 'implement a feature' → load `sdd`"
   - **Diff:** 1 new 50-line skill file

### Largest bets (breaking changes, high risk):

8. **Refactor SDD to support "user provides spec" branch**
   - Add Phase 0: "Check if spec exists at `docs/specs/<feature-slug>.md`; if yes, ask user: regenerate or use existing?"
   - **Diff:** 20 lines added to SDD skill; changes workflow for users who have existing specs
   - **Risk:** May break user workflows that assume SDD always generates a spec

9. **Create a unified rule validation system (`.nwave/rule-validator.md`)**
   - Tool that checks for:
     - Duplication between `.claude/rules/` and `.cursor/rules/`
     - Stale paths (e.g., references to deleted directories)
     - Missing `description` fields in skills
     - Conflicting guidance between skills
   - **Diff:** 1 new 100+ line validation tool; integration into CI/pre-commit
   - **Timeline:** 1-2 days to design and implement

---

## What would change my mind (missing evidence / experiments)

### Evidence I could not gather:
1. **How does Cursor IDE actually load `.cursor/rules/`?** Does it prioritize `.cursor/` over `.claude/`? Exact loading order unclear from AGENTS.md.
2. **Are agents actually using `.cursor/rules/` or ignoring them?** No telemetry or test results showing which rule file is loaded in practice.
3. **Is `.cursor/rules/` intentionally a cache/backup, or a bug?** No commit history or documentation explaining why both `.claude/` and `.cursor/` versions exist.

### Experiments that would help:
1. **Run a skill trigger test:** Configure an agent to log every time `.claude/rules/` or `.cursor/rules/` is loaded; run 10 edits across different file types; measure which rule file loads each time.
2. **Run a compliance audit:** Generate code following `.cursor/rules/react-frontend-patterns.mdc` only (omitting `.claude/` version); measure if architecture violations occur that the full `.claude/` rule would catch.
3. **Survey developers:** Ask 3 developers if they've noticed conflicting guidance from IDE vs agent; if so, what was the impact?

### Assumptions I'm making:
1. Agents should load `.claude/rules/` as the source of truth (not `.cursor/rules/`)
2. `.cursor/rules/` is a bug or legacy artifact (not intentional caching)
3. Agents and IDE should follow identical rules (not different variants)

---

## Summary

This repository has a **high-severity rule/skill conflict** that violates single-source-of-truth principles:

1. **Duplicate rule files** (`.claude/rules/` vs `.cursor/rules/`) with **substantive content differences** (27-line gap in react-frontend-patterns)
2. **Weak skill triggering** (4 skills have no `description` field; 2 have `disable-model-invocation: true` with no override)
3. **Stale architecture guidance** in rules that don't reference branding requirements
4. **Unnecessary duplication** (`.cursor/rules/general.mdc` copies AGENTS.md)
5. **No systematic way** for agents to decide which rule to follow when rules conflict

**Immediate action required:**
- Delete `.cursor/rules/` or unify it with `.claude/rules/` (not both)
- Add `description` fields to 6 skills that lack them
- Update `.cursor/rules/react-frontend-patterns.mdc` and `.typescript-standards.mdc` to match `.claude/` full content

**Risk to signers:** Architecture violations (missing DTO/view-model separation) and safety gaps (unclear where authority context belongs) could propagate if agents follow incomplete rules.
