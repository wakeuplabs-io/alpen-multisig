# Security Council — Defcon 1 (V1), Phase 7: The safe harbour is visible, and enactment is per proposal

**Functional contract:** [`security-council-defcon.md`](./security-council-defcon.md) — SSOT for
*what* V1 must do. This document never overrides it; §10 records the two places it corrects.

**Build plan:** [`security-council-defcon-implementation.md`](./security-council-defcon-implementation.md)
§4. This phase is **not** in that plan — §1 says why it exists.

**Closes:** AC 8 (tightened), and AC 18, AC 19, AC 20 added by this phase (§10).

**Order:** after Phase 6. Nothing here compiles against Phase 6, but both edit the proposals
dashboard, and Phase 6 owns that screen's labels and lists first.

## 1. Why there is a seventh phase

Manual testing of the finished Defcon 1 flow produced two Defcon 1 proposals on one regtest chain.
Both are correct — the second carries `seq_no = 2`, so the duplicate rule (AC 3, keyed on
`(action, seq_no)`) does not apply to it, upstream accepts it, and the contract's own Edge Cases
already say a Defcon 1 broadcast against an activated safe harbour "still reaches `enacted` status
correctly" (`security-council-defcon.md:464`).

What the run exposed is not that the second proposal was possible. It is that **nothing in the
system distinguishes it**:

- The dashboard showed both as *Enacted*, and the second one's *Enacted* is not evidence that its
  transaction did anything (§3).
- The council had no way to see, anywhere in the app, that the bridge was already in safe harbour
  (§4). `grep -rn "safe_harbour" desktop-app/` returns nothing: the app can create the state and
  cannot read it.

The first is a defect. The second is a gap the contract never asked for, and the reason the first
one is not academic — a signer who cannot see the state has no way to notice that the flow told
them something untrue about it.

## 2. What this phase is not

It is not a block on creating a second Defcon 1. Defcon 1 is the emergency lever; refusing it on
the strength of a state read that may be stale, or served by a node that is behind, is a worse
failure than allowing a redundant one. Everything below **warns**, and the type-to-confirm gate
that already exists is what the warning is attached to.

It is not Defcon 3, and it is not the safe-harbour address update (V2/V4). It adds no new
lifecycle state: the states are the four the contract already names.

## 3. The defect — the Defcon 1 enactment predicate collapses

`defcon1_enacted` (`orchestrator-be/src/infrastructure/asm_enactment.rs:175`) is:

```rust
fn defcon1_enacted(safe_harbour_activated: bool, defcon1_queued: bool) -> bool {
    safe_harbour_activated && !defcon1_queued
}
```

Phase 4 §3.1 established, correctly, that the queue term is a tripwire against upstream drift
rather than a discriminator: Defcon 1 executes at depth 0 and never enters the queue, so
`defcon1_queued` is `false` on every honest chain. **On every honest chain the predicate therefore
reduces to `safe_harbour_activated`** — a fact about the bridge, not about this proposal.

`safe_harbour.is_activated()` is never reset to `false` (the contract says so at `:296`). So once
any Defcon 1 has enacted, the predicate answers `true` for every Defcon 1 proposal that reaches it,
forever. The guards in `reconcile_enacted_for_authority`
(`orchestrator-be/src/application/proposals.rs:359-363`) mean a proposal must at least have a
confirmed reveal — but a confirmed reveal proves the transaction was mined, not that the ASM
accepted it. A second Defcon 1 whose signature set the ASM rejected is marked `Enacted` on the
strength of the first one's activation.

### 3.1 The missing term already exists, and every other action uses it

Upstream advances the role's sequence number when it applies an action — every action, Defcon 1
included: `handle_action` ends with `authority.update_last_seqno(seqno_token)`
(`asm/crates/subprotocols/admin/subprotocol/src/handler.rs:118`), after the depth-0 immediate-apply
branch has already run. A rejected action never reaches that line.

The multisig-update arm of this same function already reads it — `multisig_update_post_conditions_met`
opens with `if last_seqno < seq_no { return false }` (`asm_enactment.rs:243-245`). Defcon 1 was
given a post-condition and never given the seqno term, which is the whole difference.

The predicate becomes:

```rust
fn defcon1_enacted(safe_harbour_activated: bool, defcon1_queued: bool, last_seqno: u64, seq_no: u64) -> bool {
    last_seqno >= seq_no && safe_harbour_activated && !defcon1_queued
}
```

