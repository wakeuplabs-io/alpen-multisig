# Spec: Security Council — Defcon 1

**Status:** Pending — V1 functional contract for Stage 4 implementation  
**PRD:** [`06-prd-hardware-signer-and-block-payouts-update.md`](../0-prd/06-prd-hardware-signer-and-block-payouts-update.md) §3.1.4, §5.1, §5.2.2, §5.3, §5.4, §5.5  
**Stories:** [`story-map.md`](../3-stories/story-map.md) US-E12 (in scope); US-E13, US-E14 (constraints)  
**Master plan:** [`security-council.md`](./security-council.md)

---

## Objective

Define the full stack for Security Council signers to create and enact Defcon 1 proposals: immediate (depth-0) safe-harbour activation with no cancellation path. The end-to-end flow: authenticate as council signer → create Defcon 1 proposal → sign → reach quorum → broadcast via commit/reveal → Enacted (safe harbour activated in the reveal block).

This spec covers orchestrator-be (authority→role mapping, per-action lock period, enactment detection), desktop-app (create form, type-to-confirm gate, lifecycle display), and signer safety (four-line message, destructive UX, no "Approved" label, no cancel CTA).

## Scope

### Included

- Defcon 1 proposal creation, signature collection, and quorum detection.
- Orchestrator per-action lock-period model (V1 lays the spine for V2/V5).
- Enactment detection via `safe_harbour.is_activated()` post-condition.
- Frontend create form with the four canonical signing-message lines rendered verbatim.
- Type-to-confirm gate (`DEFCON 1`) before hardware wallet signing.
- Lifecycle UI state: Pending → Approved → Enacted (never Canceled).
- Broadcast via existing commit/reveal pipeline.
- Signer-safety treatment: destructive visual, authority context on all steps, non-council session blocked.

### Not included

- Defcon 3 (timelocked, V2 in scope).
- Defcon 3 cancellation (V5 in scope).
- Security Council membership update (V3 in scope, Strata Admin authority).
- Safe harbour address update (V4 in scope, Strata Admin authority).
- Bridge fund handling (protocol concern).
- Protocol validity rules (orchestrator stays coordination-only).

## Requirements Alignment

- **PRD §3.1.4**: Strata Security Council multisig MUST be usable exclusively by all Strata Security Council Signers.
- **PRD §5.2.2**: Defcon 1 transaction does not produce proposals with "Approved" or "Canceled" state — the carve-out applies to this transaction type, not to Defcon 3.
- **PRD §5.3**: Pending updates, approval signatures, 7-day expiry, quorum-reacher's Send button.
- **PRD §5.5**: Security Council multisig: Defcon 1 transaction as a supported update type.
- **Story map**: US-E12 (create Defcon 1), shared acceptance signals (stable ActionId, creator signature, duplicate rejection, multi-proposal same SeqNo).

## Protocol Recap

See [`security-council.md`](./security-council.md) §2–3 for full upstream description. Minimum recap for this contract:

- Defcon 1 is a unit-struct action with no payload (`Defcon1Update` in the upstream admin subprotocol).
- Signing message is exactly four lines: `Strata ASM Administration v1`, `Action: Defcon 1`, `Authorized By: Strata Security Council`, `Sequence: <seq_no>`. No `Action Details:` block because there is no payload (see [`security-council.md`](./security-council.md) §3.1).
- Confirmation depth is hardcoded `0` upstream — Defcon 1 is the emergency lever and applies immediately.
- Observable post-condition: `bridge.safe_harbour().is_activated() == true` in the **submission block itself**; the action **never enters** the admin queue (see [`security-council.md`](./security-council.md) §3.2).
- `e2e_defcon_probe.rs` proves the signing message and enactment post-conditions against real regtest ASM.

## Constraints from later slices

V1 must satisfy three requirements that are non-negotiable and forward-compatible with V2 (Defcon 3) and V5 (Defcon 3 cancel):

### 1. Lock period is per-action, never per-authority

**Rule:** The lock-period value used during proposal enactment detection must be resolved by action type at enactment-detection time, never from a cached or hardcoded per-authority mapping. For Defcon 1 the resolution is hardcoded: depth = 0 (upstream has no config field for Defcon 1). For Defcon 3 the resolution is from `confirmation_depths.defcon3` in the live ASM state.

**Why:** Defcon 1 and Defcon 3 are both authorized by the same authority (Strata Security Council) but have fundamentally different depths: Defcon 1 is hardcoded at 0 with no per-deployment configuration; Defcon 3 is configurable per deployment. A `lock_period_for_authority` function would collapse both into a single value, making it impossible to distinguish them at enactment time.

