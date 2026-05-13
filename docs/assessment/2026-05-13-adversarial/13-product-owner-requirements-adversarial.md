# Product Owner / Requirements / UX Journeys — Adversarial Assessment

**Assessment date:** 2026-05-13  
**Threat model:** Multisig UX safety, story/journey coherence, missing DoR, unsafe confirmation flows, payload divergence, edge-case coverage.  
**Scope:** User stories (`docs/3-stories/`), journey coherence (`docs/3-stories/story-map.md`), specs (`docs/specs/proposal-*.md`), PRD requirements (`docs/0-prd/`), frontend screens (`desktop-app/src/screens/`).

---

## Scope & Threat Model (What We're Trying to Break)

### Core Risks
Multisig applications are **safety-critical**: a signer's "Sign" action authorizes value movement on-chain. The classic multisig footgun is **payload divergence**—the user sees one thing in the UI, but signs a different message on the hardware wallet. This audit hunts for:

1. **Incoherent journeys**: Story flows that skip steps, lack emotional arc, or assume infrastructure (backend) that may not exist.
2. **Missing acceptance criteria**: Stories that sound complete but lack BDD-style Given-When-Then scenarios; specs without testable edge cases (backend unavailable, signer compromise, partial signatures, cancellation).
3. **Unsafe UX patterns**: Flows where "Sign" and "Cancel" buttons are ambiguous; displays of amounts/keys that diverge from what the signer is actually signing.
4. **Missing Definition of Ready (DoR)**: Stories that are implemented without satisfying pre-implementation gates (JTBD link, authority context, failure modes).
5. **Coverage gaps in critical flows**: Threshold change, signer key rotation/compromise, proposal cancellation, manual fallback (offline aggregation), backend unavailability.
6. **Authority context bleed**: Screens that allow cross-authority confusion (signer of Authority A accidentally seeing Authority B's proposals).

### Signer Safety Focus
A signer about to authorize a key rotation or safe-harbor update must:
- Clearly understand **what authority** they are acting on.
- See the **before/after state** of any change they are signing.
- Know **how many signatures remain** before the change goes live.
- Understand **what happens if they cancel** or if the backend goes down.
- Have **explicit confirmation** on the hardware wallet showing the same payload.

---

## Top Findings (Ranked by Severity)

### **BLOCKING / CRITICAL**

#### 1. **No explicit "What happens if backend is unavailable?" acceptance criteria in any story**
- **Severity:** CRITICAL (Safety footgun)
- **Evidence:** 
  - `docs/3-stories/story-map.md` lists US-H5 "Compose transaction manually when backend unavailable" in Slice 5 (deferred).
  - `docs/0-prd/02-multisig-backend.md` §2 states: "The backend MUST NOT be a single point of failure... signers MUST still be able to construct valid approval or cancellation transactions, aggregate signatures manually, and broadcast transactions directly to Bitcoin."
  - **No spec** defines what the desktop app does when the backend is unreachable. UI has no error messaging, fallback flow, or guidance.
  - `orchestrator-be/src/` is the backend; there is no documented "offline mode" or "graceful degradation" in the desktop app for backend downtime.
- **Risk:** Signers may believe they cannot act if the backend is down. Governance may stall. The feature is promised but not specified.
- **Recommendation:** 
  - **Immediate:** Write an acceptance criteria block for US-H5 that includes:
    - "When orchestrator is unreachable, user can see 'Backend unavailable' banner."
    - "User can export the proposal payload in a format suitable for offline aggregation (actionId, seqNo, serialized action)."
    - "User can manually construct the SPS-65 approval transaction from the exported payload."
    - "Backend unavailability does not delete or corrupt pending proposals."
  - Add a spec `docs/specs/offline-fallback-flow.md` before Slice 5.
  - Add a test scenario: "Signer is offline, backend is online; signer aggregates 2 signatures offline, then comes online and adds the 3rd."

---

#### 2. **Payload divergence footgun: no acceptance criteria for "message signer sees on hardware wallet matches UI payload"**
- **Severity:** CRITICAL (Classic multisig safety bug)
- **Evidence:**
  - `docs/specs/proposal-signing-and-dashboard-status-alignment.md` §Sign Proposal Screen lists:
    - "Sighash display: `SPS-65 Sighash (32 bytes)` label, Copy button"
    - "Hardware wallet safety callout: explicit instruction to verify value on device before confirming."
  - BUT: No acceptance criteria specifies **what the hardware wallet screen displays**.
  - `docs/2-discovery/16-poc5-trezor-findings.md` (implied in the specs) states that "sign_message with SPENDTAPROOT is rejected by current Trezor firmware."
  - **The desktop app is signing a PSBT or raw message, but the Trezor screen shows a different representation than what the user sees in the UI.**
  - `desktop-app/src/screens/sign-poc-screen.tsx` does not exist in the read output; current signing flow is unspecified.
- **Risk:** A signer can be tricked into signing a payload (e.g., a signer set change) while the hardware wallet display shows a different message (e.g., "Sign message"). The signer cannot verify the payload matches.
- **Recommendation:**
  - **Immediate:** Add a spec `docs/specs/sps65-signing-visualization.md` that defines:
    - How the desktop app will display the SPS-65 action and sighash so the signer can verify it matches the device screen.
    - What BIP-137 or PSBT message encoding will be used; what the Trezor screen will show (e.g., "Sign message" + first 32 bytes of payload).
    - A test scenario: "Signer sees 'Update Strata Admin signer' in UI, sees the before/after key, and the device screen shows a SHA256 digest. The signer can confirm the digest matches by comparing hex manually."
  - Link this to US-F1 / US-I4 acceptance criteria: "User can read and verify the displayed message on the hardware wallet screen against the UI payload."

---

#### 3. **No acceptance criteria for proposal state conflict: signer is signing, then status changes to 'enacted' or 'canceled' mid-flow**
- **Severity:** CRITICAL (Concurrent modification)
- **Evidence:**
  - `docs/specs/proposal-signing-and-dashboard-status-alignment.md` §Data and Behavior Requirements states:
    - "If proposal is no longer `pending` at time of signing: Block signing action. Show high-signal conflict message."
  - BUT: No test case, no error surface spec, no network retry logic.
  - `desktop-app/src/screens/sign-poc-screen.tsx` is not shown; current state-check implementation is unknown.
  - Story map US-F1 "Approve pending proposal" has no acceptance criteria covering "what if proposal was canceled by another signer while I'm signing?"
- **Risk:** User clicks "Sign," hardware wallet is in hand, but backend says "proposal is now canceled." User sees an error and may retry, or may believe their prior action was submitted. Ambiguous UX.
- **Recommendation:**
  - Add acceptance criteria to US-F1 and US-I4:
    - "Before signing, proposal state is fetched and verified to be pending."
    - "If proposal is no longer pending, signing is blocked with a clear, high-signal message: 'This proposal is no longer pending and cannot be signed. It was [enacted/canceled/expired] by other signers.'"
    - "User has an option to 'Back to proposals' to see the updated state."
  - Add test scenario: "Two signers reach quorum simultaneously; the first broadcasts and enacts; the second's sign flow must reject with the conflict message."

---

#### 4. **Authority context is not visually reinforced in the Sign and Broadcast screens; signer of Strata Admin can be confused about which authority they're acting on**
- **Severity:** CRITICAL (Cross-authority confusion)
- **Evidence:**
  - `desktop-app/src/screens/broadcast-proposal-screen.tsx` line 17 shows:
    ```tsx
    const authorityLabel =
        selectedRole === AuthRole.StrataAdministrator ? 'Alpen Administrator' : 'Alpen Sequencer Manager'
    ```
    This is **backwards**: StrataAdministrator is mapped to "Alpen Administrator" label. Bug or intentional? Unclear.
  - `desktop-app/src/screens/proposals-dashboard-screen.tsx` line 29 shows correct label.
  - No acceptance criteria specifies that every screen (create, sign, broadcast, dashboard) must display the current authority role prominently and consistently.
- **Risk:** A signer of multiple authorities might be on the Strata Admin screen but accidentally sign a Strata Admin update while thinking they're on the Alpen Admin screen (if both authorities offer similar update types). The backend enforces isolation, but the UX doesn't make the context salient.
- **Recommendation:**
  - Add acceptance criteria to all story creation/signing/broadcast stories (US-E*, US-F*, US-H*, US-I*):
    - "Authority label is displayed prominently in the header of every screen."
    - "Authority label color and badge style are consistent across all screens and match the dashboard."
    - "Signer cannot proceed to Sign without explicitly confirming the authority they are acting on."
  - Fix the apparent bug in `broadcast-proposal-screen.tsx`.
  - Add a test scenario: "Signer is a member of both Strata Admin and Alpen Admin. They select Strata Admin on the dashboard. The sign screen must display 'Strata Administrator' in the header and in the authority context block."

---

### **HIGH SEVERITY**

#### 5. **Threshold and signature progress not tracked per-proposal; dashboard counter uses UI heuristics instead of authoritative backend value**
- **Severity:** HIGH (State divergence)
- **Evidence:**
  - `docs/specs/proposal-signing-and-dashboard-status-alignment.md` §Data and Behavior Requirements:
    - "Proposal payload returned by backend MUST include `requiredSignatures` as a per-proposal snapshot of the authority threshold at creation time."
    - "Dashboard signature counter MUST use `collected_signatures / requiredSignatures` from proposal data and MUST NOT derive required signatures using UI heuristics."
  - `docs/3-stories/story-map.md` US-D1 acceptance signal: "each shows time left before 7-day expiry and `collected / required` approval signatures."
  - **Implementation status in spec is "Deferred / follow-up"**: "Frontend API wrapper (`desktop-app/src/api/*`) proposal call-site integration is still pending because proposal creation UI flow is not yet wired in this branch."
- **Risk:** If a threshold changes on-chain (e.g., from 3-of-5 to 2-of-5), a proposal created under 3-of-5 should still need 3 signatures, not 2. If the UI derives the required threshold from the current authority state instead of the per-proposal snapshot, the signer will see incorrect progress ("1 / 2 required") when it should be "1 / 3 required." Signer may believe the update can be broadcast prematurely.
- **Recommendation:**
  - **Immediate:** Verify that `Proposal` type in frontend includes a `requiredSignatures` field and is populated from the backend.
  - Add test scenario: "Threshold changes mid-proposal. Pending proposal created under 3-of-5 still shows '2 / 3 required' even though the authority is now 2-of-5."
  - Add acceptance criteria to US-D1: "requiredSignatures value is fetched from backend proposal snapshot, never derived from current authority state."

---

#### 6. **"Pending" state definition is ambiguous: includes both "collecting signatures" and "collected quorum but not yet broadcast"**
- **Severity:** HIGH (State confusion)
- **Evidence:**
  - `docs/specs/proposal-signing-and-dashboard-status-alignment.md` defines four sections:
    - "Quorum reached" (contains `approved`)
    - "Pending" (contains `pending`)
    - "Executed & Canceled" (contains `enacted`, `canceled`)
    - "Expired / Skipped" (contains `expired`)
  - But in the backend/SPS-65, "Approved" means "on-chain confirmed + quorum reached + ready to enact."
  - "Pending" in the UI means "offchain proposal collecting signatures."
  - **The spec conflates two meanings**: "Pending" (still collecting) vs. "Quorum reached but not broadcast" (ready to broadcast).
  - `docs/specs/proposal-signing-and-dashboard-status-alignment.md` mentions: "Pending updates that have reached quorum but have not been confirmed yet MUST have a 'Send' button." But the dashboard section named "Pending" does NOT show the Send button; instead, "Quorum reached" shows the Broadcast button.
- **Risk:** A signer may see a proposal in the "Pending" section and assume it still needs their signature. The proposal could have 2 of 2 required signatures (quorum reached) but still be listed under "Pending" if it has not yet been broadcast on-chain. Confusion about whether action is needed.
- **Recommendation:**
  - Clarify and rename dashboard sections:
    - **Option A:** "Collecting Signatures" (pending, < quorum) | "Ready to Broadcast" (pending, >= quorum) | "Enacted / Canceled" | "Expired"
    - **Option B:** Keep current 4 sections but add a visual indicator (e.g., badge "QUORUM REACHED") to pending cards that have >= required signatures.
  - Add acceptance criteria to US-D1 that precisely defines what goes in each section by status + signature count.
  - Add test scenario: "Proposal reaches quorum. Its section membership does NOT change (still under 'Pending'), but a 'Broadcast' button appears on the card."

---

#### 7. **No acceptance criteria for "signer set change" proposals; no story describes how a signer rotation or compromise is handled**
- **Severity:** HIGH (Missing critical flow)
- **Evidence:**
  - Story map lists US-E4 "Create Alpen signer update" and similar update types, but does NOT define a signer-set-change **flow**.
  - `docs/0-prd/01-multisig-ui.md` §1.15.1 lists update types:
    - "Alpen verification key update" / "Alpen Administrator Signer update" / "Strata Administrator Signer update" / etc.
  - But the PRD does NOT specify:
    - What information the signer sees when proposing a signer set change (old set vs. new set?).
    - How a signer who is being **removed** is notified or handles the update.
    - What happens if a signer's key is **compromised** (no explicit story for "I need to rotate my key before this proposal gets broadcast").
  - Acceptance criteria for US-E4 (Alpen signer update) is simply: "Proposal is persisted with stable ActionId... Creator's signature is stored."
- **Risk:** A signer may propose removing themselves from the multisig by mistake (or due to compromised key). They see "Alpen signer update" in the UI but the UI does not show them the before/after signer sets. They sign it. No story covers the negative: "What if I realize my key is compromised before the broadcast?"
- **Recommendation:**
  - Add a story US-E_ROTATE: "As a Signer whose key is compromised, I want to urgently propose my own removal from the signer set so that the compromised key cannot act, so that governance is not blocked by a rogue signer."
    - Acceptance: "Signer can propose a signer-set-change that removes their own address. The UI shows the before/after sets clearly. The signer can see how many signatures are needed to approve the removal."
  - Add a story US-D_ROTATION: "As a Signer reviewing a signer-set-change proposal, I want to see the before/after signer lists so that I can audit the proposed changes before signing."
    - Acceptance: "Proposal details block shows 'Old signer set' and 'New signer set' with clear visual diff (added/removed)."
  - Add test scenario: "Alice is compromised and proposes her own removal. Bob and Charlie see the proposal with clear before/after sets. They approve. The proposal is broadcast and enacted. Alice can no longer sign on-chain."

---

#### 8. **Definition of Ready (DoR) is NOT stated in story map or specs; no gate before handing off to DESIGN/implementation**
- **Severity:** HIGH (Process risk)
- **Evidence:**
  - `docs/3-stories/story-map.md` has no section titled "Definition of Ready" or "DoR checklist."
  - `docs/3-stories/README.md` (not fully read) may contain it; but it is not visible in the story-map or repeated in individual story cards.
  - Stories like US-F1 state: "Acceptance signals: Approval signature produced for any pending update; hardware wallet screen displays a human-readable representation of the message being signed."
  - But "human-readable representation" is vague. What is human-readable? How is it verified?
  - No story includes a field for:
    - JTBD (Job to be Done) link
    - Failure modes / error handling
    - Dependency on upstream (e.g., "Alpen crate must support this update type")
    - Testing strategy (unit / integration / e2e / manual)
- **Risk:** A story may be handed off to implementation without satisfying foundational gates (e.g., "Does the Alpen crate support this action type?" is listed as a dependency but not checked before coding starts). Developers implement against an incomplete spec.
- **Recommendation:**
  - Add a DoR section to the story-map that defines 8 mandatory items:
    1. JTBD link or rationale (why this signer action is necessary).
    2. Failure modes identified (what can go wrong?).
    3. Authority context specified (which authority? all or specific roles?).
    4. Acceptance criteria in Given-When-Then format (at least 3 scenarios).
    5. Dependencies listed (Alpen crate features, RPC methods, backend endpoints).
    6. Non-functional requirements cited (timeouts, performance, security).
    7. Test coverage plan (unit / component / integration / manual).
    8. Edge cases listed (backend down, state conflict, session expiry).
  - Mark stories that fail DoR (e.g., US-E3 thru US-E13 depending on missing Alpen crate) as "BLOCKED: awaiting upstream support."

---

### **MEDIUM SEVERITY**

#### 9. **No story for "what if I'm a signer on multiple authorities and accidentally switch mid-proposal?"**
- **Severity:** MEDIUM (UX friction)
- **Evidence:**
  - Story map lists US-C1 "Select a multisig authority" and US-C4 "Close the multisig session."
  - But there is no acceptance criteria for "what if I start creating a proposal on Strata Admin, then close the session, and open Alpen Admin?"
  - No test scenario covers the state machine: "Create Strata Admin signer update proposal → close session → open Alpen Admin → see pending proposals only from Alpen Admin → no cross-authority leakage."
- **Risk:** Signer confusion; accidental cross-authority state leakage in the UI (seeing proposals from the wrong authority cached in memory).
- **Recommendation:**
  - Add acceptance criteria to US-C4: "Closing the multisig session clears all in-flight proposal data and dashboard state. Reopening a different authority starts fresh."
  - Add test scenario: "Create proposal on Authority A → close session → open Authority B → verify no draft data from Authority A is visible."

---

#### 10. **"Expired" proposals: no acceptance criteria for what the UI does to a proposal that expires while the dashboard is open**
- **Severity:** MEDIUM (State consistency)
- **Evidence:**
  - `docs/0-prd/01-multisig-ui.md` §1.13.3: "A 'Pending' update MUST expire if it has not been approved within `7` days after the update is first proposed."
  - `docs/3-stories/story-map.md` US-D4 "List past proposals": acceptance is "Lists updates that have been enacted, canceled, or expired."
  - BUT: No story defines when the backend removes an expired proposal, or how the desktop app is notified.
  - No acceptance criteria for: "What if I'm looking at a proposal in the pending section, 7 days pass, the proposal expires. Does the app auto-refresh and move it to 'Past'? Does the user have to manually refresh?"
- **Risk:** Stale UI. User thinks a proposal is still pending and tries to sign it; backend rejects because it is now expired.
- **Recommendation:**
  - Add acceptance criteria to US-D1 and US-D4:
    - "Dashboard auto-refreshes every 60 seconds (or less). Expired proposals are automatically moved to the 'Past' section."
    - "When a proposal expires, a visual notification appears: 'Proposal expired. It did not reach quorum in 7 days.'"
  - Add a backend spec clarifying when expired proposals are deleted vs. archived.

---

#### 11. **No acceptance criteria for "Payout Admin" flows; "manual fallback" (US-I8) has no test scenario**
- **Severity:** MEDIUM (Untested complex flow)
- **Evidence:**
  - Story map lists US-I1 thru US-I9 for Payout Administrator.
  - `docs/3-stories/story-map.md` under "Dependencies & Risks": "Payout Admin architecture unknown (affects Slice 4): Payout is not part of SPS-65 — it is a Bitcoin-native UTXO spend from a bridge multisig script. Script templates and spending conditions are not documented. Slice 4 may need its own mini-discovery."
  - US-I8 "Manually construct a block_payout transaction" has acceptance criteria: "User can provide `block_payout` inputs and attach their own signature to create a new pending payout."
  - But there is no test scenario showing how the user obtains the `block_payout` inputs, or what format they are in, or how they are validated.
- **Risk:** Implementation stalls because the payout script and UTXO format are undocumented. Test scenario is impossible to execute.
- **Recommendation:**
  - Add a discovery task: "Determine payout script template, UTXO format, and signing constraints before Slice 4."
  - Defer Slice 4 until payout architecture is documented.
  - Add acceptance criteria to US-I8: "User can import a `block_payout` UTXO in the format [specify format]. The app validates the UTXO against the canonical bridge script. User can construct a spend transaction and sign it."

---

#### 12. **No journey map or emotional arc defined; stories lack context about signer motivation and goal**
- **Severity:** MEDIUM (Product coherence)
- **Evidence:**
  - `docs/3-stories/story-map.md` is a functional breakdown (Activity A → B → C...) but does not include:
    - Emotional arc (how does the signer feel at each step? Confidence → apprehension → relief).
    - Journey narrative (why is this signer doing this? Are they responding to an emergency, or managing routine governance?).
    - Failure paths (what if the signer aborts? What if the backend is down? What is the sad path?).
    - Shared artifacts / constraints (e.g., "This proposal requires Alice AND Bob to sign. Alice cannot broadcast without Bob.").
  - Story map is a flat list of stories per activity, not a narrative of how multiple signers collaborate.
- **Risk:** Frontend UX is built story-by-story without understanding the shared context. A signer may complete their action but have no visibility into whether other signers have acted.
- **Recommendation:**
  - Add a journey narrative section to story-map:
    - "A Strata Admin signer discovers an urgent need to rotate a signer key (compromised). They propose the rotation, sign it, then wait for 2 other signers to approve. Once quorum is reached, they (or any signer) can broadcast the proposal on-chain. A 7-day delay allows cancellation before enactment. If the backend goes down mid-proposal, any signer can manually aggregate signatures and broadcast."
  - Add shared-artifacts table:
    - "All signers of Authority X can view pending proposals created by any signer on Authority X."
    - "A signer cannot edit or cancel a proposal they did not create."
    - "Quorum is authority-specific; rotating from 3-of-5 to 2-of-5 does not affect pending proposals created under the old threshold."
  - Add a failure path story: "What if I realize mid-broadcast that the proposal contains an error?"

---

### **LOW SEVERITY**

#### 13. **Sign screen mentions "Trezor" by name in acceptance criteria; Ledger support is mentioned as stubbed but not tested**
- **Severity:** LOW (Incomplete platform support)
- **Evidence:**
  - `docs/specs/proposal-signing-and-dashboard-status-alignment.md` §Sign Proposal Screen, Primary action: "`Sign with Trezor`"
  - `docs/specs/poc5-trezor-hw-wallet-integration.md` §POC-5 artifacts preserved: "Ledger support (stub remains, pending Speculos validation)"
  - But story map US-B1 states "Supported hardware wallets MUST include all hardware wallets currently supported by HWI..."
  - Acceptance criteria for US-F1 should generalize to "Sign with hardware wallet" not "Sign with Trezor."
- **Risk:** Ledger signer is confused; spec says "Sign with Trezor" but Ledger signer has a Ledger device.
- **Recommendation:**
  - Update spec to use "Sign with hardware wallet" or dynamically display device type (e.g., "Sign with ${deviceName}").
  - Add test scenario for Ledger: "Ledger signer connects Ledger device, sees 'Sign with Ledger' button, proceeds to sign."

---

#### 14. **No explicit requirement that "displayed data (before/after values) MUST be human-readable and match the blockchain transaction"**
- **Severity:** LOW (Clarity gap)
- **Evidence:**
  - `docs/specs/proposal-signing-and-dashboard-status-alignment.md` mentions:
    - "Proposal summary card: Proposal identifier (seq-based label), authority, proposal type, proposal title."
    - "Payload review area: For key update style payloads: `Before` and `After` values. For non-diff payloads: fallback structured details block."
  - But no requirement that the before/after values are **human-readable** (not hex, not serialized).
  - No requirement that the displayed value matches the bytes that will be broadcast on-chain.
- **Risk:** Signer sees "Old key: abc123..." (truncated hex) and cannot verify it matches the bytes signed.
- **Recommendation:**
  - Add acceptance criteria: "Before/After values are displayed in a human-readable format (e.g., full bech32 address, not truncated hex). Signer can copy the full value to verify against external tools."
  - Add test scenario: "Signer is updating Strata Admin signer set. UI shows old set: [alice_pubkey_full_bech32, bob_pubkey_full_bech32, charlie_pubkey_full_bech32]. New set: [alice_pubkey_full_bech32, bob_pubkey_full_bech32, diana_pubkey_full_bech32] (charlie removed, diana added). Signer can copy each key and verify it matches their records."

---

## Attack Narratives (3–6): "How This Fails in Production / For a Signer / For Maintainers"

### Narrative 1: Backend Downtime During Critical Signer Rotation
**Threat:** A signer's key is compromised. They urgently propose their own removal from the Strata Admin multisig. Two other signers (Bob and Charlie) are online and ready to approve. Just as Charlie is about to sign, the orchestrator backend goes down (DB failure, network partition).

**Current State:**
- No acceptance criteria for "backend unavailable" in US-F1 or US-H5.
- No spec for offline fallback flow.
- Desktop app has no error screen or fallback guidance.

**What Happens:**
1. Charlie clicks "Sign" on the dashboard.
2. The app tries to fetch the proposal payload from the orchestrator to show the before/after signer set.
3. The orchestrator responds with a 503 (Service Unavailable).
4. The app shows an error: "Failed to load proposal. Please try again."
5. Charlie clicks "Retry" repeatedly. Nothing works.
6. Charlie gives up, assuming governance is blocked.
7. The compromised signer can still vote and approve updates on-chain (no on-chain removal is in progress because the proposal was never broadcast).

**Why This Is Bad:**
- Governance is stalled for hours/days (orchestrator may take time to recover).
- The compromised key remains active.
- No story or spec covers this; maintainers have no guidance on recovery procedure.

**Fix:** Write US-H5 acceptance criteria and offline fallback spec before going to production. Test recovery scenario.

---

### Narrative 2: Payload Divergence During Emergency Defcon-1 Response
**Threat:** The Security Council needs to execute a Defcon-1 emergency action immediately. A signer (Alice) opens the app to create the proposal. She sees a form asking for the emergency parameters (e.g., threshold change from 3 to 1). She fills it out and clicks "Sign."

**Current State:**
- No acceptance criteria linking the UI payload to the hardware wallet message.
- Spec mentions SPS-65 Sighash display but no test scenario confirming the message on the device matches the UI.
- PSBT encoding is not documented; Trezor firmware limitations are noted but mitigation is not specified.

**What Happens:**
1. Alice's desktop app encodes the Defcon-1 action as a PSBT or BIP-137 message.
2. Alice clicks "Sign with Trezor."
3. Her Trezor device shows: "Sign message: 0x3a4b5c6d..." (the SPS-65 sighash digest).
4. Alice cannot verify the digest matches the UI (no side-by-side comparison).
5. Alice signs, assuming it's correct.
6. The signature is appended to the proposal.
7. Two weeks later, the proposal is broadcast on-chain. It turns out the Defcon-1 action was **different** from what Alice thought she was signing (e.g., someone changed the parameter in the backend between the proposal creation and the broadcast).
8. Alice has authorized an unintended emergency action.

**Why This Is Bad:**
- Emergency responses require the highest signer confidence.
- Alice cannot verify the payload on the hardware wallet; she is signing blind.
- No spec covers this gap; implementers have no guidance on safe signing visualization.

**Fix:** Write `docs/specs/sps65-signing-visualization.md` before Slice 1. Define exactly what the hardware wallet screen will display and how the signer can verify it. Test with real Trezor hardware.

---

### Narrative 3: Threshold Change Snapshot Mismatch
**Threat:** Alpen Admin authority has a threshold of 3-of-5. Alice proposes an Alpen signer update. Two signers (Bob and Charlie) approve. Just as the proposal is about to broadcast, the authority threshold changes on-chain to 2-of-5 (a separate Defcon-1 action).

**Current State:**
- No acceptance criteria requiring `requiredSignatures` to be a per-proposal snapshot.
- Dashboard counter is documented to use authoritative backend value, but the feature is "Deferred" and not implemented.
- Frontend API integration is pending.

**What Happens:**
1. Proposal is created under 3-of-5 threshold; backend stores `requiredSignatures: 3`.
2. Alice, Bob, and Charlie have signed (3 / 3).
3. Authority threshold changes to 2-of-5 on-chain.
4. Charlie's dashboard shows "2 / 2 required" (derived from current authority state, not proposal snapshot).
5. Charlie believes the quorum is only 2 and that the proposal can be broadcast.
6. Charlie checks the orchestrator; it says "Approved" because 3 >= 2.
7. Charlie broadcasts the proposal.
8. On-chain, the ASM verifies the proposal has 3 valid signatures but the **current** threshold is 2-of-5. The proposal is accepted (>= quorum).
9. A moment later, another signer notices that the proposal was broadcast under different threshold assumptions. Governance confusion.

**Why This Is Bad:**
- Signer confusion about what "quorum" means for a given proposal.
- Possible governance error if the threshold change was intended to have different requirements for pending proposals.

**Fix:** Implement the `requiredSignatures` field as documented in the spec. Write a test scenario for threshold changes mid-proposal. Verify dashboard counter uses per-proposal snapshot, not current authority state.

---

### Narrative 4: Cross-Authority Signer Confusion
**Threat:** Alice is a signer on both Strata Administrator and Alpen Administrator multisigs. She logs into the app, selects Strata Administrator, and reviews the pending proposals dashboard. The UI shows a list of signer update proposals. Alice is tired and doesn't carefully read the authority label.

**Current State:**
- Authority label is present but may not be salient (bug in `broadcast-proposal-screen.tsx` shows wrong label).
- No acceptance criteria requiring consistent authority display across all screens.
- No test scenario for multi-authority signers.

**What Happens:**
1. Alice sees a pending proposal with title "Update Signer Key."
2. Alice assumes it's a Strata Admin proposal (because that's what she selected).
3. Alice clicks "Sign."
4. The sign screen loads, but the authority label is small or misaligned.
5. Alice signs the proposal.
6. Alice realizes moments later (or days later, on seeing the enacted proposal on-chain) that she signed an Alpen Admin proposal, not a Strata Admin proposal.
7. The wrong authority was updated; governance is in an unintended state.

