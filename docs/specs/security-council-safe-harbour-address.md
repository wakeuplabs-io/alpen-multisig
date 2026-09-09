# Spec: Security Council — Safe Harbour Address Update

**Status:** Contract written; implementation pending. This document is the functional contract; the
build plan is
[`security-council-safe-harbour-address-implementation.md`](./security-council-safe-harbour-address-implementation.md),
whose phase board says what has landed.

**PRD:** [`06-prd-hardware-signer-and-block-payouts-update.md`](../0-prd/06-prd-hardware-signer-and-block-payouts-update.md) §5.1, §5.2.2, §5.5

**Stories:** US-E5 (Strata Administrator: Safe Harbor address update) — actor **Strata Admin
Signer**, authority **Strata Admin**.

**Master plan:** [`security-council.md`](./security-council.md) §7 Slice board, where this slice is V4.

**Predecessors:** [`security-council-defcon.md`](./security-council-defcon.md) (V1),
[`security-council-defcon-3.md`](./security-council-defcon-3.md) (V2) and
[`security-council-signer-update.md`](./security-council-signer-update.md) (V3). Everything they
froze still holds. This is the second slice authorized by the **Strata Administrator**, and the
first whose post-conditions live in the **bridge** subprotocol rather than the administration one.

---

## Objective

Let a Strata Administrator signer set the bridge's safe harbour destination: create → sign → quorum
→ broadcast → queued for `confirmation_depths.safe_harbour_address_update` blocks → Enacted, with
the standard cancel window in the middle.

