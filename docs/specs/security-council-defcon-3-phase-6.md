# Security Council — Defcon 3 (V2), Phase 6: The queued lifecycle

**Functional contract:** [`security-council-defcon-3.md`](./security-council-defcon-3.md) — SSOT for
*what* V2 must do. This document never overrides it.

**Build plan:** [`security-council-defcon-3-implementation.md`](./security-council-defcon-3-implementation.md)
§4 Phase 6. This document is that phase at implementation detail.

**Closes:** [AC 7](./security-council-defcon-3.md#7-the-countdown-is-driven-by-the-live-depth) and
[AC 10](./security-council-defcon-3.md#10-a-queued-defcon-3-offers-a-cancel-a-defcon-1-never-does);
the § *Frontend Contract → Lifecycle display* three bullets.

## 1. The change in one sentence

A queued Defcon 3 reads correctly in **all three views that show one** — dashboard, detail and cancel
— because all three ask the same two questions (`showsActivationCountdown`, `proposalDisplayStatus`),
and those questions get the tests they never had for `defcon_3`.

## 2. What this phase is not

It is not the cancel journey and it is not `run_defcon3_canceled` — Phase 7 owns both. It adds no
backend code, no protocol rule, no new component and **no new predicate**: every rule it relies on
already exists and already answers correctly. It does not touch `activation_height`'s two known
weaknesses (it can be `null` forever, and it can go stale if the depth is reconfigured mid-flight);
both are recorded as debt in the build plan §6 and both degrade to "no countdown", which is the
right degradation.

## 3. The build plan was half right, and the half it missed is the one a signer sees

The plan calls this phase *"mostly pinning behaviour that is already right"*. Checked against the
code, that splits three ways.

### 3.1 Right, and untested for `defcon_3` — pin it

`showsActivationCountdown` (`desktop-app/src/lib/proposal-status.ts:95-97`) excludes **only**
`defcon_1`, so it already returns `true` for a `defcon_3`. `proposalDisplayStatus` (`:75-79`)
reserves `quorum_reached` for `defcon_1` alone, so an approved Defcon 3 already renders the word
**Approved** — which is what PRD 06 §5.2.2 wants, because the §5(b) carve-out names Defcon 1 only
and Defcon 3 has a real Approved state.

Both are correct **by having been written to be**, and V1's Phase 6 wrote the comment that says so:
*"Keyed on the action and not on the authority: Defcon 3 shares the authority and carries a real
configurable depth."* Neither has a single assertion naming `defcon_3` today, and the two levers
share one authority — so the way this regresses is somebody "simplifying" the predicate back onto
the authority, which is exactly the mistake Phase 3 spent a whole slice undoing on the cancel side.
§7 tests 1–4.

### 3.2 The "one genuine fix" is real but weaker than the plan implies — say so

`cancel-proposal-screen.tsx:118` decides the countdown with
`proposal.activationHeight !== null && proposal.status === 'approved'` — the shape that existed
before V1 extracted the predicate, missing the `actionType !== 'defcon_1'` term.

**It is not an observable defect today, and this spec will not pretend otherwise.** The same screen
guards at `:48-53`:

```tsx
if (proposal !== null && proposal.status !== 'approved') return <Navigate … />
if (proposal !== null && !canCancelProposal(proposal)) return <Navigate … />
```

`canCancelProposal` reads the backend's `isCancelable`, which is
`is_cancelable_for_hex` → `depth_for_action(..) > 0`
(`orchestrator-be/src/infrastructure/asm_role_membership.rs:187-192,141-148`). A Defcon 1 resolves to
`0` **structurally, not by configuration** — upstream's `ConfirmationDepths` has no `Defcon1` field at
all, so it falls through `unwrap_or(0)` in every deployment, and `asm_role_membership.rs:640-645`
pins exactly that. All three of the function's degradations point the same way: an undecodable hex
is `false`, and an unreachable ASM is `false`. A Defcon 1 never reaches line 118. What is
wrong is that the rule is written down twice and one copy is stale — the precise failure mode V1's
Phase 6 created `proposalDisplayStatus` to end, when two screens derived `awaiting_enactment` by
different expressions that happened to agree. It is corrected as a refactor, and the commit message
claims a refactor, not a fix. (Lesson from [Phase 1](./security-council-defcon-3-phase-1.md): a
premise of urgency must be checked against the code before a spec writes it down.)

### 3.3 The gap the plan did not see: the dashboard never counts down

`ActivationCountdown` has exactly two call sites — `proposal-detail-screen.tsx:176` and
`cancel-proposal-screen.tsx:120`. The **dashboard card has none.** A queued Defcon 3 there renders
(`proposals-dashboard.tsx:573-581`):

> **Reveal confirmed — awaiting ASM enactment**
> Refresh to check whether the ASM has applied it.

No activation block, no current block, no remaining time. And the second line is **specifically
misleading for a Defcon 3**: refreshing does not advance the chain tip, and for the whole of
`confirmation_depths.defcon3` blocks the honest answer is "nothing will have changed". That sentence
was written when the only queued actions were ones whose delay nobody was counting; Defcon 3 is the
first lever whose entire product difference *is* the delay.

AC 7 says *"When the proposal is displayed"*, and the contract's Lifecycle display asks for
*"activation block, current block, and an approximate remaining time"*. The dashboard is the view a
council signer opens first and returns to; satisfying AC 7 only on the detail screen satisfies it on
the view a signer reaches by a click they have no reason to make. §6 closes it.

## 4. AC 10 is closed by evidence, not by new code

*A queued Defcon 3 offers a cancel; a Defcon 1 never does, in every view.* Three views, one source:

| View | Gate | Reads |
|---|---|---|
| Dashboard card (`proposals-dashboard.tsx:582`) | `canCancel && proposal.cancelProposal === null`, inside the `awaitingEnactment` branch | `deriveProposalActions` → `canCancelProposal` → `proposal.isCancelable` |
| Detail screen (`proposal-detail-screen.tsx:201-204`) | `status === 'approved' && kind !== 'cancel' && canCancelProposal(proposal) && cancelProposal === null` | same |
| Cancel screen (`cancel-proposal-screen.tsx:51`) | `!canCancelProposal(proposal)` → redirect | same |

All three read the backend's answer, which Phase 3 derived from the same **decision function**,
`depth_for_action`. Not the same *caller*, and the difference matters: `create_cancel_proposal` goes
through `lock_period_for_action`, which does a live RPC and **errors** if the ASM will not answer,
while the listing goes through `is_cancelable_for_hex`, which resolves against a table read once and
**degrades to `false`** (§9). A Defcon 1 is `isCancelable: false` at depth `0`; a Defcon 3 is
`true` wherever `confirmation_depths.defcon3 > 0`, and where it is `0` the contract's Edge Cases
already call the missing affordance the correct degradation. Where the **ASM cannot be reached** the
same Edge Cases accept the affordance disappearing — but see §9 for how silently that happens. `derive-proposal-actions.test.ts`
covers the derivation and `is_cancelable_for_hex` covers the backend half including both its
degradations.

**So this phase writes no cancel-affordance code.** The one thing worth recording, because it looks
like an inconsistency and is not: the dashboard gate omits the detail screen's `status === 'approved'
&& kind !== 'cancel'` terms. It does not need them — `awaitingEnactment` is
`proposalSendState(proposal).kind === 'confirmed'`, which is only reachable from `approved`, and a
cancel row answers `isCancelable: false` by construction because `depth_for_action` returns `0` for
`MultisigAction::Cancel`.

A fourth place writes the field without asking the backend, and it is correct:
`manual-sign-collect.tsx:61` builds a synthetic `Proposal` for the offline route with
`isCancelable: false`. There is no orchestrator in that flow to answer, and a proposal the backend
has never seen has nothing to cancel. It is not a surviving allow-list.

## 5. The countdown is already driven by the live depth (AC 7's second half)

Nothing in the frontend contains a block count or an hour count for Defcon 3.
`ActivationCountdown` takes `activationHeight`, which the backend computed as
`reveal_block + lock_period` with the depth read live from `confirmation_depths.defcon3`, and
`currentHeight`, which comes from `useBlockHeight()` — a 15-second poll of the real chain tip. The
component's own `AVG_BLOCK_SECONDS = 600` converts *blocks* to an approximate duration and is not a
delay constant; it carries a `//TODO: revise this.` that predates this slice and stays.

This phase therefore satisfies AC 7 by **widening where that already-live countdown is shown**, not
by changing how it is computed.

## 6. The dashboard change, and the drilling it costs

`proposals-dashboard-screen.tsx` **gains** a `useBlockHeight()` call — it has none today, and this is
the one sentence in this document describing something that does not exist yet — and passes
`currentBlockHeight` down to the card:

```
ProposalsDashboardScreen → ProposalsDashboard → PendingTab → ProposalGroup → ProposalCard
                                              → PastTab    → ProposalCard
```

Five components, four hops on the long path, **six pass-through sites** — because `PendingTab`
renders `ProposalGroup` *twice*, once for "Pending" (`:244`) and once for "Quorum reached" (`:257`),
and a queued Defcon 3 lands in the second. Wiring one and not the other is the mistake this
paragraph exists to prevent; a **required** prop is what makes `tsc` catch it.

Concretely, ~18 lines across two files: five props types (`:27`, `:221`, `:290`, `:365`, `:427`),
five destructurings (`:47`, `:210`, `:273`, `:350`, `:417`), six pass-throughs, plus the hook call
and its import in the screen. It follows `signerPubkey` site for site — the same value already
threaded through the same five components.

**Deliberately not a context and deliberately not a hook call inside the card:** `useBlockHeight`
installs its own 15-second interval, so calling it per card means one poller per row on screen, and a
new context for a number that one branch of one component reads is an abstraction the codebase would
carry forever to save eighteen lines once.

**`PastTab`'s copy of the prop is dead drilling, and that is fine.** It only ever receives
`executedOrCanceled` and `expiredOrSkipped` (`proposals-dashboard-screen.tsx:74-82`), none of which
can be `approved`, so its cards never enter the `awaitingEnactment` branch. The prop exists there
only because `ProposalCard` is shared. Written down so a reviewer reads it as symmetry, not as a bug.

Inside the card, the `awaitingEnactment` branch becomes:

```tsx
{proposal.activationHeight !== null && currentBlockHeight !== null && showsActivationCountdown(proposal) ? (
	<ActivationCountdown activationHeight={proposal.activationHeight} currentHeight={currentBlockHeight} />
) : (
	<p className="…">Refresh to check whether the ASM has applied it.</p>
)}
```

The rule is the shared predicate, plus the null-check that narrows `activationHeight` to the
`number` the prop requires — and **one term the detail screen does not carry**.

`currentBlockHeight !== null` is there because this countdown *displaces* something.
`useBlockHeight` answers `null` until its first poll returns and **forever** if
`getBitcoinBlockHeight` keeps failing — a misconfigured `btc_rpc_user`, a node that is down, the
offline route — and it surfaces no error. `ActivationCountdown` then renders the activation block
alone: no current block, no remaining time. A signer would be told *"activation in block 812,345"*
with no way to know whether that is one block away or a thousand, **and** would have lost the
sentence telling them what to do next. Trading a working sentence for a countdown that counts
nothing is a worse card than the one this phase started with. The detail and cancel screens need no
such term: there the countdown is an extra block that displaces nothing, so a degraded one still
adds the activation height.

**The countdown replaces the refresh line rather than joining it.** While blocks remain, *"Refresh to
check"* is advice that cannot pay off, and the countdown answers the question it was standing in for.
The advice is not lost: the dashboard header carries a **Refresh proposals** button
(`proposals-dashboard.tsx:97-105`) on every tab and every row, so the line was restating an
affordance already on screen. An action with no depth, or one whose height failed to compute, keeps
the line unchanged.

**What this costs, stated rather than glossed:** once the tip reaches the activation height the
component reads `imminent` and the card no longer says anything about refreshing — the moment
refreshing finally *is* the right move is the moment the sentence is gone. That is accepted for the
header button, and it is the honest reading of the swap; the alternative (countdown while
`currentHeight < activationHeight`, refresh line at `imminent`) buys one sentence for a second
condition in a branch that already has two, and it would read as a flicker to anyone watching the
block land. If the manual walk says the disappearance confuses a signer, that is exactly what the
reserve phase is for.

**One label in that branch is not the dashboard's to edit.** *"Reveal confirmed — awaiting ASM
enactment"* comes from `STAGE.reveal_confirmed.label` in `lib/proposal-send-state.ts:56`, shared with
the detail screen. This phase renders it unchanged; anybody later tempted to reword it "for the
dashboard" is rewording it in both places.

## 7. Tests

Four pure claims plus one wiring guard. No mocks, no I/O, no clock.

They extend `desktop-app/src/lib/__tests__/proposal-display-status.test.ts` rather than adding a
file: same module, same unit, and its `proposal(status, actionType, broadcastStatus)` helper is
already the fixture these need. Its header comment gains the second thing it now pins.

| # | Claim | Assertion |
|---|---|---|
| 1 | The countdown shows for a queued `defcon_3` | `showsActivationCountdown({ status: 'approved', activationHeight: 101, actionType: 'defcon_3' })` is `true` |
| 2 | …and degrades to nothing without a height, and before approval | the same with `activationHeight: null`, and with `status: 'pending'`, are both `false` |
| 3 | An approved `defcon_3` reads **Approved**, never *Quorum reached* | `proposalDisplayStatus(proposal('approved', 'defcon_3'))` is `'approved'` **and** `PROPOSAL_STATUS_STYLE.approved.label` is `'Approved'` — Defcon 1's carve-out must not spread to the lever that has a real Approved state |
| 4 | Once the reveal confirms, it reads **Awaiting enactment** | `proposalDisplayStatus(proposal('approved', 'defcon_3', 'reveal_confirmed'))` is `'awaiting_enactment'` — the state the dashboard's countdown branch hangs off |
| 5 | The dashboard asks the shared question and is given a live tip | source-text guard, `src/domain/proposals-dashboard/components/__tests__/dashboard-countdown-wiring.test.ts` |

**Every claim is verified by mutation before the commit lands** — the Phase 2 lesson that a test can
pass by construction. The first draft of this table was wrong, and running it is what showed that:
the obvious mutations (flip `!== 'defcon_1'`, drop a term) are **already killed by the assertions
this file had before Defcon 3 existed**, because they change the answer for `multisig_update` too.

The mutations these four claims exist for are the ones that are *specific to `defcon_3`* — which is
exactly the regression the phase is worried about, since the two levers share an authority and the
tempting "simplification" is to treat them as one thing. All three survive the file as it was and
die against it as it is:

| Mutation to `proposal-status.ts` | Old file | With claims 1–4 |
|---|---|---|
| `&& actionType !== 'defcon_1' && actionType !== 'defcon_3'` — "neither council lever counts down" | passes | **fails** |
| `actionType === 'defcon_1' \|\| actionType === 'defcon_3' ? 'quorum_reached'` — the carve-out spreads | passes | **fails** |
| `!actionType.startsWith('defcon')` — the predicate re-keyed on what is really the authority | passes | **fails** |

Recorded as a table of two columns rather than one, because "the test goes red" proves nothing on
its own: what makes these four claims worth their lines is the left column.

**Test 5 is a structural test and the spec says so plainly.** There is no DOM runner —
`@testing-library/react` is not installed (`BLOCKED_BY_DEPENDENCY`) — so it follows
`screens/__tests__/broadcast-screens-wiring.test.ts`: read the source, assert the wiring. It is
written against **files**, not components, because `ProposalCard` and `ProposalsDashboard` live in
one file and no test here parses TSX. Four assertions, no phrasing:

- `proposals-dashboard.tsx` contains `showsActivationCountdown(` — the card must not re-derive the
  condition, which is the exact defect §3.2 removes from the other screen;
- `proposals-dashboard.tsx` contains `<ActivationCountdown` and `currentHeight={currentBlockHeight}`;
- `proposals-dashboard.tsx` gates on `currentBlockHeight !== null` too — an unknown tip keeps the
  refresh line rather than showing a countdown that counts nothing;
- `proposals-dashboard.tsx` does **not** call `useBlockHeight(`, and
  `proposals-dashboard-screen.tsx` **does** — one poller for the screen, not one per row. Matched as
  a **call** and not as a bare identifier: §6 recommends writing down *why* the hook is not called
  there, and a comment saying so must not turn this red.

**Not tested:** the rendered card, the detail screen and the cancel screen, for the reason above; the
cancel affordance, which §4 closes by evidence against tests that already exist; anything backend —
`git diff` proves this phase does not touch it.

## 8. Migration — four commits, each atomic

| # | Commit | Why it is safe on its own |
|---|---|---|
| 0 | This spec | Docs only |
| 1 | The four pure claims in `proposal-display-status.test.ts` | Tests only; they characterise behaviour that is already correct, and the mutation table is what proves they are not passing by construction |
| 2 | `cancel-proposal-screen.tsx` moves onto `showsActivationCountdown` | Behaviour-preserving by §3.2; the rule it now calls is the one commit 1 just pinned |
| 3 | The dashboard countdown, plus the wiring test, plus `6 ✅` and `Phases 1–6 shipped` in the build plan | The only commit that changes a pixel, and the only one where the phase is honestly shipped |

Commit 1 precedes commit 2 on purpose: the refactor is safe *because* the predicate is pinned, and
pinning it afterwards would be a commit that justifies the one before it.

## 9. Blast radius

- **Every authority sees the dashboard change, not only the council.** `showsActivationCountdown` is
  keyed on the action, so a `multisig_update`, a `vk_update` or an `operator_set_update` awaiting
  enactment gains the same countdown and loses the same refresh line. That is a fix for them too —
  their delays were equally invisible — and it is why the change belongs in the shared card rather
  than behind a Defcon 3 condition.
- **One new network read per dashboard mount**, `getBitcoinBlockHeight` every 15 s, on a screen that
  already polls nothing. Identical to what the detail and cancel screens already do, and the
  countdown degrades to the activation block alone when it answers `null`.
- **`proposals-dashboard.tsx` gains one required prop on four internal components.** They are
  file-local, so `tsc` catches every missed hop; nothing outside the file changes except the screen
  that renders `<ProposalsDashboard>`.
- **The "Quorum reached" group heading now contains rows badged *Awaiting enactment*.** Pre-existing
  for every queued action and untouched here: the screen's `quorumReached` filter is
  `status === 'approved' || hasReachedQuorum(proposal)`, so an awaiting-enactment row has always sat
  under that title. Recorded, not fixed — renaming a group heading is copy with no behaviour behind
  it and belongs to the reserve phase if the manual walk says it confuses anyone.
- **An unreachable ASM silently withdraws the cancel affordance everywhere.** With
  `ConfirmationDepthResolver::Unavailable`, *every* proposal answers `isCancelable: false` — the
  dashboard and detail buttons vanish and the cancel screen redirects, with no message and only a
  `tracing::warn!` on the backend (`asm_role_membership.rs:171-173`). For the duration of an ASM
  outage, AC 10's *"a queued Defcon 3 offers a cancel"* is false. The contract's Edge Cases accept the
  degradation ("cancelability degrades to no affordance and the next cycle asks again") but nothing
  tells the signer which of the two answers they are looking at. Unchanged by this phase, not listed
  in the build plan's §6, and recorded here as debt because Phase 6 is the phase that audited the
  three views and is therefore where it became visible.
- **A `null` activation height is indistinguishable from no delay, on the dashboard.** The height is
  computed once, non-fatally, at reveal confirmation (`application/proposals.rs:817-819`) and never
  retried, so a transient RPC failure leaves it `null` for the life of the row. Such a Defcon 3 keeps
  the refresh line and shows no countdown — exactly what a depth-`0` action shows. Pre-existing debt
  (build plan §6); this phase widens where it is observable without changing it.
- **The block-height poll now runs on the screen a signer leaves open all day.**
  `get_bitcoin_block_height` builds a fresh `HttpBitcoinRpcClient` per call
  (`src-tauri/src/commands/asm_state.rs:126`), which was cheap on the detail and cancel screens
  because they are visited briefly. On the dashboard it is a new client every 15 s, for every
  authority, with no back-off and no UI feedback when Bitcoin RPC is unreachable. Pre-existing and
  **deliberately not fixed here**: the fix belongs in `src-tauri`, and `git diff` proving this phase
  is desktop-only is one of its two structural checks (§10). Recorded as debt for whichever phase
  next touches that command.
- **No backend, no `src-tauri`, no new dependency, no new component, no new predicate.**

## 10. Verification

```bash
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
cd desktop-app && npm run format:check && npm run lint && npm run build && npm run test:unit
```

`npm run test:unit` discovers by glob (`scripts/run-unit-tests.mjs`), so the new wiring test runs in
CI with no change to `package.json` or `ci.yml`; the **file count goes up by exactly one**.

Two structural checks that the phase stayed inside its scope:

```bash
git diff --stat develop -- orchestrator-be/ desktop-app/src-tauri/   # empty: desktop-only

# the cancel screen's countdown now asks the shared question, and asks it once
git grep -n "activationHeight !== null" desktop-app/src/screens/cancel-proposal-screen.tsx
```

The second command must return exactly one line, and it must name `showsActivationCountdown(`. It is
written this way because the obvious grep does not work: the screen's guard reads
`status !== 'approved'`, and two later blocks (`:128`, `:131`) legitimately test
`status === 'approved'` for things that are not the countdown.

**The manual walk is the signer's, run by hand, and this phase starts nothing to help it.** The
script, from the build plan §5 points 3 and 6:

1. Create a Defcon 3, reach quorum, broadcast.
2. The dashboard moves from *Approved* to *Awaiting enactment* **with the countdown to the right
   block** in place of the refresh line.
3. The detail screen names the same activation block and the same current block.
4. The cancel screen does too.
5. A Defcon 1 created in the same session shows no countdown and no cancel affordance in any of the
   three.