**Why This Is Bad:**
- Authority context bleed is a critical safety bug in multisig UX.
- No mitigation in the current story map or spec.

**Fix:** Fix the authority label bug. Add acceptance criteria to all signing/broadcast stories requiring consistent, prominent authority display. Write a test scenario: "Multi-authority signer signs on Authority A, closes session, opens Authority B, verifies they're on the correct authority before signing."

---

### Narrative 5: Cancellation Ambiguity for Emergency Updates
**Threat:** A signer discovers that an approved update (scheduled to execute in 7 days) contains an error. The signer urgently wants to cancel it. They open the dashboard and look for the update in the "Quorum reached" section.

**Current State:**
- Cancellation flow (US-F2, US-H2) is in Slice 3 (deferred).
- No acceptance criteria for "what does the signer see in the dashboard for an approved proposal that is cancellation-eligible?"
- Story map does not define what the cancellation UI looks like or how the signer knows it's possible.

**What Happens:**
1. Signer finds the approved proposal in the dashboard.
2. Dashboard shows: "Status: Approved | Broadcast button" (from current spec).
3. Signer looks for a "Cancel" button or option. None is visible.
4. Signer assumes cancellation is not supported and gives up.
5. The update executes 7 days later, causing unintended changes.

**Why This Is Bad:**
- Cancellation is a critical safety valve for emergency updates.
- If the UX does not surface it, signers cannot use it.
- No story or spec covers the happy path for cancellation.

