# Documentation (DIVIO / Diataxis) — Adversarial Assessment

**Assessment date:** 2026-05-14  
**Mode:** Read-only adversarial audit  
**Scope:** Documentation typing (tutorial vs how-to vs reference vs explanation), runbooks, cross-links, superseded discovery, and drift between PRD/specs and runnable reality.

---

## 1. Scope & threat model

**Failure modes**

1. **Collapsed types** — One file mixing onboarding, commands, and architecture without a clear reader goal.
2. **Missing runbooks** — Ops cannot restart, migrate, or back the orchestrator from repo docs alone.
3. **NAV failure** — Specs and ADRs exist but do not point to each other; readers land on stale POCs via search.
4. **Invariant folklore** — “Backend is coordination only” lives in agent rules but lacks a formal, reviewable doc tied to SPS language.
5. **Capability drift** — PRD lists update types; discovery says some are blocked upstream; nowhere is the matrix “live.”

---

## 2. Top findings (ranked)

### Blocking / high

**1. No first-class backend operations / incident runbook in-repo**

- **Risk:** Production or long-running testnet incidents become Slack folklore; recovery time and data loss increase.
- **Evidence:**
  - `docs/architecture/overview.md` — System view; not a substitute for deploy/backup/health/migrations.
  - Glob search: no `*runbook*`, `*ops*`, `*deploy*` under `docs/` with operational focus (assessment snapshot).
  - `AGENTS.md` — Dev commands (`cargo run -p orchestrator-be`), not production topology.

**2. Coordination-only boundary is stated in conventions but not formalized as an architecture gate**

- **Risk:** A well-meaning PR adds threshold or sequence validation to the backend; reviewers lack a single checklist citation.
- **Evidence:**
  - `AGENTS.md` / `.cursor/rules/general.mdc` — Coordination-only rule.
  - `docs/0-prd/02-multisig-backend.md` §1 — Canonical validity (including threshold checks) MUST be on-chain only.
  - **No** `docs/architecture/adrs/006-*.md` (only 001–005 on disk) documenting enforcement: forbidden deps, review checklist, test hooks.

**3. Signer-safety UX not consolidated into one normative doc**

- **Risk:** Implementations vary by screen; hardware wallet constraints from discovery are not always carried into AC.
- **Evidence:**
  - PRD signer-safety language in `docs/0-prd/01-multisig-ui.md`.
  - Discovery: `docs/2-discovery/16-poc5-trezor-findings.md`.
  - Feature specs distributed under `docs/specs/` without a single `signer-safety` or `confirmation` reference doc flagged in architecture index.

### Medium

**4. README vs AGENTS.md role blur**

- **Risk:** New contributors repeat environment setup failures; “when do I need Tauri?” stays ambiguous.
- **Evidence:** `README.md` (short monolith), `AGENTS.md` (long command catalog) — Diataxis would split “first 15 minutes” from “daily reference.”

**5. Specs ↔ ADRs ↔ discovery cross-links incomplete**

- **Risk:** Implementers satisfy a spec while violating an ADR they never opened.
- **Evidence:** Many `docs/specs/*.md` files lack a stable “Related” header linking `docs/architecture/adrs/` and `docs/2-discovery/` entries.

**6. Superseded discovery docs**

- **Risk:** Search lands on POC conclusions that ADRs replaced.
- **Evidence:** `docs/2-discovery/README.md` tracks status; individual older POC files may lack prominent supersession banners (spot-check before relying on historical claims).

### Low

- **7.** Missing standalone **testing-strategy** and **release reproducibility** guides as Diataxis artifacts (referenced by PRD themes elsewhere but not consolidated).

---

## 3. Attack narratives (3–6)

### N1: 02:00 orchestrator outage

On-call opens the repo. There is no runbook for Postgres, migrations, or “safe restart with pending proposals.” They guess; a duplicate migration corrupts state.

### N2: Compliance asks “prove backend does not re-validate SPS-65”

The answer cites `AGENTS.md`. Auditors ask for signed architecture baseline; there is no ADR + dependency lint story. The conversation stalls.

### N3: UX ships without device parity doc

QA asks what MUST appear before “Sign.” Engineers point to three specs + one POC. QA misses a Trezor limitation; a release goes out with misleading confirmation copy.

### N4: Contributor implements bridge param UI

They never see `docs/2-discovery/08-alpen-crate-prd-coverage.md` or a capability matrix; blocked upstream work is rediscovered in week three.

### N5: Good architecture overview, bad navigation

`docs/architecture/overview.md` is accurate but does not enumerate sibling specs; a new hire reads overview once and lives in `specs/` without ADR context.

### N6: Incident post-mortem demands “single source of truth”

Wiki page contradicts `docs/specs/`; neither side links to Notion SPS. Root cause: no declared SSOT map in-repo.

---

## 4. Evidence index (paths)

| Kind | Path |
|------|------|
| Entry / commands | `README.md`, `AGENTS.md`, `CLAUDE.md` |
| Architecture | `docs/architecture/overview.md`, `docs/architecture/adrs/001`–`005` |
| PRD | `docs/0-prd/01-multisig-ui.md`, `docs/0-prd/02-multisig-backend.md` |
| Proposal / scope | `docs/1-proposal/` |
| Discovery index | `docs/2-discovery/README.md`, `docs/2-discovery/08-alpen-crate-prd-coverage.md`, `docs/2-discovery/16-poc5-trezor-findings.md` |
| Stories / NFR | `docs/3-stories/story-map.md`, `docs/3-stories/non-functional-items.md` |
| Feature specs | `docs/specs/` (many files) |
| Deliverable research | `docs/deliverable/research.md`, `docs/deliverable/crate-inventory.md` |
| Cursor rules (conventions) | `.cursor/rules/general.mdc`, `.claude/rules/*.md` |

---

## 5. Smallest fixes vs. largest bets

**Smallest**

- Add **“Related architecture / ADR / discovery”** headers to the 5–10 hottest specs (signing, creation, broadcast).
- Add **superseded banners** to discovery files called out in `docs/2-discovery/README.md`.
- In `docs/architecture/overview.md`, add a short **index** of ADRs + `docs/specs/` clusters.

**Largest**

- Write `docs/architecture/backend-operations.md` (deploy, config, health, backups, migrations, failure modes).
- Add **ADR-006** (coordination boundary): forbidden imports, review checklist, link to tests that prove “no duplicate protocol state machine.”
- Add **`signer-safety-model.md`** (or equivalent) bridging PRD + HW limitations + spec AC.
- Maintain **`capability-matrix.md`** (authority × update type × upstream/backend/UI/e2e) with owners.

---

## 6. What would change my mind

- A **private but linked** runbook (if org policy forbids in-repo ops) would partially mitigate finding 1 **if** the repo README points to it as SSOT for ops.
- An **automated doc check** (CI) that fails when spec headings lack ADR links would downgrade NAV concerns from cultural to mechanical.
- A **published SPS excerpt pack** in-repo (even partial) would reduce “Notion folklore” risk called out in research assessments.