This closes [§2.1's segregation invariant](./security-council.md#21-the-segregation-invariant). The
council controls *when* the sweep fires; the administrator controls *who sits on the council* (V3)
and **where the funds land** (this slice). Upstream states the reason in the code itself
(`asm/crates/params/src/subprotocols/admin/updates.rs:51-73`):

> The safe harbour destination is rotated by the administrator, not the security council: the
> council can sweep funds to the safe harbour (via Defcon signals) but must not also pick where
> they land, otherwise the same authority could both trigger a sweep and steal the proceeds.

Two things are new, and both are new *kinds* of thing rather than new instances:

1. **The payload is an address**, not a signer set or a verification key — so the application owns a
   conversion from what an operator holds (a bech32m address) to what the device signs (a BOSD
   descriptor in hex).
2. **The post-condition is read from the bridge**, not from an admin authority — and the bridge can
   accept the action and apply nothing.

## Scope

### Included

- `UpdateTxType::SafeHarbourAddressUpdate = 14` (SSZ union selector **11**, payload
  `SafeHarbourAddress`) end to end, authorized by the **Strata Administrator**.
- A distinct create-menu entry for the Strata Administrator, with a form that reads the bridge's
  current safe harbour address and shows the descriptor hex the device will display.
- Enactment detection that compares the **bridge's actual address** against the proposed one
  ([Constraint 1](#1-a-rotation-with-the-harbour-already-activated-is-accepted-and-changes-nothing)).
- The standard cancel. PRD §5.2.2 carves out only the Sequencer Manager and Defcon 1, so this action
  is fully inside §5(b): a real `Approved` state, viewable cancellation signatures, and a cancel
  broadcast flow — signed by the Strata Administrator.

### Not included

- Any protocol validity rule. The orchestrator stays coordination-only, and in particular the
  application does **not** refuse to create a rotation that the chain will swallow
  ([Constraint 1](#1-a-rotation-with-the-harbour-already-activated-is-accepted-and-changes-nothing)).
- De-escalating the safe harbour. There is no such action upstream; `is_activated()` is never set
  back to `false`.
- A second creation path. This extends `create-proposal` exactly as V1, V2 and V3 did.
- Any notion of a *target authority* on the `Proposal` row. The proposal belongs to the Strata
  Administrator, and this action modifies no authority at all — it modifies the bridge.

## Requirements Alignment

- **PRD §5.5** — *Strata Administrator multisig: Safe Harbor address update*, listed as its own menu
  item. One entry in the requirement, one entry in the menu.
- **PRD §5.1** — the Approved/Pending/Past requirements apply to the Strata Administrator multisig,
  so this action gets the full lifecycle with no carve-out.
- **PRD §5.2.2** — the §5(b) carve-out names the Sequencer Manager multisig and *"Strata Security
  Council multisig (**Defcon 1 transaction**)"*. A safe harbour rotation is neither. It has an
  Approved state and a cancel.
- **PRD §3.1.4** — *the Strata Security Council multisig MUST be usable exclusively by Security
  Council Signers*. This action is **not** on the council multisig, so §3.1.4 does not reach it. The
  gate that does is `require_authorized_for_action`, which reads upstream's `authorized_role()` and
  therefore admits only a Strata Administrator session.

## Protocol Recap

Read from the `asm` submodule at the pinned `v0.1-alpha.11` (rev `b84eb28`).

- **The action.** `UpdateTxType::SafeHarbourAddressUpdate = 14`
  (`params/src/subprotocols/admin/updates.rs:30`), SSZ union selector **11**
  (`txs/src/actions/updates/mod.rs:52` — twelfth variant, zero-based). Payload is
  `SafeHarbourAddressUpdate { address: SafeHarbourAddress }`
  (`txs/.../updates/safe_harbour_address.rs:17-20`); its `new` is **infallible**, because the whole
  invariant lives in the inner type.
- **The authorizing role is the administrator.** `authorized_role()` returns
  `Role::StrataAdministrator` (`updates.rs:61`). Byte 14 sits in the `10..=19` Administrator band.
- **The payload is a P2TR-only BOSD descriptor.**
  `SafeHarbourAddress(bitcoin_bosd::Descriptor)` is constructible only through
  `TryFrom<Descriptor>`, which rejects any `type_tag() != DescriptorType::P2tr` with
  `NotP2trDescriptor` (`bridge-v1/types/src/safe_harbour.rs:27,54-65`). The invariant is enforced on
  all four ingress paths — `TryFrom`, `Deserialize`, `SszDecode` and `Arbitrary` — so it cannot be
  bypassed with hand-made wire bytes.
- **A BOSD descriptor is a type tag plus a payload.** `Descriptor { type_tag, payload }`, serialized
  as one tag byte followed by the payload. For P2TR the tag is `0x04` and the payload is a 32-byte
  x-only public key validated on the curve. `FromStr` accepts **hex only**; an address becomes a
  descriptor through `TryFrom<Address>`, and a descriptor becomes an address through
  `to_address(Network)`.
- **The chain applies it by relaying to the bridge.** `handle_update` dispatches
  `relay_bridge_safe_harbour_address_update` (`admin/subprotocol/src/handler.rs:168-170`), which
  sends `BridgeIncomingMsg::UpdateSafeHarbourAddress`; the bridge calls
  `state.update_safe_harbour_address(address)` (`bridge-v1/subprotocol/src/subprotocol.rs:160-163`).
- **The sequence number is consumed on the Strata Administrator**, like every other action of that
  role (`handler.rs:114-119`).
- **The depth is per-deployment.** `ConfirmationDepths::get` maps tx type 14 to
  `safe_harbour_address_update` (`confirmation_depth.rs:55`); depth `0` means "apply immediately,
  never enqueued". Upstream's sample params use 144; the local stack uses 30.
- **Activation is unaffected.** Only Defcon signals toggle `activated`
  (`subprotocol.rs:165-168`); a rotation never activates or deactivates the harbour.
- **The signing message is rendered by upstream and pinned in a test**
  (`safe_harbour_address.rs:62-88`):

  ```
  Strata ASM Administration v1
  Action: Safe Harbour Address Update
  Authorized By: Strata Administrator
  Sequence: 17
  Action Details:
    New Safe Harbour Address: 0479be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798
  ```

  The detail line is `hex(descriptor.to_bytes())` — **the descriptor, not an address**. Both labels
  are frozen by contract (`roles.rs:42-45`, `updates.rs:76-79`): external signers hash the rendered
  payload.
- **The read.** The address lives in the bridge section of the anchor state
  (`bridge.safe_harbour().address()`), and there is a dedicated RPC,
  `strata_asm_getSafeHarbour(block_hash) -> Option<SafeHarbour>` (`rpc/src/traits.rs:39-40`), whose
  JSON is `{"address": "<hex>", "activated": bool}`.
- **No upstream end-to-end coverage.** `admin/subprotocol/src/handler.rs:662-686` proves the message
  is forwarded to the bridge, and `safe_harbour.rs` unit-tests the frozen-address rule. Nothing
  upstream drives tx type 14 through a chain, and nothing anywhere covers a rotation submitted after
  the harbour is up.

---

## Constraints

### 1. A rotation with the harbour already activated is accepted and changes nothing

**Rule:** the application states the consequence wherever the decision is taken, and never reports
such a rotation as `Enacted`. It does **not** block it.

**Why:** `SafeHarbour::update_address` returns `false` when `activated` is true and leaves the
address untouched (`bridge-v1/.../state/bridge.rs:114-119`), and the subprotocol **discards that
boolean** (`subprotocol.rs:160-163`). There is no ASM log and no error. By then the signature check
has passed, the sequence number has been consumed, and the queue entry has been drained. The
transaction is valid, the proposal is accepted, and the destination is exactly what it was.

This is the same shape as [V3's Constraint 4](./security-council-signer-update.md#4-acceptance-is-not-application-and-upstream-does-not-say-so)
— acceptance is not application — but it is not a corner case here: it is what happens on every
rotation attempted after an incident, which is precisely when someone would try to redirect the
sweep.

**Consequence, and it is the load-bearing one:** the post-condition compares the bridge's **actual**
address, never the sequence number alone. A swallowed rotation therefore never reads `Enacted`; it
resolves as `Superseded` once the administrator's seqno moves past it. The label is imprecise; no
signer is told a destination changed that did not.

**Why not block it.** Refusing the action in the application would be a protocol rule we invented,
which [`AGENTS.md`](../../AGENTS.md) forbids, and it would rest on an RPC read that can fail — a
node that cannot answer must not stand between the administrator and an action the chain would
accept. The treatment is informational, and it belongs in [Signer Safety](#signer-safety).

### 2. Enactment is read from the bridge, and the seqno from the administrator

**Rule:** the address compared against the update is read from the **bridge** subprotocol's
`safe_harbour().address()`. The sequence number is read from `Role::StrataAdministrator`. Neither
term may be derived from the other.

**Why:** every action shipped before this one reads both terms from the administration subprotocol
— V3 split them across two *roles*, and this splits them across two *subprotocols*. Deriving the
answer from the seqno alone is exactly the failure Constraint 1 describes; deriving it from the
address alone would report `Enacted` for a rotation someone else submitted to the same destination.

Both halves are already available at the call site: `is_proposal_enacted_on_asm` decodes the bridge
state for the Defcon arms (`orchestrator-be/src/infrastructure/asm_enactment.rs:99,118,139`) and the
admin state for everything else.

**No chain tip is needed.** Unlike Defcon 3, the post-condition does not compare against
`bitcoin_tip >= activation_height`: leaving the queue with the address installed is the whole
answer. `action_needs_chain_tip` (`asm_enactment.rs:296-302`) stays exclusive to Defcon 3, so no
other action pays that RPC.

### 3. The reviewable artifact is the descriptor hex, not the address

**Rule:** every surface that shows the destination shows the **descriptor hex** as well, and that
hex comes from the Rust renderer the device signs over — never composed in TypeScript.

**Why:** the device displays `New Safe Harbour Address: 0479be…`. If the application showed only
`bcrt1p…`, the signer would have nothing to compare against the screen in front of them, and the
address → descriptor conversion — which is **ours** — would be unverifiable. That conversion is the
one place in this slice where a bug sends funds somewhere else, so it is the one thing the signer
must be able to check.

This is why the form accepts an address rather than the hex: an operator holds an address, and
nobody distributes a safe harbour as a BOSD string. The conversion is a convenience the application
provides and then immediately exposes for verification.

### 4. Network is a signal, not a protection

**Rule:** the form rejects an address whose network is not the process's active network, naming the
expected one. The Before/After renders addresses with that same network.

**Why:** BOSD carries no network. The same x-only key yields the same script — and therefore the
same descriptor bytes — on mainnet and on regtest; only the HRP differs. So a mainnet address pasted
into a regtest deployment is not *dangerous* in the way a wrong-script address is: it produces
exactly the descriptor the operator intended. It is, however, near-conclusive evidence that they
took the address from the wrong wallet, and that is worth stopping for.

The network comes from `network_from_env()`
(`desktop-app/src-tauri/src/infrastructure/network_env.rs:31-34`), which is the process-wide
resolution this repository already treats as canonical — not from `NodeConfig`, which deliberately
carries only endpoints.

### 5. P2TR and nothing else

**Rule:** any other descriptor type is refused, with a message that names what was supplied.

**Why:** `SafeHarbourAddress::try_from` rejects it (`safe_harbour.rs:54-65`), so a non-P2TR
destination cannot reach the chain. The form's check exists to explain, not to decide: upstream's
rejection is the one that matters, and the codec re-applies it. Two gates, one authority.

### 6. Rotating to the address already installed enacts and changes nothing

**Rule:** the form refuses an update whose destination equals the bridge's current address.

**Why:** unlike Constraint 1, this one produces a **false `Enacted`**. The chain accepts the no-op,
the seqno advances, the queue drains, and the post-condition — which compares addresses — finds a
match. The proposal reports `Enacted`, indistinguishable on every surface from a rotation that
actually rotated.

That is the same reasoning that made
[V3's AC 3b](./security-council-signer-update.md#3b-the-update-must-be-a-real-change) a safety rule
rather than hygiene, and it needs the same plumbing: the validator context gains
`currentSafeHarbourAddress: string | null`, twinned with the signer-set fields. When it is null the
rule is off, which is safe only because an unavailable read also blocks submission
([Edge Cases](#edge-cases)).

### 7. Everything generic already answers, and gets tests rather than changes

**Rule:** no new branch is added to depth resolution, cancelability, authorization or the lifecycle.

**Why:** `depth_for_action` (`asm_role_membership.rs:141-148`) resolves through
`update.update_tx_type()`, so tx type 14 reaches `confirmation_depths.safe_harbour_address_update`
with no new code — what [V1's Constraint 1](./security-council-defcon.md#1-lock-period-is-per-action-never-per-authority)
bought. `is_cancelable_for_hex` derives the affordance from the live depth (V2 Phase 3).
`require_authorized_for_action` compares against upstream's `authorized_role()`, so a council
session is refused with no council-specific code. `showsActivationCountdown` excludes only
`defcon_1`.

If this slice finds itself adding a branch to any of them, that is a finding worth stopping for.

---

## State Model

The standard lifecycle, with no carve-out — an ordinary Strata Administrator proposal:

```
Pending ──→ Approved ──→ Awaiting enactment ──→ Enacted
   │            │                │
   │            ↓                ↓
   │        Canceled         Canceled
   ↓
Expired / Superseded
```

- `Approved` displays as **"Approved"**. The Defcon 1 carve-out is keyed on the action, so it does
  not reach here.
- The activation countdown shows, driven by the live
  `confirmation_depths.safe_harbour_address_update`.
- `Canceled` is reachable while the entry is queued, by a Strata Administrator quorum.
- `Superseded` applies with the standard ordering, and is also where a rotation lands that the chain
  accepted but the bridge refused
  ([Constraint 1](#1-a-rotation-with-the-harbour-already-activated-is-accepted-and-changes-nothing)).

The proposal's `authority` is `strata_admin` throughout.

---

## Backend Contract (orchestrator-be)

### Authorization, lock period, cancelability

Unchanged, all three, per [Constraint 7](#7-everything-generic-already-answers-and-gets-tests-rather-than-changes).
`required_signatures` comes from `threshold_for_authority(auth.authority)` — the administrator's
threshold, which is the threshold of who signs.

### Enactment detection

The one real change, per [Constraint 2](#2-enactment-is-read-from-the-bridge-and-the-seqno-from-the-administrator).
`asm_enactment.rs:169-171` currently answers
`BadRequest("SafeHarbourAddress enactment detection is not implemented yet")`. It becomes:

```
last_seqno(Role::StrataAdministrator) >= seq_no
  && !still_queued
  && bridge.safe_harbour().address() == update.address()
```

`>=` on the seqno rather than `==`, for the reason Defcon 3 states
(`asm_enactment.rs:256-260`): the action carries a non-zero depth, so a later administrator action
may jump `last_seqno` past this proposal before it matures, and equality would mark a successfully
enacted rotation `Superseded`.

The same predicate moves in the desktop's copy of the module, or desktop and backend disagree about
whether a rotation enacted — the lesson of V3 Phase 2.

### Cancel creation

Unchanged. `create_cancel_proposal` stores the cancel under the target proposal's authority —
`strata_admin` — and requires the session to match.

---

## Frontend Contract (desktop-app)

### The create menu

One entry joins `ACTION_TYPES_BY_AUTHORITY.strata_admin`, after the existing entries so the default
selection does not change. It appears for no other authority. Neither its title nor its description
may contain the exact substring `"Signer update"` — three WebDriver specs select the administrator's
card by that text, and `ActionTypeCard` renders both title and description as `<p>` inside the
button.

### The create form

Modelled on `vk-update-form-fields.tsx`: the current value read from chain, then one input. It
carries three things and no more:

1. **The bridge's current safe harbour address**, rendered as an address and as its descriptor hex.
2. **One input** taking a bech32m P2TR address, validated per Constraints 4, 5 and 6.
3. **The descriptor hex the device will display**, resolved as the signer types, so the comparison
   the signer must make is possible before they reach the device.

The safe-harbour note appears here when the harbour is already active
([Constraint 1](#1-a-rotation-with-the-harbour-already-activated-is-accepted-and-changes-nothing)),
reusing `SafeHarbourNote` with its own wording.

### The signing message

Resolved from Rust through the same renderer the device signs over, exactly as every other action.
No TypeScript composes it.

### Lifecycle display

Nothing action-specific. The standard Approved label, the standard activation countdown, the
standard cancel affordance driven by the DTO field. The detail view shows current vs proposed
destination, both with their hex.

---

## Signer Safety

The signer is a Strata Administrator, and what they are authorizing is where every bridge satoshi
goes if the council ever pulls the lever. Nothing about it looks urgent, and that is the risk: it is
a routine-looking form whose blast radius equals the bridge's balance.

1. **The rendered message is the reviewable artifact, and it is a hex string.** The form shows that
   same hex next to the address the signer typed, so the device screen can be compared against
   something other than itself
   ([Constraint 3](#3-the-reviewable-artifact-is-the-descriptor-hex-not-the-address)).
2. **The conversion is ours, so it is shown.** Address → descriptor is the only step in this slice
   where a defect is invisible, and exposing its output is what makes it checkable.
3. **A wrong-network address is stopped**, with a message naming the expected network
   ([Constraint 4](#4-network-is-a-signal-not-a-protection)).
4. **An already-activated harbour is stated, not blocked**, on create, preview and sign — the three
   surfaces where the decision is still reversible
   ([Constraint 1](#1-a-rotation-with-the-harbour-already-activated-is-accepted-and-changes-nothing)).
   Amber, not red: it is a fact about the chain, not an error by the signer.
5. **Authority context is visible throughout** — the Strata Administrator badge from create through
   broadcast.
6. **The cancel window is stated where the decision is taken**, with the countdown driven by the
   live depth.

A non-administrator session can never reach this form, and the backend refuses the action
independently of the UI.

---

## Acceptance Criteria

### 1. A Strata Administrator can create a safe harbour address update

**Given** an authenticated Strata Administrator session
**When** the signer opens the create-proposal flow
**Then** the safe harbour address update is offered as its own entry, selecting it renders its
fields, and the default selection is unchanged.

### 1a. No other authority can reach it

**Given** an authenticated Sequencer Manager, Alpen Administrator or **Security Council** session
**When** the signer opens the create-proposal flow
**Then** the action is not offered, and navigating directly to it is refused by the schema.

The council case is the one that matters: it is the authority that triggers the sweep, and it must
not be able to choose the destination.

### 2. Only a Strata Administrator session can create one

**Given** a session on any authority other than Strata Administrator
**When** it submits a proposal whose action hex decodes to `UpdateAction::SafeHarbourAddress`
**Then** the backend refuses it, naming the required role, and no proposal is persisted.

The refusal comes from `require_authorized_for_action` comparing against upstream's
`authorized_role()` (`updates.rs:61`) — not from a list this application maintains.

### 3. The form takes an address and shows the descriptor

**Given** the form is open on a deployment whose active network is regtest
**When** the signer enters a valid P2TR address
**Then** the form shows the descriptor hex that the device will display, and it equals
`04` followed by the address's 32-byte x-only program.

### 3a. Non-P2TR destinations are refused

**Given** a P2WPKH, P2WSH, P2SH or P2PKH address
**When** the signer enters it
**Then** the form refuses it, naming the type supplied, and no action hex is built.

### 3b. A wrong-network address is refused

**Given** an address whose network differs from the process's active network
**When** the signer enters it
**Then** the form refuses it and names the expected network
([Constraint 4](#4-network-is-a-signal-not-a-protection)).

### 3c. The update must be a real change

**Given** an address equal to the bridge's current safe harbour address
**When** the signer reaches the sign step
**Then** it is refused as producing no change
([Constraint 6](#6-rotating-to-the-address-already-installed-enacts-and-changes-nothing)), while any
other valid address is allowed.

### 4. The signing message is upstream's, with its details block

**Given** a safe harbour address update at sequence 17 carrying the generator-point descriptor
**When** the signing message is rendered
**Then** it reads exactly the six lines upstream pins in `safe_harbour_address.rs:62-88`, with
`Action: Safe Harbour Address Update` and `Authorized By: Strata Administrator` on separate lines,
the destination rendered as descriptor hex, and it is byte-identical to what the hardware signer
displays.

### 5. The action is distinguishable everywhere

**Given** a persisted proposal carrying a safe harbour address update
**When** its action hex is decoded for display
**Then** it reports as a safe harbour address update — not as `unknown` — in the list, the detail
view, the sign view and the manual bundle.

### 6. It is queued, not enacted, on broadcast

**Given** a broadcast safe harbour address update at a non-zero depth
**When** the reveal confirms
**Then** the update is in the admin queue, the bridge's address is unchanged, and the proposal shows
Awaiting enactment with a countdown to `reveal_block + depth`.

### 7. Enactment compares the bridge's address against the administrator's sequence number

**Given** a queued safe harbour address update that reached its activation height
**When** enactment is evaluated
**Then** `bridge.safe_harbour().address()` equals the proposed descriptor, the sequence-number term
is read from `state.authority(Role::StrataAdministrator).last_seqno()`, and the proposal shows
Enacted ([Constraint 2](#2-enactment-is-read-from-the-bridge-and-the-seqno-from-the-administrator)).

### 7a. Activation is untouched

**Given** the same enacted update
**When** the bridge state is read
**Then** `is_activated()` is exactly what it was before — a rotation never activates or deactivates
the harbour.

### 7b. Neither term is substituted for the other

**Given** a queued update
**When** the administrator's `last_seqno` advances for an unrelated reason while the address is
unchanged, or the address matches while the seqno has not reached this proposal
**Then** neither case reads Enacted.

This is the test that fails if a future refactor answers from the seqno alone, which is the shape
Constraint 1 forbids.

### 8. A swallowed rotation is never reported as enacted

**Given** a safe harbour that is already activated
**When** a rotation is broadcast, confirmed, and its activation height passes
**Then** the queue is empty, the administrator's `last_seqno` has advanced, the bridge's address is
unchanged, and the proposal resolves as `Superseded` — never `Enacted`
([Constraint 1](#1-a-rotation-with-the-harbour-already-activated-is-accepted-and-changes-nothing)).

### 9. A cancelled rotation never applies

**Given** a queued safe harbour address update
**When** a Strata Administrator quorum cancels it and its original activation height passes
**Then** the queue is empty, the bridge's address is unchanged, the proposal reads `Canceled`, and
nothing reads `Enacted`.

### 10. The cancel is signed by the Strata Administrator

**Given** a queued safe harbour address update
**When** the cancel is created
**Then** it is stored under the `strata_admin` authority and requires a Strata Administrator session
— and a Security Council session is refused.

### 11. An already-activated harbour is stated at every decision point

**Given** a safe harbour that is already activated
**When** the signer opens the form, reviews the preview, or opens the sign view
**Then** each surface states that the rotation will not take effect, styled as information rather
than as an error, and the action is not blocked.

### 12. The depth is the live one

**Given** a deployment whose `confirmation_depths.safe_harbour_address_update` differs from every
other depth
**When** the countdown and the cancel affordance are resolved
**Then** both use that value, and no constant stands in for it anywhere.

### 13. The manual fallback works

**Given** a safe harbour address update with a quorum of collected signatures
**When** the signer exports the bundle
**Then** it broadcasts through the existing manual route and through an external Bitcoin RPC, with
no action-specific handling — and the bundle never reports the action as `unknown`.

---

## Edge Cases

| Scenario | Behavior |
|---|---|
| `confirmation_depths.safe_harbour_address_update` is `0` | Supported degradation. Applied in the submission block: no queue entry, no countdown, no cancel affordance — all three by construction, since each reads the resolved depth. |
| The safe harbour is already activated when the rotation matures | The seqno is consumed and the queue entry drained, but the bridge's address is unchanged, so post-conditions are not met and the proposal resolves as `Superseded`, never `Enacted`. See [Constraint 1](#1-a-rotation-with-the-harbour-already-activated-is-accepted-and-changes-nothing). |
| The harbour is activated *while* the rotation is queued | Same outcome, reached differently: valid at acceptance, refused at apply. Nothing in the application predicts it, and the note on the detail view reflects the live read. |
| Two rotations to the same destination are queued | Both post-conditions match once either applies. Recorded, not solved: it is the same ambiguity every config-carrying action has, and the seqno term bounds it to proposals of the same administrator. |
| The ASM cannot answer while the create form is open | The current address is unavailable, so neither the Before/After nor the no-op rule can answer. Load-bearing, like V3's config read: the form must not offer a destination it cannot compare, so submission is blocked with a message naming what could not be read. |
| The address is valid P2TR but nobody holds the key | Accepted on chain and allowed by the application. There is no way to tell from a script, and validating destinations is not something the protocol does either. |
| A rotation expires before quorum | The standard 7-day pending window. No carve-out. |

---

## Test Plan

The rules are the repository's: behaviour that can regress, tested where it lives; nothing that pins
a mock, a language guarantee, or a phrasing.

**Rust domain/codec (`src-tauri`)** — the address → descriptor conversion as a table: a valid P2TR
address, a P2WPKH address, an address of another network, a valid hex descriptor, and an x-only
value off the curve. This is the conversion that can send funds elsewhere, and it is the highest
value test in the slice. Plus a codec round-trip in both directions with a tripwire that the variant
still encodes `UpdateTxType::SafeHarbourAddressUpdate`, and `action_type_from_hex` naming the action
distinctly.

**Signing message (`src-tauri`)** — one tripwire that runs the path the device signs over: out of
the builder, not out of a hand-built `Action`. It asserts on `message.lines()`, not `contains()`
over the whole string, and pins that the destination line carries the descriptor hex rather than an
address ([AC 4](#4-the-signing-message-is-upstreams-with-its-details-block)).

**Backend unit (`orchestrator-be`)** — the enactment predicate as a truth table, with
[AC 7b](#7b-neither-term-is-substituted-for-the-other) and
[AC 8](#8-a-swallowed-rotation-is-never-reported-as-enacted) as tests carrying their own names: one
where the seqno advanced and the address did not, one where the address matches and the seqno has
not. Plus the authorization refusal of [AC 2](#2-only-a-strata-administrator-session-can-create-one)
over one action, both directions, and the depth resolving through the existing closure seam with no
ASM — paired against tx type 10, which is the pair an authority-shaped mapping cannot separate.

**Frontend** — pure functions only: the per-authority action menu and its default; the validator
against the five address forms; the no-op rule with its counter-case; and the IPC schema contract
tests, which fail when the Zod schema and the Rust DTO diverge.

**E2E (`e2e-tests`)** — a new file, `e2e_safe_harbour_address.rs`, following the shape of
`e2e_council_rotation.rs`. Three paths: enacted (queued with the address unchanged, mine exactly
`depth`, address changed and `is_activated()` still false), cancelled (cancel inside the window,
mine `depth`, queue empty and address unchanged), and **swallowed** (fire a Defcon 1 first, then
rotate: accepted, seqno advanced, queue drained, address unchanged). The third is the only automated
proof of [Constraint 1](#1-a-rotation-with-the-harbour-already-activated-is-accepted-and-changes-nothing)
against a real chain, and it exists nowhere — upstream included.

**Not tested, deliberately:** no DOM or component tests — this repository has no DOM runner, and a
test that reads a component's source with `readFileSync` pins a phrasing rather than a behaviour.
The gap is closed by a manual walk. No ASM-backed integration test inside `orchestrator-be`: it
would be the flakiest test in the repository and would re-prove what the e2e proves.

---

## Verification

Code review checks that:

- [ ] The enactment predicate reads the address from the bridge and the seqno from the Strata
      Administrator — in both copies.
- [ ] No surface reports a rotation enacted on the strength of a consumed sequence number alone.
- [ ] No constant stands in for `confirmation_depths.safe_harbour_address_update`.
- [ ] `action_needs_chain_tip` still answers only for Defcon 3.
- [ ] The signing message is rendered by the Rust renderer, never composed in TypeScript, and every
      surface showing the destination also shows its descriptor hex.
- [ ] The network comes from `network_from_env()`, not from a per-command parse.
- [ ] `bitcoin-bosd` resolves to the same revision the `asm` submodule pins, and only the codec
      imports it.
- [ ] The new Tauri command is registered in **both** handler lists in `commands/invoke.rs`.
- [ ] Every new frontend test file falls inside CI's `src/**/*.test.ts(x)` glob.

Post-merge validation on regtest, with the local stack
(`./scripts/local-stack.sh --clean`), which already carries a depth of 30 for tx type 14 and an
initial safe harbour address:

1. A Strata Administrator signer reaches the new entry; a Security Council signer does not.
2. The form shows the bridge's current address and its descriptor hex.
3. A P2WPKH address and a wrong-network address are refused with distinct, specific messages.
4. The rendered message matches the signer's screen and carries the descriptor hex.
5. Quorum, broadcast — Approved, then Awaiting enactment with a countdown to `reveal + 30`, and the
   bridge's address unchanged.
6. Path A: mine 30 blocks → `Enacted`, and `strata_asm_getSafeHarbour` shows the new address with
   `activated` unchanged.
7. Path B: cancel inside the window → the target reads `Canceled` and the address is unchanged.
8. Path C: with the harbour already activated, the note appears on create, preview and sign, the
   action can still be signed, and it resolves as `Superseded`.
9. Pasting the address already installed is refused by the form.
10. The manual bundle exports and imports the action without reporting it as unknown.
11. `cargo test -p alpen-multisig-e2e-tests` green, including `e2e_safe_harbour_address`.