**Fix:** Add a spec `docs/specs/approved-proposal-cancellation-flow.md` before Slice 3. Define the UI for approved proposals, including a prominent "Cancel" button or action menu. Write a test scenario: "Signer sees approved proposal with 'Approve' and 'Cancel' buttons. Signer clicks 'Cancel,' enters cancellation signatures, and broadcasts the cancellation transaction."

---

### Narrative 6: Manual Fallback in a Disaster Scenario
**Threat:** The orchestrator backend is down for an extended period (e.g., ransomware attack, data loss). A critical governance action needs to happen. Three signers (Alice, Bob, Charlie) are in a secure room with their hardware wallets. They need to construct and broadcast a proposal without the backend.

**Current State:**
- US-H5 "Compose transaction manually when backend unavailable" is in Slice 5 (deferred).
- No spec for offline fallback.
- No documentation or UI guidance for manual aggregation.
- No test scenario.

**What Happens:**
1. Alice opens the desktop app. The orchestrator is unreachable.
2. Alice looks for a "Create proposal offline" option. None exists.
3. Alice considers exporting the orchestrator state before the crash, but she doesn't have a backup.
4. Alice researches SPS-65 and Bitcoin Script. She is not a cryptographer.
5. Alice manually constructs a Bitcoin transaction using a command-line tool. She makes a mistake in the script encoding.
6. The transaction is broadcast and rejected by the network (invalid script).
7. Governance is stalled for days while the backend is recovered.

