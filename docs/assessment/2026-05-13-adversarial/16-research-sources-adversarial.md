# Research Sources & Spec Folklore — Adversarial Assessment

**Review Date:** 2026-05-13  
**Scope:** Alpen Multisig repo  
**Mode:** Adversarial read-only audit  
**Output:** Blocking/High/Medium issues + attack narratives

---

## Scope & Threat Model

**What we're testing:**

1. **Spec citation rigor** — Do claims about SPS-50/51/65 anchor to specific sections or are they hand-wavy?
2. **Source authority chain** — Does "backend is coordination only" trace to PRD/proposal/SPS, or is it repo convention only?
3. **Implementation vs. documentation** — Code vs. docs: does code actually enforce what docs claim?
4. **Unverified upstream assumptions** — Where does code rely on Alpen/Strata crate behavior without tests?
5. **Version pinning** — Are critical external crate pins documented with commit/tag + rationale?
6. **Folklore contradictions** — Specs claim X, code does Y; docs assert A, implementation proves B.

**Adversarial stance:** Assume every claim without explicit evidence is either misstated, incomplete, or contradicted elsewhere. Treat "the spec says" as a red flag unless anchored to a specific SPS section + line number.

---

## Top Findings (Ranked)

### 🔴 **BLOCKING: Backend Threshold Checking Contradicts "Coordination Only" Claim**

**Severity:** CRITICAL  
**Status:** Contradicted in code + docs

**The claim** (AGENTS.md, `.cursor/rules/general.mdc`, PRD §1):
- "Backend is coordination only: proposal creation, signature collection, lifecycle tracking — never re-implement protocol validity rules"
- Backend PRD §1: "The backend MUST NOT redefine, reinterpret, or override any governance or validity rule defined in SPS-65"
- Specific validity rules the backend must NOT enforce: "Signature threshold checks"

**The contradiction** (orchestrator-be/src/application/proposals.rs, lines 103–104, 164):
```rust
// Proposal auto-transitions to Approved when signature count reaches required threshold
pub async fn approve_action(...) -> Result<Proposal, AppError> {
    // ...
    if proposal.signatures.len() >= proposal.required_signatures as usize {
        proposal.status = ProposalStatus::Approved;  // ← THRESHOLD CHECK, LINE 103
    }
}
```

Test case explicitly validates this behavior (lines 557–580):
```rust
#[tokio::test]
async fn test_approve_action_transitions_to_approved_at_threshold() {
    let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2)
        .await
        .unwrap();
    assert_eq!(created.status, ProposalStatus::Pending);
    
    // Add second signature
    let updated = approve_action(&repo, session_b, &created.action_id, &sig_b()).await.unwrap();
    
    // Backend auto-transitioned to Approved ← Backend is doing threshold validation
    assert_eq!(updated.status, ProposalStatus::Approved);
}
```

**Evidence chain:**
- Docs claim: "never re-implement protocol validity rules" + "Signature threshold checks MUST be enforced exclusively by the onchain subprotocol"
- Code fact: Backend *does* check threshold (line 103: `signatures.len() >= required_signatures`), auto-transitions proposal state
- Test fact: Explicitly tests and asserts this threshold-based state transition (line 579)

**Risk in production:**
- If the onchain ASM expects threshold validation to occur *only* on-chain, but the backend has already transitioned state to "Approved" based on its own count, there's a coordination mismatch
- A backend reaching quorum (e.g., 2-of-3) does not guarantee the *correct* 2-of-3 on-chain, only that *someone collected* 2 signatures
- If the threshold later changes (e.g., Strata Admin switches from 2-of-3 to 3-of-5), the backend's cached `required_signatures` becomes stale and may auto-transition proposals incorrectly

**What source says (SPS-65 reference):**
- Docs reference SPS-65 but do NOT cite a specific section that forbids backend quorum detection
- Docs do NOT cite SPS-65 to justify *why* the backend should detect quorum and transition state
- Proposal text (line 102): "quorum detection" is listed as a backend deliverable, but the PRD backend section contradicts this