**Implementation note:** `orchestrator-be/src/infrastructure/asm_role_membership.rs:108` currently has a per-authority mapping; `orchestrator-be/src/application/proposals.rs:623` consumes it. V1 must refactor to a per-action resolution: check the action type, return hardcoded 0 for Defcon 1, query `confirmation_depths.defcon3` for Defcon 3. This is done at enactment-detection time, never cached at startup.

### 2. Cancelability is decided per action and per live depth, never by Authority::SecurityCouncil

**Rule:** A proposal is cancellable if and only if its action-specific confirmation depth is **not zero**. The gate must never be an authority-shaped condition like `"cancel is only supported for AlpenAdmin and StrataAdmin"`.

**Why:** Defcon 1 (depth 0) can never be cancelled because it is never enqueued — a cancel targeting it fails on-chain with `UnknownAction`. Defcon 3 (configurable depth ≥ 0) IS cancellable when depth ≠ 0. V5 will deliver the Defcon 3 cancel flow. A hardcoded authority check would cement the wrong decision for Defcon 3 and force V5 to fight it.

**Implementation note:** `orchestrator-be/src/application/proposals.rs` currently gates cancel on an authority allow-list, rejecting anything outside `AlpenAdmin | StrataAdmin`. That question cannot answer this feature: Defcon 1 and Defcon 3 share one authority and have opposite answers, so no allow-list of authorities separates them.

**V1 replaces the gate rather than deferring it.** The condition becomes the action's confirmation depth — reject when the depth is zero, because a zero-depth action is never enqueued and an on-chain cancel would fail with `UnknownAction`. This is what makes [AC 11](#11-cancelability-gate-is-per-depth-not-per-authority) satisfiable: the rejection has to name the depth, which an authority-shaped check cannot do.

Deferring the change to V5 would defeat this constraint's own purpose. V1 introduces `Authority::SecurityCouncil` into a system whose gate rejects it wholesale; V5 would then have to open that gate *and* prove that opening it does not expose a cancel affordance on Defcon 1 — the very fight this rule exists to prevent. Expressed as depth, V5 touches no gate at all: Defcon 3 carries a non-zero depth and passes on its own.

The replacement does not depend on Defcon and is verifiable before it exists: the current authorities' actions already carry configurable depths, so the rewrite can land and be tested against the existing suite.

### 3. Defcon 1 never displays "Approved" and offers no cancel CTA anywhere

**Rule:** The Defcon 1 proposal lifecycle displays four states in the UI: Pending, Quorum reached (not "Approved"), Enacted, Expired. The label used when quorum is reached must be something like "Quorum reached — ready to broadcast", never the word "Approved". The cancel button/affordance must not appear on Defcon 1 proposals anywhere — not in the proposal detail screen, not in a menu, not conditional on permission.

**Why:** PRD 06 §5.2.2 explicitly carves out Defcon 1 from the Approved/Canceled lifecycle. Because Defcon 1 has depth 0, it is never enqueued and a cancel would fail on-chain with `UnknownAction`. Showing a cancel CTA would offer a user interaction that cannot work, and would obscure the fact that Defcon 1 is an irreversible one-way door.

**Implementation note:** The state machine must account for this: Defcon 1 proposals reach `approved` status in the backend but display as "Quorum reached" in the frontend. An invariant test should verify that a simulated cancel targeting a Defcon 1 fails.

> Extended in Phase 6. The desktop held **three** copies of the authority allow-list this rule
> forbids — the dashboard card, the detail screen's *Cancel this proposal* button, and the cancel
> route's redirect guard — so AC 10 held only because the council was absent from all three. They are
> now one `canCancelProposal` carrying the action term, and the test is written against the V5 future:
> the Defcon 1 it refuses carries an authority that *is* in the list. See
> [`security-council-defcon-phase-6.md`](./security-council-defcon-phase-6.md) §4.3.

## State Model

Defcon 1 proposals follow the standard lifecycle, with one label carve-out in the UI:

```
Pending → Approved (labeled "Quorum reached" in UI) → Enacted
   ↓                                                      ↓
   └──────────────── Past (expired or enacted) ─────────→
```

Backend state names: `Pending`, `Approved`, `Enacted`, `Expired` (unchanged from other proposal types).  
Frontend display labels for Defcon 1:
- `Pending` → "Pending"
- `Approved` → "Quorum reached"  
- `Enacted` → "Enacted"
- `Expired` → "Expired"

**Past proposals:** Enacted or expired Defcon 1 proposals appear in the "Past" list on the dashboard (per PRD 06 §5.4). A signer can review the historical record.