**Why This Is Bad:**
- The offline fallback is promised in the PRD but not specified or tested.
- Signers cannot execute critical governance without the backend.
- The feature is deferred to Slice 5, but Slice 0 (walking skeleton) should include this as a core invariant.

**Fix:** Re-prioritize manual fallback as a Slice 1 feature (not Slice 5). Write a detailed spec covering:
- How to export proposal payloads without the backend (from cached state, or from on-chain ASM state).
- How to manually aggregate signatures (paste signatures into a form, construct the SPS-65 action).
- How to construct and broadcast a valid Bitcoin transaction (simple step-by-step wizard).
- Test scenario: "Backend is down. Three signers are in a secure room with Trezor devices. They manually construct and broadcast a valid approval transaction using only the desktop app's offline wizard. Proposal is broadcast successfully."

---

## Evidence Index (Paths)

### PRDs & Requirements
- `docs/0-prd/01-multisig-ui.md` — UI PRD defining signer flows, hardware wallet support, proposal lifecycle.
- `docs/0-prd/02-multisig-backend.md` — Backend PRD defining coordination-only invariants, authority isolation, offline fallback.
- `docs/0-prd/03-prd-update.md` — PRD update (references external Notion links).

### Stories & Journey Maps
- `docs/3-stories/story-map.md` — Functional user story map (Slices 0–5, Activities A–I).
- `docs/3-stories/non-functional-items.md` — Non-functional concerns (build, auth, session, HA).
- `docs/3-stories/README.md` — Story map README (may contain DoR definition; not fully read).

