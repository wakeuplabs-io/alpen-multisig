# Security Council — Defcon 1 (V1) Implementation Plan

**Functional contract:** [`security-council-defcon.md`](./security-council-defcon.md) — the SSOT for *what*
V1 must do. This document is only *how* it gets built, and never overrides it.

**Master plan:** [`security-council.md`](./security-council.md) §6 Stage board, §7 Slice board.

**Story:** [`story-map.md`](../3-stories/story-map.md) US-E12.

A phase marked ✅ here means the engineering step shipped, not that every acceptance criterion in the
contract is satisfied — the contract's `## Acceptance Criteria` section stays the measure.

## 1. Purpose and scope

V1 is the first slice of the Security Council feature and it carries the **shared spine** every later
slice reuses: the authority→role mapping, per-action lock-period resolution, enactment detection, the
action codec and builder, and the signer-safety UX. Delivered in one piece it would be a large change
containing a refactor of a contract shared with every other authority.

This plan breaks it into six phases, one PR each, ordered so that **the two shared-contract refactors
land first**, before any Defcon product flow exists. A seventh was added after the flow was run by
hand — see Phase 7 for what running it exposed that reviewing it did not. The phases are **sequential, not parallel** —
each assumes the ones before it have merged, and Phase 2 in particular cannot compile without
Phase 1's resolution function. Each phase leaves the tree green and carries the tests that prove it.

**In scope**

- Defcon 1 (`UpdateTxType::Defcon1 = 41`) end to end, for the Strata Security Council authority.
- The two shared refactors V1 must perform: lock period resolved per action, and cancelability gated
  by confirmation depth rather than by an authority allow-list.

**Not in scope**

- Defcon 3 (V2) and its cancel flow (V5). They appear here only as the reason the two refactors are
  shaped the way they are. `lock_period_for_action` gains its Defcon 3 arm in this slice because the
  function cannot be written correctly without it, but no Defcon 3 product flow ships.
- Security Council signer update (V3) and Safe Harbour address update (V4) — both authorized by the
  Strata Administrator.
- Any protocol validity rule. The orchestrator stays coordination-only.

## 2. Traceability

