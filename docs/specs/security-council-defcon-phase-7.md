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
| `fetch_safe_harbour_status(rpc_url) -> Result<SafeHarbourStatus, String>` | `infrastructure/asm_status_rpc.rs` — decode bridge state, return `{ activated, address }` |
| `asm_safe_harbour_status` IPC command | `commands/asm_state.rs`, beside `get_multisig_config` |
| `useSafeHarbourStatus` | `desktop-app/src/domain/create-proposal/hooks/` or the dashboard's hooks dir, following whichever the sibling reads use |
| Dashboard banner (council session only) | `desktop-app/src/screens/proposals-dashboard-screen.tsx` |
| Create-form warning | `desktop-app/src/domain/create-proposal/components/defcon-1-form-fields.tsx` |

### 4.1 What the warning may say, and what it may not

The state carries no activation height — there is no block number in `SafeHarbour` and inventing
one from the tip would be a fabrication. Provenance, when it is available, comes from the
orchestrator: the most recent Defcon 1 proposal the app knows to be `Enacted`. When there is none —
the activation happened outside this app — the banner says the state and says nothing about where
it came from.

The warning is about cost and about meaning, not about danger:

> **Safe harbour is already active.** The bridge is already in safe harbour. Another Defcon 1 does
> not change that — it consumes a council sequence number, costs fees, and needs a full quorum.
> Create one only if you have reason to believe this state is wrong.

It renders above the type-to-confirm gate, in the same warning treatment the `Irreversible` callout
already uses, and it **does not disable the submit control** (§2).

### 4.2 The banner is the council's, not everyone's

The dashboard banner renders for a Security Council session. Other authorities have no action that
reads on it and no lever that answers it; showing them a bridge-wide alarm they cannot act on is
noise. The create-form warning is Defcon-1-only by construction — it lives in the Defcon 1 fields
component.

## 5. Migration — two commits

Ordered so that neither repairs the other and both leave the tree green.

**Commit A — enactment is per proposal.** The predicate gains its seqno term, the Defcon 1 arm
reads the council's `last_seqno` from the admin state it already decodes, and the tests of §6.1
land with it. Backend only; no frontend change depends on it.

**Commit B — the safe harbour is visible.** The Rust read, the IPC command, the hook, the dashboard
banner and the create-form warning, with the tests of §6.2. Frontend plus one infrastructure
function; nothing in commit A is touched.

## 6. Tests

### 6.1 Commit A

| # | Claim | Shape |
|---|---|---|
| 1 | The seqno term is required | `defcon1_enacted(true, false, 1, 2)` is false — activated, queue clear, and still not this proposal's enactment |
| 2 | The three terms together suffice | `defcon1_enacted(true, false, 2, 2)` is true; equality is enough, the council's seqno need not have moved past it |
| 3 | The existing two terms still hold | the Phase 4 cases, re-expressed with a satisfied seqno, stay as they were |

Test 1 is the regression: it is the manual run, reduced to four booleans and two integers, and it
fails against the predicate as Phase 4 left it.

### 6.2 Commit B

| # | Claim | Shape |
|---|---|---|
| 4 | The status is decoded from bridge state, not guessed | `fetch_safe_harbour_status` over a fixture anchor returns `activated` matching the encoded `SafeHarbour`; the existing `asm_status_rpc` decode tests are the pattern |
| 5 | The warning is shown when the state is active and absent when it is not | a component test over the Defcon 1 fields with the status prop in both positions |
| 6 | The warning never disables submission | with the status active and the confirm text typed, the submit control is enabled — the assertion that pins §2 against a future "just block it" |

No test asserts the banner's colours; `src/lib/__tests__/color-tokens.test.ts` already fails the
build on a stray red hex, and the rest is design judgement, as Phase 5 §8 argued.

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
