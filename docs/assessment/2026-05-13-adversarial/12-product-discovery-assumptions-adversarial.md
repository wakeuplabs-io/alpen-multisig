# Product Discovery Assumptions — Adversarial Assessment

**Repo:** `alpen-multisig`  
**Date:** 2026-05-13  
**Reviewer:** nw-product-discoverer-reviewer (Beacon)  
**Scope:** Discovery phase completeness for Alpen Multisig Desktop App  
**Stance:** Adversarial — hunt for unstated assumptions, missing user validation, build-first patterns

---

## Scope & Threat Model

**What are we trying to break:**
- The claim that the Alpen Multisig Desktop App as currently scoped meets user needs without evidence from actual signers
- Hidden assumptions baked into architecture, UI flows, and feature prioritization that lack validation
- Risk that the final product ships features no signer needs, missing features signers actually require, or UX patterns that don't survive contact with real users
- The "backend is coordination only" assumption — does evidence support that signers will tolerate manual fallback as realistic?

**Boundaries:**
- Reading spec (PRD, SPS-50/51/65) is *not* user discovery
- POCs are *not* user validation
- Feature lists in stories are PRD requirements, not validated user needs

---

## Top Findings (Ranked)

### 🔴 CRITICAL — Zero User Interviews / Signer Feedback

**Severity:** Blocking  
**Evidence:**
- `docs/2-discovery/02-discovery.md` — discovery iteration 1 contains 6 "Risk Areas" and 5 "Open Questions" **all framed as technical unknowns**, zero user input
- POC outcomes (POC 1–5 across `docs/2-discovery/03-poc1-findings.md` through `docs/2-discovery/16-poc5-trezor-findings.md`) validate *protocols work*, not *signers want this*
- `docs/3-stories/story-map.md` derives stories directly from PRD §requirements, not from signer interviews
- **No evidence of:** interviews with target signers, usability testing, feedback loops, or adoption signals

**Risk narrative:** The app is being built from protocol requirements alone. If signers are institutional custodians (not solo founders), they may demand approval workflows, audit trails, and key ceremony documentation the PRD never mentions. If signers are geographically distributed, latency on manual sig aggregation may make the "manual fallback" feature aspirational rather than usable.

**Smallest fix:** Conduct 5–8 interviews with actual or proxy signers (Alpen Labs core team, testnet participants) asking:
1. "Walk me through the last time you had to coordinate a governance decision across 5+ parties. What did you use?"
2. "If the backend went down, could your team really aggregate signatures and broadcast manually?"
3. "What would cause you to *not* trust this application with signing decisions?"

---

### 🔴 CRITICAL — "Authority Context" Assumption Not Validated

**Severity:** Blocking  
**Evidence:**
- PRD §1.8.3 states: "The user MUST be able to clearly read and understand each message they are signing on their hardware wallet screen, to be able to visually verify that the message they are signing matches what they are expecting based on what they are seeing in the application UI."
- **Not validated:** Does the 32-byte SPS-65 digest print legibly on Ledger/Trezor screens? Can a non-cryptographer signer *actually* verify it matches the UI?
- `docs/2-discovery/16-poc5-trezor-findings.md` shows Trezor can display a 32-byte hex string; **no evidence** that real signers can validate it in <5 seconds or understand what they're validating

**Risk narrative:** A signer sees a 32-byte digest on their device. The UI says "Update Strata Sequencer Key." The signer has no way to cryptographically verify the digest matches that claim. A compromised client could lie about the action, and the signer has only a hex string to verify against.

The PRD assumes "clearly read and understand"—but 32 bytes of hex is not understandable. This is a **signer safety assumption that lacks evidence**.

**Smallest fix:** Test with 3 non-developer signers (or proxies) using a real Ledger/Trezor:
1. Present a legitimate action (e.g., add signer) + digest
2. Present a different action + the same digest
3. Measure: Can they spot the difference? How long does it take?

---

### 🔴 CRITICAL — "Manual Fallback Works" Not Tested With Users

**Severity:** Blocking  
**Evidence:**
- Backend PRD (§2. Operational Assumptions): "In the event that the backend becomes unavailable, signers MUST still be able to: construct valid approval or cancellation transactions, aggregate signatures manually, broadcast transactions directly to Bitcoin."
- **Not validated:** Can 5 signers actually do this without backend in <30 minutes?
- POC findings test protocol correctness, not UX of manual fallback
- No scenario testing: backend down, 4 signatures collected, signer needs to export them and coordinate broadcast