**Recommendation:**
- Either: (a) Remove threshold checking from backend and rely on UI state polling, or
- (b) Explicitly source the decision to do backend quorum detection in SPS-65 with line number

---

### 🔴 **BLOCKING: "Backend Coordination Only" Claim Is Unsubstantiated — No SPS Source Anchor**

**Severity:** CRITICAL  
**Status:** Unsourced repo convention

**The claim** (AGENTS.md line 64, `.cursor/rules/general.mdc` line 9):
- "Backend is coordination only: proposal creation, signature collection, lifecycle tracking — never re-implement protocol validity rules"

**Source hunt:**
- Search for "coordination only" in `docs/0-prd/*` — **not found**
- Search for "coordination only" in `docs/1-proposal/*` — **not found**
- Search in e2e tests, proposals.rs, backend PRD (`docs/0-prd/02-multisig-backend.md`) — **not found** as a term
- The backend PRD uses "MUST NOT redefine, reinterpret, or override" but does NOT use "coordination only"

**What the PRD actually says** (docs/0-prd/02-multisig-backend.md §1):
> "The backend MUST function exclusively as an offchain coordination service for: Proposal creation. Signature collection. Proposal state tracking prior to quorum."

**Problem:** "Coordination service" ≠ "coordination only in the sense that docs claim"
- The PRD says the backend *coordinates* these activities, not that it is *entirely passive*
- The PRD does NOT say "coordination" means "never check thresholds" — it says "coordination" means handling those three activities

**Missing source chain:**
- AGENTS.md cites SPS-50/51/65 as "source of truth" but does NOT specify which SPS section justifies the "coordination only" constraint
- The backend-as-coordination-only rule appears to be a **repo convention layer**, not a PRD or SPS requirement
- This convention is used to justify architectural decisions (e.g., "backend should not validate signatures") but the original source is not traceable

**Evidence:**
- `backend-api-conventions.mdc` line 1: "Treat the backend as an offchain coordination service only" — prescriptive, not sourced
- `general.mdc` line 9: "Backend is coordination only" — prescriptive, not sourced
- No ADR, no discovery doc, no PRD section that formally derives this rule from SPS-65

**Risk:**
- Developers inherit this rule as gospel, but cannot trace its origin
- A future audit or requirement change could conflict with this rule without any documented justification to defend it
- If Alpen's interpretation of SPS-65 expects backends to perform additional validation, the codebase has no written evidence of the deliberate decision to ignore that expectation

**Recommendation:**
- Add an ADR: "Why Backend Performs Only Coordination, Not Validation" — source it to specific SPS-65 sections
- If it's a WakeUp Labs constraint, document it as such and link to the business decision, not to SPS

---

### 🟠 **HIGH: Unverified Assumption — Upstream Alpen/Strata Sighash Implementation**

**Severity:** HIGH  
**Status:** Test exists but covers only 1 specific config

**The assumption:**
The backend assumes that the Alpen `sighash_payload()` function in `strata-crypto` computes byte-identical sighashes for every multisig role (Alpen Admin, Strata Admin, Sequencer Manager, Security Council, Payout Admin) and every update type.

**Evidence of assumption:**
- `orchestrator-be/src/infrastructure/signing.rs` line 53: `/// Compute the SPS-65 tagged sighash for a given action and sequence number.`
- Backend calls upstream `sighash_payload(action, seq_no)` and trusts the output (no local verification)
- Desktop app calls the same: `desktop-app/src-tauri/src/infrastructure/signing.rs` line 123

**Verification in code:**
- `e2e-tests/tests/e2e_admin_commit_reveal.rs` — tests one scenario: Strata Admin with 3-of-2 threshold
- Test validates sighash round-trip: sign → encode → parse → verify (lines 85–127)
- **Does NOT test:** Alpen Admin, Sequencer Manager, Security Council, Payout Admin
- **Does NOT test:** Different threshold configs (1-of-N, N-of-N edge cases)
- **Does NOT test:** UpdateAction variants not yet in Alpen crates (Safe Harbor, Alpen verification key, etc.)

