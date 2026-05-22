# Research Sources & Spec / Code Provenance — Adversarial Assessment

**Assessment date:** 2026-05-14  
**Mode:** Read-only adversarial audit  
**Scope:** SPS/PRD claims vs code, Alpen crate pins, test coverage as evidence, sourcing discipline, and folklore detection.

---

## 1. Scope & threat model

**Questions**

1. Does every **MUST** in `docs/0-prd/` trace to a **test**, **module boundary**, or **explicit exception**?
2. Are **SPS-50/51/65** references actionable (section-level) or decorative?
3. Does **workspace pin** strategy (`Cargo.toml`, ADR-001) imply guarantees the tests do not actually give?
4. Where do **blocked** upstream capabilities leak into selectable UX without code-level guardrails?

---

## 2. Top findings (ranked)

### Blocking / critical

**1. PRD forbids backend from treating threshold logic as authoritative; code transitions proposals on signature count**

- **Risk:** Confusion about what “Approved” means off-chain vs on-chain; audits flag inconsistency; wrong mental model for incident response.
- **Evidence:**
  - `docs/0-prd/02-multisig-backend.md` §1 — Signature threshold checks MUST be enforced exclusively on-chain (`required_signatures` listed under canonical rules).
  - `orchestrator-be/src/application/proposals.rs` — Off-chain `Pending` → `Approved` when signature count meets `required_signatures`:

```102:115:orchestrator-be/src/application/proposals.rs
    if proposal.status == ProposalStatus::Pending
        && proposal.signatures.len() >= proposal.required_signatures as usize
    {
        let approved = repo
            .update_broadcast_status(
                action_id,
                proposal.broadcast_status,
                Some(ProposalStatus::Approved),
                None,
                None,
                None,
            )
            .await?;
        return approved.ok_or(AppError::NotFound);
    }
```

```163:176:orchestrator-be/src/application/proposals.rs
    if proposal.status == ProposalStatus::Pending
        && proposal.signatures.len() >= proposal.required_signatures as usize
    {
        let recovered = repo
            .update_broadcast_status(
                action_id,
                proposal.broadcast_status,
                Some(ProposalStatus::Approved),
                None,
                None,
                None,
            )
            .await?;
        return recovered.ok_or(AppError::NotFound);
    }
```

  - Tests in same module assert threshold-triggered approval behavior.
- **Adversarial framing:** Either the backend counts signatures **only as coordination metadata** (allowed narrative) or it **performs threshold checks** (PRD wording says such checks MUST be on-chain). Team must reconcile language precisely (coordination vs validation).

**2. “SPS is source of truth” without in-repo, citable excerpts**

- **Risk:** Engineers and auditors cannot verify that AGENTS.md interpretations match current Alpen spec text.
- **Evidence:**
  - `AGENTS.md` cites SPS-50/51/65 as SSOT with Notion URLs — no mirrored excerpts under `docs/specs/sps-*` in this assessment’s file search.
  - Many specs say “per SPS-65” without § anchor stable in git.

### High

**3. Sighash / encoding correctness is pinned to Alpen crates but e2e matrix may be narrow**

- **Risk:** Passing CI does not imply all authorities × action families are byte-compatible with current ASM deployments.
- **Evidence:**
  - `docs/architecture/adrs/001-alpen-crate-dependencies.md` — Pin strategy and update ritual.
  - `e2e-tests/` — Review coverage breadth when extending roles (don’t assume one scenario exports all sighash variants).
  - Desktop + backend `signing.rs` delegate to upstream `sighash`/SSZ paths.

**4. Discovery documents upstream blockers; product enums expose full authority set**

- **Risk:** Users select “supported-looking” authorities that fail at signing or broadcast with opaque errors.
- **Evidence:**
  - `docs/2-discovery/08-alpen-crate-prd-coverage.md` — Items not implemented in pinned crates.
  - `orchestrator-be/src/domain/authority.rs`, `desktop-app/src-tauri/src/domain/authority.rs` — Five variants without inherent “blocked” encoding.

### Medium

**5. ADR-001 explains rev vs tag but convergence criteria stay qualitative**