**Risk narrative:** If the backend fails during a critical governance action, signers must manually:
1. Export the proposal payload
2. Coordinate signature collection via email/Slack
3. Paste signatures back into the app
4. Construct the Bitcoin transaction
5. Pay attention to fee rates

Each step is a drop-off point. If even one signer doesn't understand, the action stalls. A signer might panic and restart, creating duplicate signatures. The proposal might expire during coordination.

No evidence that this workflow is usable at all.

**Smallest fix:** Run a tabletop sim with 3 actual signers:
- Backend deliberately offline
- Propose a real action type
- Time to execution / measure friction points
- Capture unscripted questions

---

### 🟠 HIGH — Feature Prioritization: Payout Admin Sliced Into Iteration 4 Without User Demand Signal

**Severity:** High  
**Evidence:**
- PRD §16–19 (Payout Administrator) is ~20% of PRD content and defines critical bridge security
- Story map places it in Slice 4 (last substantive feature slice), while Slice 0 (walking skeleton) is Strata Admin only
- **No evidence that:**
  - Payout signers prioritize this over other authorities
  - Manual `block_payout` construction is a real workflow (or a sign of backend failure)
  - The "automatic block payouts" feature is actually wanted vs. needed for compliance

**Risk narrative:** WakeUp built `block_payout` features (manual + automatic) based on PRD, but the PRD came from Alpen, not from payout signers. If payout signers rarely create block payouts (e.g., because automation is built upstream), the entire feature may be theater. Alternatively, if they *constantly* need to, deferring it to iteration 4 means the walking skeleton doesn't include them.

**Smallest fix:** Ask Alpen: "Who are the payout signers today, and how often do they manually construct block payouts?" If <1x per month per person, it's candidate for deferral.

---

### 🟠 HIGH — "Five Multisig Authorities" Treated As One Signer Type; No Role-Specific Evidence

**Severity:** High  
**Evidence:**
- PRD lists 5 authorities (Alpen Admin, Strata Admin, Sequencer Manager, Security Council, Payout Admin) with identical governance workflows but **different operational contexts**
- **No evidence of:**
  - How many signers are members of *multiple* authorities (role sprawl risk)
  - Whether the "same signer set" assumption holds across authorities
  - If Security Council role is emergency-only (different UX expectations?)
  - If Sequencer Manager signers are technical operators vs. governance actors

**Risk narrative:** A signer who is both Strata Admin and Sequencer Manager sees both authorities in the list. The UI doesn't distinguish "routine governance" from "emergency action." A signer accidentally approves the wrong action because they weren't paying attention to which authority they were in. Or they confuse sequence numbers across authorities and think a proposal is stale when it's not.

**Smallest fix:** Interview 2–3 signers from different authorities (or proxies):
1. "Are you typically in one authority or multiple?"
2. "Do different authorities have different urgency/risk profiles?"
3. "How do you track which sequence number belongs to which authority?"

---

### 🟠 HIGH — UX Assumption: Copy/Paste Signature Workflow Is Realistic

**Severity:** High  
**Evidence:**
- PRD §1.13.2, §1.13.3, §1.17.2, §1.17.3: "The user MUST be able to copy all available approval signatures for a given update to their clipboard."
- **Not validated:** Do signers actually use copy/paste, or do they balk at the security implications?
- No testing with real signers on whether they trust pasting 70+ character hex strings

**Risk narrative:** A signer copies a signature to clipboard, pastes it in Slack to coordinate, and the string now lives in Slack's servers and chat history. A signer pastes an old signature by accident instead of the new one. The workflow is technically possible but operationally risky in ways the PRD never discusses.

**Smallest fix:** Ask target signers: "How would you currently share a signature with 4 other people?" Let them describe their process, then test if copy/paste fits it.

---

### 🟡 MEDIUM — Hardware Wallet Selection: No User Testing of Address Listing UX

**Severity:** Medium  
**Evidence:**
- PRD §1.6.2, §1.6.5: User browses first 20 addresses, selects one, verifies on-device
- `docs/2-discovery/16-poc5-trezor-findings.md` confirms Trezor can list addresses; **no UX testing**
- No evidence: Do signers know which address to pick? Do they understand the derivation path? Do they panic when they see 20 different addresses?