`last_seqno` is read from the council's authority config in the same decoded admin state the arm
already holds — `admin.authority(Role::StrataSecurityCouncil)`, exactly as the multisig arm does at
`:129-141`. No new RPC, no new decode.

The role is named literally, not resolved through the module's `authority_to_role`
(`asm_enactment.rs:302-311`), which still returns an error for `Authority::SecurityCouncil`. That
is the shape the `EeStfVk` arm already uses for `Role::AlpenAdministrator` (`:54-69`): an arm that
matches one action variant knows its role, and routing through the map would make the arm fail on a
mapping it does not need. Teaching that map about the council belongs to the slice that gives the
council a second action.

With the term, the second proposal of the manual run reaches `Enacted` when the council's
`last_seqno` reaches `2` — that is, when **its own** transaction executed.

### 3.2 What stays ambiguous, stated plainly

The seqno term does not make the answer exact. A council action created outside this app, carrying
the same `seq_no`, would consume that seqno and satisfy the term without this proposal ever having
executed. That is the same residual ambiguity every other action in the module carries, and the
same one the module's own header declares (`asm_enactment.rs:1-4`). This phase brings Defcon 1 up
to the rigour of the rest; it does not claim more.

Within the app the collision cannot be produced: two Defcon 1 proposals with one `seq_no` are byte-
identical actions, so the second is refused by AC 3's duplicate rule, and Defcon 1 is the council's
only action type in V1.

### 3.3 A proposal that can no longer execute must not wait forever

Adding the term creates a state the flow did not have: an `approved` proposal with a confirmed
reveal whose transaction the ASM rejected now stays `approved` instead of being (wrongly) promoted.
`expire_if_overdue` (`application/proposals.rs:280-287`) only expires `Pending` proposals, so
nothing retires it.

**This phase does not add a new sweeper for it, and the reason is that no correct rule is available
yet.** `last_seqno >= seq_no` with the post-conditions unmet is exactly the ambiguity of §3.2 — it
cannot tell "somebody else consumed the seqno" from "the ASM has not processed the reveal yet",
and expiring a live proposal is worse than leaving a dead one visible. The proposal remains listed
at quorum with its reveal txid, which is what a signer needs to investigate. Recorded here as a
known consequence rather than left to be rediscovered; the sweeper belongs with V2, where a matured
Defcon 3 gives the module a second, independent post-condition to reason with.

There is a second consequence this section missed when it was written, and it is user-visible.
`report_broadcast_progress` (`application/proposals.rs:667-690`) is the one caller that does not
swallow a `false` predicate: it turns it into `AppError::Conflict("ASM state does not yet show
proposal enactment")`. So a signer who marks a Defcon 1 enacted in the window between its reveal
confirming and the ASM consuming the council's seqno now gets that conflict, where before the
seqno term they got a silent success. This is not a regression introduced here — it is the
behaviour every other action in the module already has, and the conflict says the true thing where
the silent success said a false one. The two reconciling callers
(`reconcile_enacted_for_authority`, `reconcile_enacted_for_action`) are unaffected: they log and
retry on the next poll.

## 4. The gap — the app creates the state and cannot read it

`SafeHarbour` upstream is `{ address, activated }`
(`asm/crates/subprotocols/bridge-v1/types/src/safe_harbour.rs:119-122`). The desktop already
decodes the bridge section that carries it: `decode_bridge_state`
(`desktop-app/src-tauri/src/infrastructure/asm_status_rpc.rs:243`), used today for the operator
set. So the read is one function and one IPC command in a module that already exists.

**It stays in the desktop rather than becoming an orchestrator endpoint.** The precedent is
`get_multisig_config` (`commands/asm_state.rs:29`): live ASM facts the create form needs are read
by the desktop straight from the node, and the orchestrator stays coordination-only. Routing this
one through the backend would add an endpoint, a client method and a serialisation for a boolean
the desktop can already decode.

| Piece | Where |
|---|---|
| `fetch_safe_harbour_activated(rpc_url) -> Result<bool, String>` | `infrastructure/asm_status_rpc.rs` — decode bridge state, return `safe_harbour().is_activated()` |
| `get_safe_harbour_status` IPC command | `commands/asm_state.rs`, beside `get_current_operators` |
| `useSafeHarbourStatus` | `desktop-app/src/hooks/` — both consumers are outside `create-proposal`, so it sits with `use-device-signing-message.ts` rather than inside one domain |
| Dashboard banner (council session only) | `desktop-app/src/screens/proposals-dashboard-screen.tsx` |
| Create-form warning | `desktop-app/src/domain/create-proposal/components/defcon-1-form-fields.tsx` |

