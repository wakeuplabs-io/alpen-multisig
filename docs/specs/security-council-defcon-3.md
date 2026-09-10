# Spec: Security Council — Defcon 3

**Status:** Shipped — all seven phases. This document is the functional contract; the build
plan is [`security-council-defcon-3-implementation.md`](./security-council-defcon-3-implementation.md),
whose phase board says what has landed.

**PRD:** [`06-prd-hardware-signer-and-block-payouts-update.md`](../0-prd/06-prd-hardware-signer-and-block-payouts-update.md) §3.1.4, §5.1, §5.2.2, §5.5

**Stories:** US-E13 (create a Defcon 3) and **US-E14 (cancel a queued Defcon 3)** — both in scope here.

**Master plan:** [`security-council.md`](./security-council.md) §7 Slice board, where this slice is V2.

**Predecessor:** [`security-council-defcon.md`](./security-council-defcon.md) — the Defcon 1 contract.
Everything it froze about the council still holds; this document states only what Defcon 3 adds or
contradicts.

---

## Objective

Give a Security Council signer the second, timelocked lever: create → sign → quorum → broadcast →
**queued for `confirmation_depths.defcon3` blocks** → Enacted, with a real cancellation window in the
middle that the council itself can use.

Defcon 1 and Defcon 3 relay the *same* message to the bridge — upstream routes both through
`relay_bridge_defcon`, and the bridge cannot tell them apart. The only difference between the levers
is **when** that message is emitted: immediately, or after a delay during which the council can
change its mind. That single difference is the whole of this slice.

## Scope

### Included

- Defcon 3 (`UpdateTxType::Defcon3 = 43`, SSZ union selector 10, empty payload) end to end for the
  Strata Security Council authority.
- The queued lifecycle: a real `Approved` state, an activation countdown driven by the live depth,
  and enactment at the activation height.
- **The Defcon 3 cancel (US-E14)**, signed by the council itself — absorbed from what the master
  plan called slice V5.
- The two debts [`security-council-defcon.md`](./security-council-defcon.md#what-v2-inherits-and-must-revisit)
  left for this slice: redundancy decided by activation height, and cancelability carried on the
  proposal instead of guessed by authority in the desktop.

### Not included

- Security Council membership update (V3) and Safe Harbour address update (V4) — both authorized by
  the **Strata Administrator**, not the council.
- De-escalation. There is none in the protocol: `is_activated()` is never set back to `false`. A
  cancel prevents an activation; it never undoes one.
- Any protocol validity rule. The orchestrator stays coordination-only.
- A second creation path. Defcon 3 extends `create-proposal` exactly as Defcon 1 did.

## Requirements Alignment

- **PRD §5.5** *Security Council multisig: Defcon 3 transaction* → `UpdateTxType::Defcon3 = 43`.
- **PRD §5.2.2** — the §5(b) carve-out names *"Strata Security Council multisig (**Defcon 1
  transaction**)"* only. Defcon 3 is therefore **fully inside §5(b)**: it has an Approved state, its
  cancellation signatures are viewable, and it has a cancel broadcast flow.
- **PRD §3.1.4** — usable exclusively by Security Council signers. Already enforced generically by
  `require_authorized_for_action`, which reads upstream's `authorized_role()`; this slice adds the
  proof, not the mechanism.

## Protocol Recap

Read from the `asm` submodule at the pinned tag; see [`security-council.md`](./security-council.md)
§2–3 for the full derivation.