**Risk narrative:** A non-technical signer connects a Ledger and sees a list of 20 BIP-86 addresses. They don't understand why there are 20. They pick the first one because "it's default." Later, they can't find their address in another tool because they didn't understand the derivation path. Frustration → support ticket.

**Smallest fix:** Usability test with 3 non-developer users:
- Connect hardware wallet
- Ask: "Pick the address you'd use for signing governance"
- Measure: Do they pick the right one? How long? Do they ask questions?

---

### 🟡 MEDIUM — "Signers Will Understand Sequence Numbers" Not Validated

**Severity:** Medium  
**Evidence:**
- PRD §1.13.1: "each shows time left before 7-day expiry and `collected / required` approval signatures"
- Backend PRD: Sequence number validation, max_seqno_gap enforcement
- Story map (US-D1): Signers understand that proposals advance by sequence number
- **No evidence** that non-developer signers understand sequence numbers or can debug them

**Risk narrative:** A proposal arrives with seqno 5. The last confirmed seqno is 2. The UI says "waiting for signatures." A signer doesn't understand: Is seqno 3 missing? Should I reject this? The backend logic allows skipping, but a signer might think it's an error. Or an attacker could propose seqno 100 to test if the signer would accept garbage. Without signer understanding, this mechanism is security theater.

**Smallest fix:** Ask 3 signers: "What does a sequence number mean to you? How would you know if one was wrong?"

---

### 🟡 MEDIUM — No Market Segmentation: Are Signers Institutional vs. Individual?

**Severity:** Medium  
**Evidence:**
- PRD lists "roles" but never segments by organizational context
- No evidence: Are signers employed by Alpen Labs? Externally operating custodians? DAOs? Solo founders?
- Story map treats all signers identically

**Risk narrative:** If signers are employed and physically co-located, they'll demand in-person key ceremonies. If they're distributed custodians, they'll demand legal agreements and liability waivers. If they're DAOs, they'll demand shared controls and audit logs. The PRD assumes all signers are independent operators who can unilaterally decide to sign, but institutional signers have layers of approval.

**Smallest fix:** Ask Alpen: "Who are the actual signers today? Employment status, geography, governance constraints?"

---

## Attack Narratives (How This Fails in Production)

### Attack 1: Signer Confusion on Authority Scope

**Scenario:** A Strata Admin Signer is also a Sequencer Manager Signer. They log in, see both authorities in the list. They select "Strata Administrator" without paying attention. They then approve a proposal, thinking it's a Strata update, but it's actually a Sequencer Manager action (same signer set, different authority).

**Why it happens:** No warning on authority switch. No confirmation that "you are now viewing Sequencer Manager proposals." The story map assumes all authorities are "the same work."

**Impact:** Unintended approval. Sequence number desync across authorities. Operational chaos.

**Evidence of risk:** Zero UX testing with multi-authority signers.

---

### Attack 2: Backend Down, Manual Fallback Fails

**Scenario:** It's Saturday, 2AM UTC. The backend is down for maintenance. A critical Strata Admin action needs approval from 4 of 5 signers. One signer attempts the manual fallback:
1. Opens the app, exports the proposal payload (hex string)
2. Pastes it in a Slack DM to the other signers
3. Waits 2 hours for signatures (people are asleep)
4. Collects 3 signatures via Slack
5. Realizes they need 4, tries again
6. 6 hours later, the proposal expires (7-day window passes during the wait)

**Why it happens:** The "manual fallback" workflow is not practiced. Signers don't have a documented process. No one knows how to encode the payload or validate signatures offline. It becomes ad-hoc panic engineering.

**Impact:** Governance actions expire and must be re-proposed. Operational delays. Loss of trust in the system.

**Evidence of risk:** No tabletop sim of backend failure. No documented offline procedure.

---

### Attack 3: Signer Fatigue on Digest Verification

**Scenario:** A signer receives 20 governance proposals in a month. Each one displays a 32-byte digest on their Ledger. After the 15th one, the signer stops verifying the digest. They just glance at the UI, assume it's right, and approve. A 16th proposal is an attack: it updates the sequencer key to a malicious address, but the digest doesn't match the action. The signer approves without noticing.