### Specs
- `docs/specs/proposal-creation-authorization.md` — Authority-scoped proposal creation.
- `docs/specs/proposal-signing-and-dashboard-status-alignment.md` — Dashboard state, signing flow, status mapping.
- `docs/specs/proposal-broadcast-commit-reveal.md` — Broadcast mechanism (not fully read).
- `docs/specs/poc5-trezor-hw-wallet-integration.md` — Hardware wallet integration, address derivation.
- `docs/specs/proposal-creation-signer-update.md` — Signer update proposal creation (not fully read).

### Discovery & Architecture
- `docs/2-discovery/16-poc5-trezor-findings.md` — Trezor firmware limitations (SPS-65 message signing).
- `docs/2-discovery/08-alpen-crate-prd-coverage.md` — Alpen crate support gaps (8 of 13 update types missing).
- `docs/architecture/overview.md` — System architecture overview.
- `docs/architecture/adrs/` — Architecture Decision Records.

### Frontend Implementation
- `desktop-app/src/screens/proposals-dashboard-screen.tsx` — Dashboard screen (state grouping, CTA wiring).
- `desktop-app/src/screens/broadcast-proposal-screen.tsx` — Broadcast screen (authority label bug?).
- `desktop-app/src/screens/sign-poc-screen.tsx` — Sign screen (not found; POC implementation unknown).
- `desktop-app/src/screens/screen-shell.tsx` — Shell layout.
- `desktop-app/src/screens/wallet-connect-screen.tsx` — Wallet connection (not fully read).