**Coverage gap:**
- Test passes for **1 scenario** (Strata Admin, 2-of-3 Alpen crate types)
- Backend is designed to support **all 5 roles + all 13+ update types** (docs/2-discovery/08-alpen-crate-prd-coverage.md)
- When Alpen crates add new roles/types, the sighash assumption remains unverified

**Evidence:**
- ADR-001 (line 90): "Any divergence [in wire format] signals an incompatible wire format" — test guards against this
- But test is pinned to a single Alpen crate version and a single scenario
- Docs mention "Phase 1 — Protocol Research & Architecture" but never list "sighash compatibility validation across all 5 roles and 13+ types" as a test milestone

**Risk in production:**
- A new Alpen crate update changes sighash computation for Sequencer Manager updates → e2e test still passes (only covers Strata Admin) → backend ships with broken sighash for Seq Manager → signers create signatures that don't verify on-chain → governance action rejected with no clear signal

**What's missing:**
- Parameterized e2e test covering all 5 roles × all supported update types (or at minimum, 1 representative from each role)
- Documentation linking each sighash computation to the specific SPS-65 section it implements
- Comment in `signing.rs` stating which test(s) validate this function's output

**Recommendation:**
- Expand `e2e_admin_commit_reveal.rs` to test all 5 authority roles with at least one representative update type each
- Add test fixture comments mapping each test to the Authority + UpdateAction variant it covers
- Document the sighash validation scope in the README or in `signing.rs` comments

---

### 🟠 **HIGH: Alpen Admin & Safe Harbor Update Types — Blocked, Not Documented in Code**

**Severity:** HIGH  
**Status:** Blocked upstream, but code/docs don't uniformly warn

**The issue:**
Proposal text (docs/1-proposal/01-alpen-multisig-proposal.md line 102) lists "fifteen or more message types" and "all five multisig types" as deliverables. However, docs/2-discovery/08-alpen-crate-prd-coverage.md explicitly documents that Alpen Admin and Safe Harbor are **not implemented in Alpen crates** as of the current pin (rev `a8559d3`).

**Evidence of blockage:**
- docs/2-discovery/08-alpen-crate-prd-coverage.md §2 "Not implemented in Alpen crates":
  - Alpen verification key update — **Blocked**
  - Alpen Administrator Signer update — **Blocked**
  - Safe Harbor address update — **Blocked**

**Where code lacks warning:**
- `orchestrator-be/src/domain/authority.rs` and `desktop-app/src-tauri/src/domain/authority.rs` both define:
  ```rust
  pub enum Authority {
      AlpenAdmin,
      StrataAdmin,
      SequencerManager,
      SecurityCouncil,
      PayoutAdmin,
  }
  ```
- No comment explaining which authorities are fully implemented vs. blocked on upstream
- No `#[deprecated]`, `#[doc]` hint, or runtime warning when a user selects AlpenAdmin
- Frontend likely shows all 5 authorities as selectable, but creating/signing Alpen Admin updates will fail with unclear error messages

**Evidence of mismatch:**
- Proposal claims 5 roles → implementation has 5 roles → but 2 are non-functional
- Discovery doc documents the blockage clearly, but the blockage is not reflected in code comments or inline documentation
- Developers editing `Authority` enum 6 months from now may assume all 5 are functional and attempt to extend them

**Risk:**
- User selects Alpen Admin in UI → frontend creates proposal → backend accepts it → attempt to broadcast fails at hardware wallet stage (Alpen crates reject it) → user sees opaque error
- Codebase maintainers 6 months later, unaware of the blockage, attempt to add new Alpen Admin update types → discover too late that upstream support doesn't exist

**What PRD claims vs. what's delivered:**
- Proposal deliverable list (line 102): "all five multisig types fully supported"
- Reality (discovery doc): 3 fully, 2 blocked (no upstream types defined)

**Recommendation:**
- Mark AlpenAdmin authority with `#[doc = "BLOCKED: Alpen crates do not yet define Role::AlpenAdministrator or update types. See docs/2-discovery/08-alpen-crate-prd-coverage.md §2."]`
- Add issue/comment in handlers that accept AlpenAdmin: "This returns NotFound or InternalServerError until alpenlabs/asm adds Alpen Admin role"
- Update proposal deliverable list to reflect actual scope: "3 fully-supported multisigs + 2 blocked on upstream"

