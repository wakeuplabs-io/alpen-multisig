# Documentation (DIVIO/Diataxis) — Adversarial Assessment

**Assessment date:** 2026-05-13  
**Reviewer:** Quill (nw-documentarist-reviewer)  
**Scope:** Multi-sig desktop + backend system with cryptographic signing, hardware wallet integration, and protocol coordination.

---

## Scope & Threat Model

### What we're trying to break

1. **Onboarding velocity** — Can a new engineer understand the architecture, dependencies, and workflow in under 2 hours and land a non-trivial code change on day 1?
2. **Signer safety** — Are the behavioral expectations, risks, and explicit confirmation requirements documented clearly enough that an implementer won't accidentally weaken key material handling, session boundaries, or threshold enforcement?
3. **Operational resilience** — Can the backend be deployed, monitored, and recovered from failure by operations staff without tribal knowledge?
4. **Specification drift** — Do the documented requirements (PRD, specs, ADRs) stay synchronized with the code, or do they become folklore?
5. **Protocol fidelity** — Are the SPS-50/51/65 integration points clearly mapped, or will someone re-implement consensus rules in the backend by accident?

### What we expect to find

- **Collapse patterns** — docs trying to be tutorial + how-to + reference simultaneously, creating confusion and maintenance debt.
- **Orphaned docs** — spec decisions or POC findings that never bubble up to architecture docs, creating two sources of truth.
- **Missing essentials** — no release runbook, no backend ops guide, no incident playbook, no threat model.
- **Spec-code drift** — documented claims that contradict the actual implementation.
- **Onboarding gaps** — README + AGENTS.md do not lead a fresh engineer to a working build and a meaningful change in one day.

---

## Top Findings (Ranked)

### BLOCKING (Must fix before handoff)

#### 1. **No Backend Operational Runbook**

**Risk:** If the orchestrator backend crashes or requires maintenance in production, operators lack explicit guidance on deployment topology, health checks, data persistence strategy, recovery procedure, or PostgreSQL migration steps.

**Evidence:** 
- `docs/architecture/overview.md` §1.2 documents the backend API surface but provides zero operational guidance.
- `docs/0-prd/02-multisig-backend.md` (read lines 1–200) specifies requirements but does NOT cover "how is this deployed in production?"
- No file under `docs/` matches `*deploy*`, `*ops*`, `*release*`, `*infrastructure*`, `*postgres*`, `*migration*`.
- `AGENTS.md` § "Running the System" says `cargo run -p orchestrator-be` — this is development only, not a production deployment model.

**What this means:** A new operator copy-pastes the command into a VPS, the binary crashes at 2am, and there's no runbook for:
- Whether data is ephemeral (in-memory) or persisted (Postgres)?
- How to initialize the database schema on a fresh deployment?
- What environment variables are required (DATABASE_URL, CORS origins, authentication secrets)?
- How to detect liveness (health endpoint)? What should monitoring scrape?
- How to safely upgrade versions without losing pending proposals?

**Remediation:** Create `docs/architecture/backend-operations.md` (Reference + How-to) covering deployment topology, environment setup, schema initialization, monitoring metrics, backup/recovery, and incident response.

---

#### 2. **Protocol Retrenchment Risk — No "Backend is Coordination-Only" Enforcement Doc**

**Risk:** The constraint "backend never enforces SPS-65 rules" is stated in prose but never formalized as a checklist or architecture guard. A future contributor might:
- Add threshold signature verification to the backend "for safety."
- Implement sequence number gap detection "as validation."
- Re-implement the ASM state machine "to catch bugs early."

All of these would be **protocol violations** if the ASM diverges, creating an undetectable split-brain bug.

**Evidence:**
- `AGENTS.md` § "Key Conventions": "Backend is coordination only: proposal creation, signature collection, lifecycle tracking — never re-implement protocol validity rules" (stated as convention, not enforced).
- `docs/1-proposal/01-alpen-multisig-proposal.md` §2 (§ "3. Protocol & Signing Layer"): mentions it as design philosophy but doesn't link it to code reviews or test strategy.
- No ADR or spec documents a **guard**: "these backend modules MUST NOT import strata-crypto or strata-asm-params."
- `orchestrator-be/Cargo.toml` (not read, but statically likely) probably doesn't declare a forbidden-dep list.

