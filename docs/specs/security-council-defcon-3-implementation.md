# Security Council — Defcon 3 (V2) Implementation Plan

**Functional contract:** [`security-council-defcon-3.md`](./security-council-defcon-3.md) — the SSOT
for *what* V2 must do. This document is only *how* it gets built, and never overrides it.

**Master plan:** [`security-council.md`](./security-council.md) §6 Stage board, §7 Slice board.

**Stories:** [`story-map.md`](../3-stories/story-map.md) US-E13 and US-E14.

**Status:** All seven phases shipped. Phase 8 is held in reserve for what the manual walk exposes.

A phase marked ✅ means the engineering step shipped, not that every acceptance criterion in the
contract is satisfied — the contract's `## Acceptance Criteria` section stays the measure.

## 1. Purpose and scope

V2 is cheap because V1 was expensive. V1 carried the shared spine — the authority→role mapping, the
per-action lock period, the depth-shaped cancel gate, the codec, the action builder, the council
session and the signer-safety UX — and it deliberately taught two of those about Defcon 3 because
they could not be written correctly otherwise. What is left is one enactment predicate, one form
variant, two inherited debts, and the cancel.

**In scope**

- Defcon 3 (`UpdateTxType::Defcon3 = 43`) end to end for the Strata Security Council.
- The Defcon 3 cancel (US-E14), absorbed from what the slice board called V5 — see
  [`security-council.md`](./security-council.md#73-why-v5-was-absorbed-into-v2).
- The two debts V1 recorded: redundancy by activation height, and cancelability carried on the
  proposal DTO.

**Not in scope**

- Security Council signer update (V3) and Safe Harbour address update (V4) — both authorized by the
  Strata Administrator.
- Any protocol validity rule. The orchestrator stays coordination-only.

## 2. Traceability

| Phase | Name | Closes (contract) | Touches |
|---|---|---|---|
| 1 ✅ | `defcon_3` is a readable type — [phase spec](./security-council-defcon-3-phase-1.md) | (none directly — prerequisite) | `src-tauri`, `desktop-app` |
| 2 ✅ | Redundancy by activation height — [phase spec](./security-council-defcon-3-phase-2.md) | AC 9; [debt A](./security-council-defcon.md#what-v2-inherits-and-must-revisit) | `desktop-app` |
| 3 ✅ | Cancelability travels on the proposal — [phase spec](./security-council-defcon-3-phase-3.md) | AC 13; [Constraint 4](./security-council-defcon-3.md#4-cancelability-is-answered-by-the-backend-for-every-authority) | `orchestrator-be`, `src-tauri`, `desktop-app` |
| 4 ✅ | Defcon 3 enactment detection — [phase spec](./security-council-defcon-3-phase-4.md) | AC 6, AC 8, AC 12; [Constraints 2](./security-council-defcon-3.md#2-defcon-3-enactment-cannot-reuse-defcon-1s-seqno-equality) and [3](./security-council-defcon-3.md#3-a-cancelled-defcon-3-must-never-be-reported-as-enacted) | `orchestrator-be` |
| 5 ✅ | Frontend — create and sign — [phase spec](./security-council-defcon-3-phase-5.md) | AC 1, 1a, 2, 3, 4, 5, 15 | `src-tauri`, `desktop-app` |
| 6 ✅ | Frontend — queued lifecycle — [phase spec](./security-council-defcon-3-phase-6.md) | AC 7, AC 10 | `desktop-app` |
| 7 ✅ | The cancel, end to end — [phase spec](./security-council-defcon-3-phase-7.md) | AC 11, AC 12, AC 14; [Constraint 3](./security-council-defcon-3.md#3-a-cancelled-defcon-3-must-never-be-reported-as-enacted) | `desktop-app`, `src-tauri`, `e2e-tests` |
| 8 | Reserve — what the manual walk exposes | — | — |

## 3. Architecture

### What already exists and is reused

| Piece | Location | Why it matters |
|---|---|---|
| `lock_period_for_action` / `depth_for_action` | `orchestrator-be/src/infrastructure/asm_role_membership.rs` | Already resolves `confirmation_depths.defcon3`; already takes the depth lookup as a closure, which is the seam Phase 3 needs |
| `require_authorized_for_action` | same module | Generic over upstream's `authorized_role()`, so Defcon 3 is council-only with no new code |
| `create_cancel_proposal` | `orchestrator-be/src/application/proposals.rs` | Depth-gated since V1; admits a Defcon 3 and refuses a Defcon 1 unchanged |
| `supersede_if_seq_no_consumed` | same module | "Presence in the queue outranks the seqno" — what keeps a maturing Defcon 3 alive |
| `compute_and_store_activation_height`, `record_reveal_confirmed_facts` | same module | The activation height and the queue `UpdateId` are already persisted per proposal |
| `defcon1_enacted` | `orchestrator-be/src/infrastructure/asm_enactment.rs` | The shape Phase 4's predicate follows — a free function over plain observations |
| `showsActivationCountdown`, `activation-countdown.tsx` | `desktop-app/src/lib/proposal-status.ts`, `domain/cancel-proposal/components/` | Already correct for Defcon 3: the predicate excludes only `defcon_1` |
| `SigningMessage::for_action` via `render_signing_message` | `desktop-app/src-tauri/src/infrastructure/signing.rs` | The four canonical lines come out for free |
| `SafeHarbourNote`, `useSafeHarbourActivated` | `desktop-app/src/components`, `src/hooks` | The already-in-harbour warning is a component, not a Defcon 1 detail |
| `run_defcon3` | `e2e-tests/tests/e2e_defcon_probe.rs` | Proves queue → depth → activation on a real regtest ASM |
| `e2e_cancel_proposal.rs` | `e2e-tests/tests/` | The shape Phase 7's cancelled-path e2e follows |

### Where Defcon 3 lives in the frontend

Same answer V1 settled for Defcon 1: **it extends `desktop-app/src/domain/create-proposal/`** and
gets no route of its own. The domain already dispatches by action type, so Defcon 3 is one more entry
in `ACTION_TYPES_BY_AUTHORITY` and one more fields component.

**The form component is parameterized, not duplicated.** The two variants differ in exactly three
things: the confirmation string, the destructive paragraph, and the safe-harbour note's wording.
Duplicating would fork the signing-message wiring — the resolve, the mirror into a form value, and
the CTA gate that depends on it — which is the safety-critical half. The validator entries stay
separate, because the schema enum is per action type.

The risk parameterizing introduces is that the two confirmation gates could come to accept each
other's string. That is why the contract states their mutual exclusion as a requirement and Phase 5
tests it directly.

### The one breaking point

Phase 3 changes a contract shared with **every** authority. Everything else in V2 is additive.

Cancelability moves from a desktop allow-list to a backend answer. The desktop cannot read a live
confirmation depth, so it has been guessing with `CANCELABLE_AUTHORITIES` while the backend gated on
depth alone. V2 is the first slice where the guess is load-bearing: the council must gain the
affordance for Defcon 3 and keep having none for Defcon 1, and no authority-shaped condition can
express that, because the two share one authority.

Three consequences, all of them intended:

- **Sequencer Manager proposals gain a visible cancel affordance.** The backend has allowed this
  since V1's Phase 2; only the desktop hid it.
- **Cancelability becomes a live read at list time.** It must degrade rather than fail — the pattern
  is `live_last_seqno`, which answers `None` when the ASM cannot be reached. A proposal whose
  cancelability is unknown offers no affordance.
- **The depth table is resolved once per request.** `lock_period_for_action` does one RPC per call;
  a listing of N proposals would be N round trips. `depth_for_action` already takes the lookup as a
  closure, so the fix is to pass a table read once — not to add a cache.

## 4. Phased plan

Every phase: its own branch off `feat/security-council-defcon-3`, one atomic commit (never a commit
that repairs the one before it), and the full [`AGENTS.md`](../../AGENTS.md) pre-commit CI checklist
green before pushing. The phases are **sequential, not parallel**.

### Phase 1 — `defcon_3` is a readable type, end to end

Make `defcon_3` a legal value everywhere it is *read*, with no way to create one yet: the Tauri
action enum and codec, `DecodedAction`, `action_type_from_hex`, and on the TypeScript side the
`ActionType` union, the IPC schemas, the decoded-action schema and the type label.

**Why this precedes the two debts, even though the debts were asked for first.** `actionType` is a
**closed** Zod enum. A Tauri that emits `defcon_3` against a schema that does not accept it fails the
parse of *every proposal in the same list*, not just the new one — which is the reason the existing
test in `src-tauri/src/commands/proposals.rs` exists at all. Emitter and acceptor therefore cannot be
split across two PRs. Beyond that, Phases 2, 3, 5 and 6 each need `defcon_3` to be a legal
`ActionType` merely to **write a fixture**. Landing this later would force a commit that repairs the
one before it.

It is a prerequisite, not a product step: nothing in the application can produce a Defcon 3 hex when
it merges.

**Tests.** Codec round-trip plus a tripwire that the variant still encodes `UpdateTxType::Defcon3`;
`action_type_from_hex` names `defcon_3`; the IPC schema contract test accepts both the new
`actionType` and the new decoded-action kind.
**Not tested:** anything end to end. There is no producer yet, and hand-writing a hex fixture to
assert one would only restate what the codec test owns.

### Phase 2 — Redundancy by activation height

`redundantDefcon1ActionIds` picked the earliest enacted Defcon 1 *by sequence number* as the proposal
that activated the safe harbour. Defcon 3 activates the same flag on a timelock, so from this slice
on the earliest by seqno is not necessarily the one that turned it on. The activator becomes the
enacted harbour-activating proposal with the **lowest activation height**, over both Defcon types.
The module is renamed to match what it now answers — it shipped as
`desktop-app/src/lib/safe-harbour-redundancy.ts`, exporting `changedNothingActionIds`.

The two heights are comparable because a Defcon 1's lock period is `0`, so its activation height *is*
its reveal block.

**Tests (pure TS).** The discriminating fixture is the one the current code gets backwards: a Defcon
3 with the **lower sequence number** but a **higher activation height** than a Defcon 1 revealed
after it. Ordering by seqno names the Defcon 3 the activator; ordering by height correctly names the
Defcon 1 and marks the Defcon 3 redundant. Plus `activationHeight === null`, which stays excluded so
the answer keeps erring towards saying nothing.
**Not tested:** the dashboard. There is no DOM runner, and a `readFileSync` component test pins a
phrasing rather than a behaviour. The call site is one function call whose behaviour is fully covered.

### Phase 3 — Cancelability travels on the proposal

The backend answers "can this be cancelled", derived from the **same** function `create_cancel_proposal`
gates on, and the desktop deletes `CANCELABLE_AUTHORITIES`. See
[§3 The one breaking point](#the-one-breaking-point) for the blast radius and the three consequences
this phase accepts.

Two decisions the phase spec must settle before writing code, because both are visible on the wire:
whether the field is added to the domain struct the handlers serialize directly or to a response DTO;
and whether "unknown" is a third state or collapses to `false`. The contract requires only that an
unknown cancelability offers no affordance.

**Ordering within the commit:** the backend serves the field before the desktop reads it. Serde
ignores unknown fields in that direction; the reverse would break the parse.

**Tests.** Rust unit on the derivation through the existing closure seam, with no ASM: Defcon 1 →
false, Defcon 3 at a non-zero depth → true, a cancel → false. That last one is free by construction
(`depth_for_action` returns `0` for `MultisigAction::Cancel`) and is asserted anyway, because a
cancel row is the one whose own action hex is meaningless to this question. Plus the DTO carrying it,
the IPC schema contract test, and the rewrite of `derive-proposal-actions.test.ts` to the field-driven
shape — keeping a case that proves the frontend no longer has an opinion about authority.
**Not tested:** an HTTP round trip; the handler is a thin map over an already-tested function.
**If it must be split:** "the backend computes and serves it" then "the desktop consumes it and drops
the allow-list", accepting one PR where the field is served and unused.

### Phase 4 — Defcon 3 enactment detection

The `Defcon3` arm of `is_proposal_enacted_on_asm` stops returning `BadRequest` and becomes a free
predicate over four observations — seqno consumed, gone from the queue, tip past the activation
height, harbour active — shaped like `defcon1_enacted` so its truth table is testable without an ASM.

The two traps are [Constraint 2](./security-council-defcon-3.md#2-defcon-3-enactment-cannot-reuse-defcon-1s-seqno-equality)
(`>=`, never `==`, or a successfully enacted proposal is marked `Superseded`) and
[Constraint 3](./security-council-defcon-3.md#3-a-cancelled-defcon-3-must-never-be-reported-as-enacted)
(leaving the queue is not evidence of enactment). Both are stated in the contract; the phase spec's
job is to say where each term is read from and to leave the residual out-of-band case recorded rather
than silently resolved.

Also in this phase: the stale comment in `application/proposals.rs` promising that "V2 adds a Defcon 3
branch to it".

**This precedes the create flow**, for the same reason V1's Phase 4 did: never let a signer create a
proposal that can only park at Approved forever.

**Tests.** One truth-table row per meaningful case, with the two traps as tests carrying their own
names — a test called `defcon3_enacted_when_a_later_action_consumed_the_seqno` is the documentation.
**Not tested:** an ASM-backed integration test inside `orchestrator-be`. It would be the flakiest
test in the repository, and `e2e_defcon_probe.rs::run_defcon3` already proves the chain behaviour the
predicate encodes.

### Phase 5 — Frontend: create and sign

The builder command registered in **both** lists in `invoke.rs`; `security_council` offering
`['defcon_1', 'defcon_3']`, which also makes Defcon 1 the council's default deliberately rather than
by accident; the `DEFCON 3` validator; and the parameterized form variant with its own destructive
copy and its own safe-harbour wording.

**The signing message needs no code and must not get any.** It resolves through the same Rust
renderer the device signs over. It gets exactly one Rust tripwire: the Defcon 3 message is non-empty
and **differs** from Defcon 1's — an upstream change that rendered them identically would otherwise
be discovered on a signer's screen.

**Tests.** The builder (build → decode → `Defcon3`); the signing-message tripwire; pure TS for the
**mutual exclusion** of the two confirmation strings, which is the one property parameterizing the
component could break silently; and the per-authority menu and default.
**Not tested:** the form component and the sign view. No DOM runner. The honest substitute is the
manual walk, budgeted below — V1 records that running the flow found what reviewing it did not.

### Phase 6 — Frontend: the queued lifecycle

Mostly **pinning behaviour that is already right** rather than changing code:
`showsActivationCountdown` already returns true for `defcon_3`, and `proposalDisplayStatus` already
reserves `quorum_reached` for `defcon_1` alone — which is exactly what the PRD wants, since Defcon 3
has a real Approved state.

One genuine fix: the cancel screen decides the countdown on `status === 'approved'` alone, out of step
with the shared predicate. It moves onto `showsActivationCountdown`.

**Tests.** Pure TS on `proposal-status.ts`: the countdown shows for `defcon_3` and stays hidden for
`defcon_1`; an approved `defcon_3` displays as `approved`, not `quorum_reached`.
**Not tested:** any screen, for the reason given throughout.

### Phase 7 — The cancel, end to end

Mostly verification plus small gaps. `build_cancel_action_hex` requires a non-null queue `UpdateId`,
which a queued Defcon 3 has; `create_cancel_proposal` stores the cancel under the target's authority
and requires the session to match, which for a Defcon 3 is the council itself. No new backend code is
expected — and if the phase discovers otherwise, that discovery is the phase's most valuable output.

**It discovered otherwise, and not in the backend.** The bet held for AC 11 and AC 12: the phase
added no `orchestrator-be` code at all. It failed for AC 14. The offline route refuses any hex whose
decoded kind is `unknown`, and a `MultisigAction::Cancel` decoded to exactly that because the
desktop's domain `Action` enum has no `Cancel` variant — so on the one route built for *"the
orchestrator is unavailable"*, a council signer could not import, let alone aggregate or broadcast, a
Defcon 3 cancel. See [the phase spec](./security-council-defcon-3-phase-7.md) §4.1 and §6.

The deliverable is the e2e: `run_defcon3_canceled` in `e2e_defcon_probe.rs`, following the shape of
`e2e_cancel_proposal.rs` — submit a Defcon 3, assert queued with the harbour off, submit a
council-signed cancel of its update id, take the tip past the height the Defcon 3 would have
activated at, and assert the queue is empty **and the harbour is still off**. That last assertion is
the only automated coverage of
[Constraint 3](./security-council-defcon-3.md#3-a-cancelled-defcon-3-must-never-be-reported-as-enacted).

**Anti-flake:** reuse the existing `bitcoind`-availability skip. *"Mine an exact depth"* turned out
to be the wrong instruction — `submit_and_mine_tx` mines a variable number of blocks — so
`submit_council_action` returns the measured reveal height and the test computes the activation
height from it. Never sleep.
**Not tested:** the desktop cancel journey. Manual walk.

### Phase 8 — Reserve

V1 needed two phases nobody planned (#511, #512) plus four close-out PRs, all of them born from
running the flow by hand rather than from reviewing it. Budget one and expect it to be about copy and
about a lifecycle state nobody predicted.

## 5. Verification

Per phase: the `AGENTS.md` checklist, plus evidence that the acceptance criteria the phase claims to
close are covered by tests.

**Every commit that adds a frontend test file confirms CI picks it up.** CI now globs
`src/**/*.test.ts(x)` rather than enumerating scripts — V1 shipped a phase where 21 of 62 test
scripts never ran, twice — so the check is that the new file falls inside the pattern.

End to end, once all seven land, on regtest with the local stack
(`./scripts/local-stack.sh --clean` if any state predates the ASM pin bump):

1. A council signer reaches the Defcon 3 form; every other authority sees no entry point to it.
2. The rendered four-line message matches the signer's screen, has no `Action Details:` block, and
   differs from Defcon 1's.
3. Quorum, broadcast — the proposal shows Approved, then Awaiting enactment with a countdown to the
   right block, and the harbour stays off.
4. Path A: mine `depth` blocks → `Enacted`, harbour on.
5. Path B: cancel inside the window → the target reads `Canceled`, the harbour stays off, and nothing
   reads `Enacted`.
6. A Defcon 1 created in the same session still shows no countdown and no cancel affordance.
7. `cargo test -p alpen-multisig-e2e-tests` green, including `run_defcon3_canceled`.

## 6. Known debt this slice does not take

Swept on `chore/sweep-recorded-debt` (not repeated here): shared HTTP client across RPC calls in
both processes; `reconcile_reveal_confirmed_facts` on proposal GET; `scripts/asm-params.example.json`
deserializes against the pin (with a substituted-placeholder test); honest `kind` on the offline
route (Phase 7's compensating `actionType === 'cancel'` label arm removed).

Still open — each wants its own slice:

- **A proposal that enacted but reads `Superseded` drops out of the redundancy answer.**
  [`proposal-lifecycle-seqno-truth.md`](./proposal-lifecycle-seqno-truth.md) §4.1 records a residual
  ambiguity: a proposal that enacted while nothing was reading, and was then jumped past by a later
  action, resolves as `Superseded` rather than `Enacted`. `changedNothingActionIds` filters on
  `status === 'enacted'`, so such a proposal leaves the candidate set, the next one is named the
  activator, and its badge is lost. Same failure shape as a null activation height — an activation
  that is not in the set — reached by a different route. Pre-existing and unchanged by Phase 2: V1
  filtered on the same status. Recorded because Phase 2 is what made the two routes visible as one
  family, and because the fix belongs where the label is decided, not in the badge.
- **A stored `activation_height` can go stale.** It is `reveal_block + lock_period` with the depth
  read **live at reveal-confirmation time**, so changing `confirmation_depths.defcon3` while an
  update is queued leaves a height the chain no longer agrees with. Pre-existing — the activation
  countdown already trusts the same field.
- **N+1 `strata_asm_getStatus` reads in the reconciliation loop**, recorded at V1 close-out. Phase 3
  adds a per-request depth read and is the natural place to revisit it, but hoisting the whole loop
  requires restructuring mocks keyed by RPC URL and is not this slice's to carry.
- **Cancelability has no third wire state when the ASM is down.** Listing succeeds and the
  affordance collapses to "no" ([phase 3](./security-council-defcon-3-phase-3.md),
  [phase 6](./security-council-defcon-3-phase-6.md)); distinguishing unknown from false needs a DTO
  change of its own.

## 7. Close-out

Four places do not update themselves, and V1 needed a follow-up PR to fix exactly this drift: the
`Status:` header of [`security-council-defcon-3.md`](./security-council-defcon-3.md), the header of
[`security-council.md`](./security-council.md), its §6 Stage board and its §7 Slice board — where V2
now also carries the absorbed V5 row.