---

### 🟡 **MEDIUM: Crate Dependency Version Pinning — No Rationale for Tag vs. Rev Choice**

**Severity:** MEDIUM  
**Status:** Documented in ADR-001 but rationale is incomplete

**The issue:**
ADR-001 documents two different pinning strategies:
- `alpenlabs/asm`: pinned by **rev** (`a8559d3`)
- `alpenlabs/strata-common`: pinned by **tag** (`v0.1.0-alpha-rc16`)

**Stated rationale** (ADR-001 line 17):
> "The dedicated ASM repo was spun off on 2026-03-17. Two tag schemes coexist (`v0.1-alpha.N` and `v0.1.0-rcN`), none stable. Current rev `a8559d3` equals tag `v0.1-alpha.5`. Switch to `tag` pinning once upstream converges on a single release cadence."

**Problem 1: Rev vs. Tag trade-off not explicitly evaluated**
- Docs say "prefer `tag` over `rev` whenever possible" (line 20)
- But then pin asm by rev, justifying it with "upstream hasn't converged on single tag scheme"
- No explicit evaluation of the trade-off: "rev is unambiguous but opaque; tag is human-readable but two schemes coexist"
- Missing: "We chose rev because [explicit risk analysis], acknowledging that [specific risk]"

**Problem 2: Migration plan unclear**
- Docs say "Switch to `tag` pinning once upstream converges" (line 20)
- Missing: What does "converges" mean? When do we know it's safe?
- Missing: Who watches upstream for convergence? (No issue/comment in the code)
- Missing: What's the rollback plan if we switch to tag and upstream tags move incompatibly?

**Problem 3: Update procedure doesn't validate the pin strategy itself**
- ADR-001 §Update procedure (lines 93–98) says "Run cargo build and cargo test"
- Missing: No mention of verifying that the new rev/tag is stable or that the strategy (rev vs. tag) is still appropriate

**Evidence of fragility:**
- Repository split (ASM moved to separate repo) happened during this project's lifetime (2026-03-17)
- This migration is not explicitly tested in e2e suite — only mentioned in discovery doc
- If another upstream reorganization happens, the codebase has no recorded decision-making process to guide the new pin choice

**Risk:**
- Future maintainer inherits "pin asm by rev" without understanding the trade-off
- Six months from now, if upstream converges on tags, the maintainer doesn't know if switching is safe
- A third-party audit may flag "unpinned rev" as a risk without understanding the deliberate choice

**Recommendation:**
- Add explicit trade-off analysis to ADR: "Why rev for ASM but tag for strata-common?"
  - Pro/con table: unambiguous (rev) vs. human-readable (tag) vs. fork recovery
  - Explicit risk: "If ASM tags become the canonical reference and we're on rev, we may miss important releases"
  - Explicit safeguard: "e2e test covers the specific types/functions we consume, so a silent incompatibility is unlikely"
- Document the "convergence signal" — what will we look for? (e.g., "upstream publishes a stable v0.1 tag scheme")
- Link this ADR from update procedure

---

### 🟡 **MEDIUM: "SPS-65 is Source of Truth" — Claim Unverified Against Actual SPS Document**

**Severity:** MEDIUM  
**Status:** Cited but not validated in code

**The claim** (AGENTS.md line 63, `.cursor/rules/general.mdc` line 8):
- "Protocol alignment: SPS-50, SPS-51, SPS-65 are source of truth"

**Verification problem:**
- The repo contains **no readable copy** of SPS-50/51/65 (they are Notion links, not source files)
- Docs reference SPS-65 as "the full governance state machine" (docs/2-discovery/01-conceptual-overview.md line 251) but never quote the specific section that defines validity rules
- No document shows a mapping: "SPS-65 §X.Y.Z says [rule], our code implements it at [file:line]"