Defcon 1 proposals **never** display "Approved" or "Canceled" labels and **never** display a cancel CTA.

## Backend Contract (orchestrator-be)

### Authority and Role Mapping

Map `Role::StrataSecurityCouncil` (upstream ASM role, SPS-50 byte 40–49) to `Authority::SecurityCouncil` (application enum). Only council members pass the access control gate when creating or signing Defcon 1 proposals. Non-council sessions are denied at the `POST /proposals` handler before any proposal object is created.

### Per-Action Lock Period

Enactment detection must read the action-specific lock period from live ASM state, **never** from a cached or hardcoded mapping:

```rust
pub(crate) async fn lock_period_for_action(
    rpc_url: &str,
    action_hex: &str,
) -> Result<u64, AppError>
```

The shape follows the existing convention of the module it joins — `threshold_for_authority`,
`lock_period_for_authority` and `update_id_in_queue_for_action` all take the ASM RPC URL as `&str`.
The action is resolved from the stored `action_hex` via `decode_multisig_action_hex`.

For `UpdateAction::Defcon1(_)`, this returns a hardcoded `0` (no configurable field upstream).  
For `UpdateAction::Defcon3(_)`, this reads `confirmation_depths.defcon3` from the live ASM state (deployment-specific).

V1 only calls this for Defcon 1, but the function signature must accept any action so V2/V5 can reuse it.

### Proposal Creation

Create a new proposal with type `defcon_1`:

```rust
pub async fn create_defcon_proposal(
    repo: &dyn ProposalRepository,
    seq_no: u64,
    signer_pubkey: String,
    signature_hex: String,
) -> Result<Proposal>
```

Logic:
1. Construct `action_hex` from `MultisigAction::Update(UpdateAction::Defcon1(Defcon1Update))` and the provided `seq_no`.
2. Compute `ActionId = hash(action_hex, seq_no)` (stable across resubmissions).
3. Check for existing proposal with same `(action_hex, seq_no)`. If found, reject naming its `ActionId`, mutating nothing (PRD 02 §3.4).
4. Persist new `Proposal` with `authority = SecurityCouncil`, `status = Pending`, and the creator's signature.

### Enactment Detection

When the backend receives a new ASM block (e.g., via RPC poll in `reconcile_enacted_for_authority`):

1. Fetch the live ASM state for `Role::StrataSecurityCouncil`.
2. For each `approved` proposal of type `defcon_1`:
   - Read `bridge.safe_harbour().is_activated()` and check the admin queue.
   - If `safe_harbour.is_activated() == true` in the **same block as or after the reveal was confirmed**, AND no Defcon 1 entry exists in `admin.queued()`, mark the proposal `enacted`. The queue bypass is essential because Defcon 1 (depth 0) never enters the queue — if an entry is present, it is a different action.
3. The orchestrator remains coordination-only: it does not validate the protocol's decision to activate; it only reads the post-conditions.

### API Endpoints

**New: `POST /proposals`**

Extend existing endpoint to accept `type: "defcon_1"` as a proposal kind:

```json
{
  "type": "defcon_1",
  "seqNo": 1,
  "signerPubkey": "02...",
  "signatureHex": "..."
}
```

Returns `201 Created` with the proposal JSON. A duplicate `(action, seq_no)` is rejected with `409 Conflict` naming the existing `ActionId` (PRD 02 §3.4).

**Existing: `GET /proposals`, `GET /proposals/:action_id`**

Include Defcon 1 proposals in lists and detail views. Display `authority: "security_council"` and `type: "defcon_1"` in responses.

## Frontend Contract (desktop-app)

### Authorization Gate

Only a Security Council session can reach the Defcon 1 create form. A non-council session (Alpen Admin, Strata Admin, Sequencer Manager, Payout Admin) sees no "Create Defcon 1" button anywhere — not in the form list, not in a modal, nowhere.

### Create Form Layout

**Route:** `/proposals/create` — the existing creation route, whose action-type menu offers Defcon 1
to a Security Council session and to no other authority.

