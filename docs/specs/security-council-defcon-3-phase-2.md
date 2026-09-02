# Security Council — Defcon 3 (V2), Phase 2: Redundancy by activation height

**Functional contract:** [`security-council-defcon-3.md`](./security-council-defcon-3.md) — SSOT for
*what* V2 must do. This document never overrides it.

**Build plan:** [`security-council-defcon-3-implementation.md`](./security-council-defcon-3-implementation.md)
§4 Phase 2. This document is that phase at implementation detail.

**Closes:** [AC 9](./security-council-defcon-3.md#9-the-activating-proposal-is-the-one-with-the-lowest-activation-height),
and [debt A](./security-council-defcon.md#what-v2-inherits-and-must-revisit) — the one V1 recorded
against this exact module.

## 1. The change in one sentence

The proposal that activated the safe harbour stops being *the earliest enacted Defcon 1 by sequence
number* and becomes *the enacted harbour-activating proposal with the lowest activation height*, over
both Defcon types.

## 2. What this phase is not

It is not the enactment predicate. **No Defcon 3 in this app can reach `enacted` yet** — see §3 — so
this phase changes no rendered pixel today. It is not the cancel affordance (Phase 3), the create
flow (Phase 5) or the queued lifecycle (Phase 6).

It is not a copy change. `proposals-dashboard-screen.tsx:112` still reads *"Another Defcon 1 does not
change that"*, and stays that way here — see §8.

## 3. Why this lands before it can be observed

Both routes to `status === 'enacted'` go through `is_proposal_enacted_on_asm`, whose `Defcon3` arm
returns `BadRequest("Defcon3 enactment detection is not implemented yet")`
(`orchestrator-be/src/infrastructure/asm_enactment.rs:136-138`). In `reconcile_one` that error is
caught and logged (`application/proposals.rs:521-529`); in `report_broadcast_progress` it propagates
as a client error. A Defcon 3 therefore parks at Approved until **Phase 4**.

That is the point of the ordering, not a gap in it: the corrected rule lands *before* there is any
data it could get wrong. The alternative — shipping enactment detection first — would open a window
in which a matured Defcon 3 is enacted and the badge silently names the wrong proposal, which is
precisely the failure [debt A](./security-council-defcon.md#what-v2-inherits-and-must-revisit)
describes.

The consequence to state plainly: **the discriminating fixture is necessarily synthetic**, and there
is no manual walk for this phase. Honest, not a hole.

## 4. Why this is not a regression for today's data

For a history of Defcon 1s alone, with heights present, **the old rule and the new one give the same
answer.** Upstream jumps `last_seqno` to the accepted value at the *reveal*, not at maturity, and a
Defcon 1's lock period is `0` — so its activation height *is* its reveal block. A Defcon 1 accepted
earlier therefore has both the lower sequence number and the lower activation height: the two
orderings are monotone in each other. Changing the sort key moves nothing that exists today.

Two behaviour deltas remain, and both are wanted:

1. A Defcon 3 enters the candidate set — inert until Phase 4 (§3).
2. Null heights leave it — §5.

## 5. A null activation height is a missing observation, not a low one

`activationHeight` is nullable in all four layers, and an **enacted proposal can carry `null`
permanently**:

- `compute_and_store_activation_height` (`orchestrator-be/src/application/proposals.rs:771-784`) does
  two network reads and is called only from `record_reveal_confirmed_facts`, which is **non-fatal by
  design**: on failure it logs `tracing::warn!` and moves on.
- **There is no repair path.** `confirm_reveal_if_mined` returns early unless the status is still
  `RevealBroadcasted`, so it never fires twice for one proposal. `reconcile_update_id_in_queue`
  exists to re-populate a null `update_id_in_queue` on later cycles and deliberately does not touch
  the height.
- The migration `20260520000000_add_cancel_and_activation_fields.sql:3` adds the column with **no
  backfill**, so every proposal that enacted before it carries `null` forever.

**Null rows are excluded from both roles: never the activator, never redundant.**

**This costs a badge V1 used to show, and the trade is deliberate.** V1 never read the height, so a
proposal whose height failed to compute was still ranked by its sequence number and everything
behind it was still badged. Concretely: a Defcon 1 at seqNo 1 activates the harbour but its height
write fails; a Defcon 1 at seqNo 2 then enacts against an already-true flag. V1 badged the second
one — correctly. Here the first drops out of the ranking, the second becomes the lowest known height
and is therefore named the activator, and a proposal that burned a sequence number and its fees for
nothing goes unbadged. That is a false negative, and §8's test 4 is built to show it rather than to
hide it.

What the exclusion buys is that the badge never says *"changed nothing"* about the activation
itself. Ranking a null row against one with a real number means guessing its position from the
sequence number — which is exactly the premise this phase exists to drop. The guess is *sound* for a
Defcon 1, whose activation height is its reveal block, and *false* for a Defcon 3, whose height is
the reveal plus a delay nobody recorded. Applying it to one type and not the other would reintroduce
the V1 bug on a subset, and ordering the two classes honestly needs a partial order rather than a
sort — a `null` Defcon 1 at a lower seqno precedes everything with a higher seqno but cannot be
placed against a known height at a lower one.

So the frontend takes the bounded error: a missing badge, never a false one. **The real fix is
upstream** — a backend that retries a failed `activation_height` removes the case entirely, and it
is recorded as debt in the build plan §6 rather than patched around here.

**The degenerate case is accepted deliberately.** If every enacted candidate has a null height there
is no activator and no badge at all. The signer is not left uninformed: the dashboard's
`SafeHarbourNote` comes from a **live** chain read (`use-safe-harbour-status.ts`) that does not touch
`activationHeight`, so the app still says the bridge is in safe harbour — it just stops attributing
the activation to a row. Test 5 pins this, so that a later author who "fixes" it by falling back to
the sequence number goes red.

## 6. The tie-break, and its tension with AC 9

Two proposals can share an activation height — two Defcon 1s revealed in the same block, or a Defcon
1 and a Defcon 3 maturing together. `Array.prototype.sort` is stable, so without a tie-break the
winner is **the order the backend returned them in**, which the call site assembles as
`[...executedOrCanceled, ...expiredOrSkipped]`. A badge that moves between renders on identical data
is worse than one that picks either of two indistinguishable rows.

The tie is broken by `seqNo`.

**AC 9 says the activator is the lowest height "regardless of sequence number", and this does not
contradict it.** The sequence number is not a sort key again: it decides only between rows the height
has already declared indistinguishable, which is exactly where AC 9 says nothing, because both rows
*are* the lowest height.

**The tie-break is unobservable on chain, and the spec must not claim otherwise.**
`activate_safe_harbour()` is an unguarded `set_activated(true)` on a flag that is never reset, so two
actions applied in the same block leave identical state either way, and nothing records which one set
it. It agrees with the ASM's own ordering in every case a user can construct — the queue drains
before the block's own transactions are handled, and acceptance jumps `last_seqno`, so a lower-seqno
action processed second would be rejected rather than enacted — but **not universally**:
`remove_queued` uses `swap_remove`, so cancelling an unrelated queued update can permute the queue.
The tie-break exists to make the answer a function of the proposals rather than of their arrival
order, and that is the whole claim.

## 7. The signature, and why it is a hand-written type

The function takes a structural subset instead of the transport DTO:

```ts
type HarbourActivationCandidate = {
	actionId: string
	actionType: ActionType
	status: ProposalStatus
	activationHeight: number | null
	seqNo: number
}
```

**This is the house pattern, not a new one.** `src/lib/` and `src/domain/*/model/` hold six such
types — `DisplayStatusInput`, `ActivationCountdownInput` (`proposal-status.ts:62,85`),
`SendStateInput` (`proposal-send-state.ts:26`), `ProposalActionInput`, `CancelableInput`
(`derive-proposal-actions.ts:6,30`) and `BroadcastConfirmGateInput`
(`broadcast-proposal.ts:135`) — against three functions that take a whole `Proposal`, two of which do
not need one either. `derive-proposal-actions.ts:3-5` even writes down the reason:

> Kept as a structural subset of `Proposal` so callers pass the real domain object while tests can
> build lean fixtures.

`proposal-detail.tsx:90` already passes a whole `Proposal` to `deriveProposalActions`, so the call
site works by structural assignability with no cast and no `.map` — verified field by field against
`api/proposals.ts`. Narrowing a parameter is contravariant: nothing that compiled before can stop
compiling.

**Not `Pick<Proposal, …>`**, despite `typescript-standards.md:23` naming utility types. A `Pick`
keeps `import type { Proposal } from '@/api/proposals'`, so it would preserve the coupling to the
transport DTO that line 18 of the same file asks us to drop, and it has zero precedent in this
codebase. The honest caveat: the new type still imports `ActionType` and `ProposalStatus` from
`api/proposals`, because those unions have no other home. This drops the dependency on the `Proposal`
*shape*, not on the transport module.

**The test is the concrete win.** The old fixture builds sixteen fields to feed a function that reads
four, and closes with `as unknown as Proposal` — a cast forced by declaring `actionType: string`
instead of `ActionType`. The consequence is that today's test has **no type checking on the very
field the function discriminates**: a typo'd `'defcon_2'` compiles and exercises nothing. The new
fixture is five typed fields with `Partial<…>` overrides, following
`derive-proposal-actions.test.ts:8-18`, and the cast is gone.

## 8. Tests

Seven claims, all pure, no mocks, no I/O, no clock.

| # | Claim | Fixture |
|---|---|---|
| 1 | Height beats sequence number, across both Defcon types | a `defcon_3` at seqNo 5 / height 120 and a `defcon_1` at seqNo 6 / height 118 — the Defcon 3 was revealed at 100 with `defcon3 = 20`, and a Defcon 1 swept the bridge two blocks before it matured. The set is exactly the Defcon 3. |
| 2 | V1's answer is preserved for a Defcon-1-only history | the old four-row case, heights monotone in seqno (§4) |
| 3 | Only harbour-activating types count | the `vk_update` case, now with **non-null** heights — today it passes for the wrong reason |
| 4 | A null height is neither activator nor redundant — **and the badge this costs** | one enacted at `null` beside enacted rows at 100 and 105; the set is exactly the 105, so the row at 100 goes unbadged even though the null row may have been the activation (§5) |
| 5 | All heights null ⇒ empty set | the degenerate case of §5, deliberate |
| 6 | The tie is broken by `seqNo`, not by arrival order | two rows at the same height, the higher-seqno one placed **first** in the array |
| 7 | A single enactment is the activation | unchanged from V1 |

Test 1 is the discriminating fixture the build plan names, and it fails on both halves against the
old implementation: wrong activator, and a genuinely redundant Defcon 3 left unbadged. The two halves
are asserted separately so they fail with separate messages.

**Not tested: the dashboard.** There is no DOM runner, and a `readFileSync` component test pins a
phrasing rather than a behaviour — the precedent is `safe-harbour-note-gating.test.ts`, which is
structural for a reason that does not apply here. The call site is one function call whose behaviour
is fully covered.

## 9. The badge copy stays true for a Defcon 3

Checked sentence by sentence against `proposals-dashboard.tsx:537-541`:

> **Changed nothing on chain.** The bridge was already in safe harbour when this executed. It
> consumed a council sequence number and its fees, and left the state as it found it.

A Defcon 3 consumes the council sequence number (at its reveal, on the acceptance path both depths
share), pays commit and reveal fees, and `activate_safe_harbour()` is idempotent whichever lever
called it. All three clauses hold verbatim. The one nuance — for a Defcon 3 the sequence number was
consumed at reveal, not at enactment — survives because the copy says *consumed*, not *consumed just
now*. No copy change in this phase.

## 10. Blast radius

- **One production call site and one test.** `redundantDefcon1ActionIds` is called from
  `proposals-dashboard.tsx:72` and nowhere else; `changedNothing` never leaves that file, and `:404`
  hardcodes `false` for the non-past group with a comment that stays true.
- **The rename touches four references by name** outside the module: the JSDoc at
  `proposals-dashboard.tsx:428`, two specs, and the orphan `test:redundant-defcon-1` script in
  `package.json:66`. The script is **deleted, not renamed** — CI runs `test:unit`, which discovers by
  glob, and Phase 1 §9 already ruled that the enumerated list is the debt the runner exists to kill.
- **Known-stale-on-Phase-5.** `proposals-dashboard-screen.tsx:112` and
  `create-proposal/components/defcon-1-form-fields.tsx:50` both say *"Another Defcon 1 does not change
  that"*. That becomes false when a signer can **create** a Defcon 3 — Phase 5 by the traceability
  table. Changing it here would be copy with no behaviour behind it, in a phase that has no way to
  test copy. The other two harbour notes are already action-type-agnostic: *"Signing this does not
  change that"* and *"Sending this does not change that"*.
- **The new rule inherits every way `activation_height` can be wrong**, not only null — it is
  computed with the depth read **live at reveal-confirmation time**, so changing
  `confirmation_depths.defcon3` mid-flight leaves a stored height the chain no longer agrees with.
  Pre-existing (`showsActivationCountdown` already trusts the same field) and recorded as debt rather
  than fixed here.

## 11. Verification

The frontend half of the [`AGENTS.md`](../../AGENTS.md) checklist is what this phase can fail; the
Rust half is run anyway before pushing.

```bash
cd desktop-app && npm run format:check && npm run lint && npm run build && npm run test:unit
```

`npm run test:unit` discovers by glob, so the **file count stays the same** — one added, one deleted.
That is the opposite of Phase 1's check, where it had to go up by one.

Three structural checks that the phase stayed inside its scope:

The first is scoped to `desktop-app/` on purpose: the specs keep naming the old symbol in the past
tense, which is how a phase records what it changed.

```bash
git grep -n "redundant-defcon-1\|redundantDefcon1ActionIds" -- desktop-app/   # nothing left
git grep -n "as unknown as Proposal" desktop-app/src/lib/     # the cast died with the old signature
git diff --stat -- orchestrator-be/ desktop-app/src-tauri/    # empty: this phase is desktop-only
```

No manual walk, for the reason in §3.