### Backend Code Structure
- `orchestrator-be/src/handlers/` — HTTP handlers (proposal creation, signing, broadcasting).
- `orchestrator-be/src/application/` — Application layer (proposal service, auth service).
- `orchestrator-be/src/infrastructure/` — Repository implementations (in-memory, db adapters).
- `orchestrator-be/src/state.rs` — Proposal state machine.
- `orchestrator-be/src/error.rs` — Error types.

---

## Smallest Fixes vs. Largest Bets (Be Explicit)

### Smallest Fixes (1–2 hours each)
1. **Fix authority label bug in `broadcast-proposal-screen.tsx`** (line 17).
   - Change `StrataAdministrator ? 'Alpen Administrator'` to `StrataAdministrator ? 'Strata Administrator'`.
   - Verify authority label is consistent across all screens.

2. **Add "What if backend is unavailable?" acceptance criteria to US-H5**.
   - Define error state UI, fallback guidance, and recovery procedure.
   - Link to backend specs.

3. **Add per-proposal `requiredSignatures` verification test**.
   - Ensure dashboard counter uses proposal snapshot, not current authority state.
   - Write test: threshold changes, pending proposal still shows old threshold.

4. **Rename dashboard sections for clarity** (Optional polish).
   - "Collecting Signatures" (instead of "Pending").
   - "Ready to Broadcast" (instead of "Quorum reached").