**Evidence of mismatch:**
- Docs claim "SPS-65 defines threshold checks" but don't cite where in SPS-65
- Code implements threshold checks in backend, contradicting the docs' claim that "backend must not implement threshold checks per SPS-65"
- No way for a reviewer to verify: "Does SPS-65 actually forbid backend threshold checks, or did the docs/code author misinterpret?"

**Why this matters:**
- SPS-65 is cited as the authority for architectural decisions, but the chain of evidence is broken
- A future disagreement about backend responsibilities could cite "SPS-65 says..." but without access to the spec, no one can verify the citation

**What's missing:**
- A `docs/specs/sps-*.md` file that archives the key sections of SPS-50/51/65 relevant to this codebase
- Or at minimum: comments in the code linking to specific Notion section IDs (e.g., "See SPS-65 §3.2 'Validity Rules' for the authority of this check")

**Risk:**
- A future auditor asks "Where does SPS-65 say the backend should not check thresholds?" → Answer: "AGENTS.md says so, but we don't have a copy of SPS-65 to verify"
- Developers treat AGENTS.md as ground truth, unaware it may have misinterpreted or oversimplified SPS-65
- When Alpen publishes an updated SPS-65, the team has no mechanism to detect breaking changes

**Recommendation:**
- Create `docs/specs/sps-reference/` folder with key excerpts from SPS-50/51/65, linked from the Notion documents
- Add a comment in `proposals.rs` and `signing.rs`: "SPS-65 §3 'Validity Rules' defines threshold checks; this backend only coordinates signatures, not validates them"
- Document the source-of-truth chain in AGENTS.md: "SPS-65 is the protocol spec. Interpretations and constraints are documented in code comments with section references."

---

## Attack Narratives (How This Breaks in Production)

### 1. **Alpen Admin governance gets stuck because role is non-functional**