Two corrections to that table, both found by reading the code the phase was about to touch:

- **The read carries `activated` and nothing else.** This document first had it return
  `{ activated, address }`, while §8 puts the address out of scope — a field no caller could use
  and, because `SafeHarbourAddress` wraps a `bitcoin_bosd::Descriptor` the desktop does not depend
  on, a new crate dependency for the privilege. `is_activated()` is an inherent method on the
  decoded state, so the boolean costs no dependency at all.
- **The command is named for its neighbours.** `asm_safe_harbour_status` was this document's
  invention; the three commands already in that file are `get_multisig_config`,
  `get_current_operators` and `get_current_vk`.

### 4.1 What the warning may say, and what it may not

The state carries no activation height — there is no block number in `SafeHarbour` and inventing
one from the tip would be a fabrication. This document first proposed recovering provenance from
the orchestrator — the most recent Defcon 1 proposal the app knows to be `Enacted` — and that is
cut. It would couple a chain-state read to proposal data for one sentence, it is wrong whenever the
activation happened outside this app, and this document's own next clause already accepted a banner
that says nothing about provenance. **The banner and the warning state the fact and stop there.**

The warning is about cost and about meaning, not about danger:

> **Safe harbour is already active.** The bridge is already in safe harbour. Another Defcon 1 does
> not change that — it consumes a council sequence number, costs fees, and needs a full quorum.
> Create one only if you have reason to believe this state is wrong.

It renders above the type-to-confirm gate and it **does not disable the submit control** (§2).

