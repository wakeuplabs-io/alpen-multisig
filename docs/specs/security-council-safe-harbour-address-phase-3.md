# V4 Phase 3 — What the manual walk exposed

> **Functional contract:** [`security-council-safe-harbour-address.md`](./security-council-safe-harbour-address.md)
> — SSOT for *what* V4 must do. This document never overrides it.
> **Build plan:** [`security-council-safe-harbour-address-implementation.md`](./security-council-safe-harbour-address-implementation.md)
> §4 Phase 3, which reserved this phase without contents: "expect it to be about copy — how 'safe
> harbour' reads to someone who has not read the PRD — and about how the destination is displayed."
> Both halves turned out to be right, and one of them is not cosmetic.
> **Ticket:** [#547](https://github.com/wakeuplabs-io/alpen-multisig/issues/547).
> **Predecessors:** [Phase 1](./security-council-safe-harbour-address-phase-1.md) (#548),
> [Phase 2](./security-council-safe-harbour-address-phase-2.md) (#549).
> **Source:** the manual walk on regtest, 2026-09-10 — three runs: an enactment, a cancel followed
> by an enactment, and a rotation submitted with the harbour already up.
> **Status:** implemented; automated checks green. A walk over this phase's own surfaces is pending.

## 1. The change in one sentence

The one path this slice exists to describe — a rotation the bridge accepts and discards — is the one
path the app explains wrongly, and it explains it wrongly because the administrator's dashboard never
reads the harbour at all.

## 2. What the walk confirmed

Recorded because a phase that only lists defects misreports the walk. Working, on a real chain, with
no code in this phase touching them: the menu entry under Strata Administrator; the current
destination and its descriptor on the form; the refusal of the destination already installed; the
signing message resolved from the Rust renderer; quorum, broadcast, `Approved`, `Awaiting enactment`
with a countdown to `reveal + 30`; `Enacted` on path A; `Canceled` on path B with the destination
untouched; the before/after on the detail and cancel screens that Phase 2 added; and the note on the
create form, the preview and the sign view on path C.

Three checks in the contract's `## Verification` were not exercised in this walk and stay open for
the next one: that a Security Council session cannot reach the entry (item 1), the two specific
refusals for a P2WPKH address and for an address of another network (item 3 — the distinct messages
exist at `desktop-app/src-tauri/src/domain/action.rs:141-147` and have tests, but no one has read
them off the screen), and the manual bundle round-trip (item 10).

## 3. The dashboard does not read the harbour for the authority that rotates it

This is the root the three copy defects below grow out of, so it comes first.

`screens/proposals-dashboard-screen.tsx:34-36` reads:

```ts
// The council only: no other authority has a lever that answers a bridge-wide state, so no
// other session reads it either.
const isCouncil = selectedRole === AuthRole.StrataSecurityCouncil
const safeHarbourActivated = useSafeHarbourActivated(isCouncil)
```

That comment was true when V2 wrote it and V4 falsified it. The administrator now has an action
whose entire outcome is decided by that bridge-wide state — and on the administrator's dashboard the
flag is not read, so it is `false` for every safe harbour rotation any signer will ever look at.

The gate widens to both authorities that have something to say about the harbour. It does not widen
to *all* sessions: the rule the comment encodes — read it only where a lever answers it — is right,
and the fix is that V4 added a second such lever, not that the rule was wrong.

## 4. `Superseded` tells the signer the one thing that did not happen

**The most important item in this phase.** On path C the walk produced proposal #5 in `Superseded`,
which is correct, above this detail (`lib/proposal-send-state.ts:76-79`):

> This transaction was mined, but **another action had already used its sequence number**, so the
> ASM did not apply it. […] a replacement has to be created and signed.

No other action used it. *This* action used it. It was mined, the ASM accepted it, the signature
verified, the queue entry drained, and `SafeHarbour::update_address` refused the change and returned
a boolean the bridge subprotocol discards — the behaviour
[Constraint 1](./security-council-safe-harbour-address.md#1-a-rotation-with-the-harbour-already-activated-is-accepted-and-changes-nothing)
is written about, and the reason `safe_harbour_address_enacted` decides on the destination rather
than on the sequence number (`orchestrator-be/src/infrastructure/asm_enactment.rs:294-320`).

So the backend knows the difference and the screen does not. Worse, the advice inverts: told a rival
action beat them, a signer creates a replacement, collects a quorum, pays commit and reveal fees
again, and the bridge discards that one too, because the harbour is still up and nothing in the
protocol takes it down.

### 4.1 The shape

`proposalSendState` gains a third superseded variant and one input, `harbourFrozeDestination`,
computed by the two callers rather than read inside — the module is pure and stays pure. The
condition is: the proposal's `actionType` is `safe_harbour_address_update`, and the harbour is
activated now.

**"Now" is sound here, and this is the one place it needs an argument.** The flag is set once and
there is no de-escalation upstream — the contract's Scope says so, and it is why V2's redundancy
badge and every `SafeHarbourNote` render site already read it live. A harbour that is up today was
up when the rotation was mined, or came up after; it cannot have gone down.

The residual ambiguity is the reverse case: a rotation genuinely superseded by a rival action, with
the harbour activated only afterwards. The attribution is then wrong — but the *advice* is not, and
the advice is what a signer acts on: a replacement really would be discarded. Today's copy is wrong
on both. Recorded in §8 rather than solved with a stored reason.

### 4.2 The copy

> **Superseded.** The safe harbour is already active, so the bridge's destination is frozen: this
> transaction was mined and the ASM accepted it, and nothing changed. The signatures are bound to a
> sequence number that is now spent, and a replacement would be discarded the same way while the
> harbour is up. The commit and reveal fees were spent.

It names the state, says what was lost, and — unlike both existing variants — does not prescribe a
replacement, because here a replacement is the wrong move.

`SUPERSEDED_BEFORE_CONFIRMATION` is untouched: a rotation that never confirmed cannot have been
swallowed, since being swallowed requires reaching a block.

## 5. The sign view is the one destination screen with no "before"

`domain/sign-proposal/components/sign-proposal-view.tsx:122-155` renders `New sweep destination`
alone. The preview shows `Replacing bcrt1p0xl…`; the detail and cancel screens show the two-column
Current/Proposed table Phase 2 built. The sign view shows neither.

This is Phase 2 §4's argument, one screen further on and pointed the other way. Phase 2 fixed the
detail screen because the co-signers never saw the create form. The sign view is where those same
co-signers actually commit, and it is the *least* informative of the three: the signer who chose the
destination sees the comparison, and the signers who did not, do not.

The fix is reuse, not new markup: `SafeHarbourAddressDetails` already reads the harbour, so it
switches from `useSafeHarbourActivated()` to `useSafeHarbour()` — one read that returns both the flag
and the installed destination — and renders `SafeHarbourChangeTable` over
`buildSafeHarbourChange({ installed, proposed, isEnacted: false })`. `isEnacted` is a constant here:
nothing enacted is ever signed.

`buildSafeHarbourChange` returning `null` when the destination cannot be read keeps its meaning —
no section rather than a lone address whose role is unstated — and the note above it still renders,
since the flag degrades to `false` independently.

The gating test at `screens/__tests__/safe-harbour-note-gating.test.ts` accepts `useSafeHarbour(` and
a `.activated` guard, so the switch satisfies it as written. That was designed in; this phase is the
first caller to use it.

## 6. A form error that outlives what it was about

Walk evidence, 10:24:56: a valid address in the field, the signing message resolved in full, and
below it, in red, **"Error: not a valid Bitcoin address"**.

`use-create-proposal.ts` clears `error` only at the start of the next submit or preview
(`:187`, `:225`), and `create-proposal-form.tsx:479` renders it until then. So a rejected preview
leaves its verdict on screen while the signer fixes the input — asserting the opposite of what the
same screen shows two fields above.

The error is cleared when the form changes, which is when the statement stops being about anything.
A `watch` subscription in the form calling a `clearError` the hook already has in `setError(null)`.

This is the second half of the defect Phase 2 §5 fixed. That one took the parser's words out of the
signing-message panel; this one is the same verdict, at the bottom of the same form, surviving the
edit that answered it.

## 7. Two lines of copy, and one dash

- **The sign view drops the consequence.** The create form and the preview end the note with "It
  will not report as Enacted." `sign-proposal-view.tsx:137` omits that sentence — on the last screen
  before the signature, which is where it matters most. The three surfaces say the same thing.
- **An untitled proposal previews as "—".** `create-proposal-preview.tsx:115` falls back to a dash
  where the app already knows the action's own label, which is what the rest of the app titles an
  untitled proposal with.

## 8. What the walk found and this phase does not take

**The expiry countdown is wrong by six days, and its warning fires from the first second.**
`desktop-app/src-tauri/src/config/mod.rs:3` hardcodes `PROPOSAL_EXPIRY_DAYS = 1` under the comment
*"Must match the orchestrator's PROPOSAL_EXPIRY_DAYS setting"*, while the orchestrator's default is
`7` (`orchestrator-be/src/config.rs:76-79`) and nothing checks the two agree. On top of that,
`components/pending-expiry-countdown.tsx:40` warns below 24 h — the entire lifetime under the
desktop's own constant — so every proposal is born reading "⚠ Expiring soon", which is why the walk
shows that badge on a proposal one minute old.

Not this slice's: it predates V4, touches both crates and every action type, and the honest fix is
to serve the expiry from the backend instead of duplicating the constant. Filed as its own issue.

**A stored supersession reason.** §4.1's ambiguity would close if the backend recorded *why* it
superseded a proposal, at the point where it knows. That is a domain field, a DTO field, two
repository implementations and a migration, to sharpen a case whose advice is already right. Not
now; recorded here so the next reader knows it was weighed.

## 9. Tests

Pure TypeScript, in the runner CI already globs (`src/**/*.test.ts(x)`):

- `proposal-send-state`: a superseded safe harbour rotation with the harbour up gets the frozen
  detail; the same rotation with the harbour down keeps the sequence-number detail — the pair that
  proves the new arm is conditional and not a relabelling; a superseded Defcon with the harbour up
  keeps the sequence-number detail, which is the tripwire against gating on the flag alone.
- `buildSafeHarbourChange` needs no new test — the sign view is a new caller of a covered function.
  The sign view's own wiring has no DOM runner, exactly as Phase 1 §4 recorded; the walk is the
  substitute.
- The note-gating test is expected to keep passing unchanged. If it does not, the change is wrong.

## 10. Migration — five commits

Each atomic, none repairing the one before it.

| # | Contents |
|---|---|
| 1 | The dashboard reads the harbour for the administrator too (§3). Inert on its own: nothing consumes the widened flag yet. |
| 2 | The frozen-destination superseded variant, its input and the two call sites, red then green (§4). |
| 3 | The sign view shows the change table and the full note (§5, §7 first item). |
| 4 | The form error is cleared when the form changes (§6). |
| 5 | The preview's untitled fallback (§7 second item), and the close-out of §11. |

## 11. Close-out, in this pull request

The build plan §7 records that V1, V2 and V3 each needed a follow-up pull request for exactly this
drift. It is avoidable here because Phase 3 is the last phase, so the four places move in commit 5:
the `Status:` header of the contract, the `Status:` header of the build plan, and in
[`security-council.md`](./security-council.md) the Stage board (§6) and the Slice board (§7).

They record what is true: V4's three phases implemented, the walk of 2026-09-10 run with its findings
closed, and the three verification items of §2 outstanding for a walk over this phase's own surfaces.
With V4 closed, Stage 6 — the compliance audit and issue #117 — is the only work left on the feature.

## 12. Verification

The `AGENTS.md` checklist plus `npm run test:unit`. Then, on the local stack, the four surfaces this
phase changes: an untitled proposal's preview, a rejected preview followed by a corrected address, a
co-signer's sign view for a rotation, and — with the harbour already up — the detail of a rotation
that reached `Superseded`, which is the sentence this phase exists for.
