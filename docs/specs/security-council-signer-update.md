# Spec: Security Council — Signer Update

**Status:** Not started. This document is the functional contract; the build plan is
[`security-council-signer-update-implementation.md`](./security-council-signer-update-implementation.md),
whose phase board says what has landed.

**PRD:** [`06-prd-hardware-signer-and-block-payouts-update.md`](../0-prd/06-prd-hardware-signer-and-block-payouts-update.md) §5.1, §5.2.2, §5.5

**Stories:** US-E7 (create a Security Council signer update) — actor **Strata Admin Signer**,
authority **Strata Admin**.

**Master plan:** [`security-council.md`](./security-council.md) §7 Slice board, where this slice is V3.

**Predecessors:** [`security-council-defcon.md`](./security-council-defcon.md) (V1) and
[`security-council-defcon-3.md`](./security-council-defcon-3.md) (V2). Everything they froze about
the council still holds. This is the first slice of the feature authorized by the **Strata
Administrator** rather than by the council, so it states what that inversion changes.

---

## Objective

Let a Strata Administrator signer rotate the Security Council's membership and threshold: create →
sign → quorum → broadcast → queued for `confirmation_depths.strata_security_council_multisig_update`
blocks → Enacted, with the standard cancel window in the middle.

This is [§2.1's segregation invariant](./security-council.md#21-the-segregation-invariant) becoming
executable. The council controls *when* the sweep fires; the administrator controls *who sits on the
council*. Upstream says so in the code itself
(`asm/crates/params/src/subprotocols/admin/roles.rs:35-38`):

> Its own membership is rotated by the `Role::StrataAdministrator`, not itself, so the council
> cannot lock itself out via self-rotation.

Mechanically this is the same `ThresholdConfigUpdate` the application has shipped for three
authorities since V1. **One thing, and only one thing, is new: the role that authorizes the action
and the role the action modifies are different.** Every existing action has them identical, and the
application has that identity baked into two places. That single divergence is the whole of this
slice.

## Scope

### Included

- `UpdateTxType::StrataSecurityCouncilMultisigUpdate = 15` (SSZ union selector **3**, payload
  `ThresholdConfigUpdate`) end to end, authorized by the **Strata Administrator**.
- A distinct create-menu entry, `council_signer_update`, alongside the administrator's own
  `signer_update` — which is how PRD §5.5 lists them.
- Enactment detection that reads the **target** role's config and the **authorizing** role's
  sequence number ([Constraint 1](#1-enactment-reads-two-roles-not-one)).
- The standard cancel. PRD §5.2.2 carves out only the Sequencer Manager and Defcon 1, so this action
  is fully inside §5(b): a real `Approved` state, viewable cancellation signatures, and a cancel
  broadcast flow — signed by the Strata Administrator, the role that authorized the update.

### Not included

- Safe Harbour address update (V4). Same authority, same segregation invariant, different payload —
  and a P2TR BOSD descriptor is a validation problem this slice does not need to solve.
- Any protocol validity rule. The orchestrator stays coordination-only; the ASM decides whether a
  rotation is legal.
- Repairing [Constraint 4](#4-acceptance-is-not-application-and-upstream-does-not-say-so). It is
  pre-existing, it applies identically to the three signer updates already shipped, and fixing it
  inside a slice about the council would be fixing it in the wrong place.
- A second creation path. This extends `create-proposal` exactly as Defcon 1 and Defcon 3 did.
- Any notion of a *target authority* on the `Proposal` row. The proposal belongs to the Strata
  Administrator; the target lives in the action, which is where the chain reads it from
  ([Constraint 2](#2-the-target-comes-from-the-action-never-from-the-session)).

## Requirements Alignment

- **PRD §5.5** — *Strata Administrator multisig: Security Council Signer update*, listed as its own
  menu item next to *Strata Administrator Signer update*. Two entries in the requirement, two entries
  in the menu.
- **PRD §5.1** — the Approved/Pending/Past requirements apply to the Strata Administrator multisig,
  so this action gets the full lifecycle with no carve-out.
- **PRD §5.2.2** — the §5(b) carve-out names the Sequencer Manager multisig and *"Strata Security
  Council multisig (**Defcon 1 transaction**)"*. A council **rotation** is neither: it is a Strata
  Administrator proposal. It has an Approved state and a cancel.
- **PRD §3.1.4** — *the Strata Security Council multisig MUST be usable exclusively by Security
  Council Signers*. This action is **not** on the council multisig, so §3.1.4 does not reach it. The
  gate that does is `require_authorized_for_action`, which reads upstream's `authorized_role()` and
  therefore admits only a Strata Administrator session.

## Protocol Recap

Read from the `asm` submodule at the pinned `v0.1-alpha.11` (rev `b84eb28`).

- **The action.** `UpdateTxType::StrataSecurityCouncilMultisigUpdate = 15`
  (`params/src/subprotocols/admin/updates.rs:32`), SSZ union selector **3**
  (`txs/src/actions/updates/mod.rs:45` — fourth variant, zero-based). Payload is a bare
  `ThresholdConfigUpdate` (`txs/src/actions/updates/strata_security_council_multisig.rs:13-14`),
  byte-identical in shape to the three multisig updates already shipped.
- **The authorizing role is the administrator.** `authorized_role()` returns
  `Role::StrataAdministrator` (`updates.rs:64`), with the reason in the source above it: *"Security
  council membership is rotated by the administrator, not by the council itself."* Byte 15 sits in
  the `10..=19` Administrator band, not the `40..=49` Council band.
- **The applied role is the council.** `handle_update` dispatches
  `apply_multisig(state, Role::StrataSecurityCouncil, update.config())`
  (`subprotocol/src/handler.rs:145-147`).
- **The sequence number is consumed on the authorizing role.** `handle_action` resolves the role
  once and advances `last_seqno` on it (`handler.rs:114-119`), so a council rotation consumes the
  **administrator's** counter. Seqno is per-role, and these two roles are different.
- **The depth is per-deployment.** `ConfirmationDepths::get` maps tx type 15 to
  `strata_security_council_multisig_update` (`params/.../confirmation_depth.rs:43-45`); depth `0`
  means "apply immediately, never enqueued".
- **The signing message is rendered by upstream and pinned in a test**
  (`strata_security_council_multisig.rs:52-75`):

  ```
  Strata ASM Administration v1
  Action: Strata Security Council Multisig Update
  Authorized By: Strata Administrator
  Sequence: 7
  Action Details:
    New Threshold: 2
    Members to Add: 1
    1. Add Member: 020202…0202
    Members to Remove: 0
  ```

  The action name carries **what is being changed**; `Authorized By` carries **who is signing**. The
  two are different lines because they are different roles. Both labels are frozen by contract
  (`roles.rs:42-45`, `updates.rs:76-79`): external signers hash the rendered payload.
- **No upstream end-to-end coverage.** `asm/tests/harness/admin.rs:251-277` supports
  `Role::StrataSecurityCouncil`, but no test in `asm/tests/asm/admin.rs` ever passes it — every
  multisig-update test uses `Role::StrataAdministrator`. Only the signing-message unit test exercises
  tx type 15. This is the gap [`security-council.md` §7.2](./security-council.md#72-coverage-upstream-does-not-have)
  names as one of the three highest-value tests for us to write.

---

## Constraints

### 1. Enactment reads two roles, not one

**Rule:** the config compared against the update (`keys`, `threshold`) is read from the **target**
role, derived from the action variant. The sequence number is read from the **authorizing** role,
derived from the proposal's authority. For every action shipped before this slice the two are the
same role; for tx type 15 they are not, and neither term may be derived from the other.

**Why:** `is_proposal_enacted_on_asm` resolves one role from the proposal's authority
(`orchestrator-be/src/infrastructure/asm_enactment.rs:183`) and reads all three terms from it
(`:192-199`). For a council rotation the proposal's authority is `strata_admin`, so it would compare
the update against the **administrator's** signer set — which the update never touches, so the
post-conditions can never be met and the proposal parks at Approved forever. Deriving the whole thing
from the target role instead is equally wrong in the other direction: `last_seqno` on the council is
advanced only by Defcon actions, so a rotation would look enacted the moment the council fired an
unrelated Defcon, or never at all.

Both terms wrong in opposite directions is the reason this is Constraint 1 rather than an
implementation note. The failure is silent in both directions — a proposal that reads the wrong state
does not error, it reports the wrong lifecycle.

**Implementation note:** `extract_multisig_config_update` (`asm_enactment.rs:303-356`) currently
matches on the pair `(authority, variant)` and encodes "authority == variant" as a data-integrity
error at `:327-337`. That rule stays true for the three self-rotating updates and stops being
universal. The target role belongs to the variant; nothing else about the match changes.

There is a second copy of this logic in `desktop-app/src-tauri/src/infrastructure/asm_enactment.rs`
and it has the same shape. Both move together or the desktop and the backend disagree about whether
a rotation enacted.

**Known divergence to fix while here:** `asm_enactment.rs:424-434` has a private `authority_to_role`
that maps only three authorities, while `asm_role_membership.rs:266-281` maps four. Two functions
with the same name and different answers is how the council reaches a wrong arm silently.

### 2. The target comes from the action, never from the session

**Rule:** which authority a multisig update modifies is decided by the action type the signer chose,
and travels in the action hex. It is never inferred from the session.

**Why:** today it is inferred. `use-create-proposal.ts:94` sends
`role: authorityFromRole(selectedRole) as 'strata_admin' | 'sequencer_manager' | 'alpen_admin'` — a
cast that is already untrue, because `selectedRole` can be `security_council`. Every authority
shipped so far rotates only itself, so the session happened to be the right answer. It stops being
the right answer here, and a cast is not a place to discover that.

The same assumption is written into a doc comment on the domain type
(`desktop-app/src-tauri/src/domain/action.rs:112-113`): *"Protocol rule: must equal the `Authority` of
the `Proposal` that carries this action (a role can only modify its own config)."* That sentence is
false against upstream for tx type 15, and it has to change with the code, not after it.
`MultisigUpdate.role` means **the authority being modified**; the authorizing role is upstream's to
derive, and it already does.

**Blast radius, accepted deliberately:** the wire type of the builder's `role`
(`desktop-app/src/api/action-builder.ts:6`) is a closed union of three values. Widening it is what
lets the fourth be expressed. Nothing about the three existing action types changes.

### 3. The form validates against the target's config, never the session's

**Rule:** for a council rotation, the current signer set and threshold that populate the form,
drive its validation, and render its Before/After preview are the **council's**, read live. The
session's own config never reaches this form.

**Why:** the create-proposal domain carries exactly one config, the session's — it reaches the schema
as `currentMultisigSigners` (`create-proposal.schema.ts:69-73`), the validator context as one field
(`validators/types.ts:6-10`), and the form and preview as `currentSigners`/`currentThreshold`. Two
of `signer-update.ts`'s rules read it to make a decision: *"Signer already exists in the current set"*
(`:98-111`) and `threshold <= resultingSignerCount` (`:113-157`). Against the wrong config both give
confidently wrong answers — a key that is already on the council is accepted, and a threshold that
will exceed the resulting council is allowed through to a hardware signer.

The read itself needs no new plumbing: `get_multisig_config(authority)`
(`desktop-app/src-tauri/src/commands/asm_state.rs:34-69`) already accepts any authority and already
maps `SecurityCouncil` to `Role::StrataSecurityCouncil`. What changes is which authority the create
flow asks about.

**Why this is not split into two commits:** an intermediate state that offers the menu entry while
still validating against the session's config is a form that lies to a signer about a signer set. See
the build plan's phase 3.

### 4. Acceptance is not application, and upstream does not say so

**Rule:** recorded, not repaired. A rotation that the chain accepts is not necessarily a rotation the
chain applies, and no surface may claim otherwise.

**Why:** `ThresholdConfig::validate_update` rejects five cases — `DuplicateAddMember`,
`DuplicateRemoveMember`, `MemberAlreadyExists`, `MemberNotFound`, and `InvalidThreshold` (threshold
greater than the resulting member count). All five are checked at **apply** time, and
`apply_multisig` (`asm/crates/subprotocols/admin/subprotocol/src/handler.rs:174-182`) logs the error
and returns. By then the signature check has passed, the sequence number has been consumed
(`handler.rs:114-119`), and the queue entry has been drained. A rotation can reach quorum, be
accepted on chain, sit through its whole confirmation depth, leave the queue — and change nothing.

**What saves us is already there:** `multisig_update_post_conditions_met`
(`asm_enactment.rs:358-390`) compares the actual keys and threshold rather than trusting the sequence
number, so a swallowed failure does **not** produce a false `Enacted`. The proposal resolves as
`Superseded` instead. The label is imprecise; the failure is not silent, and no signer is told a
rotation happened that did not.

**Known limit, recorded rather than solved:** this is not specific to the council. The three signer
updates shipped since V1 have carried it since V1, and the honest fix — a client-side pre-check
mirroring the five rules, plus a lifecycle state that says "accepted, changed nothing" — belongs to
whichever slice decides to own multisig-update correctness for every authority at once. Constraint 3
narrows the blast radius here for free: validating against the target's live config is exactly what
makes `MemberAlreadyExists`, `MemberNotFound` and `InvalidThreshold` unreachable in practice for a
form-built rotation.

### 5. A rotation can disable the emergency lever, and nothing on chain prevents it

**Rule:** the application states the consequence at the point of decision. It does not add a
protocol rule of its own to block it.

**Why:** `validate_update` prevents `threshold > members`. It does not prevent removing every current
council member and adding one key nobody holds, with threshold 1 — that update is valid, and it ends
the council's ability to sign a Defcon. The only safeguards are the ones the protocol actually
provides: the administrator's own quorum, and the cancel window this slice delivers.

Refusing such an update in the application would be re-implementing a protocol rule that does not
exist, which
[`AGENTS.md`](../../AGENTS.md) forbids and which would also be wrong — replacing a compromised
council is a legitimate reason to do exactly this. So the treatment is informational, and it belongs
in [Signer Safety](#signer-safety).

---

## State Model

The standard lifecycle, with no carve-out of any kind — this is an ordinary Strata Administrator
proposal that happens to name the council:

```
Pending ──→ Approved ──→ Awaiting enactment ──→ Enacted
   │            │                │
   │            ↓                ↓
   │        Canceled         Canceled
   ↓
Expired / Superseded
```

- `Approved` displays as **"Approved"**. The Defcon 1 carve-out is keyed on the action
  (`defcon_1`), so it does not reach here.
- The activation countdown shows, driven by the live
  `confirmation_depths.strata_security_council_multisig_update` —
  `showsActivationCountdown` already excludes only `defcon_1`.
- `Canceled` is reachable while the entry is queued, by a Strata Administrator quorum.
- `Superseded` applies with the standard ordering, and is also where a rotation lands that the chain
  accepted but did not apply ([Constraint 4](#4-acceptance-is-not-application-and-upstream-does-not-say-so)).

The proposal's `authority` is `strata_admin` throughout. Only Strata Administrator signers see it,
sign it, broadcast it and can cancel it.

---

## Backend Contract (orchestrator-be)

### Authorization

Unchanged and already correct. `require_authorized_for_action`
(`asm_role_membership.rs:243-264`) compares the session's role against upstream's
`tx_type.authorized_role()`, which for tx type 15 is `StrataAdministrator`. A council session
attempting to rotate itself is refused by that comparison with no council-specific code, which is
precisely the segregation invariant being enforced by the same generic mechanism as everything else.

`required_signatures` comes from `threshold_for_authority(auth.authority)`
(`handlers/proposals.rs:107-108`) — the **authorizing** authority's threshold. Correct here, and
worth naming as such: it is the threshold of who signs, never of who is modified.

### Lock period and activation height

Unchanged. `depth_for_action` (`asm_role_membership.rs:141-149`) resolves through
`update.update_tx_type()`, so tx type 15 flows to
`confirmation_depths.strata_security_council_multisig_update` with no new branch. This is what
[V1's Constraint 1](./security-council-defcon.md#1-lock-period-is-per-action-never-per-authority)
bought.

The depth is read live and never hardcoded, for the same reason V2 states it: a deployment that sets
it to `0` degrades correctly — no queue entry, no countdown, no cancel affordance, all three by
construction.

### Enactment detection

The one real change, per [Constraint 1](#1-enactment-reads-two-roles-not-one):

- the target role comes from the action variant;
- the authorizing role comes from the proposal's authority;
- `keys` and `threshold` are read from the target's `AuthorityConfig`;
- `last_seqno` is read from the authorizing role's;
- `multisig_update_post_conditions_met` is unchanged — it already takes those four terms as
  arguments and has no opinion about where they came from.

### Cancelability and cancel creation

Unchanged. Since V2 Phase 3 (#525) cancelability is derived by the backend from the live depth and
travels on the proposal DTO, so a council rotation is cancelable exactly when its depth is non-zero,
with no authority-shaped condition anywhere. `create_cancel_proposal` stores the cancel under the
target proposal's authority — `strata_admin` — and requires the session to match, which is the
correct rule here: the council has no veto over its own rotation, and must not appear to.

---

## Frontend Contract (desktop-app)

### The create menu

`council_signer_update` joins `ACTION_TYPES_BY_AUTHORITY.strata_admin`, after the administrator's own
`signer_update` so the default selection does not change. It appears for no other authority. Its
title names the target unambiguously — a Strata Administrator sees two signer-update entries and must
never have to infer which is which.

### The create form

The `signer_update` fields component is **reused, not duplicated**: the fields, the add/remove rows
and the threshold input are identical, and the only differences are which config populates them and
which copy frames them. Duplicating would fork the signing-message wiring, which is the
safety-critical half.

What changes is the source of truth: `getMultisigConfig('security_council')` rather than the
session's authority, threaded to the schema, the validator context, the form's threshold reset and
the preview ([Constraint 3](#3-the-form-validates-against-the-targets-config-never-the-sessions)).

The Before/After preview renders the **council's** current signers against the proposed set, and
the threshold line reads `"{current} of {before}"` → `"{new} of {after}"` over council counts.

### The signing message

Resolved from Rust through the same renderer the device signs over, exactly as every other action.
No TypeScript composes it. Unlike Defcon, this message has an `Action Details:` block, and that block
**is** the reviewable artifact: the signer is approving a specific list of keys.

### Lifecycle display

Nothing action-specific. The standard Approved label, the standard activation countdown, the standard
cancel affordance driven by the DTO field. This slice adds no display predicate; if it needs one,
that is a finding worth stopping for.

---

## Signer Safety

The signer here is a Strata Administrator, and what they are authorizing is who may pull the
emergency brake. That is a different shape of risk from Defcon — not irreversible, but invisible to
the people it affects.

1. **The rendered message is the reviewable artifact, and it has details.** It comes from the Rust
   renderer verbatim and is byte-identical to the hardware signer's screen. The form shows the same
   keys in Before/After form so a signer can check the message against something other than itself.
2. **The council cannot see its own rotation.** `list_proposals` is scoped strictly to the session's
   authority (`orchestrator-be/src/handlers/proposals.rs:157`), which PRD §5.1 requires and the
   segregation invariant intends. It follows that the council has neither visibility into nor a veto
   over who sits on it. The administrator's own quorum and the cancel window are the only checks, and
   the form says so where the decision is taken.
3. **A rotation can end the council's ability to sign a Defcon**, and the application does not
   prevent it ([Constraint 5](#5-a-rotation-can-disable-the-emergency-lever-and-nothing-on-chain-prevents-it)).
   Where the proposed set removes current members, the form states the consequence plainly rather
   than blocking it. It is not styled as an error: replacing a compromised council is a legitimate
   use of this action, and this repository reserves red for errors.
4. **Authority context is visible throughout.** The Strata Administrator badge from create through
   broadcast, and the target named on every surface that shows the action. "Signer update" alone is
   never sufficient copy on this action.
5. **The cancel window is stated where the decision is taken**, with the countdown driven by the live
   depth.

A non-administrator session can never reach this form, and the backend refuses the action
independently of the UI.

---

## Acceptance Criteria

### 1. A Strata Administrator can create a council signer update

**Given** an authenticated Strata Administrator session
**When** the signer opens the create-proposal flow
**Then** both the administrator's own signer update and the council signer update are offered as
separate entries, and selecting the council entry renders the signer-update fields.

### 1a. No other authority can reach it

**Given** an authenticated Sequencer Manager, Alpen Administrator or **Security Council** session
**When** the signer opens the create-proposal flow
**Then** the council signer update is not offered, and navigating directly to it is refused.

The council case is the one that matters: this is the action that rotates it, and it must not be
able to rotate itself.

### 2. Only a Strata Administrator session can create one

**Given** a session on any authority other than Strata Administrator
**When** it submits a proposal whose action hex decodes to
`UpdateAction::StrataSecurityCouncilMultisig`
**Then** the backend refuses it, naming the required role, and no proposal is persisted.

The refusal comes from `require_authorized_for_action` comparing against upstream's
`authorized_role()` (`updates.rs:64`) — not from a list this application maintains.

### 3. The form is populated and validated from the council's config

**Given** a Strata Administrator session whose own signer set differs from the council's
**When** the council signer update form loads
**Then** the current signers, the current threshold and the Before/After preview are the
**council's**, and a key already on the council is rejected as already present while the same key on
the administrator is not.

### 3a. The threshold rule counts council members

**Given** a proposed update that would leave the council with N members
**When** the signer enters a threshold greater than N
**Then** the form refuses it, counting the council's resulting members and not the administrator's.

### 3b. The update must be a real change

**Given** a council signer update whose add and remove sets are both empty once blank rows are
discarded, and whose threshold equals the council's current threshold
**When** the signer reaches the sign step
**Then** it is refused as producing no change.

Today's `signer_update` validator requires one *row* in each of add and remove
(`validators/signer-update.ts:6-11`), but blank rows are discarded downstream
(`use-create-proposal.ts:95-96`), so an update that changes nothing is currently buildable. It would
be accepted on chain, consume a sequence number, and apply a no-op. A threshold-only change is a real
change and stays allowed.

### 4. The signing message is upstream's, with its details block

**Given** a council signer update carrying one added member and threshold 2, at sequence 7
**When** the signing message is rendered
**Then** it reads exactly the nine lines upstream pins in
`strata_security_council_multisig.rs:52-75`, with `Action: Strata Security Council Multisig Update`
and `Authorized By: Strata Administrator` on separate lines, and it is byte-identical to what the
hardware signer displays.

The two lines naming two different roles are the wire-level expression of the segregation invariant.
A change that collapsed them would be an upstream break, and is worth a tripwire rather than trust.

### 5. The action is distinguishable from an administrator signer update

**Given** a persisted proposal carrying a council rotation
**When** its action hex is decoded for display
**Then** it reports as a council signer update, not as a generic `multisig_update`, everywhere the
action type is shown — list, detail, sign view and manual bundle.

### 6. It is queued, not enacted, on broadcast

**Given** a broadcast council signer update at a non-zero depth
**When** the reveal confirms
**Then** the update is in the admin queue, the council's config is unchanged, and the proposal shows
Awaiting enactment with a countdown to `reveal_block + depth`.

### 7. Enactment compares the council's config against the administrator's sequence number

**Given** a queued council signer update that reached its activation height
**When** enactment is evaluated
**Then** the added keys are present in `state.authority(Role::StrataSecurityCouncil).config()`, the
removed keys are absent, the threshold matches, and the sequence-number term is read from
`state.authority(Role::StrataAdministrator).last_seqno()` — and the proposal shows Enacted
([Constraint 1](#1-enactment-reads-two-roles-not-one)).

### 7a. Neither role is substituted for the other

**Given** the same queued update
**When** the administrator's signer set changes for an unrelated reason, or the council's
`last_seqno` advances because the council submitted a Defcon
**Then** neither event changes the enactment answer.

This is the test that fails if a future refactor collapses the two roles back into one, which is the
shape the code had before this slice.

### 7b. The new council can act and the removed signers cannot

**Given** an enacted council signer update that removed one member and added another
**When** a Defcon action is submitted afterwards
**Then** it verifies against the new council: a quorum drawn from the new set is accepted, and a
signature from a removed member no longer counts toward it.

This is the case [`security-council.md` §7.2](./security-council.md#72-coverage-upstream-does-not-have)
names as untested anywhere, upstream included — the rotation is only meaningful if it changes who can
pull the emergency lever.

### 8. A cancelled rotation never applies

**Given** a queued council signer update
**When** a Strata Administrator quorum cancels it and its original activation height passes
**Then** the queue is empty, the council's config is unchanged, the proposal reads `Canceled`, and
nothing reads `Enacted`.

### 9. The cancel is signed by the Strata Administrator

**Given** a queued council signer update
**When** the cancel is created
**Then** it is stored under the `strata_admin` authority and requires a Strata Administrator session
— and a Security Council session is refused.

### 10. The council never sees the proposal that rotates it

**Given** a pending, approved or enacted council signer update
**When** a Security Council session lists proposals
**Then** it appears in no list and is not reachable by direct navigation.

Asserted rather than assumed: the absence is a PRD requirement, not an accident of scoping.

### 11. A rotation that removes current members states the consequence

**Given** a proposed update that removes one or more current council members
**When** the signer reaches the confirmation step
**Then** the consequence is stated plainly, not styled as an error, and the action is not blocked
([Constraint 5](#5-a-rotation-can-disable-the-emergency-lever-and-nothing-on-chain-prevents-it)).

### 12. The depth is the live one

**Given** a deployment whose `confirmation_depths.strata_security_council_multisig_update` differs
from every other depth
**When** the countdown and the cancel affordance are resolved
**Then** both use that value, and no constant stands in for it anywhere.

### 13. The manual fallback works

**Given** a council signer update with a quorum of collected signatures
**When** the signer exports the bundle
**Then** it broadcasts through the existing manual route and through an external Bitcoin RPC, with
no council-specific handling.

---

## Edge Cases

| Scenario | Behavior |
|---|---|
| `confirmation_depths.strata_security_council_multisig_update` is `0` | Supported degradation. Applied in the submission block: no queue entry, no countdown, no cancel affordance — all three by construction, since each reads the resolved depth. |
| The chain accepts the rotation but `validate_update` rejects it at apply time | The seqno is consumed and the queue entry drained, but the council's config is unchanged, so post-conditions are not met and the proposal resolves as `Superseded`, never `Enacted`. The label is imprecise; no signer is told a rotation happened that did not. See [Constraint 4](#4-acceptance-is-not-application-and-upstream-does-not-say-so). |
| The rotation removes every current council member | Valid on chain and allowed by the application. The council loses its ability to sign a Defcon. Stated at the confirmation step, not blocked — see [Constraint 5](#5-a-rotation-can-disable-the-emergency-lever-and-nothing-on-chain-prevents-it). |
| A Defcon 3 is queued while a council rotation is queued | Independent. The Defcon 3 was validated against the council at acceptance and stands; a *cancel* of it after the rotation enacts would need a quorum of the **new** council. Already recorded in V2's edge cases, restated here because this is the slice that can cause it. |
| A council rotation and an administrator signer update are both queued | Independent entries with different action bytes and different target roles. Both consume the administrator's seqno, so the standard ordering applies and the second must carry the later sequence number. |
| The ASM cannot answer while the create form is open | The council's config is unavailable, so the form cannot validate against it. Unlike the Defcon safe-harbour note — which is informational and degrades to `false` — this read is load-bearing: without it the form would validate against nothing. The form must not offer a threshold or an add/remove decision it cannot check. |
| The administrator rotates the council while a Defcon 3 cancel is being collected offchain | The collected signatures are the old council's and will not verify. Out of band for this slice; recorded because it is the concrete shape of "the council loses its signing ability", which `security-council.md` §7.2 names as untested upstream. |
| A council rotation expires before quorum | The standard 7-day pending window. No carve-out. |

---

## Test Plan

The rules are the repository's, not this slice's: behaviour that can regress, tested where it lives;
nothing that pins a mock, a language guarantee, or a phrasing.

**Backend unit (`orchestrator-be`)** — the two-role enactment, as a truth table with
[AC 7a](#7a-neither-role-is-substituted-for-the-other) as two named tests: one where the
administrator's config changes and the answer must not, one where the council's `last_seqno` advances
and the answer must not. A test whose name says which role each term came from is the documentation.
Plus the authorization refusal of [AC 2](#2-only-a-strata-administrator-session-can-create-one), and
the depth resolving to `strata_security_council_multisig_update` through the existing closure seam
with no ASM.

**Tauri unit (`src-tauri`)** — codec round-trip both directions with a tripwire that the variant
still encodes `UpdateTxType::StrataSecurityCouncilMultisigUpdate`; `action_type_from_hex` naming the
council rotation distinctly from a generic multisig update
([AC 5](#5-the-action-is-distinguishable-from-an-administrator-signer-update)); the action builder;
and one tripwire that the rendered signing message is non-empty, contains both `Strata Security
Council Multisig Update` and `Authorized By: Strata Administrator`, and **differs** from an
administrator signer update's — so an upstream change that collapsed the two roles is caught here
rather than on a signer's screen.

**Frontend** — pure functions only: the per-authority action menu and its default; the validator
answering against a supplied signer set rather than an ambient one, with a fixture where the two sets
disagree ([AC 3](#3-the-form-is-populated-and-validated-from-the-councils-config),
[AC 3a](#3a-the-threshold-rule-counts-council-members)); and the IPC schema contract tests, which fail
when the Zod schema and the Rust DTO diverge.

**E2E (`e2e-tests`)** — a new file, `e2e_council_rotation.rs`, not an addition to
`e2e_defcon_probe.rs`. Submit an administrator-signed council rotation, assert it is queued with the
council's config unchanged, mine exactly `depth` blocks, and assert the council's config changed and
the administrator's `last_seqno` advanced. Then the cancelled path, following the shape of
`e2e_cancel_proposal.rs`: cancel inside the window, mine `depth`, assert the queue is empty **and the
council's config is still unchanged**. Together these are the only automated coverage of
[Constraint 1](#1-enactment-reads-two-roles-not-one) against a real chain, and they are the
end-to-end test upstream does not have.

The local stack supports this today: `scripts/asm-params.json` carries a four-key council at
threshold 2 and a depth of 30 for tx type 15.

**Not tested, deliberately:** no DOM or component tests — this repository has no DOM runner, and a
test that reads a component's source with `readFileSync` pins a phrasing rather than a behaviour. The
gap is closed by a manual walk, which in V1 is what found what reviewing did not. No ASM-backed
integration test inside `orchestrator-be`: it would be the flakiest test in the repository and would
re-prove what the e2e proves. No test that the chain swallows a failed `validate_update`
([Constraint 4](#4-acceptance-is-not-application-and-upstream-does-not-say-so)) — that is upstream
behaviour this slice records rather than depends on.

---

## Verification

Code review checks that:

- [ ] The target role is derived from the action variant, and the authorizing role from the
      proposal's authority — in both copies of the enactment logic.
- [ ] No constant stands in for `confirmation_depths.strata_security_council_multisig_update`.
- [ ] No `as` cast decides which authority a multisig update targets.
- [ ] The doc comment on `MultisigUpdate.role` no longer claims a role can only modify its own
      config.
- [ ] Only one `authority_to_role` answers for the backend, and it maps four authorities.
- [ ] The signing message is rendered by the Rust renderer, never composed in TypeScript.
- [ ] No surface says a rotation was enacted on the strength of a consumed sequence number alone.
- [ ] Every new frontend test file falls inside CI's `src/**/*.test.ts(x)` glob.

Post-merge validation on regtest, with the local stack
(`./scripts/local-stack.sh --clean` if any state predates the ASM pin bump):

1. A Strata Administrator signer sees two signer-update entries and reaches the council one; a
   Security Council signer sees neither.
2. The form shows the council's four current signers and threshold 2 — not the administrator's.
3. The rendered message matches the signer's screen, names both roles on separate lines, and carries
   the details block.
4. Quorum, broadcast — Approved, then Awaiting enactment with a countdown to `reveal + 30`, and the
   council's config unchanged.
5. Path A: mine 30 blocks → `Enacted`, and `strata_asm_getAnchorState` shows the council's new
   config.
6. Path B: cancel inside the window → the target reads `Canceled`, the council's config is unchanged,
   and nothing reads `Enacted`.
7. A Security Council session sees none of the above at any point.
8. `cargo test -p alpen-multisig-e2e-tests` green, including `e2e_council_rotation`.