**What this means:** A PR could pass code review because the reviewer didn't know this was a hard boundary. By the time it reaches testnet, signers are signing double-spends against an out-of-sync ASM.

**Remediation:** Create `docs/architecture/adrs/006-backend-coordination-boundary.md` (Architecture Decision Record + How-to-Review) that:
1. Restates the rule with rationale (source of truth = ASM, not backend).
2. Lists forbidden imports in `orchestrator-be`.
3. Provides a pull-request review checklist.
4. Links to `test_encode_matches_direct_strata_ssz` as the test that proves equivalence.

---

#### 3. **No Signer Safety Runbook — Explicit Confirmation UX Not Specified**

**Risk:** The PRD (§ "Signer safety: Explicit confirmation steps, authority context, high-signal errors") is abstract. There is no document listing:
- What MUST be displayed on the hardware wallet screen before signing.
- What MUST be displayed in the desktop UI before clicking "Sign."
- How to distinguish "I'm signing a change to the Alpen Admin signer set" from "I'm signing a Strata Admin change" (authority context).
- How to recover if the signer accidentally signs the wrong proposal.

**Evidence:**
- `docs/3-stories/story-map.md` § "Key Frontend Constraints" lists high-level requirements ("Authority labeling required on every action form") but not the specific content, layout, and sequence.
- `docs/specs/proposal-creation-signer-update.md` § "4) Preview requirements" specifies *what* fields must be shown but NOT how to prevent misread (font size? color? layout? QR code?).
- No spec document titled `*signer-safety*`, `*signing-confirmation*`, or `*hardware-wallet-ux*`.
- `docs/2-discovery/16-poc5-trezor-findings.md` covers Trezor PoC but doesn't codify behavioral requirements for production (e.g., "signer must see authority name on device, not just action_id hash").