- The payload `Defcon3Update` is an empty unit struct. Two Defcon 3 actions with the same sequence
  number are byte-identical — see [Edge Cases](#edge-cases).
- `ConfirmationDepths::get` returns `None` for a depth of `0` ("apply immediately, bypass the
  queue") and `Some(depth)` otherwise. `Defcon1` is hardcoded to `0` with no field;
  `Defcon3` reads the `defcon3` field.
- A queued update carries an `activation_height`, and `process_queued` drains at
  `activation_height <= tip` — so exactly `depth` blocks after the reveal, not `depth + 1`.
- A cancel's authorizing role is **the role of the update being cancelled**, so a Defcon 3 cancel is
  signed by the Security Council. There is no cross-role veto.
- Accepting an action **jumps** the role's `last_seqno` to the accepted value. Acceptance happens at
  the reveal, not at maturity — which is why [Constraint 2](#2-defcon-3-enactment-cannot-reuse-defcon-1s-seqno-equality) exists.

---

## Constraints

### 1. The delay is always the live depth, never a constant

**Rule:** Every place that needs the Defcon 3 delay — the activation height, the countdown, the
cancel gate — resolves it from `confirmation_depths.defcon3` in live ASM state, on every read. No
constant, no UI default, and in particular not the 432 blocks (72 hours) Alpen's public documentation
describes.

**Why:** the depth is a per-deployment parameter with no default anywhere in the ASM. Taking whatever
the deployment reports is correct everywhere without a code change, and a deployment that configured
something unusual then degrades correctly rather than being lied about on screen.

**Implementation note:** `lock_period_for_action` already does this, and V1's Phase 1 exists
precisely so it could. This constraint is inherited, not new — it is restated because V2 is the first
slice that can actually exercise a non-zero council depth.

### 2. Defcon 3 enactment cannot reuse Defcon 1's seqno equality

**Rule:** the Defcon 3 enactment predicate requires `last_seqno >= seq_no`, never `==`.

**Why:** upstream consumes the sequence number when it **accepts** the action, at the reveal — not
when the queued entry matures. In the window between the two, the council may accept another action
and move `last_seqno` past this proposal, and an equality test would then answer `false` forever.
The consequence is worse than a stuck label: `reconcile_one` checks enactment first and otherwise
falls through to `supersede_if_seq_no_consumed`, which spares the proposal only while it is still in
the queue. The moment it matures and leaves, "not enacted" plus "seqno passed" would mark a
successfully enacted Defcon 3 as **Superseded**.

Defcon 1 can use equality precisely because it is never queued: acceptance and enactment are the
same event for it.

### 3. A cancelled Defcon 3 must never be reported as Enacted

**Rule:** leaving the queue is not evidence of enactment. The predicate additionally requires that
the chain tip reached the proposal's stored `activation_height`, and a proposal already in a terminal
state is never re-evaluated.

**Why:** a cancel removes the entry from the queue *before* its activation height, and
`safe_harbour_activated` may already be `true` from an earlier Defcon 1 — so the naive conjunction
"harbour on and not queued" is satisfied by a Defcon 3 that was cancelled. The height term is what
separates "matured" from "was taken out early".

**Inside the application, the cancel owns the target's outcome.** A cancel that reached the chain
takes its target out of the queue while the target's sequence number is already consumed — which is
also the shape the supersession sweep reads as "dead". The reconciliation therefore stops deciding a
proposal whose cancel is on chain and lets the cancel write `Canceled`; without that, the target
lands on `Superseded` and the cancel on `Expired`, whichever order the sweep happens to use.

**Known limit, recorded rather than solved:** if the harbour was already active *and* a cancel was
broadcast entirely outside this application *and* the tip has since passed the activation height, no
observable ASM state distinguishes the two outcomes — there is no cancel proposal to defer to. This
is the same class of limit V1's Phase 4 recorded for reveal-block ordering, and the
[Phase 7 e2e](#test-plan) is what pins the in-band behaviour.

### 4. Cancelability is answered by the backend, for every authority

**Rule:** the proposal DTO carries whether the proposal can be cancelled, derived from the same
depth resolution the write gate uses. The desktop must hold no authority allow-list.

**Why:** `create_cancel_proposal` has gated on depth alone since V1; the desktop still asks
`CANCELABLE_AUTHORITIES.includes(authority) && actionType !== 'defcon_1'` because it cannot read a
live depth. The two sides ask different questions, and V2 is the first slice where the difference is
load-bearing: the council must gain a cancel affordance for Defcon 3 and keep having none for
Defcon 1 — which no authority-shaped condition can express, since they share one authority.

**Blast radius, accepted deliberately:** Sequencer Manager proposals gain a visible cancel
affordance. The backend has permitted this since V1; only the desktop's list hid it. That is a UI
change in an authority this slice is not about, and it is the correct behaviour rather than a
regression.

### 5. Defcon 3 is destructive, but it is not irreversible

**Rule:** no Defcon 3 surface may reuse Defcon 1's *Irreversible* copy. Defcon 3 keeps the
destructive visual treatment and the type-to-confirm gate, and states the truth instead: the sweep is
**delayed and cancelable until it activates**, and irreversible from that point on.

**Why:** the copy is the safety mechanism. Telling a signer that a cancelable action cannot be
cancelled is not a harmless overstatement — it withholds the one lever that could stand the alarm
down, and it trains signers to discount the same warning on Defcon 1, where it is true.

---

## State Model

Defcon 3 uses the **standard** lifecycle with no label carve-out — the opposite of Defcon 1:

```
Pending ──→ Approved ──→ Awaiting enactment ──→ Enacted
   │            │                │
   │            ↓                ↓
   │        Canceled         Canceled
   ↓
Expired / Superseded
```

- `Approved` is displayed as **"Approved"**, the word Defcon 1 is carved out of. Once the reveal
  confirms it displays as **"Awaiting enactment"** with the activation countdown, which is the
  behaviour every other queued action already has.
- `Canceled` is reachable, and reachable only while the entry is queued.
- `Superseded` applies as it does to every other action, with the standard ordering: enactment is
  decided first, and presence in the queue outranks a consumed sequence number.

Backend state names are unchanged. The only Defcon-specific display rule that survives from V1 is
Defcon 1's, and it stays keyed on the action so that adding Defcon 3 changes nothing about it.

---

## Backend Contract (orchestrator-be)

### Authorization

No new gate. `require_authorized_for_action` resolves the action's `authorized_role()` from upstream
and compares it to the session authority, so Defcon 3 is council-only by construction. This slice
adds the test that says so ([AC 2](#2-only-a-council-session-can-create-a-defcon-3)), because a
requirement with no test is a requirement nobody checked.

### Lock period and activation height

Unchanged mechanism: `compute_and_store_activation_height` stores
`block_height_of(reveal_txid) + lock_period_for_action(action_hex)` when the reveal confirms, and
`record_reveal_confirmed_facts` stores the queue's `UpdateId` alongside it. For a Defcon 3 the lock
period is the live `defcon3` depth; for a Defcon 1 it is `0`, which is why a Defcon 1's activation
height equals its own reveal block. That equality is what makes the two comparable in
[AC 9](#9-the-activating-proposal-is-the-one-with-the-lowest-activation-height).

### Enactment detection

The `Defcon3` arm of `is_proposal_enacted_on_asm` stops returning `BadRequest` and becomes a pure
predicate over four observations:

| Term | Source | Why |
|---|---|---|
| `last_seqno >= seq_no` | council authority in admin state | the chain accepted this action ([Constraint 2](#2-defcon-3-enactment-cannot-reuse-defcon-1s-seqno-equality)) |
| not present in `admin.queued()` | admin state | it left the queue |
| `tip >= activation_height` | Bitcoin tip vs the stored height | it left by maturing, not by being cancelled ([Constraint 3](#3-a-cancelled-defcon-3-must-never-be-reported-as-enacted)) |
| `safe_harbour().is_activated()` | bridge state | the effect is visible |

The predicate is a free function beside `defcon1_enacted`, taking these as plain arguments so that
its truth table is testable without an ASM.

### Cancelability on the DTO

`GET /proposals` and `GET /proposals/:action_id` carry a field saying whether the proposal can be
cancelled, computed from the action's confirmation depth — non-zero means cancellable. Rules:

- It is derived from the **same** function `create_cancel_proposal` gates on, so the affordance and
  the API cannot drift.
- A `MultisigAction::Cancel` is never cancellable. This is free (`depth_for_action` returns `0` for
  cancels) and is asserted rather than assumed.
- The listing resolves the depth table **once per request**, not once per proposal.
- When the ASM cannot answer, the field degrades the way `live_last_seqno` already does — the read
  never fails because a cleanup could not be computed. A proposal whose cancelability is unknown
  offers no cancel affordance, because the honest failure is a missing button and not a button that
  cannot work.

### Cancel creation

No new code is expected. `create_cancel_proposal` already stores the cancel under the **target's**
authority and requires the session authority to match it — which for a Defcon 3 target is the
council itself, exactly as the protocol requires. Its depth gate already admits a Defcon 3 and
already refuses a Defcon 1.

---

## Frontend Contract (desktop-app)

### Authorization gate

A council session sees **two** action types, Defcon 1 and Defcon 3, in that display order. No other
authority sees either. The first entry is also the default selection, so this makes Defcon 1 the
council's default deliberately rather than by accident — the emergency lever is the one a signer is
most likely to be reaching for under time pressure, and the delayed one is the considered choice.

### Create form

Defcon 3 extends `create-proposal` and gets no route of its own, exactly as Defcon 1 does. The form
carries, in order:

1. The safe-harbour note, when the bridge is **already** in safe harbour — with its own wording, not
   Defcon 1's. It is told, never enforced: a warning, never a block.
2. A destructive callout stating the **delayed and cancelable** nature of the action
   ([Constraint 5](#5-defcon-3-is-destructive-but-it-is-not-irreversible)).
3. The rendered signing message, verbatim from the same Rust renderer the device signs over, with no
   `Action Details:` block.
4. The type-to-confirm input: **`DEFCON 3`**, matched case-insensitively with no trimming, the same
   rule Defcon 1 uses.

The two confirmation strings must be **mutually exclusive**: typing `DEFCON 1` must not satisfy the
Defcon 3 gate, and vice versa. This is the property most at risk if the form component is shared.

### Lifecycle display

Everything here already behaves correctly for a non-Defcon-1 action; this slice mostly pins it:

- Approved shows the word **Approved**, and after the reveal confirms, **Awaiting enactment** with
  the activation countdown — activation block, current block, and an approximate remaining time.
- The cancel affordance appears for a queued Defcon 3 and never for a Defcon 1.
- The countdown decision is one shared predicate. The cancel screen currently asks a different
  question (`status === 'approved'` alone) and is brought onto the shared one.

### The redundancy badge

The badge that says an enacted proposal changed nothing on chain is currently computed by ordering
enacted **Defcon 1** proposals by sequence number. Defcon 3 activates the same flag on a timelock, so
that premise breaks the moment this slice ships. It is replaced by ordering **all** enacted
harbour-activating proposals by their **activation height**; the lowest is the one that turned the
harbour on, and every one after it changed nothing — Defcon 3 included.

### Manual fallback

Unchanged and required: a council signer can export the bundle, aggregate signatures and broadcast a
Defcon 3, and its cancel, outside the orchestrator through the existing `/manual` route.

---

## Signer Safety

Defcon 3 authorizes sweeping **all bridge funds**, and the action carries no payload — so the
four-line signing message is once again the entire reviewable artifact.

1. **The message is rendered, never composed.** Resolved through upstream's renderer and printed
   verbatim; a failed resolve disables the CTA rather than letting a signer confirm four lines they
   never saw.
2. **Type-to-confirm** `DEFCON 3` before the sign CTA enables.
3. **Destructive visual treatment**, and honest copy about the delay
   ([Constraint 5](#5-defcon-3-is-destructive-but-it-is-not-irreversible)).
4. **Authority context on every step**, from create through broadcast.
5. **The cancel window is stated where the decision is taken** — the countdown is visible on the
   proposal, not buried in a detail view.

A non-council session can never reach these forms, and the backend refuses the action even if one
did.

---

## Acceptance Criteria

### 1. A council signer can create a Defcon 3

**Given** an authenticated Strata Security Council session
**When** the signer opens the create-proposal flow
**Then** both `DEFCON 1` and `DEFCON 3` are offered, and selecting `DEFCON 3` renders the Defcon 3
form.

### 1a. No other authority can reach it

**Given** a session for any other authority
**When** the signer opens the create-proposal flow
**Then** no Defcon action type is offered, and no route reaches the form.

### 2. Only a council session can create a Defcon 3

**Given** a session for an authority other than the Security Council
**When** it posts a Defcon 3 action to `POST /proposals`
**Then** the request is refused before any proposal object is created, naming the role the action
requires.

### 3. A duplicate Defcon 3 is rejected

**Given** a Defcon 3 proposal exists for a given sequence number
**When** the same `(action, seq_no)` is submitted again
**Then** the request is rejected naming the existing `ActionId`, and nothing is mutated.

### 4. The signing message is the four canonical lines

**Given** the Defcon 3 form with a resolved sequence number
**When** the message renders
**Then** it reads `Strata ASM Administration v1` / `Action: Defcon 3` /
`Authorized By: Strata Security Council` / `Sequence: <n>`, with **no** `Action Details:` block, and
it differs from the Defcon 1 message.

Upstream pins this exact string in `admin/txs/src/actions/updates/defcon3.rs:32-46`, and the label
is frozen by contract (`updates.rs:91-92`) because external signers hash the rendered payload.

### 5. The type-to-confirm gate is exact and mutually exclusive

**Given** the Defcon 3 form
**When** the signer types anything other than `DEFCON 3` (case-insensitive, untrimmed)
**Then** the sign CTA stays disabled — and in particular `DEFCON 1` does not satisfy it, nor does
`DEFCON 3` satisfy the Defcon 1 gate.

### 6. A broadcast Defcon 3 is queued, not enacted

**Given** a Defcon 3 whose reveal has confirmed
**When** the chain tip is below its activation height
**Then** the proposal is **Awaiting enactment**, the safe harbour is **not** activated, and the ASM
admin queue holds its entry.

### 7. The countdown is driven by the live depth

**Given** a queued Defcon 3
**When** the proposal is displayed
**Then** the countdown shows its activation block against the current block, derived from
`confirmation_depths.defcon3` as the deployment reports it — never a constant.

### 8. It enacts at exactly its depth

**Given** a queued Defcon 3
**When** the tip reaches `reveal_block + defcon3`
**Then** the entry leaves the queue, the safe harbour is activated, and the proposal becomes
`Enacted` — including when the council accepted another action in the meantime
([Constraint 2](#2-defcon-3-enactment-cannot-reuse-defcon-1s-seqno-equality)).

### 9. The activating proposal is the one with the lowest activation height

**Given** several enacted proposals that activate the safe harbour
**When** the redundancy badge is computed
**Then** the one with the lowest activation height is named the activation and every later one is
marked as having changed nothing — regardless of sequence number, and regardless of which of the two
Defcon types each is.

### 10. A queued Defcon 3 offers a cancel; a Defcon 1 never does

**Given** a council session
**When** it views a queued Defcon 3 and an approved Defcon 1
**Then** the Defcon 3 offers the cancel flow and the Defcon 1 offers none, in every view.

### 11. The cancel is signed by the council itself

**Given** a queued Defcon 3
**When** a council signer starts its cancel
**Then** the cancel proposal is created under the Security Council authority and requires a fresh
council quorum.

### 12. A cancelled Defcon 3 never activates the harbour

**Given** a queued Defcon 3 whose cancel enacted
**When** the tip passes the original activation height
**Then** the entry is gone from the queue, the safe harbour is **not** activated by it, and the
proposal reads `Canceled` — never `Enacted`
([Constraint 3](#3-a-cancelled-defcon-3-must-never-be-reported-as-enacted)).

### 13. Cancelability travels on the proposal

**Given** any proposal
**When** the desktop decides whether to offer a cancel
**Then** it reads the backend's answer, holds no authority allow-list of its own, and a cancel
proposal is itself never cancellable.

### 14. The manual fallback works for both

**Given** the orchestrator is unavailable
**When** a council signer uses the offline route
**Then** a Defcon 3 and its cancel can both be aggregated and broadcast, and the exported bundle
carries the raw transaction hex.

### 15. The safe-harbour note appears with its own wording

**Given** the bridge is already in safe harbour
**When** a council signer opens the Defcon 3 form
**Then** a note says so — stating that a Defcon 3 additionally costs a full delay before changing
nothing — and it never blocks creation.

---

## Edge Cases

| Scenario | Behavior |
|---|---|
| `confirmation_depths.defcon3` is `0` in a deployment | Supported degradation. The action becomes immediate and uncancellable: no queue entry, no countdown, no cancel affordance — all three by construction, since each reads the resolved depth. Nothing hardcodes an alternative. |
| Two live Defcon 3 proposals | The payload is empty, so two Defcon 3 actions are byte-identical and a queue lookup by action matches the **first** entry. Each proposal then reads the other's queue state. Sequence numbers differ, so acceptance still orders them; **prevented at creation** by [AC 3](#3-a-duplicate-defcon-3-is-rejected) only when the seqno also matches. Recorded as a known ambiguity and revisited if a deployment ever needs two in flight. |
| Safe harbour already active when a Defcon 3 is created | Allowed, warned about ([AC 15](#15-the-safe-harbour-note-appears-with-its-own-wording)). Refusing the lever on the strength of a state read is the worse failure. |
| A Defcon 3 is cancelled after its activation height | Impossible on chain — the entry is gone. The cancel fails with `UnknownAction`, and the proposal shows Enacted. |
| Council membership rotated while a Defcon 3 is queued | The queued entry stands; the ASM validated it at acceptance. A *cancel* would need a quorum of the new council. |
| The ASM cannot answer during a listing | The listing succeeds; cancelability degrades to "no affordance" and the next cycle asks again. |
| A Defcon 3 expires before quorum | The standard 7-day pending window applies, exactly as for Defcon 1. No emergency carve-out. |

---

## Test Plan

The rules are the repository's, not this slice's: behaviour that can regress, tested where it lives;
nothing that pins a mock, a language guarantee, or a phrasing.

**Backend unit (`orchestrator-be`)** — the Defcon 3 enactment truth table, with
[Constraint 2](#2-defcon-3-enactment-cannot-reuse-defcon-1s-seqno-equality) and
[Constraint 3](#3-a-cancelled-defcon-3-must-never-be-reported-as-enacted) as named tests; the
cancelability derivation (Defcon 1 → false, Defcon 3 with depth > 0 → true, cancel → false); the
authorization refusal of AC 2.

**Tauri unit (`src-tauri`)** — codec round-trip and the `UpdateTxType::Defcon3` tripwire; the action
builder; `action_type_from_hex`; and one tripwire that the Defcon 3 signing message is non-empty and
differs from Defcon 1's, so an upstream change that collapsed them is caught here rather than on a
signer's screen.

**Frontend** — pure functions only: the mutual exclusion of the two confirmation strings, the
per-authority action menu, the display-status and countdown predicates, the redundancy ordering by
activation height, and the field-driven cancel decision. Plus the IPC schema contract tests, which
fail when the Zod schema and the Rust DTO diverge.

**E2E (`e2e-tests`)** — the existing `run_defcon3` proves queue → depth → activation. This slice adds
the cancelled path: submit a Defcon 3, assert queued with the harbour off, submit a council-signed
cancel, mine exactly `depth` blocks, and assert the queue is empty **and the harbour is still off**.
That assertion is the only automated coverage of
[Constraint 3](#3-a-cancelled-defcon-3-must-never-be-reported-as-enacted).

**Not tested, deliberately:** no DOM or component tests — this repository has no DOM runner, and a
test that reads a component's source with `readFileSync` pins a phrasing rather than a behaviour. The
gap is closed by a manual walk, which in V1 is what found what reviewing did not. No ASM-backed
integration test inside `orchestrator-be`: it would be the flakiest test in the repository and would
re-prove what the e2e already proves.

---

## Verification

Code review checks that:

- [ ] No constant stands in for the Defcon 3 depth anywhere.
- [ ] The enactment predicate uses `>=` on the sequence number and carries the height term.
- [ ] The desktop holds no authority allow-list for cancelability.
- [ ] No Defcon 3 surface claims the action is irreversible while it is still queued.
- [ ] The two confirmation strings cannot satisfy each other's gate.
- [ ] The signing message is rendered by the Rust renderer, never composed in TypeScript.
- [ ] Every new frontend test file is picked up by CI's glob.

Post-merge validation on regtest, with the local stack:

1. A council signer creates, signs to quorum and broadcasts a Defcon 3; the harbour stays off.
2. The proposal shows Approved, then Awaiting enactment with a countdown to the right block.
3. Path A: mine `depth` blocks → Enacted, harbour on.
4. Path B: cancel inside the window → the target reads Canceled, the harbour stays off, and nothing
   reads Enacted.
5. A Defcon 1 created in the same session still shows no countdown and no cancel affordance.