### Largest Bets (Weeks of work)
1. **Write `docs/specs/sps65-signing-visualization.md`** (2–3 days).
   - Define exactly what the hardware wallet screen will display for each action type.
   - Coordinate with Trezor firmware team on BIP-137 message encoding or custom PSBT field.
   - Implement corresponding UI in desktop app (before/after values, sighash display, device confirmation screen).
   - Test with real Trezor hardware against SPS-65 actions.

2. **Re-prioritize manual offline fallback to Slice 1** (1–2 weeks).
   - Write spec covering state export, signature aggregation, and transaction construction.
   - Implement wizard UI in desktop app (offline mode detection, export proposal form, paste signatures form, construct transaction form).
   - Write integration test: "Backend down, three signers construct and broadcast a proposal using only the app."
   - Validate against on-chain ASM state derivation (no backend dependency).

3. **Add comprehensive DoR checklist and gate stories before handoff to DESIGN** (1 week).
   - Define 8 mandatory DoR items (JTBD, failure modes, authority context, AC in GWT, dependencies, NFRs, test plan, edge cases).
   - Audit all 40+ stories in story-map against DoR checklist.
   - Mark stories as "BLOCKED: awaiting upstream Alpen crate support" (stories US-E3 thru US-E10, US-E11 depend on missing types).
   - Write a story-map DoR compliance matrix (story ID | JTBD | AC format | Dependencies | Test plan | Blocked? Y/N).

4. **Add signer-set-change flows (rotation / compromise)** (3–5 days).
   - Write stories US-E_ROTATE (urgent self-removal) and US-D_ROTATION (review before/after signer sets).
   - Add acceptance criteria with before/after visual diff.
   - Implement in desktop app: form to select signers to add/remove, visual diff display, confirmation.
   - Write test scenario: "Alice is compromised, proposes her own removal, Bob approves, proposal is broadcast and enacted, Alice can no longer sign on-chain."

5. **Write journey narrative and shared artifacts map** (2–3 days).
   - Capture emotional arc (apprehension → confidence → relief).
   - Document shared constraints (multiple signers, quorum, authority scope, dependency on backend).
   - Write failure path stories (abort mid-sign, backend down, threshold changes, cancellation).
   - Illustrate with sequence diagrams (Alice proposes → Bob approves → Charlie broadcasts → on-chain enactment).

6. **Implement backend unavailability detection and graceful error handling** (1 week).
   - Add retry logic and timeout configuration to orchestrator client.
   - Implement "Offline mode" detection in desktop app (backend unreachable for N seconds).
   - Surface "Backend unavailable" banner with guidance.
   - Cache proposal state locally so signer can still review and export payloads.
   - Write integration test: "Backend goes down mid-proposal, signer can still view and export payloads, backend comes back, signer can resume."

---

## What Would Change My Mind (Missing Evidence / Experiments)

### Evidence That Would Elevate These Findings
1. **Find a DoR checklist in `docs/3-stories/README.md`** or linked elsewhere.
   - If DoR exists and is enforced, reduce findings 8 & 9 from HIGH to MEDIUM.

2. **Find a signer-set-change story (US-E_ROTATE)** or hidden story covering key rotation / compromise.
   - If story exists with before/after visual diff, reduce finding 7 from HIGH to MEDIUM.

3. **Find a backend-unavailability spec** (e.g., `docs/specs/offline-fallback-flow.md`) that defines error handling, caching, and manual aggregation.
   - If spec exists and is implemented, reduce findings 1 & 5 (narratives 1 & 6) from CRITICAL to MEDIUM.

4. **Find an acceptance criteria in US-F1 or US-I4** that links the hardware wallet message to the UI payload (e.g., "Signer can verify the sighash on the device against the UI payload").
   - If AC exists and test scenario covers it, reduce finding 2 from CRITICAL to MEDIUM.

5. **Find a spec defining state conflict handling** (proposal state changes mid-sign, signed transaction replayed, etc.).
   - If spec exists with test coverage, reduce finding 3 from CRITICAL to MEDIUM.

6. **Run a manual test on real Trezor hardware** signing a SPS-65 action; verify the device screen matches the UI payload.
   - If test passes, reduce finding 2 from CRITICAL to LOW.

### Experiments to De-Risk Largest Bets
1. **POC: Offline proposal aggregation**.
   - Implement a CLI tool that exports a proposal payload (actionId, seqNo, action hex) and constructs the SPS-65 transaction locally.
   - Test: Three developers manually aggregate signatures and broadcast without the orchestrator.
   - Result: Validates that the offline fallback is technically feasible and that the format is signer-friendly.