- **Risk:** Future pin changes repeat debate; rollback strategy under-specified.
- **Evidence:** `docs/architecture/adrs/001-alpen-crate-dependencies.md`.

**6. Dual documentation of protocol behavior (Notion PRD copy + internal markdown)**

- **Risk:** Drift between “external copy” headers in `docs/0-prd/` and evolving internal specs.
- **Evidence:** `docs/0-prd/02-multisig-backend.md` header notes external provenance.

### Low

**7. Ledger / additional HW paths**

- **Risk:** “Supported wallets” language in PRD vs partial implementation in `desktop-app/src-tauri/src/infrastructure/hw_wallet/`.
- **Evidence:** Spot-check `ledger.rs` and HW integration specs.

---

## 3. Attack narratives (3–6)

### N1: External auditor

They request PRD §1 proof for backend behavior. They read Pythonic English (“threshold exclusively on-chain”) and Rust that increments approval at `N` signatures. The gap becomes a release blocker.

### N2: New Alpen tag bumps sighash

CI stays green on a single happy-path e2e. A Sequencer-manager action reaches mainnet with wrong sighash; nodes reject; “tests passed.”

### N3: Product demo selects Alpen Admin

Crate lacks the action variant; backend still accepted proposal artifacts during dev. Narrative focuses on UX while upstream dependency was the real gate.

### N4: Research doc was right; JIRA was wrong

`08-alpen-crate-prd-coverage.md` says blocked; ticket says “_slice 2 done.” Ship checklist cites JIRA, not discovery. Customer hits runtime error.

### N5: SPS paragraph changes on Notion

No excerpt snapshot in git; team learns via incident, not diff.

### N6: Coordination vs validation debate mid-incident

Ops toggles a setting thinking “backend won’t approve until chain confirms.” Code already marked `Approved` off-chain. Runbook wrong; comms wrong.

---

## 4. Evidence index (paths)

| Kind | Path |
|------|------|
| Backend PRD (validity / threshold language) | `docs/0-prd/02-multisig-backend.md` §1 |
| UI PRD | `docs/0-prd/01-multisig-ui.md` |
| Proposal / commercial scope | `docs/1-proposal/01-alpen-multisig-proposal.md` |
| Crate coverage research | `docs/2-discovery/08-alpen-crate-prd-coverage.md` |
| ASM / Bitcoin model | `docs/2-discovery/10-asm-bitcoin-state-model.md` |
| Pin strategy | `docs/architecture/adrs/001-alpen-crate-dependencies.md`, root `Cargo.toml` |
| Approval / threshold in code | `orchestrator-be/src/application/proposals.rs` |
| Sighash helpers | `orchestrator-be/src/infrastructure/signing.rs`, `desktop-app/src-tauri/src/infrastructure/signing.rs` |
| SSZ / action | `desktop-app/src-tauri/src/infrastructure/action_codec.rs` |
| E2E | `e2e-tests/` |
| Conventions | `AGENTS.md`, `.cursor/rules/general.mdc` |

---

## 5. Smallest fixes vs. largest bets

**Smallest**

- Add **inline comments + ADR paragraph** reconciling off-chain `Approved` with PRD §1 (“coordination state only; not authoritative quorum”).
- Annotate `authority.rs` with **doc links** to `08-alpen-crate-prd-coverage.md` for partially blocked roles.
- Maintain a **table in one markdown file** mapping PRD MUST → code location → test name (even 10 rows to start).

**Largest**

- **Mirror minimal SPS excerpts** (allowed by license/policy) into `docs/specs/sps-reference/` with paragraph anchors.
- **Parameterized integration tests** across authorities and representative action variants.
- **Runtime capability probe** on startup: compare configured authorities to compiled Alpen feature set.

---

## 6. What would change my mind

- A **quoted SPS-65 paragraph** (with section ID) stating that off-chain services may track signature counts without constituting canonical threshold enforcement would reclassify finding 1 from “contradiction” to “wording debt.”
- A **widened e2e matrix** with documented per-case SPS references would downgrade finding 3.
- **UI gating** that disables blocked authorities based on the same data as `08-alpen-crate-prd-coverage.md` would downgrade finding 4.