**Why it happens:** Digest verification requires active cryptographic work. No human can do this for 20 proposals. It's security theater.

**Impact:** Silent governance attack. The sequencer is replaced. The attacker now controls Strata sequencing.

**Evidence of risk:** No evidence that signers can verify 32-byte digests at scale. PRD assumes they can "clearly read and understand" but defines no training or UX affordances.

---

### Attack 4: Institutional Signer Blocked by Lack of Audit Trail

**Scenario:** A custodian firm (3-of-5 multisig members) integrates the app. Their internal compliance requires an audit trail: who signed what, when, with what justification. The app doesn't provide this:
- No "reason for signing" field
- No multi-step approval within the custodian firm (e.g., a lawyer must review before signing)
- No log of rejected proposals
- No external signature of the approval (everything is ephemeral)

**Why it happens:** PRD was written for individual signers, not institutional ones. Story map assumes signers are autonomous.

**Impact:** Custodian firm cannot legally use the app. They build their own wrapper or don't participate in governance.

**Evidence of risk:** No interviews with institutional signers. PRD doesn't mention audit trails, legal holds, or custody workflows.

---

### Attack 5: Sequence Number Desync After Proposal Expiry

**Scenario:** A Strata Admin proposal is posted with seqno=5. It expires after 7 days (no quorum). The backend deletes it (expired proposals are "kept offchain and accessible/visible only to multisig signers"). A new proposal arrives with seqno=6. But a signer was offline during the seqno=5 window and doesn't know it expired. They think they missed something. They ask the other signers, "Did we already approve seqno=5?" No clear answer. Confusion spreads. A signer approves seqno=6 without understanding the gap.

**Why it happens:** Expired proposals are not visible post-expiry. The sequence number gap is not explained. No reconciliation workflow.

**Impact:** Signers lose confidence in the proposal list. They start double-checking off-chain. Operational overhead.

**Evidence of risk:** No user testing of expired proposal flow. No evidence that signers understand why proposals vanish.

---

## Evidence Index (Paths)

### Discovery Artifacts (Where Evidence Should Be)
- `docs/2-discovery/02-discovery.md` — POC plans (POC 1–5), **not user discovery**
- `docs/2-discovery/03-poc1-findings.md` through `docs/2-discovery/16-poc5-trezor-findings.md` — Technical POCs, **no signer feedback**

### PRD / Spec (Unstated Assumptions)
- `docs/0-prd/01-multisig-ui.md` — 125 requirements, **zero sourced from user research**
- `docs/0-prd/02-multisig-backend.md` — Backend design, assumes "signers MUST still be able to" do manual fallback **without testing**
- `docs/1-proposal/01-alpen-multisig-proposal.md` — Phase 2 (UX Design) lists "Wireframes: Low–mid fidelity screens covering main states and edge cases" **no evidence of user validation**

### Features Baked Into Code (High-Risk Assumptions)
- `desktop-app/src/screens/wallet-connect-screen.tsx` — Authority selection hardcoded; Sequencer Manager, Security Council, Payout Admin **disabled** (line 36, 44, 54); **no evidence these should be disabled**
- `desktop-app/src/screens/proposals-dashboard-screen.tsx` — Proposal list assumes signers understand state machine (Pending → Approved → Enacted)

### User-Facing Workflows (Untested)
- Manual `block_payout` construction (PRD §1.19–1.24) — assumes users can calculate standardness limits and fee rates
- Copy/paste signature aggregation (PRD §1.13.2, §1.17.2) — assumes signers trust clipboard + copy/paste security model

---

## Smallest Fixes vs. Largest Bets

### Smallest Experiments (5–10 days, unblock critical risks)

| Experiment | Effort | Risk Reduced | Gate | Recommendation |
|-----------|--------|--------------|------|-----------------|
| **E1: Signer interviews (5–8)** | 3 days | CRITICAL×3 | Understand real signer type, multi-authority workflows, manual fallback willingness | **DO FIRST** |
| **E2: Digest verification UX test (3 signers, Ledger)** | 2 days | CRITICAL (Authority Context) | Can signers actually verify 32-byte digests? | **DO BEFORE LAUNCH** |
| **E3: Manual fallback tabletop (3 signers, offline scenario)** | 2 days | CRITICAL (Manual Fallback) | Can signers actually coordinate without backend? | **DO BEFORE LAUNCH** |
| **E4: Address selection UX test (3 non-devs, hardware)** | 1 day | MEDIUM (HW Wallet UX) | Do they understand which address to pick? | Do before Iteration 1 |
| **E5: Institutional signer interview (1 custodian)** | 1 day | MEDIUM (Audit Trail) | Do institutional signers have legal requirements? | Do if Alpen has a custodian partner |