**Scenario:** Alpen Labs needs to update the rollup admin keys. They select "Alpen Administrator" in the UI.
- UI accepts the selection (no warning that it's unimplemented)
- User creates a proposal for "Alpen Administrator Signer Update"
- Backend accepts it (no check that Alpen crates support this role)
- User collects signatures from 2-of-3 Alpen signers
- Backend marks proposal as Approved (quorum met)
- User attempts to broadcast: Alpen crate rejects action type (no `Role::AlpenAdministrator` variant)
- Error message: "Role variant not found" (opaque, no explanation)
- **Governance blocked for days until someone figures out it's an upstream availability issue**

**Root cause:** No runtime validation that selected authority is actually supported by current Alpen crate pin.

---

### 2. **Strata Sequencer governance is rejected on-chain because backend threshold was wrong**

**Scenario:** Strata Sequencer Manager just changed threshold from 2-of-3 to 3-of-5 on-chain.
- Old backend still has `required_signatures = 2` cached or hardcoded
- Signer submits a Sequencer Update; backend auto-transitions to Approved at 2 signatures
- User broadcasts the transaction with 2-of-5 signatures (not 3-of-5)
- ASM rejects the transaction: "Insufficient signatures for current threshold"
- **Transaction sits in mempool or is dropped; governance delayed**

**Root cause:** Backend performs its own threshold check and caches `required_signatures` per-proposal, but doesn't sync with on-chain threshold updates. If ASM changed the threshold after a proposal was created, the backend's cached value becomes stale.

**Evidence:** `proposals.rs` line 52 stores `required_signatures` at creation time; no mechanism to re-sync if on-chain config changes.

---

### 3. **Auditor cannot verify "backend is coordination only" because claim is not sourced**

**Scenario:** Security audit of the multisig system.
- Auditor reads AGENTS.md: "Backend is coordination only; never re-implement protocol validity rules"
- Auditor searches for the SPS-65 section that justifies this constraint
- Auditor finds it's not documented; traces to backend code and sees threshold checks happening
- Auditor report: "Contradiction between documented architecture and implementation; no source authority for design decision"
- **Credibility of entire backend architecture questioned due to missing source chain**

**Root cause:** Architectural principle cited as law, but the law (SPS-65 section) is not provided, and code contradicts the principle.

---

### 4. **Sighash verification fails silently for a new authority role because test doesn't cover it**

**Scenario:** Alpen crates add Alpen Admin role + Alpen verification key update.
- WakeUp Labs updates `Cargo.toml` to the new Alpen crate version
- `cargo build` and `cargo test` pass (e2e test still only covers Strata Admin)
- New role is added to the UI; signers begin creating Alpen Admin proposals
- Hardware wallet signs the sighash; signature is sent to backend
- **On-chain ASM rejects the signature: sighash mismatch**
- Root cause: Alpen's sighash computation for Alpen Admin differs from what we're computing, but the test doesn't catch it (only tests Strata Admin)

**Root cause:** Test coverage is too narrow (1 scenario instead of all roles × representative types).

---

### 5. **ASM repository split causes unexpected dependency breakage**

**Scenario:** WakeUp Labs is currently pinned to `alpenlabs/asm` rev `a8559d3` (post-split).
- Upstream merges `alpenlabs/asm` back into `alpenlabs/alpen` for a consolidated release
- New Alpen crate pin no longer points to `alpenlabs/asm`; it's in a different branch
- Maintainer updates `Cargo.toml` without realizing the repo location changed
- Build fails with: "Repository not found"
- **Deployment blocked; team must investigate upstream reorganization without any documented context**

**Root cause:** ADR-001 documents the split and the rev pin, but doesn't document the conditions under which the pin strategy would need to change. No process exists to detect or recover from upstream reorganization.

---

## Evidence Index (Paths)

### Claims & Contradictions

| Claim | Source | Contradiction | Code Evidence |
|-------|--------|---------------|----|
| Backend never implements threshold checks | AGENTS.md:64, backend PRD §1 | Code does implement and test threshold checks | `orchestrator-be/src/application/proposals.rs:103–104`, test line 557–580 |
| Backend is coordination only | AGENTS.md:64, `.cursor/rules/general.mdc` | No SPS source anchor; not in PRD | Untraced rule; appears to be repo convention |
| All 5 multisig types fully supported | Proposal deliverable line 102 | Alpen Admin + Safe Harbor blocked on upstream | `docs/2-discovery/08-alpen-crate-prd-coverage.md` §2 |
| Threshold signatures verified by hardware wallet | Discovery doc line 296 | Hardware wallet only signs governance ECDSA hash, Bitcoin tx signing is separate | `docs/2-discovery/10-asm-bitcoin-state-model.md` §Signature layers |
| SPS-65 is source of truth | AGENTS.md:63 | Notion links only; no local spec copy; not cited in code | No `docs/specs/sps-*.md` files |

### Unverified Assumptions

| Assumption | Code Location | Scope of Verification | Evidence Gap |
|-----------|------|----------|-----------|
| Alpen sighash_payload() is correct for all 5 roles | `orchestrator-be/src/infrastructure/signing.rs:53` | Only 1 role tested (Strata Admin) | `e2e_admin_commit_reveal.rs` covers 1 scenario; no parameterized test for all roles |
| Alpen crate pins are compatible | `root Cargo.toml [workspace.dependencies]` | Single round-trip test per version | `test_encode_matches_direct_strata_ssz` guards wire format, but not role/type coverage |
| Authority threshold is fetched correctly from ASM | `orchestrator-be/src/handlers/proposals.rs:79` | Mock data only for non-production checks | `asm_role_membership.rs:273` has mock fallback; real RPC behavior untested |

### Blocked/Incomplete Features

| Feature | Status | Impact | Evidence |
|---------|--------|--------|----------|
| Alpen Administrator authority support | BLOCKED (upstream) | Users can select but cannot use | `docs/2-discovery/08-alpen-crate-prd-coverage.md` §2, no code warning |
| Safe Harbor address update | BLOCKED (upstream) | Strata governance incomplete | Same as above |
| Ledger hardware wallet support | NOT IMPLEMENTED | Only Trezor/Coldcard work; Ledger marked as pending | `desktop-app/src-tauri/src/infrastructure/hw_wallet/ledger.rs:4` |
| Seq number auto-resolution | FUTURE SLICE | Requires orchestrator endpoint (not yet built) | `docs/specs/poc4-step1-desktop-proposal-flow.md:26` |

### Spec Source Citations

| Spec | Cited in | Section References | Missing From |
|------|----------|-------|---------|
| SPS-50 | Proposal, discovery docs, e2e tests | Transaction format, OP_RETURN tag | Code comments (only mentioned in function names) |
| SPS-51 | Same | Witness envelope format, chunking | Same |
| SPS-65 | AGENTS.md, backend PRD, discovery docs | "Source of truth", threshold rules | Actual section numbers, quoted text |

---

## Smallest Fixes vs. Largest Bets

### Quick wins (1–4 hours each)

1. **Add documentation warnings to blocked authorities**
   - Mark `AlpenAdmin` and `SecurityCouncil` with `#[doc]` comments explaining upstream blockage
   - File: `orchestrator-be/src/domain/authority.rs`, `desktop-app/src-tauri/src/domain/authority.rs`

2. **Document the threshold-checking decision**
   - Add ADR: "Why Backend Performs Quorum Detection (Proposal Approved State) Despite 'Coordination Only' Claim"
   - Link to specific SPS-65 sections (once obtained)
   - Risk mitigation: explain why on-chain ASM is the ground truth and backend state is advisory only

3. **Add inline comments sourcing key code to SPS sections**
   - Example: `signing.rs` line 53: `// SPS-65 § [X.Y] defines the governance sighash; see [link]`
   - Flag where code assumes upstream correctness without local validation

4. **Pin specific Notion section IDs as references**
   - Replace vague "SPS-65" citations with "SPS-65, section 'Validity Rules', subsection 'Threshold Enforcement'"
   - Store in a comment or `.txt` file so future auditors can regenerate the spec

### Medium effort (1–2 days)

5. **Expand e2e test to cover all authority roles**
   - Parameterize `e2e_admin_commit_reveal.rs` to run the same test across all 5 authorities
   - Add representative update type for each (e.g., Strata Admin multisig update, Sequencer Manager signer update, etc.)
   - Validates sighash round-trip for each role

6. **Add SPS-50/51/65 excerpts to docs**
   - Create `docs/specs/sps-reference/` with key sections copied or linked from Notion
   - Map each excerpt to the code that implements it
   - Enables future auditors to verify claims without external access

7. **Implement authority availability check at startup**
   - Backend: fetch all authority configs from ASM RPC on startup, validate that current Alpen crate can construct proposals for them
   - Warn/fail if unsupported authorities are required by governance
   - Example: "AlpenAdmin authority is configured on-chain but not yet supported by current Alpen crate pin"

### Largest bets (1+ weeks)

8. **Implement dynamic threshold sync**
   - Backend caches `required_signatures` per authority; add periodic sync (e.g., every proposal creation) with on-chain ASM
   - Risk: if threshold changes mid-proposal, mark proposal as stale or re-validate
   - Ensures backend's Approved state is never out of sync with on-chain requirements

9. **Add spec archive & validation layer**
   - Capture actual SPS-50/51/65 documents (PDF or HTML) in the repo
   - Build a validation layer that checks code comments against spec content
   - Enables CI to flag "code references SPS-65 § X.Y but spec says something else"

10. **Implement comprehensive coverage matrix test**
    - Test all roles × all supported update types × all threshold configs
    - Covers sighash, encoding, signature verification, transaction construction
    - Parameterized, so adding a new role/type auto-expands test matrix

---

## What Would Change My Mind

**Evidence that would reduce severity of findings:**

1. **SPS-65 excerpt showing "backend may perform threshold detection"**
   - If SPS-65 explicitly allows backends to transition proposals to Approved based on local signature count, the contradiction vanishes
   - Cite section & paragraph; quote the text

2. **Design decision document or ADR linking "coordination only" to SPS-65**
   - "Decision: Backend is coordination only because SPS-65 § X requires the ASM to be the sole authority for governance transitions"
   - "Constraint: Quorum detection is advisory; the true threshold is enforced on-chain"

3. **Parameterized e2e test covering all authorities + key update types**
   - Single test run today proves sighash correctness across all 5 roles
   - Removes the "only 1 scenario tested" risk

4. **Upstream SPS-65 section explaining why Safe Harbor and Alpen Admin are not yet in Alpen crates**
   - Confirms these are deliberate omissions in the protocol, not oversights
   - Provides timeline for when they will be added

5. **Comment in `authority.rs` explaining which roles are blocked and why**
   - "AlpenAdmin is not yet supported by Alpen crates (alpenlabs/asm rev a8559d3); feature blocked on upstream"
   - Prevents future maintainers from assuming all 5 are functional

6. **Test fixture or comment showing which e2e test covers which authority**
   - "This test covers: Strata Admin, StrataAdminMultisigUpdate, 2-of-3 threshold"
   - Next maintainer adds new authority and expands test matrix accordingly

**Evidence that would increase severity:**

1. **Code or spec showing backend threshold check causes governance to fail on-chain**
   - Today it's a logic contradiction; if we find a scenario where it breaks governance, it's critical
   - Example: "Backend approved the proposal at 2-of-3, but on-chain threshold is 3-of-5; transaction rejected"

2. **Auditor report citing SPS-65 saying "backend must never check signatures"**
   - Currently no access to the spec; if it forbids backend threshold checks, this is a critical architectural violation

3. **Report of failed deployments due to missing Alpen Admin support**
   - Today it's a risk; if it's happened in testnet, severity escalates

---

## Summary Table: Top 5 Findings

| Rank | Finding | Severity | Type | Blocking | Fix Complexity |
|------|---------|----------|------|----------|-----------------|
| 1 | Threshold checking contradicts "coordination only" | **CRITICAL** | Logic contradiction | YES — blocks spec compliance | **MEDIUM** (add ADR, remove check or document) |
| 2 | "Coordination only" claim unsourced from SPS | **CRITICAL** | Missing provenance | YES — undermines architecture | **MEDIUM** (trace to SPS section) |
| 3 | Sighash verification unvalidated for 4 of 5 authorities | **HIGH** | Test coverage gap | NO — works today, fails silently on new versions | **MEDIUM** (expand e2e test) |
| 4 | Alpen Admin & Safe Harbor blocked but no code warning | **HIGH** | Missing failure mode documentation | NO — user gets opaque error | **SMALL** (add comments) |
| 5 | Crate pin strategy underdocumented (rev vs. tag) | **MEDIUM** | Incomplete ADR | NO — works today, confusing for maintenance | **SMALL** (expand ADR with trade-off analysis) |

---

## Final Notes

### Adversarial Assessment Stance

This audit applied **strict evidential standards**: claims without explicit sources are treated as unsupported. This is intentionally harsh because:

1. **Multisig governance is critical** — assumptions that seem reasonable ("SPS-65 forbids backend threshold checks") can silently violate protocol invariants
2. **Upstream dependencies are pre-1.0** — Alpen crates are actively evolving; the codebase needs explicit assumptions and validation mechanisms, not folklore
3. **Source chain matters** — "the docs say" is only valid if the docs cite their source; "the code does" is only valid if the code matches the docs

### Recommendations Priority

**Immediate (before next release):**
- Add ADR or decision doc explaining the threshold-checking behavior and how it relates to SPS-65
- Document which authorities are blocked and why
- Link code comments to SPS sections (even if via Notion IDs)

**Before merging new authority/update type:**
- Expand e2e test to cover all existing authorities
- Validate that new role/type is supported by current Alpen crate pin
- Add test coverage for the new role before shipping

**Medium-term (next 2–4 weeks):**
- Create SPS-reference docs or capture spec excerpts
- Implement authority availability check at startup
- Document the source-of-truth chain (SPS-65 → code → tests) in AGENTS.md

---

**Report compiled:** 2026-05-13  
**Auditor:** nw-researcher-reviewer (Scholar)  
**Scope:** Read-only; no code modifications  
**Output file:** `/home/elmol/Documents/wakeup/alpen-multisign/repo/alpen-multisig/docs/assessment/2026-05-13-adversarial/16-research-sources-adversarial.md`