> Corrected in Phase 5. This line previously specified a dedicated `/proposals/create/defcon-1`
> route. Stage 5 was given that decision by *Critical Files* below, and settled it the other way:
> the creation flow is a two-step machine with a frozen preview, a re-authentication modal, a
> navigation guard and a sighash pre-flight, and a sibling screen would have reimplemented all of
> it to change a colour scheme and add one input. What replaces the route guard is the
> authority-keyed menu plus the backend gate [AC 17](#17-the-backend-refuses-defcon-1-creation-from-a-non-council-session)
> pins. See [`security-council-defcon-phase-5.md`](./security-council-defcon-phase-5.md) §4.

**Form structure:**

```
← Back to dashboard

Create Defcon 1 proposal                  [Security Council badge] [Session] [Disconnect]
──────────────────────────────────────────────────────────────────────────────────────
  ┌─ Signing message (read-only, monospace) ─────────────────────────────────────────┐
  │  Strata ASM Administration v1                                                    │
  │  Action: Defcon 1                                                                │
  │  Authorized By: Strata Security Council                                          │
  │  Sequence: [input field for seq no]                                              │
  │                                                                                  │
  │  NOTE: This is exactly what you will see on your hardware signer screen.         │
  └──────────────────────────────────────────────────────────────────────────────────┘

  ┌─ Confirmation gate ──────────────────────────────────────────────────────────────┐
  │  Type to confirm:  [ input field: "DEFCON 1" ] (case-insensitive)               │
  │                                                                                  │
  │  ⚠  WARNING: Defcon 1 activates safe harbour immediately.                       │
  │      This action cannot be cancelled.                                            │
  └──────────────────────────────────────────────────────────────────────────────────┘

  [Sign with <signer>]  ← disabled until seq_no is set and type-to-confirm matches
```

**Validation rules:**

1. `seq_no` must be a non-negative integer.
2. Type-to-confirm field must match `"DEFCON 1"` exactly (case-insensitive matching: `input.toUpperCase() === "DEFCON 1"`).
3. Sign button is disabled until both conditions pass.
4. Duplicate `(action, seq_no)` submissions are rejected naming the existing `ActionId` (see
   "Proposal Creation" in Backend Contract, and the correction under [AC 3](#3-actionid-is-stable-and-duplicate-rejection-works)).

**Signing message rendering:** The four lines are rendered verbatim from the signing-message bytes, monospace, in a read-only text area. No abbreviations, no line wrapping beyond the natural message boundary.

### Lifecycle Display

**Pending state:**

```
Status: Pending                          Signatures: 1 / 3
⏱ Expires in 6 days 23 hours

[Your signature] ✓

[Sign with <signer>]  ← re-enable if signer hasn't yet signed
[Copy signatures]     ← copy all collected signatures to clipboard
```

**Quorum reached (not "Approved"):**

```
Status: Quorum reached                            Signatures: 3 / 3
Quorum reached — ready to send

[Send]                  ← enable commit/reveal broadcast (UX similar to wallet send screen)
[Copy signatures]       ← copy all collected signatures to clipboard
```

**Enacted:**

```
Status: Enacted                          Block: 850,123
Safe harbour activated: ✓
```

**No Cancel CTA anywhere** — not on this screen, not in a detail view, not in a status column. If the user asks "How do I undo this?", the answer is "You cannot — Defcon 1 is irreversible."

### Broadcast Flow

Reuse the existing commit/reveal broadcast flow ([`proposal-broadcast-commit-reveal.md`](./proposal-broadcast-commit-reveal.md)):

1. Fetch proposal + signatures from orchestrator.
2. Prepare commit address and fee estimate (local).
3. On user confirmation, build and sign commit + reveal, then broadcast via `submitpackage` or sequential `sendrawtransaction`.
4. Poll orchestrator for status until `reveal_confirmed`.
5. Poll ASM state until `bridge.safe_harbour().is_activated()` is detected.

### Manual Fallback Path

Per PRD 06 §5.3.2.2, when the coordination backend is unavailable or the app is offline, a signer must be able to:

1. **Export collected signatures:** Copy all approval signatures collected on the proposal to the clipboard.
2. **Compose the transaction:** Reuse the existing manual route `/manual` (`desktop-app/src/screens/manual-proposal-screen.tsx`) and its `proposals_broadcast_manual` IPC command, which already compose commit and reveal from locally aggregated signatures. Defcon 1 introduces no new manual mechanism — it only has to be reachable from this path. See [`manual-execution-flow.md`](./manual-execution-flow.md) for that flow and its known gaps.
3. **Broadcast externally:** Copy the assembled raw transaction hex to clipboard and broadcast via any Bitcoin RPC (e.g., `sendrawtransaction` in Bitcoin Core or a third-party service).

This capability ensures governance can proceed even if the orchestrator is unreachable. See [`signer-safety-model.md`](./signer-safety-model.md) for the trust model — broadcast truth is persisted in the orchestrator when it is available, but should never block a legitimate manual flow.

## Signer Safety

Defcon 1 authorizes sweeping **all bridge funds** with no de-escalation path (`safe_harbour.is_activated()` is never reset to `false`). This drives four critical safeguards:

### 1. Four-line signing message is the reviewable artifact

The hardware signer displays exactly the four canonical lines — no payload, no details block. The form displays these same four lines in monospace in the UI. A signer comparing the two must see byte-identical text. If the hardware signer shows anything different, the update is either not for Defcon 1 or has been tampered with.

**Implementation:** Do not abbreviate or reformat the message. Render it verbatim from the signing-message bytes.

### 2. Type-to-confirm gate (`DEFCON 1`) before hardware wallet signing

Typing out the action name is a deliberate friction that forces the signer to read the form and the warning. The sign CTA remains disabled until the typed text matches exactly.

**Implementation:** Case-insensitive matching is acceptable (e.g., `"defcon 1"` or `"Defcon 1"` both match), but the input field shows what the signer typed so they can verify they got it right.

### 3. Distinct destructive visual treatment

The Defcon 1 form must be unmistakably different from every other proposal creation form — in color scheme, icon treatment, spacing, typography, or all of the above. "Destructive" visual patterns (red accent, skull icon, severe warning box) are appropriate here because Defcon 1 is genuinely irreversible.

**Decision:** Exact color/icon choices are not specified by this contract — they are a design choice. But the form MUST differ visually from, say, the Strata Admin signer update form.

### 4. Authority context on every step

The "Security Council" badge must be visible at the top of the create form, the signing message, the approval screen, and the broadcast confirmation. A signer must never wonder which authority they are acting for.

**Implementation:** Include the badge in the header/title area on all screens in the Defcon 1 flow.

### Non-council sessions blocked

A session authenticated for Alpen Admin, Strata Admin, Sequencer Manager, or Payout Admin cannot reach the Defcon 1 create route at all. The frontend route guard checks `session.authority === "security_council"` before rendering the form.

## Acceptance Criteria

### 1. Non-council session sees no Defcon 1 entry point
**Given** a user authenticated as Alpen Admin, Strata Admin, Sequencer Manager or Payout Admin  
**When** they open the proposals dashboard  
**Then** no "Create Defcon 1" CTA is rendered anywhere on the page.

### 1a. Direct navigation by a non-council session is refused
**Given** a user authenticated as any non-council authority  
**When** they navigate directly to `/proposals/create/defcon-1`  
**Then** the Defcon 1 form is not rendered and the router redirects to `/` (the wallet-connect screen), matching the existing catch-all behaviour in `desktop-app/src/App.tsx`.

### 2. Defcon 1 proposal creation
**Given** a Security Council signer on the create form with seq_no = 1 (first council proposal)  
**When** they set seq_no in the signing message and type "DEFCON 1" in the confirm field  
**Then** the Sign button becomes enabled; clicking it opens the hardware wallet signing flow.

### 3. ActionId is stable and duplicate rejection works
**Given** two signers independently create proposals with identical `(action: Defcon1, seq_no: 1)`  
**When** the first signer's proposal is persisted  
**Then** the second signer's POST is **rejected** and names the existing `ActionId`, so the signer can
approve that proposal instead; backend state is not mutated by the duplicate attempt — including the
signature the duplicate arrived with.

> Corrected in Phase 3. This criterion previously read "returns the existing proposal (idempotent)",
> which contradicts [PRD 02](../0-prd/02-multisig-backend.md) §3.4.1 — "the backend MUST reject
> duplicate creation" — its own title, and the story map's "duplicate rejection" signal. The PRD is
> the client's SSOT and wins. Both readings agree on §3.4.2, that the existing proposal must not be
> mutated; naming the `ActionId` in the rejection is what preserves the intent the old wording was
> reaching for.

### 4. Signing message rendered verbatim
**Given** a Defcon 1 proposal on the create form  
**When** the form displays the signing message  
**Then** the text is exactly:
```
Strata ASM Administration v1
Action: Defcon 1
Authorized By: Strata Security Council
Sequence: <seq_no>
```
with no `Action Details:` block, no wrapping, no abbreviation.

### 5. Type-to-confirm gate enforced
**Given** the create form with seq_no set  
**When** the confirm field is empty or contains any text other than "DEFCON 1" (case-insensitive)  
**Then** the Sign button remains disabled; no signing can proceed.

### 6. Quorum detection and broadcast enable
**Given** a Defcon 1 proposal with 2 signatures of 3 required  
**When** the third signature is collected and confirmed on-chain  
**Then** the proposal status updates to "Quorum reached"; the send button becomes enabled.

### 7. Broadcast construction and transmission
**Given** a Defcon 1 proposal at quorum  
**When** the signer clicks the broadcast button  
**Then** the app builds the commit and reveal transactions, signs both locally, broadcasts them through the existing commit/reveal pipeline (via `submitpackage` or sequential `sendrawtransaction`), and the proposal's broadcast status advances through the orchestrator.

### 8. Enactment detected via safe harbour activation and queue bypass
**Given** a Defcon 1 proposal whose commit/reveal has been broadcast and confirmed  
**When** the orchestrator polls ASM state  
**Then** it detects both: (1) `bridge.safe_harbour().is_activated() == true` in the reveal block, AND (2) no Defcon 1 entry in the admin queue (the action never entered the queue because depth 0 means immediate execution); marks the proposal `enacted`.

### 9. No "Approved" label for Defcon 1
**Given** a Defcon 1 proposal at quorum  
**When** displaying its status in the UI  
**Then** the label shown is "Quorum reached", never "Approved".

> Corrected in Phase 6. This criterion, the State Model's label list and the *Lifecycle Display*
> wireframe all named the string "Quorum reached — ready to broadcast"; all three now name the
> shipped badge. Three reasons, and [Constraint 3](#3-defcon-1-never-displays-approved-and-offers-no-cancel-cta-anywhere)
> — "something like ..., never the word 'Approved'" — is the latitude they are taken under. The app's
> verb has been *Send* since #432, so a status naming *broadcast* would name a control that is not on
> screen. A 34-character badge is not a badge: it reflows the card header for one action type, which
> is a worse signal than the word it replaces. And "Quorum reached" is already the app's name for this
> moment, on the dashboard group heading and the post-signature modal. The full sentence is not lost —
> both screens render *Quorum reached — ready to send* beside the badge on a proposal that can be
> sent. **The non-negotiable half is unchanged: the word "Approved" never appears for a Defcon 1, in
> any state.** See [`security-council-defcon-phase-6.md`](./security-council-defcon-phase-6.md) §4.1.

### 10. No cancel CTA anywhere
**Given** any Defcon 1 proposal in any state (Pending, Quorum reached, Enacted)  
**When** viewing the proposal screen or dashboard list  
**Then** there is no "Cancel" button, no "Cancellation signatures" section, no cancel affordance anywhere.

### 11. Cancelability gate is per-depth, not per-authority
**Given** a Defcon 1 proposal  
**When** a cancel is attempted (either programmatically or via API)  
**Then** the request is rejected with a reason that references the confirmation depth of the action (depth 0 means never enqueued), NOT an authority-based rejection like "cancel is only supported for AlpenAdmin and StrataAdmin".

### 12. The lock period is resolved per action, not per authority
**Given** two proposals on the same Security Council authority — one Defcon 1 and one Defcon 3 — in a harness whose `confirmation_depths.defcon3` is non-zero  
**When** the backend resolves each proposal's lock period during enactment detection  
**Then** the Defcon 1 proposal resolves to `0` and the Defcon 3 proposal resolves to the deployment's configured `confirmation_depths.defcon3`; two proposals sharing an authority therefore resolve to different lock periods.

### 12a. The lock period is read live, not cached
**Given** a running backend that has already completed at least one enactment-detection cycle  
**When** the ASM's `confirmation_depths` change and a further detection cycle runs without restarting the backend  
**Then** the newly resolved lock period reflects the changed ASM state, proving the value is read at detection time rather than captured at startup.

### 13. Seven-day expiry applies normally
**Given** a Defcon 1 proposal created at timestamp T  
**When** 7 calendar days have elapsed (604,800 seconds)  
**Then** the proposal shows status "Expired" in the UI and is no longer available for signing.

### 14. Authority context visible throughout
**Given** any screen in the Defcon 1 flow (create, sign, broadcast)  
**When** the signer views the screen  
**Then** the header area renders an authority badge whose text reads "Security Council", and no other authority's badge is rendered on that screen.

### 15. Manual fallback: the collected signatures can be exported
**Given** a Defcon 1 proposal that has collected one or more approval signatures  
**When** the signer invokes the copy-signatures action on the proposal screen  
**Then** the clipboard contains every approval signature collected so far for that proposal, in the format the manual path consumes.

### 15a. Manual fallback: the exported bundle broadcasts through the existing manual route
**Given** an exported Defcon 1 action payload and its quorum of approval signatures  
**When** the signer pastes them into the existing manual route `/manual` and confirms the broadcast  
**Then** the commit and reveal transactions are composed and broadcast through the existing `proposals_broadcast_manual` IPC command, without the orchestrator being reachable.

### 15b. Manual fallback: the raw transaction can be broadcast elsewhere
**Given** a composed Defcon 1 commit/reveal pair on the manual route  
**When** the signer chooses to copy the raw transaction instead of broadcasting from the app  
**Then** the clipboard contains the raw transaction hex, which is accepted by any external Bitcoin RPC, satisfying PRD 06 §5.3.2.2.

### 16. Past proposals are listed
**Given** a Defcon 1 proposal that has reached `Enacted` or `Expired`  
**When** the signer opens the proposals dashboard  
**Then** the proposal appears in the "Past" list, distinct from the Pending and quorum-reached listings, as required by PRD 06 §5.4.

### 17. The backend refuses Defcon 1 creation from a non-council session
**Given** a session authenticated for any authority other than the Security Council  
**When** it sends a Defcon 1 creation request to `POST /proposals`  
**Then** the request is refused before any proposal is persisted, and no proposal exists for that `(action, seq_no)` afterwards.

> AC 1 and AC 1a cover what the UI renders and where it routes. This criterion covers the server-side
> half of PRD 06 §3.1.4 — the "usable exclusively by" requirement holds against a caller that never
> touches the UI.

## Edge Cases

| Scenario | Behavior |
|---|---|
| User navigates to `/proposals/create/defcon-1` with a non-council session | Redirect to `/` (wallet-connect screen). No Defcon 1 form rendered. |
| Two signers submit concurrent Defcon 1 proposals with same `seq_no` | Second POST is rejected naming the existing `ActionId`; backend state unchanged. The second signer approves that proposal. |
| User clicks Sign but the hardware wallet refuses the signature | Error shown; form remains; user can retry or change seq_no and try again. |
| User closes browser before broadcast completes | Proposal remains in "Quorum reached" state on the backend; user can reconnect and retry broadcast anytime. |
| seq_no is not a valid integer (e.g., `"1.5"` or `"abc"`) | Validation error shown; Sign button disabled. |
| Type-to-confirm field has extra spaces or case mismatch (`"defcon1"` or `"DEFCON 1 "`) | Sign button disabled; message: `"Type must match 'DEFCON 1' exactly (case-insensitive)."` |
| ASM state is unavailable when reconciling enacted proposals | Orchestrator retries on next poll cycle; proposal stays in `approved` status until post-condition is confirmed. |
| Safe harbour is already activated before Defcon 1 is broadcast | The activation is idempotent (`set_activated(true)`). Defcon 1 proposal still reaches `enacted` status correctly. |
| User attempts to copy "all cancel signatures" for Defcon 1 | No such button exists; UI only shows "Copy approval signatures" and "Broadcast" actions. |

## Critical Files

| File | Change |
|---|---|
| `orchestrator-be/src/domain/proposal.rs` | Add `kind: ProposalKind` enum variant `Defcon1`; map to upstream action type. |
| `orchestrator-be/src/infrastructure/asm_role_membership.rs` | Add `lock_period_for_action`, resolving the depth from the action rather than the authority; retire `lock_period_for_authority`. |
| `orchestrator-be/src/application/proposals.rs` | Implement `create_defcon_proposal`; update `reconcile_enacted_for_authority` to detect safe-harbour activation; refactor enactment detection to use per-action depth; **replace the cancel gate's authority allow-list with the action's confirmation depth** (see [Constraint 2](#2-cancelability-is-decided-per-action-and-per-live-depth-never-by-authoritysecuritycouncil)). |
| `orchestrator-be/src/handlers/proposals.rs` | Extend `create_proposal_handler` to route `type: "defcon_1"` to `create_defcon_proposal`. |
| `orchestrator-be/src/handlers/mod.rs` | Ensure Security Council role mapping is wired; route guards check `authority == SecurityCouncil`. |
| `desktop-app/src/types/proposal.ts` | Add `kind: "defcon_1"` union variant to `ProposalKind`. |
| `desktop-app/src/domain/create-proposal/` | **Settled in Phase 5:** Defcon 1 extends this domain. One `ACTION_TYPES_BY_AUTHORITY` entry, one validator, one `defcon-1-form-fields.tsx` carrying the warning, the rendered signing message and the type-to-confirm gate. No sibling domain. |
| `desktop-app/src/types/auth-role.ts`, `lib/authority-label.ts`, `api/orchestrator-auth.ts`, `screens/wallet-connect-screen.tsx`, `src-tauri/src/domain/auth.rs` | **Added in Phase 5, and not anticipated by this contract:** the desktop app had no Security Council session at all. Two of these were `default:` arms that substitute silently rather than fail. |
| `desktop-app/src/screens/proposals-dashboard-screen.tsx` | Display Defcon 1 proposals with "Security Council" label; no cancel affordance shown. |
| ~~`desktop-app/src/screens/defcon-proposal-create-screen.tsx`, route in `App.tsx`~~ | **Not built.** See the correction under *Create Form Layout*. The route is never registered, so `App.tsx`'s catch-all answers direct navigation for every session — which is what [AC 1a](#1a-direct-navigation-by-a-non-council-session-is-refused) describes. |

## Test Plan

### Backend Unit Tests

Run `cargo test -p orchestrator-be` (see AGENTS.md for CI checklist).

- **Duplicate creation is rejected:** a second call with the same `(action, seq_no)` is refused, the refusal names the existing `ActionId`, and the stored proposal — signatures included — is unchanged.
- **Defcon 1 stability:** Action type round-trips through codec; signing message matches upstream test vector.
- **Per-action resolution (AC 12):** on a single Security Council authority, a Defcon 1 proposal resolves to `0` while a Defcon 3
  proposal resolves to the deployment's configured `confirmation_depths.defcon3` — two proposals sharing an authority resolve to
  different lock periods.
- **Live read, not cached (AC 12a):** change the ASM's `confirmation_depths` between two enactment-detection cycles without
  restarting the backend; the resolved lock period must reflect the changed state.
- **Enactment detection:** When `safe_harbour.is_activated() == true` is detected in ASM state AND no Defcon 1 entry exists in admin queue, proposal transitions to `Enacted`.
- **Cancelability gate:** Attempt to cancel a Defcon 1 proposal is rejected with a reason referencing the action's depth (0), not the authority.
- **Council-only creation (AC 17):** a `POST /proposals` Defcon 1 request carrying a non-council session is refused, and the repository holds no proposal for that `(action, seq_no)` afterwards.
- **Expiry:** Defcon 1 proposal expires after 7 days (wall-clock) without reaching quorum.

### Backend Integration Tests

Backend integration tests are part of the standard test suite. Verify:

- Full flow: create Defcon 1 → collect two signatures → broadcast (commit/reveal) → safe harbour activates → enactment reconciliation runs → proposal marked `enacted`.
- Manual fallback: signatures exported and payload composed offline; raw transaction buildable without orchestrator.

### Frontend Component and E2E Tests

Desktop app tests use granular scripts (see `desktop-app/package.json` for available commands). Verify:

- **Type-to-confirm validation:** Sign button disabled until input matches "DEFCON 1" (case-insensitive).
- **Four-line message rendering:** Signing message displays verbatim without wrapping or abbreviation.
- **Non-council access blocked:** (AC 1) a non-council session on the proposals dashboard sees no "Create Defcon 1" CTA;
  (AC 1a) direct navigation to `/proposals/create/defcon-1` renders no form and redirects to `/`.
- **Status labels:** "Quorum reached" label shown (not "Approved"); "Enacted" label shown post-enactment.
- **No cancel CTA:** Defcon 1 proposal screens do not render cancel button or cancellation-signature UI.
- **Manual broadcast:** Signatures can be exported to clipboard; raw transaction hex can be composed and broadcast externally.
- **Past list:** Enacted or expired Defcon 1 proposals appear in the Past proposals list on the dashboard.
- **Authority badge (AC 14):** every screen in the Defcon 1 flow — create, sign, broadcast — renders a badge reading
  "Security Council", and no other authority's badge is rendered on those screens.

## Verification

**Code review checklist:**

- [ ] Per-action lock period is read from live ASM state at enactment time, never a cached per-authority value.
- [ ] Cancelability gate is per-action/per-depth, not an authority check (`Authority::SecurityCouncil`).
- [ ] Frontend never displays "Approved" label for Defcon 1; the badge reads "Quorum reached" (corrected under AC 9).
- [ ] No cancel button, cancel CTA, or cancellation-signature UI appears for Defcon 1 proposals.
- [ ] Type-to-confirm field exists and enforces exact match (case-insensitive) before signing.
- [ ] Signing message is rendered verbatim, monospace, without line wrapping or abbreviation.
- [ ] Security Council badge is visible on all screens in the Defcon 1 flow.
- [ ] Non-council sessions cannot reach `/proposals/create/defcon-1`.
- [ ] ActionId computation is stable across resubmissions (hash of action + seqno).
- [ ] Duplicate `(action, seqno)` submissions are rejected and name the existing `ActionId`, mutating nothing.

**Post-merge validation:**

- The full pre-commit CI checklist in [`AGENTS.md`](../../AGENTS.md) passes locally: `cargo fmt --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and from `desktop-app/`
  `npm run format:check`, `npm run lint`, `npm run build`.
- No CI regressions.
- Security Council Defcon 1 flow works end-to-end on a regtest ASM.