It does **not** reuse the `Irreversible` callout's treatment, which this document first asked for.
That callout is `danger-*` red, and it is the strongest thing on the screen for a reason: the
action is irreversible. Rendering a second red block directly above it flattens that hierarchy and
leaves the signer with two alarms of equal weight, one of which is merely a fact about the chain.
The state note takes the app's existing amber attention treatment — `border-accent-border
bg-highlight-surface`, as used at `sign-screen.tsx:278` and `proposal-detail-screen.tsx:167` — and
red stays with the irreversibility.

### 4.2 The banner is the council's, not everyone's

The dashboard banner renders for a Security Council session. Other authorities have no action that
reads on it and no lever that answers it; showing them a bridge-wide alarm they cannot act on is
noise. The create-form warning is Defcon-1-only by construction — it lives in the Defcon 1 fields
component.

## 5. Migration — four commits

Ordered so that none repairs the one before it and each leaves the tree green on its own.

**Commit A — enactment is per proposal.** The predicate gains its seqno term, the Defcon 1 arm
reads the council's `last_seqno` from the admin state it already decodes, and the tests of §6.1
land with it. Backend only; no frontend change depends on it.

Commit B of this document's first draft — "the Rust read, the IPC command, the hook, the dashboard
banner and the create-form warning" — is three commits, not one. It spans two languages and four
layers, and the read is useful and reviewable before either surface consumes it:

**Commit B1 — the read reaches the API layer.** `fetch_safe_harbour_activated`, the
`get_safe_harbour_status` command in both `invoke.rs` handler lists, the Zod schema and the
`api/asm-state.ts` wrapper. No UI change; `npm run test:ipc-schemas` and `npm run build` cover it.

**Commit B2 — the warning on the Defcon 1 form.** `useSafeHarbourStatus` and the amber note in
`defcon-1-form-fields.tsx`. This is where the decision is made, so it lands before the banner.

**Commit B3 — the dashboard banner.** The council-only banner in `proposals-dashboard-screen.tsx`,
which is the same note in a second place and depends on nothing B2 introduced beyond the hook.

## 6. Tests

### 6.1 Commit A

| # | Claim | Shape |
|---|---|---|
| 1 | The seqno term is required | `defcon1_enacted(true, false, 1, 2)` is false — activated, queue clear, and still not this proposal's enactment |
| 2 | The three terms together suffice | `defcon1_enacted(true, false, 2, 2)` is true; equality is enough, the council's seqno need not have moved past it |
| 3 | The existing two terms still hold | the Phase 4 cases, re-expressed with a satisfied seqno, stay as they were |

Test 1 is the regression: it is the manual run, reduced to four booleans and two integers, and it
fails against the predicate as Phase 4 left it.

### 6.2 Commits B1–B3 — no new automated test, and why

This document first listed three: a Rust decode test over a fixture anchor, and two component tests
over the Defcon 1 fields. All three were written against test infrastructure this repository does
not have, and the honest answer is that the UI half of this phase is verified by §9 and by review.

- **The fixture anchor does not exist.** `desktop-app/src-tauri` contains no `AnchorState` fixture
  and no builder for one — `asm_status_rpc.rs`'s own tests cover `decode_state_bytes_from_status`
  over hand-written JSON and stop there, because constructing a valid SSZ `AnchorState` with a
  bridge section means constructing upstream's whole state. `fetch_safe_harbour_activated` adds no
  decoding of its own: it composes `rpc_call`, `decode_anchor_state_from_status` and
  `decode_bridge_state`, all three already exercised by `fetch_current_operators`. A test worth
  its fixture would be testing upstream's SSZ, not this function.
- **There is no DOM runner.** The desktop has neither vitest nor testing-library; every
  `__tests__/*.test.tsx` in the repo is a *contract test* that reads the component's source with
  `readFileSync` and asserts on substrings. "The submit control is enabled with the status active"
  cannot be asserted that way — only "the source does not mention the status near `disabled`",
  which pins a phrasing rather than a behaviour and goes stale the first time the file is
  reformatted. §9 step 2 asserts the real thing against the real app.

What does hold the line automatically: `npm run test:ipc-schemas` fails if the new command's Zod
schema and its Rust DTO drift apart, and `src/lib/__tests__/color-tokens.test.ts` fails the build
on a raw red or amber hex, so the note has to use the tokens. The rest is design judgement, as
Phase 5 §8 argued.

## 7. Blast radius

`defcon1_enacted` has one caller, in the Defcon 1 arm of `is_proposal_enacted_on_asm`. Every other
action's post-condition is untouched, and the multisig arm's own seqno term is the precedent being
copied, not modified.

The new read is additive: a node that cannot serve the bridge section fails the new command only.
The dashboard and the create form must both render with the status *unknown* — a failed read shows
no banner and no warning rather than an error, because a missing read must never stand between the
council and the emergency lever.

## 8. Out of scope

- A sweeper for the proposal of §3.3.
- Defcon 3, its maturation, and its cancel (V2/V5).
- Showing the safe-harbour address, or verifying it against the deployment's configuration (V4).
- Surfacing the state to non-council authorities.

## 9. Manual verification

On regtest with the local stack, continuing from a chain where one Defcon 1 has already enacted:

1. The council dashboard shows the safe-harbour banner; another authority's dashboard does not.
2. `/proposals/create` shows the warning above the type-to-confirm gate, and the submit control
   still enables once the text matches.
3. Create, sign and broadcast a second Defcon 1. It reaches `Enacted` only after the council's
   `last_seqno` has advanced to its `seq_no` — not the moment its reveal confirms.
4. A council session against a node that cannot serve the bridge section still reaches the form and
   can still create a proposal; no banner, no warning, no error dialog.

## 10. Back-propagation to the contract

Two edits, in the back-propagation commit, in the style Phases 3 and 5 used:

- **Edge Cases (`security-council-defcon.md:464`).** The row today reads that the activation is
  idempotent and the proposal "still reaches `enacted` status correctly". That is true of the
  chain and was read as a licence for the predicate. It gains the second half: the proposal reaches
  `enacted` **on its own sequence number**, and the app says the safe harbour is already active
  before the signer commits to a second lever.
- **Acceptance Criteria.** Three new criteria after AC 17:
  - **AC 18** — a Defcon 1 proposal whose reveal has confirmed while the council's `last_seqno` is
    still below its `seq_no` is **not** marked `Enacted`, even with the safe harbour active.
  - **AC 19** — a council session whose chain has the safe harbour active sees that state on the
    proposals dashboard and on the Defcon 1 create form, before signing.
  - **AC 20** — the state being active never disables creation: the type-to-confirm gate remains
    the only gate.

AC 8 keeps its wording; it describes the activation and the queue bypass, both of which remain
required. The seqno term is a third conjunct, and AC 18 is where it is pinned.
