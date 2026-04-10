---
name: sdd
description: "Spec-Driven Development: generates a detailed spec from a prompt, implements with TDD, runs exhaustive verification, and creates a PR."
disable-model-invocation: true
---

# Spec-Driven Development (SDD)

You receive a description of what needs to be implemented. Follow this process phase by phase, never skipping any. Mark each phase with a heading as you complete it.

**Input:** $ARGUMENTS

---

## Phase 1 — Understand the context

1. Read relevant documents in `docs/` (PRD, proposal, discovery, architecture)
2. Read existing code related to the request
3. Read applicable rules in `.claude/rules/`
4. Read `.claude/skills/rust-specialist/SKILL.md` — all Rust code must follow its standards
5. Identify which authorities, types, and protocol flows are involved

**Gate:** Do not proceed until scope and protocol constraints are clear.

---

## Phase 2 — Generate Spec

Create a spec document at `docs/specs/<feature-slug>.md` with this format:

```markdown
# Spec: <feature name>

## Objective
<what we want to achieve and why>

## Scope
<what is included and what is NOT included>

## Technical Design
<types, structs, functions, endpoints, components involved>
<flow diagrams if applicable>

### Production code vs. test helpers
Clearly separate:
- **Production functions**: reusable, exposed to consumers (Tauri commands, lib API, etc.)
- **Test helpers**: utilities for test setup only (key generation, fixture builders, etc.)

Test helpers must NOT be registered as Tauri commands or exposed in production APIs unless explicitly requested.

## Test Cases
<exhaustive list of scenarios to test>
Tests must target production functions only — not test helpers.
- Happy path
- Edge cases
- Expected errors
- Authority isolation (if applicable)
- Offline fallback (if applicable)

## Module structure
Describe how code should be organized for reuse across crates (e.g., a shared lib that both desktop-app and e2e-tests can consume).

For each new file/module, state its **single responsibility** in one sentence. If you can't describe it in one sentence, the module is doing too much — split it.

Verify dependency direction: business logic depends on abstractions (traits), not the reverse. Types shared between trait and business logic belong in their own module or with the trait.
```

Show the spec to the user and wait for confirmation before continuing. Ask: **"Does the spec look good? Any changes before implementation?"**

---

## Phase 3 — Branch

```bash
git checkout -b feature/<feature-slug>
```

If the branch already exists, ask the user what to do.

---

## Phase 4 + 5 — TDD: Incremental red-green cycles

Do NOT write all tests at once and then implement everything. Work in small incremental cycles, one function at a time:

### For each production function in the spec:

1. **Red:** Write the test(s) for that single function
   - Only test production functions, not test helpers
   - Run tests — they must fail for the right reason
2. **Green:** Write the minimum implementation to pass those tests
   - Follow `.claude/skills/rust-specialist/SKILL.md` conventions
   - Rust: `thiserror` for libs, `anyhow` for binaries, no `.unwrap()` in production, iterators over loops, `pub(crate)` by default
3. **Verify:** Run tests — they must pass
4. **Move to the next function**

Repeat until all spec functions are implemented and tested.

**Gate:** All tests pass. Each production function has corresponding tests.

---

## Phase 6 — Refactor

### 6a. Code-level cleanup

Review the implemented code looking for:
- Duplication (especially repetitive patterns like error handling, request/response mapping)
- Functions that are too long (>40 lines)
- Non-descriptive names
- Types that could be stricter
- Test helpers mixed with production code — separate them
- Production code that should be extractable into a shared module/crate

### 6b. Structural review — separation of concerns

This is the most important part of refactoring. Review the module structure looking for:

- **Single Responsibility:** Does each file/module have one clear reason to change? A file with types + trait + implementation + business logic + tests is a red flag.
- **Dependency direction:** Do high-level modules (business logic) depend on abstractions, not the reverse? If a trait file imports from a business logic file, the dependency is inverted.
- **Cohesion:** Are things that change together located together? Types that define a contract (DTOs, request/response) should live with or near the contract they define (e.g., the client trait), not in the business logic that uses them.
- **God files:** If a single file has >200 lines of production code (excluding tests), consider splitting by responsibility: types, traits/interfaces, implementations, business logic.
- **Repetitive patterns:** If multiple methods repeat the same pattern (e.g., token → build request → check status → deserialize), extract a helper method to reduce boilerplate.

### 6c. Apply and verify

1. Refactor while keeping tests green
2. Run tests again to confirm
3. Verify that each file has a clear, single responsibility you can describe in one sentence

---

## Phase 7 — Exhaustive verification

Run everything and report results:

```bash
cargo build
cargo test
cargo clippy
cargo fmt --check
```

If frontend is involved:
```bash
cd desktop-app && npm run build
```

**Gate:** Zero errors, zero clippy warnings, correct formatting. Fix anything that fails before continuing.

---

## Phase 7b — Documentation update

If the implementation introduced or changed any of the following, update the relevant docs:

- **New modules, files, or directory structure** → update `docs/architecture/overview.md` (file layout diagrams, module descriptions)
- **New abstractions (traits, ports, service boundaries)** → update the relevant ADR or create a new one in `docs/architecture/adrs/`
- **Changes to data flow or communication patterns** → update sequence/flow diagrams in `docs/architecture/overview.md`
- **New dependencies** → verify `docs/architecture/adrs/001-alpen-crate-dependencies.md` is up to date
- **POC/discovery progress** → update status in `docs/2-discovery/` plan documents (e.g., slice status from "Planning" to "In progress" or "Done")

Only update docs that are actually affected by the changes. Do not create new documentation files unless the change introduces a concept not covered by existing docs.

**Gate:** Architecture docs reflect the current state of the code. No stale diagrams or descriptions.

---

## Phase 8 — PR

1. Commit with a descriptive message:
   ```
   feat(<scope>): <concise description>
   ```
2. Push the branch:
   ```bash
   git push -u origin feature/<feature-slug>
   ```
3. Create the PR with `gh`:
   ```bash
   gh pr create --title "feat(<scope>): <description>" --body "..."
   ```
   The PR body must include:
   - Link to the spec (`docs/specs/<feature-slug>.md`)
   - Summary of changes
   - Test plan (what tests were added and what they cover)

4. Add reviewer:
   ```bash
   gh pr edit <pr-number> --add-reviewer juandahl
   ```
5. Show the PR link to the user.

---

## General rules

- **Do not skip ahead:** Each phase has a gate; do not advance without meeting it
- **Spec is the contract:** Implementation must match the spec. If you discover the spec needs changes, update the spec first and notify the user
- **Offline survivability:** If the feature touches the backend, ensure the system still works without it
- **Signer safety:** Never expose private keys in UI, logs, or storage
- **Production vs. test separation:** Test helpers (key generators, fixture builders, demo actions) must live in `#[cfg(test)]` blocks or dedicated test modules — never in production paths
- **Rust standards:** All Rust code must follow `.claude/skills/rust-specialist/SKILL.md`
- **Reusability:** When signing/crypto logic is shared across crates, extract it into a module that can be consumed by both desktop-app and e2e-tests