2. **Trezor firmware dialogue with manufacturer**.
   - Confirm what message format and display the device will show when signing SPS-65 actions.
   - Determine if a custom PSBT field or BIP-137 variant can be used to show before/after values on the device.
   - Result: Unblocks the signer-visualization spec.

3. **User testing with multi-authority signers**.
   - Have 3 signers (each on 2+ authorities) use the app to create, sign, and broadcast proposals.
   - Record confusion points (e.g., "Which authority am I on?", "Can I cancel this?", "Is my signature needed?").
   - Result: Identifies UX patterns that reduce cross-authority confusion.

4. **Chaos test: backend failure during proposal lifecycle**.
   - Spin up a test harness with a running orchestrator.
   - Start a proposal (creator signs).
   - Kill the orchestrator mid-flow.
   - Try to sign, broadcast, and fall back to offline mode.
   - Result: Identifies what app state is lost and what recovery looks like.

5. **Threshold change during pending proposal**.
   - Create a proposal under 3-of-5.
   - Execute a separate on-chain action to change threshold to 2-of-5.
   - Verify proposal still shows "3 / 3 required" in dashboard.
   - Broadcast proposal with 3 signatures; verify it is accepted on-chain.
   - Result: Validates that per-proposal `requiredSignatures` snapshot is enforced.

---

## Summary & Recommendation

### Blocking Issues (Prevent Production Release)
1. **Finding 2 (Payload divergence)**: No acceptance criteria linking UI payload to hardware wallet message. Risk: Signers cannot verify what they are signing. **Action:** Write signer-visualization spec and test with real Trezor before Slice 1.
2. **Finding 1 (Backend unavailability)**: No error handling or offline fallback spec. Risk: Governance stalls on backend failure. **Action:** Write offline-fallback spec and re-prioritize to Slice 1.
3. **Finding 3 (State conflict)**: No acceptance criteria for proposal state changes during signing. Risk: Confusion about whether signature was accepted. **Action:** Add state-conflict AC to US-F1/F2/I4 and write test scenario.

### High-Risk Issues (Fix Before Slice 2)
4. **Finding 4 (Authority context)**: Authority label bug and inconsistent display. Risk: Cross-authority confusion. **Action:** Fix bug, add AC for consistent authority display across all screens.
5. **Finding 5 (Threshold tracking)**: Dashboard counter may use current threshold instead of proposal snapshot. Risk: Incorrect quorum indication. **Action:** Implement `requiredSignatures` field and verify test coverage.
6. **Finding 7 (Signer-set changes)**: No story for key rotation or compromise response. Risk: Signers unprepared for emergency. **Action:** Write US-E_ROTATE and US-D_ROTATION stories with before/after visual diff.
7. **Finding 8 (DoR missing)**: No Definition of Ready gate before handoff to DESIGN. Risk: Implementation starts on incomplete specs. **Action:** Add 8-item DoR checklist to story-map and audit all stories.

### Medium-Risk Polish (Fix Before Slice 3)
8. **Finding 6 (Pending state ambiguity)**: "Pending" conflates two meanings (collecting signatures vs. quorum reached). Risk: Signer confusion about next action. **Action:** Rename dashboard sections or add visual indicator for quorum-reached proposals.
9. **Finding 9 (Multi-authority UX)**: No story for switching between authorities without state leakage. **Action:** Add AC to US-C4 and test scenario.
10. **Finding 10 (Expiry handling)**: No auto-refresh or state sync when proposals expire. **Action:** Add AC for dashboard auto-refresh and test scenario.

### Nice-to-Have (Fix After Slice 3)
11. **Finding 11 (Payout Admin untested)**: Payout architecture is undocumented. **Action:** Conduct payout discovery before Slice 4 gates.
12. **Finding 12 (Journey narrative missing)**: No emotional arc or shared artifacts map. **Action:** Write journey narrative section to story-map.
13. **Finding 13 (Device-specific UI)**: Sign screen mentions "Trezor" by name. **Action:** Generalize to "Sign with [device name]" or auto-detect device type.

### Approval Verdict
**`REJECTED_PENDING_REVISIONS`**

The story map and specs are coherent at a high level, but **three critical safety gaps** (payload divergence, backend unavailability, state conflicts) prevent this feature from being production-ready. Additionally, **Definition of Ready is not enforced**, and several high-risk stories (signer-set changes, manual fallback) are deferred without clear justification or acceptance criteria.

**Recommendation:** Invest 2–3 weeks to write the three critical specs (signer-visualization, offline-fallback, state-conflict handling), implement corresponding UX, and add a DoR checklist before handing off to Slice 1 DESIGN. Re-prioritize manual fallback and signer-rotation to Slice 1 (not Slice 5) to ensure safety-critical flows are covered from the walking skeleton onward.

---

## Appendix: TOP-5 Findings at a Glance

| Rank | Finding | Severity | Risk | Recommendation |
|------|---------|----------|------|-----------------|
| 1 | No acceptance criteria for "signer cannot verify what they are signing on hardware wallet" | CRITICAL | Payload divergence footgun; signer may authorize unintended action | Write SPS-65 signing visualization spec; test with real Trezor |
| 2 | No error handling or spec for backend unavailability; offline fallback deferred to Slice 5 | CRITICAL | Governance stalls; promised fallback is not implemented | Write offline-fallback spec; re-prioritize to Slice 1; implement recovery UI |
| 3 | No acceptance criteria for "proposal state changes mid-signing" (concurrent modification) | CRITICAL | Confusion about whether signature was accepted; stale UI | Add state-conflict detection to sign screens; validate proposal status before and after signing |
| 4 | Authority label bug (wrong label in broadcast screen) + no consistent authority display requirement | CRITICAL | Cross-authority signer confusion; signer may sign on wrong authority | Fix label bug; add AC requiring consistent authority display across all screens |
| 5 | Definition of Ready not stated or enforced; stories may be handed off without JTBD, test plan, or edge-case coverage | HIGH | Implementation stalls on missing dependencies (Alpen crate, RPC methods); incomplete acceptance criteria | Add 8-item DoR checklist; audit all 40+ stories; mark blocked stories |