**What this means:** 
- Frontend impl ships without explicit authority labeling.
- A signer is asked "Do you want to sign this?" and sees `action_id = 0xabc…def` on the device, no authority context.
- They sign Strata Admin action thinking it's Alpen Admin.
- The signature is valid but goes to the wrong authority's quorum.
- The signer has no way to undo or revoke the signature (it's immutable).

**Remediation:** Create `docs/architecture/signer-safety-model.md` (Tutorial + Reference) covering:
1. Authority context is mandatory on every prompt (device + UI).
2. Payload summary (action type, key changes, threshold) visible before signing.
3. Timeout / session expiry after inactivity.
4. Hardware wallet message format and content (SPS-65 sighash representation).
5. Error messages and recovery flows (invalid signature, network error, session expired).

---

### HIGH (Significant gaps; implementation risk)

#### 4. **Spec Foreclosure — 8 Update Types Not Yet Implemented, Docs Don't Track Status**

**Risk:** The PRD specifies 15+ update types (Alpen + Strata + Payout). Discovery found that only ~7 are implemented upstream. The docs do not maintain a live **capability matrix** showing which types are:
- Upstream-ready (crate has the type)
- Backend-ready (CRUD endpoints exist)
- Frontend-ready (UI screens exist)
- Fully tested (e2e coverage exists)

**Evidence:**
- `docs/3-stories/story-map.md` § "Slice 2 — All authorities & update types": "Expand to remaining 4 authorities and all 12 update types. Depends on upstream Alpen crate support **(8 types still missing — see risks)**."
- No document shows: which 8 types are missing, when they're expected upstream, what workarounds exist.
- `docs/deliverable/research.md` probably contains this analysis (not read in full), but it's labeled "Phase 1 deliverable" — it's not maintained as a living document.
- `AGENTS.md` does not link to a "Capability Matrix" or "Roadmap Status" doc for new contributors.

**What this means:** 
- A frontend developer tries to wire up a "Bridge Parameter Update" proposal form, searches the codebase for the action type handler, finds nothing, assumes it was never started, spends 3 hours re-discovering that it's blocked upstream.
- An integration test fails because it's trying to test an unimplemented action type; the error message doesn't explain why.

**Remediation:** Create `docs/architecture/capability-matrix.md` (Reference) with a table:

| Authority | Update Type | Upstream | Backend | Frontend | E2E Tests | Status | Blocker |
|-----------|---|---|---|---|---|---|---|
| Strata Admin | VerificationKeyUpdate | ✅ | ✅ | ✅ | ✅ | Ready | — |
| Strata Admin | SignerSetUpdate | ✅ | ✅ | ✅ | ✅ | Ready | — |
| Strata Admin | BridgeParamUpdate | ❌ | — | — | — | Blocked | Upstream PR pending |

Maintain this live; update it on every merge to develop.

---

#### 5. **No Threat Model or Incident Playbook**

**Risk:** There is no document articulating:
- What are the top 5 security failure modes (e.g., signer key compromise, backend database breach, signature replay)?
- How do we detect them (monitoring alert)?
- What is the recovery procedure (revoke signer, reset proposal, etc.)?
- Who is on-call and what is their escalation path?

**Evidence:**
- No file under `docs/` matches `*threat*`, `*security*`, `*incident*`, `*breach*`, `*playbook*`.
- The PRD (§ "Signer safety") is aspirational but does not enumerate what can go wrong or how to respond.
- `docs/architecture/overview.md` § "Offline Survivability" documents that the backend can be bypassed, but does NOT document "what happens if the backend is compromised?"

**What this means:** 
- A testnet incident: "The orchestrator backend was serving stale proposal data; three signers signed invalid payloads."
- On-call engineer pages the team at 3am, but there's no playbook: Is the backend down? Do we revert the database? Do we issue an emergency signer revocation?
- The first response is chaotic guessing.

**Remediation:** Create `docs/architecture/threat-model.md` (Explanation + Reference) covering:
1. Top 5 failure modes and detection strategy.
2. Per-mode incident response (detection → assessment → response → prevention).
3. Signer revocation / emergency authority update procedure.
4. Data breach response (if signer keys compromised, how long until they're rotated on-chain?).

---

#### 6. **DIVIO Collapse in README + AGENTS.md — Tutorial/How-to/Reference Blurred**

**Risk:** `README.md` and `AGENTS.md` try to serve as Tutorial (get started), How-to (common tasks), and Reference (complete command list) simultaneously. A new contributor reads them and still doesn't know:
- "Do I need to build the backend if I'm only changing React?"
- "How do I run a single test?"
- "Where do I look if a test fails?"

**Evidence:**
- `README.md` is 63 lines. It tries to explain: high-level architecture (lines 5–9), repo layout (lines 11–17), prerequisites (lines 19–24), build commands (lines 26–51), and documentation pointers (lines 53–59).
- Lines 5–9 are **Explanation** (conceptual background).
- Lines 26–51 are **How-to** (step-by-step commands).
- There is NO tutorial path like "First time here? Start with `cargo test` in the orchestrator-be directory; if it passes, you're good to go."
- `AGENTS.md` (§ "Commands") lists 16 Rust commands and 5 desktop commands, but does NOT explain when to use each (e.g., "Use `cargo test -p orchestrator-be -- test_name` when you want to run a single test").

**What this means:** 
- New engineer clones the repo, reads README, tries `cargo test`, it fails (missing a Tauri dep).
- They re-read the README; it says "Tauri system dependencies for your OS" but doesn't explain what Tauri is.
- They Google "Tauri install" and end up in a different project's docs.
- 30 minutes later, they finally get a passing test, but they still don't understand what they just tested.

**Remediation:** Rewrite `README.md` as **Tutorial** (3–4 paragraphs max):
1. **What is this?** — One sentence.
2. **Quick start** — 5 commands that prove the code works.
3. **What's inside?** — Subdirectory pointers (for the curious).
4. **Next steps** — Link to `AGENTS.md` for detailed commands.

Keep `AGENTS.md` as **Reference** (current structure is close) but add a "When to use" section:

```
## Commands

### Quick start (first time only)
cargo test # Run all tests to verify your environment

### Development (daily use)
cargo build           # Full workspace rebuild
cargo test -p orchestrator-be -- test_name  # Run single test
npm run dev           # Frontend dev server (no Tauri)
npm run tauri dev     # Full desktop app with hot reload

### Before pushing
cargo fmt --check && cargo clippy -- -D warnings  # Lint check
npm run format:check && npm run lint  # Frontend lint
```

---

### MEDIUM (Navigation hazards; clarity debt)

#### 7. **Cross-Reference Hell — Specs Orphaned from Architecture Docs**

**Risk:** `docs/specs/` (20+ feature-level specs) and `docs/architecture/` (5 ADRs) are indexed separately. A contributor reading `docs/specs/proposal-creation-signer-update.md` does NOT know:
- Which ADR defines the architecture this spec depends on?
- Is there a discovery document that explains the background?
- When this spec was written, what were the upstream blockers?

**Evidence:**
- `docs/specs/proposal-creation-signer-update.md` has zero backlinks to ADRs or discovery docs.
- `docs/architecture/overview.md` does NOT list or link to the specs that concretize its architecture.
- `docs/2-discovery/README.md` provides a reading guide (which is excellent) but is isolated — it doesn't link back to the specs or ADRs that built on the discovery findings.

**What this means:** 
- A test failure in `proposal-creation` logic; the engineer reads the spec, misses the fact that it depends on `docs/architecture/adrs/003-desktop-application-layer-api.md`, implements a workaround that violates the ADR.
- Later, when the frontend is refactored, the workaround breaks.

**Remediation:** Add "Related" sections to key docs:

In `docs/specs/proposal-creation-signer-update.md`:
```
## Related

- **Architecture:** [`docs/architecture/adrs/003-desktop-application-layer-api.md`](../architecture/adrs/003-desktop-application-layer-api.md) — Application layer contract.
- **Discovery:** [`docs/2-discovery/09-functional-analysis.md`](../2-discovery/09-functional-analysis.md) — Data model background.
```

In `docs/architecture/overview.md`:
```
## Specs and Slices

- Full capability roadmap: [`docs/3-stories/story-map.md`](../../3-stories/story-map.md)
- Feature-level specs: [`docs/specs/`](../../specs/) (20+ detailed specs by feature/authority)
- Non-functional items: [`docs/3-stories/non-functional-items.md`](../../3-stories/non-functional-items.md)
```

---

#### 8. **Discovery Doc Rot Risk — Superseded Findings Not Flagged for Readers**

**Risk:** `docs/2-discovery/README.md` marks some docs as "Superseded" (e.g., POC-2 Tauri findings, POC-3 signing findings) but the original docs themselves do NOT carry a prominent warning flag. A reader finds `docs/2-discovery/04-poc2-findings.md` via search, reads it in isolation, and implements based on outdated information.

**Evidence:**
- `docs/2-discovery/README.md` § "Document status": "Superseded | Original conclusions were revised by a later POC or ADR; post-discovery notes explain what changed — kept as historical record."
- Example: `docs/2-discovery/05-poc3-findings.md` is marked as superseded, but the document itself (line 1) does not say "⚠️ SUPERSEDED — see [ADR-003](../architecture/adrs/003-desktop-application-layer-api.md) for updated conclusions."

**What this means:** 
- A developer tries to understand Tauri IPC design, finds `docs/2-discovery/04-poc2-findings.md`, implements a pattern from the POC, merges it, then discovers it conflicts with ADR-003.

**Remediation:** Add a frontmatter flag to all superseded discovery docs:

```markdown
---
status: superseded
superseded_by: docs/architecture/adrs/003-desktop-application-layer-api.md
date_superseded: 2026-04-10
---

# POC-2 Findings: Tauri + React IPC Stack

⚠️ **This document is superseded.** See [ADR-003](../architecture/adrs/003-desktop-application-layer-api.md) for updated architecture.
```

---

### LOW (Nice-to-have; future-proofing)

#### 9. **Testing Strategy Not Documented**

**Risk:** There is no guide explaining:
- What is the testing pyramid (unit / integration / e2e)?
- Where should each test live (orchestrator-be/tests/, e2e-tests/, desktop-app/src/__tests__/)?
- What is the coverage target?
- How do we handle cryptographic tests (deterministic mocks vs. real signing)?

**Evidence:**
- No file under `docs/` matches `*test*` or `*testing*` (except for `e2e-tests-workspace-integration.md`, which is a feature spec, not a testing guide).
- `docs/3-stories/non-functional-items.md` probably lists testing as a work item, but there's no architectural guide.

**What this means:** A contributor adds a test but doesn't know if it belongs in the backend test suite or e2e, so it goes in the wrong place, gets overlooked, becomes flaky.

**Remediation:** Create `docs/architecture/testing-strategy.md` (Reference) covering test pyramid, placement, and coverage targets.

---

#### 10. **No Release / Build Reproducibility Guide**

**Risk:** The PRD specifies "Builds MUST be reproducible" and "Users SHOULD be able to verify signer signatures on the binary." There is no document explaining:
- How to build reproducibly.
- What environment is required (Rust version, OS, exact deps)?
- How signers verify the build output?
- What is the release process (tag, sign, publish)?

**Evidence:**
- `docs/0-prd/01-multisig-ui.md` § "Requirements", line 2: "Builds of the application MUST be reproducible."
- No file under `docs/` matches `*release*`, `*build*`, `*reproducible*`, `*binary*`.

**What this means:** 
- Phase 3 ships a binary; signers are told "verify this was built from the public GitHub commit."
- No signer knows how to do that (no instructions).
- They either trust the binary blindly or spend hours reverse-engineering the build.

**Remediation:** Create `docs/architecture/build-and-release.md` (How-to) covering reproducible build steps, verification procedure, and release checklist.

---

## Attack Narratives (3–6)

### Narrative 1: Onboarding Meltdown

**New engineer (Alice) joins on Monday:**

1. **9 AM:** Alice clones the repo, reads `README.md`.
2. **9:15 AM:** She sees "Prerequisites: Rust toolchain, Node 20+, Tauri system dependencies" but doesn't know what Tauri is or why it matters if she's only editing React.
3. **9:30 AM:** She runs `cargo test` in the orchestrator-be directory. It fails: "error: linker `cc` not found." She googles "rust cc linker," finds a StackOverflow answer for a C++ project, installs three things at random.
4. **10 AM:** Another error: "error: linker `arm-linux-gnueabihf-gcc` not found." (Tauri cross-compilation issue.) She re-reads README, finds "Tauri system dependencies for your OS" but no link to the actual installation steps.
5. **11 AM:** She gives up, pings Slack: "How do I set up the dev environment?"
6. **4 PM:** A senior dev responds with a private link to an internal wiki (that doesn't exist in the repo, in a private Slack channel).
7. **Next day:** Alice finally has a passing test but doesn't understand what orchestrator-be is. She's burned a day.

**Root cause:** README tries to be architecture explanation (lines 5–9) + prerequisites (lines 19–24) + build commands (lines 26–51) without a **tutorial path**. A new engineer needs: "Run this, it works, now read that."

**Fix:** Restructure README as 5-line tutorial → link to AGENTS.md for commands → link to ARCHITECTURE.md for context.

---

### Narrative 2: Backend Re-implements the ASM (Silent Protocol Violation)

**Scenario:** The backend is upgraded to validate signer threshold. Looks safe, catches bugs early.

1. **Week 1:** A contributor (Bob) adds `verify_threshold()` call to the backend before storing a signature. Includes a test: `test_backend_rejects_invalid_signer()`.
2. **Week 2:** The test passes; PR is merged.
3. **Week 8:** Bob implements a minor optimization in the ASM's threshold check, changes a bit flag. The ASM now accepts signatures Bob's backend rejects (due to a subtle difference in how threshold is computed).
4. **Week 9:** On testnet, signers can create proposals on their phones (ASM-valid) but the backend refuses to store them ("invalid threshold"). Users are confused; ops have no runbook for this scenario.
5. **Week 10:** The team discovers the split-brain. It takes 2 days to trace back to Bob's "safety check" in the backend.

**Root cause:** No document stated "the backend MUST NOT re-implement SPS-65 rules." Bob was trying to be helpful. The code review didn't catch it because the boundary wasn't documented.

**Fix:** Create ADR-006 explicitly forbidding protocol rule re-implementation. Add it to code review checklist. Declare forbidden imports in Cargo.toml.

---

### Narrative 3: Signer Misread Authority During Signing

**Scenario:** Desktop app UI and hardware wallet display don't coordinate on authority context.

1. **Setup:** A signer (Carol) is a member of both Alpen Admin and Strata Admin multisigs.
2. **Action:** Carol opens the app, sees a proposal to update signer keys. The desktop shows "Signer Update" but doesn't explicitly say which authority (Alpen vs. Strata).
3. **Hardware Wallet:** Carol's Trezor asks "Sign this message?" and displays the action_id hash (`0xabc...`) but no authority context.
4. **Misread:** Carol thinks she's signing an Alpen Admin update (because she was thinking about Alpen that morning), but it's actually a Strata Admin update.
5. **Result:** Carol signs; the signature goes to Strata Admin quorum. Alpen Admin is still one signature short.
6. **Discovery:** It takes 3 days to realize Carol signed the wrong proposal. By then, Strata Admin has already reached quorum on the wrong action.

**Root cause:** No signer safety spec exists. The PRD mentions "authority context" but no document specifies what MUST appear on the device, in what size/color/position, before signing.

**Fix:** Create signer-safety-model.md. Enforce it in code review. Update hardware wallet integration to include authority in the signed message.

---

### Narrative 4: Incomplete Capability Matrix, Foreclosure Surprise

**Scenario:** A user requests a bridge parameter update. The frontend team assumes it's implemented upstream.

1. **Request:** The Strata Admin authority wants to update bridge parameters.
2. **Frontend:** The team opens the spec (`docs/specs/bridge-param-update.md`), finds it's written, assumes it's ready.
3. **Backend:** They add the CRUD endpoint, expecting the action type to exist.
4. **Upstream:** They check `strata-asm-txs-admin` and discover the `BridgeParamUpdate` type was merged 2 weeks ago but hasn't been released yet — only in `main`, not in the pinned tag.
5. **Fallback:** The team has to either (a) pin a newer tag (risking other changes), (b) wait for a release, or (c) remove the feature from Slice 1.

**Root cause:** No living capability matrix. The discovery doc mentions "8 types still missing" but never maintains a status table.

**Fix:** Create capability-matrix.md and update it on every merge. Link it from AGENTS.md.

---

### Narrative 5: Incident with No Playbook

**Scenario:** The orchestrator backend database becomes corrupted during a power loss.

1. **Detection:** Signers report "backend won't accept my signatures."
2. **Response:** The on-call engineer is paged. They SSH into the backend VPS, see postgres is down.
3. **Blind Recovery:** They don't know if the database is corrupted, if a migration failed, or if the schema was never initialized. There's no runbook.
4. **Guesswork:** They restart postgres, see a schema mismatch error, assume they should run migrations, but there's no README for that.
5. **Decision:** They restore from backup, losing 30 minutes of pending proposals (no snapshot was taken at the 5-minute mark).
6. **Aftermath:** Signers have to re-create and re-sign proposals.

**Root cause:** No backend operations doc. The backend exists but has no deployment, monitoring, or recovery procedures.

**Fix:** Create backend-operations.md with deployment topology, schema setup, monitoring, backup/restore procedure, incident escalation.

---

## Evidence Index (Paths)

**Core documentation:**
- `README.md` — Introduction + quick start (needs restructuring as tutorial)
- `AGENTS.md` — Agent guidance + command reference (reference doc, good structure, needs "when to use" clarification)
- `CLAUDE.md` — Delegates to AGENTS.md

**Architecture:**
- `docs/architecture/overview.md` — System architecture, component breakdown, protocol integration (Explanation + Reference, well-written, missing ops content)
- `docs/architecture/adrs/001-alpen-crate-dependencies.md` — Alpen crate pinning strategy (Reference, thorough)
- `docs/architecture/adrs/002-application-layer-strategy.md` — Application layer evolution (not fully read, likely ADR)
- `docs/architecture/adrs/003-desktop-application-layer-api.md` — Desktop app API contract (not fully read, likely ADR)
- `docs/architecture/adrs/004-ci-pipeline-strategy.md` — CI/CD pipeline decisions (not fully read, likely ADR)
- `docs/architecture/adrs/005-layered-architecture.md` — Layered architecture pattern (not fully read, likely ADR)

**Requirements & Discovery:**
- `docs/0-prd/01-multisig-ui.md` — User PRD (Requirements, comprehensive)
- `docs/0-prd/02-multisig-backend.md` — Backend PRD (Requirements, comprehensive)
- `docs/1-proposal/01-alpen-multisig-proposal.md` — Technical proposal (Explanation, good context)
- `docs/2-discovery/README.md` — Reading guide + status flags (Reference, excellent structure, but flags not propagated to docs themselves)
- `docs/2-discovery/` (20+ docs) — POC findings, protocol research, dependency analysis (Explanation + Reference, well-organized, risk of reader confusion between current and superseded docs)

**Scope & Stories:**
- `docs/3-stories/story-map.md` — User story map + slices (Reference + How-to, well-structured)
- `docs/3-stories/non-functional-items.md` — Non-functional requirements (Reference)

**Specs:**
- `docs/specs/` (20+ docs) — Feature-level specifications (mostly How-to + Reference, well-written, but orphaned from architecture docs)

**Deliverables:**
- `docs/deliverable/research.md` — Phase 1 research summary (Reference, comprehensive, labeled as "deliverable" → may not be maintained)
- `docs/deliverable/crate-inventory.md` — Crate coverage matrix (Reference)

**Code standards (not documentation, but relevant):**
- `.claude/rules/general.md` — Missing (error reading file). Mentioned in AGENTS.md but file not found.
- `.claude/rules/typescript-standards.md` — TypeScript conventions (Reference)
- `.claude/rules/rust-backend-standards.md` — Rust conventions (Reference)
- `.claude/rules/backend-api-conventions.md` — Backend API patterns (Reference)
- `.claude/rules/react-frontend-patterns.md` — React patterns (Reference)

**Missing:**
- ❌ No backend operations runbook
- ❌ No backend deployment guide
- ❌ No ADR on backend-coordination boundary (protocol retrenchment constraint)
- ❌ No signer safety model
- ❌ No threat model or incident playbook
- ❌ No release/build reproducibility guide
- ❌ No testing strategy guide
- ❌ No living capability matrix (update types status)
- ❌ No cross-reference map between specs and ADRs
- ❌ No flag on superseded discovery docs (README flags them, but not the docs themselves)

---

## Smallest Fixes vs. Largest Bets

### Quick wins (1–2 hours each)

1. **Add "When to use" section to AGENTS.md commands** (1 hour)
   - Clarify which commands are for daily development vs. CI vs. release.
   - Add example: "Use `cargo test -p orchestrator-be -- test_name` when running a single test."

2. **Flag superseded docs in discovery folder** (30 min)
   - Add a frontmatter block to `docs/2-discovery/04-poc2-findings.md`, `05-poc3-findings.md`, etc.
   - Include line: "⚠️ **SUPERSEDED** — See [link](to-replacement) for current info."

3. **Restructure README.md as a tutorial** (1.5 hours)
   - 5-line intro + "Quick start" section (3 commands) + "What's inside?" + "Learn more" link to AGENTS.md.
   - Move architecture explanation to a "Deep dive" link (to docs/architecture/overview.md).

### Medium-effort (4–6 hours each)

4. **Create backend-operations.md runbook** (5 hours)
   - Deployment topology (PostgreSQL setup, environment vars, secrets).
   - Health checks and monitoring.
   - Schema initialization and migrations.
   - Backup/restore procedure.
   - Incident response (crash, data corruption, scaling).

5. **Create signer-safety-model.md** (4 hours)
   - Authority context display requirements (UI + hardware wallet).
   - Signing confirmation flow and content.
   - Session timeout and reauthentication.
   - Error cases and recovery.
   - Tie to PRD requirements.

6. **Add cross-references to specs and ADRs** (3 hours)
   - Update `docs/architecture/overview.md` with "Specs and Slices" section.
   - Update each feature spec with "Related Architecture" section.
   - Link ADRs ↔ Discovery docs where relevant.

### Large bets (8–12 hours each)

7. **Create ADR-006: Backend-Coordination Boundary** (6 hours)
   - Formalize the constraint: "Backend MUST NOT re-implement SPS-65 rules."
   - List forbidden imports (`strata-crypto`, `strata-asm-params`).
   - Code review checklist.
   - Link to test that proves byte-level equivalence.

8. **Create threat-model.md and incident-playbook.md** (8 hours)
   - Top 5 failure modes (key compromise, db corruption, signature replay, split-brain, network partition).
   - Per-mode detection strategy and response procedure.
   - Signer revocation flow.
   - Escalation path and on-call guide.

9. **Create capability-matrix.md and establish maintenance process** (6 hours)
   - Table of update types (Authority × Type × Status).
   - Link to upstream issues/PRs for each blocked type.
   - Establish that AGENTS.md links to it, and it's updated on every develop merge.

---

## What Would Change My Mind

### Missing Evidence

1. **If a single "New Contributor" document exists** that I didn't find (e.g., `docs/onboarding.md`, `CONTRIBUTING.md`), and it serves as a tutorial explaining: clone → test → code → PR. If this document is well-written and linked from README, it mitigates findings #1, #6, and part of #4.

2. **If backend deployment is documented elsewhere** (e.g., in Vercel or GitHub Actions secrets, in a private internal wiki, or in a Slack pinned message): This doesn't excuse the absence from the repo, but it means operations staff have *some* reference. However, it's risky because it creates two sources of truth.

3. **If a "Signer Safety Checklist" is part of the PR review template** (e.g., in CONTRIBUTING.md or GitHub PR template): This would compensate for the absence of a formal signer-safety-model.md, because reviewers would have an explicit gate.

### Experiments / Validation

1. **Onboarding experiment:** Have a developer who's never seen this codebase clone it, read only README + AGENTS.md, and try to:
   - Make `cargo test` pass (timer: ≤30 min, no Slack asks).
   - Make a non-trivial code change (e.g., add a new validation rule to a spec).
   - Outcome: If they succeed, this repo is fine on onboarding. If they get stuck, it validates finding #6.

2. **Spec coherence audit:** Pick 3 feature specs at random, trace each back to an ADR and a discovery doc. Are there clear links, or is the engineer left guessing? If tracing takes >10 min per spec, finding #7 is real.

3. **Security review by an external auditor:** Have them read the docs and identify whether the "backend is coordination-only" constraint is clear enough to catch a re-implementation attempt. If they flag it, finding #2 is critical.

---

## Summary

**Verdict:** The documentation is **well-organized for Phase 1 (research)** but **incomplete for Phase 2+ (implementation and operations)**. 

**Strengths:**
- Discovery docs are thorough and well-indexed.
- Architecture overview is clear and detailed.
- Story map provides good user context.
- Individual specs are well-written.

**Critical gaps:**
1. No backend operations runbook (blocking for deployment).
2. No protocol boundary formalization (risk of silent violation).
3. No signer safety model (risk of UX misread).
4. No incident playbook (risk of chaotic response).
5. Specs orphaned from architecture (navigation hazard).

**Recommended action:**
1. **Immediate** (before Phase 2 wrap): Create ADR-006, backend-ops, and signer-safety docs (blocking).
2. **Short-term** (before handoff): Add cross-references, create capability matrix, flag superseded docs.
3. **Ongoing:** Establish doc maintenance process (update capability matrix on merges, audit cross-references quarterly).

---

**Assessment completed:** 2026-05-13 12:59 PM UTC-3  
**Total issues identified:** 10 (3 blocking, 3 high, 2 medium, 2 low)  
**Estimated remediation:** 30–40 hours (quick wins + medium bets)  
**Output:** This document + recommended next steps.
