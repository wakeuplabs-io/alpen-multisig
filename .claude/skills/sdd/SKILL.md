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
4. Identify which authorities, types, and protocol flows are involved

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

## Test Cases
<exhaustive list of scenarios to test>
- Happy path
- Edge cases
- Expected errors
- Authority isolation (if applicable)
- Offline fallback (if applicable)

```

Show the spec to the user and wait for confirmation before continuing. Ask: **"Does the spec look good? Any changes before implementation?"**

---

## Phase 3 — Branch

```bash
git checkout -b feature/<feature-slug>
```

If the branch already exists, ask the user what to do.

---

## Phase 4 — TDD: Tests first

1. Write tests based on the spec's test cases
   - Backend (Rust): unit tests in the same file (`#[cfg(test)] mod tests`) or in `tests/`
   - Frontend (TypeScript): test files as `.test.ts` / `.test.tsx`
2. Run the tests and **verify they fail** (red phase):
   ```bash
   cargo test -p orchestator-be
   ```
3. **Do NOT write implementation code yet**

**Gate:** All tests must exist and fail for the right reason (not due to meaningless compilation errors).

---

## Phase 5 — Implementation (Green phase)

1. Implement the minimum code to make the tests pass
2. Follow project conventions:
   - Backend: thin handlers, logic in domain/, `thiserror` for libs, `anyhow` for binaries
   - Frontend: function components, `use*` hooks, strict types, tabs, single quotes
3. Run tests after each significant change:
   ```bash
   cargo test -p orchestator-be
   ```
4. Iterate until **all tests pass** (green phase)

---

## Phase 6 — Refactor

1. Review the implemented code looking for:
   - Duplication
   - Functions that are too long (>40 lines)
   - Non-descriptive names
   - Types that could be stricter
2. Refactor while keeping tests green
3. Run tests again to confirm:
   ```bash
   cargo test -p orchestator-be
   ```

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

4. Show the PR link to the user.

---

## General rules

- **Do not skip ahead:** Each phase has a gate; do not advance without meeting it
- **Spec is the contract:** Implementation must match the spec. If you discover the spec needs changes, update the spec first and notify the user
- **Offline survivability:** If the feature touches the backend, ensure the system still works without it
- **Signer safety:** Never expose private keys in UI, logs, or storage