| Phase | Name | Closes (contract) | Touches |
|---|---|---|---|
| 1 | Per-action lock period | AC 12, AC 12a; [Constraint 1](./security-council-defcon.md#1-lock-period-is-per-action-never-per-authority) | `orchestrator-be` |
| 2 | Cancel gate by depth | AC 11; [Constraint 2](./security-council-defcon.md#2-cancelability-is-decided-per-action-and-per-live-depth-never-by-authoritysecuritycouncil) | `orchestrator-be` |
| 3 | Backend Defcon 1 — role, codec, creation | AC 2, AC 3, AC 17 | `orchestrator-be`, `src-tauri` |
| 4 | Enactment detection | AC 8 | `orchestrator-be` |
| 5 | Frontend — create and sign | AC 1, AC 1a, AC 4, AC 5, AC 14 | `desktop-app`, `src-tauri` |
| 6 | Frontend — lifecycle | AC 6, AC 7, AC 9, AC 10, AC 13, AC 15/15a/15b, AC 16 | `desktop-app` |
| 7 | Safe harbour visible, enactment per proposal | AC 8 (tightened), AC 18, AC 19, AC 20 | `orchestrator-be`, `desktop-app`, `src-tauri` |

## 3. Architecture

### What already exists and is reused

| Piece | Location | Why it matters |
|---|---|---|
| `Authority::SecurityCouncil` | `orchestrator-be/src/domain/authority.rs` | The variant exists; only its role mapping is missing |
| `decode_multisig_action_hex` | `orchestrator-be/src/infrastructure/action_codec.rs` | Turns a stored `action_hex` back into a `MultisigAction` |
| `is_proposal_enacted_on_asm` | `orchestrator-be/src/infrastructure/asm_enactment.rs` | Already takes `action_hex` and dispatches on the action variant |
| `update_id_in_queue_for_action` | `orchestrator-be/src/infrastructure/asm_role_membership.rs` | Decodes an action and scans the live ASM queue |
| `mock_lock_period`, `mock_is_enacted` | same modules | Let the phases be tested without a live ASM |
| `ACTION_TYPES_BY_AUTHORITY` | `desktop-app/src/domain/create-proposal/model/action-type-config.ts` | Maps an authority to its action types; the extension point for the form |
| `build_*_hex` per action | `desktop-app/src-tauri/src/commands/action_builder.rs` | One builder per action type; Defcon 1 adds one more |
| Manual broadcast path | `/manual` route, `proposals_broadcast_manual` IPC | The fallback the contract requires is already built |

### Where Defcon 1 lives in the frontend

The contract deliberately left this open. It is settled here: **Defcon 1 extends
`desktop-app/src/domain/create-proposal/`**, it does not get a domain of its own. That domain already
dispatches by action type — `ACTION_TYPES_BY_AUTHORITY` plus one `*-form-fields.tsx` component per
type — so Defcon 1 is one more entry (`security_council: ['defcon_1']`) and one more fields
component. The destructive treatment and the type-to-confirm gate live inside that component. A
separate domain would duplicate the creation flow to change its styling.

### The two breaking points

Both are shared contracts, and both are why this slice is ordered the way it is.

**1. Lock period is resolved from the authority.** `authority_to_update_tx_type` maps one authority to
one `UpdateTxType`, and `lock_period_for_authority` builds on it. The Security Council has **two** tx
types with opposite depths — Defcon 1 fixed at `0` with no configuration field upstream, Defcon 3
per-deployment — so no authority-keyed lookup can answer for it. Resolution has to key off the action.

**2. Cancelability is gated by an authority allow-list.** `create_cancel_proposal` rejects anything
outside `AlpenAdmin | StrataAdmin`. No allow-list of authorities can separate Defcon 1 from Defcon 3,
because they share one authority and have opposite answers. The question that does separate them is
the action's confirmation depth: zero means never enqueued, so an on-chain cancel would fail with
`UnknownAction`.

Neither refactor needs the Defcon **product flow** to exist — no proposal is created, no screen is
built, nothing is broadcast. But both need the Defcon **action variants to be resolvable**, and this
is worth being precise about, because it is what makes Phases 1 and 2 testable at all:

- The test that distinguishes per-action resolution from a per-authority mapping needs two actions on
  **one** authority resolving to **different** depths. No current authority offers that pair; only the
  Security Council does.
- The depth-zero rejection in Phase 2 needs an action whose depth is actually zero. Only Defcon 1 is.

So **Phase 1 teaches the resolver about both Defcon variants** — Defcon 1 is a hardcoded `0`, Defcon 3
reads live state — while the codec, the action builder and proposal creation stay in Phase 3. The
resolver knowing an action exists is not the same as the product being able to create it.

## 4. Phased plan

Every phase: its own branch off current `develop`, one atomic commit (never a commit that repairs the
one before it), and the full [`AGENTS.md`](../../AGENTS.md) pre-commit CI checklist green before
pushing.

### Phase 1 — Per-action lock period ✅

Replace `lock_period_for_authority` with a resolution keyed on the action. Defcon 1 resolves to `0` —
upstream has no `ConfirmationDepths` field for it — and Defcon 3 reads `confirmation_depths.defcon3`
from live ASM state. The value is read on every call, never cached at startup.

**Shipped as `lock_period_for_action`. Detail spec:**
[`security-council-defcon-phase-1.md`](./security-council-defcon-phase-1.md), which supersedes the
bullets below where they differ. Three things it settled that this plan had wrong:

- **The resolution composes upstream's table rather than restating it.** `UpdateAction::update_tx_type()`
  and `ConfirmationDepths::get()` already map every variant and already hardcode Defcon 1 to `0`, so
  writing our own per-variant match would have been a second copy of a table we do not own. Same
  observable behaviour; the AC 12 test asserts upstream's hardcoding directly as a tripwire.
- **It is a bug fix, not a no-op.** The per-authority mapping was already wrong for actions in
  production — a Strata-admin VK update resolved to the authority's multisig depth and now resolves to
  `ol_stf_vk_update`. The regression test therefore pins the *corrected* mapping; phrased as "the same
  depths as before" it would have pinned the bug.
- **AC 12a is structural evidence, not a unit test.** The only place a cache could live is the RPC
  layer a unit test would stub out. There is also no repeated resolution cycle in the code:
  `compute_and_store_activation_height` runs once per proposal at reveal confirmation, not at
  enactment detection as the contract's wording suggests.

- **Call site:** `compute_and_store_activation_height` in `orchestrator-be/src/application/proposals.rs`
  already holds the whole `Proposal`, so it has `action_hex` on hand. The migration is local.
- **Expand/contract:** add the new resolution, migrate the call site, then retire the old function.
  Not one large commit that leaves the tree broken midway.
- **Shape:** the resolution lives beside its neighbours in `asm_role_membership.rs` and follows their
  convention — an `async fn` taking the ASM RPC URL as `&str`, like `threshold_for_authority` and
  `update_id_in_queue_for_action` — and resolves the action from the stored `action_hex` through
  `decode_multisig_action_hex`.

### Phase 2 — Cancel gate by depth ✅

Replace the authority allow-list in `create_cancel_proposal` with the action's confirmation depth:
reject when the depth is zero, and say so in the rejection.

**Detail spec:** [`security-council-defcon-phase-2.md`](./security-council-defcon-phase-2.md), which
supersedes the bullets below where they differ. Three things it settled that this plan left open:

- **"The action's" depth is the *target's*, never the cancel's.** Every `MultisigAction::Cancel`
  resolves to `0` (Phase 1), so resolving the `action_hex` this function already receives — the
  cancel's own — would compile, read plausibly, and reject every cancel in the system.
- **The Sequencer Manager becomes cancellable.** This plan required only that the Alpen and Strata
  administrators not change; depth-only gating also opens the Sequencer Manager, whose updates are
  enqueued with a configurable depth. Knowingly accepted, and invisible to users because the desktop
  hides the affordance behind its own authority allow-list — which V5 owns replacing.
- **The rewritten test needed a fixture the plan did not anticipate.** `ACTION_HEX = "deadbeef"` is
  not valid SSZ. Decoding the target was optional under the old gate and is mandatory under this one,
  so the cancel tests move to the valid fixture the handler tests already use.

- **Test to rewrite, not delete:** `test_create_cancel_proposal_rejects_unsupported_authority` in
  `orchestrator-be/src/application/proposals.rs` encodes the old gate. It becomes a depth-based
  rejection test. Existing cancel behaviour for the Alpen and Strata administrators must not change.

### Phase 3 — Backend Defcon 1: role, codec, creation ✅

**Detail spec:** [`security-council-defcon-phase-3.md`](./security-council-defcon-phase-3.md), which
supersedes the bullets below where they differ. Four things it settled that this plan left open or
had wrong:

- **The builder takes no sequence number.** `seq_no` is a field of the creation request everywhere
  else in this codebase, not part of the action; folding it into the builder would give Defcon 1 a
  shape no other builder has.
- **The authorization gate is not council-shaped.** It compares the action's upstream
  `authorized_role()` with the session's authority, so it closes the same hole for all five
  authorities — a Strata admin session posting an Alpen admin action is refused too, which today it
  is not.
- **Two IPC boundaries stay on `Unknown` until Phase 5, and moving either earlier would be a
  regression**, not just premature: both TypeScript counterparts are closed schemas, so an
  unregistered value is a parse error rather than an unknown-action fallback. Besides
  `decode_action_hex` there is `actionType` on the proposal DTO, where the failure takes down the
  parse of the whole proposal list. Phase 5 registers both.
- **The duplicate rule follows the PRD, not AC 3's old wording.** PRD 02 §3.4.1 requires a
  duplicate creation to be *rejected*; the contract said "returns the existing proposal", against
  its own title and the story map. AC 3 was corrected. The `409` stays and gains the existing
  `ActionId` in its message. No `create_defcon_proposal` is written.

- Map `Authority::SecurityCouncil` to `Role::StrataSecurityCouncil` in `authority_to_role_impl`, which
  currently falls through to an error for it.
- Add the Defcon 1 action to the codec and to the Tauri action builder. `Defcon1Update` is a
  payload-less unit struct, so the builder takes only the sequence number.
- Proposal creation with a stable `ActionId` — a duplicate `(action, seq_no)` is rejected naming the
  existing `ActionId` and mutates nothing. `seq_no` is a non-negative integer and **may repeat**
  across distinct proposals.
- **The backend authorization gate**, which the contract requires under Backend Contract → Authorization
  Gate: a non-council session is refused at the `POST /proposals` handler before any proposal object
  is created. This is the server-side half of council-only access; AC 1 and AC 1a cover only what the
  UI renders, so without this the requirement would ship untested. It is pinned by AC 17, added to the
  contract alongside this plan.

### Phase 4 — Enactment detection ✅

Add the Defcon 1 arm to the action dispatch in `orchestrator-be/src/infrastructure/asm_enactment.rs`.
Enacted requires **both** post-conditions: `safe_harbour().is_activated()` is true, **and** no Defcon 1
entry sits in the ASM admin queue.

**Detail spec:** [`security-council-defcon-phase-4.md`](./security-council-defcon-phase-4.md),
which supersedes this section where they differ. Two things it settled that this plan had wrong or
left open:

- **The queue check is a tripwire against upstream drift, not a disambiguator.** This plan said the
  check exists because the activation flag "cannot distinguish a Defcon 1 from a Defcon 3 that has
  matured". The premise is right — upstream routes both variants to the same `relay_bridge_defcon`
  — but the conclusion is not: a matured Defcon 3 has already left the queue, so no queue check
  sees it. What the check does buy is protection against upstream ever giving Defcon 1 a non-zero
  depth, which would start enqueuing it; without the check a proposal would be marked `Enacted`
  while its update was still pending and still cancellable.
- **The post-condition reads a value, and the reveal-block ordering is never checked.** The
  contract requires a Defcon 1 to enact even when the safe harbour was already activated
  beforehand, so no transition-watching design is admissible. It also asks for the activation to be
  observed at or after the reveal block — and no height comparison exists anywhere in this module,
  for any action. The phase records that as an explicit V1 constraint rather than building a
  per-block read that nothing in V1 can exercise; it becomes reachable only once Defcon 3 gains a
  product flow.

### Phase 5 — Frontend: create and sign ✅

**Detail spec:** [`security-council-defcon-phase-5.md`](./security-council-defcon-phase-5.md), which
supersedes the bullets below where they differ. Four things it settled that this plan left out or
had wrong:

- **The desktop app had no Security Council session at all**, and this plan's two bullets assume
  one. Five places encoded three-or-four authorities and none was the council — two of them
  `default:` arms that substitute silently rather than fail, so a council role added to TypeScript
  without touching `authorityFromRole` would have produced a session that authenticates, works, and
  is the wrong authority. A sixth site turned up only in review: `get_multisig_config` refused the
  council through a catch-all `_` arm, which is why adding the variant compiled cleanly.
- **The commit order is the reverse of the obvious one.** Session first reads as the prerequisite
  but is the broken half: `getActionTypeOptions` falls back to the Strata Administrator's menu for
  an unregistered authority, so a council session that existed before `security_council: ['defcon_1']`
  did would be offered three actions its role cannot authorize. The menu entry lands first, and both
  commits are then correct in isolation.
- **The four-line message needs no new code and must not get any.** Upstream's
  `SigningMessage::for_action` already appends `Action Details:` only when `render_details` produced
  lines, and Defcon 1's is empty — so AC 4 is a *placement* requirement, not a formatting one. The
  form resolves the message through the same Rust renderer the device signs over and prints it
  verbatim; a hand-written template would review identically and diverge the moment upstream bumps a
  version. The resolved message is mirrored into a form value, so a failed resolve disables the CTA
  instead of letting a signer confirm four lines they never saw.
- **No `/proposals/create/defcon-1` route is built.** This plan §3 already said Defcon 1 extends
  `create-proposal`; the contract still described a dedicated route and screen, which it had marked
  as Stage 5's call. AC 1a holds either way — the route is never registered, so `App.tsx`'s
  catch-all redirects every session, which is the behaviour AC 1a describes. The contract's
  *Create Form Layout* and *Critical Files* were corrected alongside this phase.

### Phase 6 — Frontend: lifecycle ✅

The display carve-out and the states after quorum: never the word "Approved" for Defcon 1, no cancel
affordance in any state or view, the "Send" control once quorum is reached, enacted and expired
proposals in the "Past" list, and the manual fallback reachable through the existing `/manual` route.

**Detail spec:** [`security-council-defcon-phase-6.md`](./security-council-defcon-phase-6.md), which
supersedes the bullets above where they differ. Four things it settled that this plan left open or
had wrong:

- **Half of the phase's eight criteria need no code, and saying so with evidence is the work.**
  The lifecycle screens are action-agnostic, so AC 6, 7, 13 and 16 hold for Defcon 1 the moment
  Phase 5 lets one exist. Writing Defcon-shaped copies of correct code is what Phase 5 avoided by
  refusing a sibling create screen; the phase's §3 is a `file:line` audit instead.
- **The badge reads "Quorum reached", not the contract's literal string.** The app's verb has been
  *Send* since #432 and a 34-character badge reflows the card header. Constraint 3 grants the
  latitude; AC 9, the State Model row and the wireframe are corrected together, since correcting one
  would have left the contract contradicting itself twice.
- **The cancel gate had three copies, not one.** The dashboard card, the detail screen's *Cancel
  this proposal* button and the cancel route's redirect guard each declared the authority allow-list
  Constraint 2 forbids. AC 10 held only because the council was absent from all three. The V5 that
  adds `security_council` for Defcon 3 would have opened all three at once.
- **Three of the seven defects it found are not Defcon-specific.** The offline route rejected the
  app's own clipboard export, discarded the raw transaction hex it exists to hand over (AC 15b, which
  the phase's first audit wrongly marked as already satisfied — the only row it stated without
  evidence), and labelled every imported action by guessing at a hex prefix. In scope because
  AC 15/15a/15b are this phase's to close and that path is the fallback the slice leans on for an
  irreversible action.

**Known divergence:** the contract's *Lifecycle Display* wireframe draws an enacted proposal with a
block number and a `Safe harbour activated: ✓` line. Neither is rendered and neither is required by
an acceptance criterion; the safe-harbour read does not exist in the desktop until Phase 7, which
spends it on the dashboard banner and the create-form warning.

### Phase 7 — The safe harbour is visible, and enactment is per proposal ✅

**Detail spec:** [`security-council-defcon-phase-7.md`](./security-council-defcon-phase-7.md).

Not in this plan's original six. It exists because running the finished flow produced a second
Defcon 1 on a chain whose safe harbour was already active, and neither half of the system
distinguished it: the enactment predicate reduces to "the safe harbour is active" on any honest
chain, so the second proposal read as `Enacted` on the strength of the first one's activation; and
the app that creates the state cannot read it, so the council had nothing on screen saying the
bridge was already in safe harbour.

The predicate gains the sequence-number term every other action's post-condition already carries,
which makes the answer per-proposal. The desktop gains a safe-harbour read, a note on the council
dashboard and the same note on the Defcon 1 form — a warning, never a block: refusing the emergency
lever on the strength of a state read is the worse failure.

Adds AC 18–20 to the contract and tightens the Edge Case that was read as licence for the predicate.
Four things the detail spec settled that it had itself got wrong, all found by reading the code
before writing any:

- **Its test plan was written against infrastructure this repository does not have.** Two of the
  three tests it named needed a DOM runner (there is none — "component tests" here read source text
  with `readFileSync`) and the third needed an `AnchorState` fixture that exists nowhere in
  `src-tauri`. The Rust read composes three functions `fetch_current_operators` already exercises,
  so the honest coverage is the predicate's unit tests plus the manual run; §6.2 now says that with
  evidence instead of listing tests nobody could write.
- **The read carries `activated` and nothing else.** It first returned the safe-harbour address
  too, which §8 puts out of scope and which would have added a `bitcoin_bosd` dependency to the
  desktop for a field no caller has.
- **The note is amber, not the `Irreversible` callout's red**, which the spec first asked it to
  reuse. Two red blocks in one column leave the signer with two alarms of equal weight, one of them
  merely a fact about the chain.
- **"Two commits" was five.** The frontend half spans two languages and four layers; the read is
  reviewable before either surface consumes it, and the banner and the form warning are one shared
  note component rather than two blocks that drift.

**Known consequence, recorded in the detail spec §3.3:** with the seqno term,
`report_broadcast_progress` returns a conflict in the window between a reveal confirming and the
ASM consuming the seqno, where before it silently succeeded. That is the behaviour every other
action already has, and the conflict is true where the silent success was not.

## 5. Verification

Per phase: the `AGENTS.md` checklist, plus evidence that the acceptance criteria the phase claims to
close are actually covered by tests.

End to end, once all six land — on regtest with the local stack (`./scripts/local-stack.sh --clean`
if any state predates the ASM pin bump):

1. A council signer reaches the Defcon 1 form; every other authority sees no entry point to it.
2. The rendered four-line signing message matches what the hardware signer displays, with no
   `Action Details:` block.
3. Quorum, broadcast, then `safe_harbour().is_activated()` true in the reveal block with the admin
   queue empty.
4. No cancel affordance and no "Approved" label in any state.
5. `e2e-tests/tests/e2e_defcon_probe.rs` still passes — it is the upstream capability evidence this
   slice is built on.