### Largest Bets (Defer Until Evidence Arrives)

| Feature | Assumption | Evidence Needed | Risk |
|---------|-----------|-----------------|------|
| **Manual `block_payout` construction** | Signers will calculate standardness limits and fee rates | User testing with non-technical signer | HIGH: Feature may be unusable or dangerous |
| **Copy/paste signature workflow** | Signers will trust clipboard + Slack + email for signature aggregation | Signer interviews + security audit | HIGH: May expose signatures or create operational friction |
| **Sequence number validation** | Signers will understand seqno gaps and reject invalid sequences | User testing + error message testing | MEDIUM: Signers might approve garbage or reject valid proposals |
| **Payout Administrator full feature set** | Payout signers want manual + automatic block payouts | Interview payout signers (Alpen, not developers) | MEDIUM: Feature may be built for non-existent workflow |
| **All 5 authorities in Slice 1** | All authorities have identical governance workflows and UX | Role-specific signer interviews | MEDIUM: Security Council may need emergency UX; Sequencer Manager may need technical operator UX |

---

## What Would Change My Mind

**Evidence that WOULD approve handoff to build (no, not yet — needs all of these):**

1. **5+ signer interviews** — structured, past-behavior focused:
   - "Tell me about the last governance action you approved. Walk me through it."
   - "Have you ever been unable to coordinate with other signers? Why?"
   - "If the backend was down, could you and your peers aggregate signatures in <1 hour?"

2. **Usability testing of authority context** (3+ signers):
   - Present a proposal with a 32-byte digest
   - Measure: Can they verify it matches the action in <1 min?
   - Measure: Would they approve it in production after 15+ proposals in a week?

3. **Institutional signer discovery** (1+ custodian):
   - "What does your audit trail require?"
   - "Can you legally use a tool with no multi-step internal approvals?"
   - "Do you need a lawyer to review governance actions?"

4. **Manual fallback scenario test** (3+ signers, backend offline):
   - Benchmark time-to-execution
   - Identify blockers (signature format confusion, coordination friction, expiry risk)
   - Measure willingness to rely on this for critical governance

5. **Feature validation** (targeted per slice):
   - Before Slice 1: "Do signers actually want on-device address verification, or is it security theater?"
   - Before Slice 4: "Do payout signers need manual construction, or is it a sign of bad upstream design?"

6. **Negative case testing** (error scenarios):
   - Non-signer tries to access → clear error + no inference
   - Proposal expires → signer can see it, understands why, reconciles seqno
   - Hardware disconnected mid-signing → recoverable or data loss?

---

## Summary

### Approval Status: **REJECTED — PENDING REVISIONS**

**Blocking Issues:**

1. **Zero user research** — PRD and story map are spec-derived, not signer-validated. Cannot approve handoff without interviews.
2. **Authority context assumption untested** — Signer verification of 32-byte digests is claimed but not validated. Security risk.
3. **Manual fallback untested** — Assumption that signers can coordinate without backend is aspirational, not proven.

**Critical Remediation (before build gate):**

- Conduct 5–8 signer interviews (or proxy if real signers unavailable)
- Run digest verification UX test with real Ledger/Trezor
- Run manual fallback tabletop simulation

**Conditional Approvals (per slice):**

- **Slice 0:** Approve if E1 + E2 + E3 complete and signers confirm basic flow works
- **Slice 1:** Approve address selection only if E4 confirms UX is intuitive
- **Slice 4 (Payout):** Defer or validate with payout signers specifically

---

**Recommendation:** Do not initiate frontend/backend build (Phases 3–4 of proposal) until **at minimum:**
- 5+ signer interviews establish real workflows vs. assumed ones
- 3+ digest verification tests confirm authority context is recognizable
- 3+ manual fallback tests confirm offline coordination is feasible

Current discovery is protocol-focused, not user-focused. The risk of shipping features no signer needs or missing features signers require is **unacceptably high**.

