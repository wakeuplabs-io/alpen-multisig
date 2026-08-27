# Security Council — Defcon 1 (V1), Phase 6: Frontend lifecycle

**Functional contract:** [`security-council-defcon.md`](./security-council-defcon.md) — SSOT for
*what* V1 must do. This document never overrides it.

**Build plan:** [`security-council-defcon-implementation.md`](./security-council-defcon-implementation.md)
§4 Phase 6. This document is that phase at implementation detail.

**Closes:** AC 6, AC 7, AC 9, AC 10, AC 13, AC 15/15a/15b, AC 16.

## 1. The change in one sentence

After Phase 5 a Defcon 1 proposal exists and carries a signature; this phase makes every screen that
handles it afterwards tell the truth about it — it never reads *Approved*, it never counts down to an
activation it does not have, it is never offered a cancel that would fail on chain, and the offline
path names it, accepts the export the app told the signer to make, and keeps the raw transactions a
signer is left with when nothing else works.

## 2. What the acceptance-criteria list hides, in both directions

Phase 6 claims eight acceptance criteria, which reads like the largest phase in the slice. Half of
them need **no new code**: the lifecycle screens are action-agnostic — `deriveProposalActions`,
`proposalSendState`, the Pending/Past tabs and the commit/reveal pipeline were all built without
knowing which action they carry, so a Defcon 1 flows through them the moment Phase 5 lets one exist.
Writing Defcon-shaped copies of code that is already correct is the failure Phase 5 avoided by
refusing a sibling create screen, and §3 is the audit that says which criteria those are, with
evidence, instead of pinning `proposalSendState` a second time.

The other direction is the one that cost more. Auditing them turned up **seven** defects, and three
are not Defcon-specific at all — the offline route rejects the app's own clipboard export (§4.5),
discards the raw transaction hex it exists to hand over (§4.6), and labels every imported action by
guessing at a hex prefix (§4.4). They are in scope because AC 15/15a/15b are Phase 6's to close and
because the offline path is the fallback the whole slice leans on for an irreversible action.

Two of the seven were found only after this document had been written and reviewed: the cancel gate
has three copies rather than one (§4.3), and AC 15b was marked *"already offered"* on no evidence
(§4.6). Both are recorded in place rather than quietly corrected, because the pattern — a claim with
nothing under it, and a rule duplicated until one copy is forgotten — is the same one Phase 5 ended
with.

## 3. What already holds, and the evidence

| AC | Claim | Where it already holds | Verdict |
|---|---|---|---|
| 6 | Quorum reached → the send control enables | `proposalSendState` (`lib/proposal-send-state.ts:69`) returns `ready` on `approved` + `broadcastStatus === 'idle'`, for any action; both screens read it (`proposals-dashboard.tsx:424`, `proposal-detail.tsx:90`) | No code |
| 7 | Broadcast builds and transmits commit + reveal | The `/proposals/:id/broadcast` screen and `broadcast-details-card.tsx` never branch on the action; Phase 5 fixed the one authority-keyed read the send path had (`src-tauri/.../asm_role_membership.rs` `ordered_keys_for_authority`) | No code |
| 13 | Seven-day expiry applies normally | The backend marks a proposal `expired` (`orchestrator-be/src/application/proposals.rs:527-544`) and the Tauri DTO mapper computes `expiresAtMs` as `created_at_ms + PROPOSAL_EXPIRY_DAYS` (`src-tauri/src/commands/proposals.rs:205`); both are action-agnostic, as are `PendingExpiryCountdown` and the `expired` style (`proposal-status.ts:46`) | No code |
| 16 | Enacted or expired proposals land in *Past* | The dashboard buckets on `status` alone (`proposals-dashboard-screen.tsx:65-69`) | No code |
| 15 | Collected signatures can be exported | `Copy bundle` / `Download bundle` on the detail screen serialise the whole `Proposal`, signatures included (`proposal-detail.tsx:102-114`) | Export holds; **the clipboard half is not consumable on `/manual`** — §4.5 |
| 15a | The export broadcasts through `/manual` | `processBundle` (`use-manual-proposal.ts:241`) accepts the downloaded proposal JSON — it reads the four fields it needs and ignores the rest — and `security_council` is already in its `AUTHORITIES` list (`:24`) | Display defect only, §4.4 |
| 15b | The raw transaction can go to an external RPC | **Half true.** The *Send manually* panel — the raw commit and reveal hex, each with a copy button and a `sendrawtransaction` instruction — exists at `broadcast-phase-progress.tsx:138-147` and is rendered by `/proposals/:id/broadcast` and the cancel broadcast screen. `/manual`, the route AC 15b names, throws the same structured error away and prints its raw string (`manual-proposal-screen.tsx:332`) | §4.6 |
| 9 | Never the word *Approved* | **Fails.** `PROPOSAL_STATUS_STYLE.approved.label` is `'Approved'` and is keyed on status alone | §4.1 |
| 10 | No cancel affordance anywhere | **Holds by accident, in three places.** `CANCELABLE_AUTHORITIES = ['alpen_admin', 'strata_admin']` is declared three times — the dashboard card, the detail screen's *Cancel this proposal* button, and the cancel route's own redirect guard — and every one of them is the authority-shaped gate [Constraint 2](./security-council-defcon.md#2-cancelability-is-decided-per-action-and-per-live-depth-never-by-authoritysecuritycouncil) forbids | §4.3 |

The two "no code" columns are not an invitation to skip verification: §10 walks the flow. They are a
refusal to add tests that would pin `proposalSendState` a second time.

## 4. The seven defects

### 4.1 A Defcon 1 at quorum reads *Approved* (AC 9)

`PROPOSAL_STATUS_STYLE` is a `Record<DisplayStatus, StatusStyle>` — the label is a function of the
status and nothing else, so every proposal that reaches `approved` renders the word PRD 06 §5.2.2
carves Defcon 1 out of. Both screens hit it: `proposals-dashboard.tsx:475` and
`proposal-detail.tsx:128`.

**The fix is a display status, not a second table.** `DisplayStatus` already carries one UI-only
refinement of `approved` — `awaiting_enactment` — and this is the second one. Add
`quorum_reached` beside it and one function that resolves a proposal to its display status:

```ts
export function proposalDisplayStatus(proposal: DisplayStatusInput): DisplayStatus
```

with the rule: `approved` + `reveal_confirmed` → `awaiting_enactment` (unchanged, and it applies to
Defcon 1 too — *Awaiting enactment* is accurate and says nothing forbidden); otherwise `approved` +
`actionType === 'defcon_1'` → `quorum_reached`; otherwise the backend status verbatim.

**This also removes a duplication.** The two screens each derive `awaiting_enactment` themselves
today, by different expressions that happen to agree — `sendState.kind === 'confirmed'`
(`proposals-dashboard.tsx:425`) and an inline conditional (`proposal-detail.tsx:92-95`). The
carve-out has to land in both, so it lands in one place they both read, which is what the status
table itself was extracted for (#416).

**The badge reads `Quorum reached`, and the contract's AC 9 is corrected to say so.** AC 9 names the
string *"Quorum reached — ready to broadcast"*. Three reasons the badge does not carry it verbatim:

1. The status badge is `whitespace-nowrap` and sits beside the proposal title on a card. A 34-character
   badge is not a badge; it reflows the card header for one action type, which is a worse signal
   than the word it replaces.
2. **The app's verb is *Send*, not *Broadcast*.** Every control and every stage label was reworded
   that way (`proposal-send-state.ts`, `sendButtonLabel`, the `Send` CTA). Reintroducing *broadcast*
   in one badge would make Defcon 1 the only proposal whose status names a control that does not
   exist on screen.
3. The sentence is not lost. The detail screen renders *Quorum reached — ready to send* next to the
   badge on any quorum (`proposal-detail.tsx:153`), and the dashboard renders it on the card whose
   Send button is live (`proposals-dashboard.tsx:524`). On a dashboard card that has quorum but
   cannot yet send, the line is the shorter *Quorum reached* (`:577`) — so the pairing is the rule,
   not a universal, and the badge is what is constant.
4. *Quorum reached* is not a phrase invented here. It is the app's existing name for this moment:
   the dashboard's own group heading (`proposals-dashboard.tsx:248`) and the post-signature modal's
   title (`sign-screen.tsx:302`) both already read it. The badge joins a vocabulary rather than
   starting one.

Constraint 3 grants exactly this latitude — *"something like 'Quorum reached — ready to broadcast',
never the word 'Approved'"* — and AC 9 is the stricter restatement of it. The contract is corrected
in the back-propagation commit, the way Phase 3 corrected AC 3 and Phase 5 corrected *Create Form
Layout*. **The non-negotiable half is unchanged and is what the test pins: the word *Approved* never
appears for a Defcon 1 in any state.**

The palette is the `approved` palette. The carve-out is about the word, not the colour: Defcon 1 at
quorum occupies the same lifecycle position as any other approved proposal, and giving it a
different colour would spend a visual signal on a distinction the signer does not have to act on.
The destructive palette stays where Phase 5 put it — the creation form's `Irreversible` callout and
the type-to-confirm gate.

### 4.2 A Defcon 1 counts down to an activation it does not have (AC 9, signer safety)

The detail screen renders `ActivationCountdown` whenever `activationHeight !== null && status === 'approved'`
(`proposal-detail-screen.tsx:173`). The backend stores an activation height for **every** proposal —
`compute_and_store_activation_height` writes `reveal_block + lock_period` (`proposals.rs:638`) — and
Phase 1 resolves Defcon 1's lock period to `0`. So a broadcast Defcon 1 renders

> ⏱ Activation in block 850,123 · current block 850,123 · imminent

which describes a delay that does not exist, on the one action whose entire point is that it applies
in the block it lands in. *imminent* is the countdown's zero-case wording, not a statement that there
is nothing to wait for.

**Suppress the countdown for `defcon_1`.** Not for the authority: Defcon 3 shares the authority,
carries a real configurable depth, and must keep the countdown when V2 ships. Keyed on the action,
which is [Constraint 1](./security-council-defcon.md#1-lock-period-is-per-action-never-per-authority)'s
rule applied to the display half.

The frontend cannot read the depth — it has no ASM handle and the proposal DTO carries the resolved
height, not the period — so `actionType === 'defcon_1'` is the available expression of "this action
has no lock period". Recorded here so a later reader does not mistake it for an authority check in
disguise.

**It is a predicate in `lib/`, not a JSX condition.** The rule is this phase's only signer-safety
claim, `npm run build` cannot catch its deletion, and the screen is the wrong owner for it —
`showsActivationCountdown` sits beside `proposalDisplayStatus`, so both answers to "what does a
Defcon 1 show after quorum" are in one file and both are pinned by the same test.

The cancel route renders the same countdown under the same condition
(`cancel-proposal-screen.tsx:119`). It needs no change of its own: §4.3 makes that route unreachable
for a Defcon 1, so the countdown there is dead code for this action rather than a second copy of the
defect.

### 4.3 The cancel affordance is gated on the authority — in three places (AC 10)

AC 10 holds today only because `security_council` is missing from a list of two authorities. That is
the wrong reason for a correct outcome, and it is the exact shape
[Constraint 2](./security-council-defcon.md#2-cancelability-is-decided-per-action-and-per-live-depth-never-by-authoritysecuritycouncil)
was written to forbid — the backend gate was rewritten in Phase 2 for the same reason, and the
desktop kept its copy, the way it kept the authority→role mapping Phase 5 had to chase down.

It kept three copies. `const CANCELABLE_AUTHORITIES = ['alpen_admin', 'strata_admin']` is declared
verbatim in three files:

| Where | What it gates |
|---|---|
| `proposals-dashboard.tsx:21` → `:554` | the *Cancel* button on a card awaiting enactment |
| `proposal-detail-screen.tsx:22` → `:200-211` | a full-width **Cancel this proposal** button — the affordance AC 10 names first, *"not in the proposal detail screen"* |
| `cancel-proposal-screen.tsx:21` → `:52` | a redirect guard on `/proposals/:id/cancel` |

The cost is paid by V5, not by this phase: when Defcon 3 gains a cancel flow the allow-list gains
`security_council`, and on that day every Defcon 1 grows a Cancel button on two screens *and* an
open cancel route, all refused by the backend with a depth error the signer cannot act on.

**The third row also corrects an assumption worth stating, because the first draft of this document
got it wrong.** The cancel route is not unguarded and reachable only by typing a URL: it is guarded,
by the wrong predicate. So the choice is not "add a client-side copy of the depth rule or don't" —
the copy is already there, and the work is to give it the term it is missing.

**`deriveProposalActions` gains `canCancel`, and all three sites read it.** It is already documented
as the "single source of truth for signer-facing action availability" and already answers `canSign`
and `canBroadcast`; the allow-list moves in beside them and gains its action term:

```ts
canCancel = CANCELABLE_AUTHORITIES.includes(authority) && actionType !== 'defcon_1'
```

`ProposalActionInput` widens by two fields it can read straight off the real `Proposal`. The three
constants collapse into one, which is the other half of the fix: a rule with three copies has three
chances to be given the term and two chances to be missed.

At the two button sites the surrounding conditions stay where they are — `proposal.cancelProposal === null`
and `kind !== 'cancel'` are about the cancel that already exists, not about whether one may be
started.

**What this still does not do:** it does not make the client the enforcement point. `create_cancel_proposal`
refuses a zero-depth target with a depth-shaped error (Phase 2) and stays the thing that decides.
The client-side term exists because AC 10 is about affordances, and an affordance that only fails at
the backend has already misled the signer.

### 4.4 The offline path guesses the action from a hex prefix (AC 15a)

`ManualSignCollect` builds a synthetic `Proposal` for reuse of the detail layout, and derives its
`actionType` like this (`manual-sign-collect.tsx:28-32`):

```ts
function derivedActionType(actionHex: string): ActionType {
	const h = actionHex.toLowerCase()
	if (h.startsWith('01')) return 'vk_update'
	return 'multisig_update'
}
```

A Defcon 1 bundle is not either of those, so the offline screen — the one a signer reaches precisely
when the orchestrator cannot tell them what they are holding — labels the bridge's emergency lever
*Signer update* or *Verification key update*. It is a wrong-action display, which is the same class
of failure Phase 5 refused to accept in `create-proposal-preview.tsx`.

**The decode already happened.** Both import paths call `decodeActionHex` — Rust, the same decoder
the sign screen uses — and both reject `unknown` before proceeding (`use-manual-proposal.ts:193`,
`:270`). The kind is in hand and then thrown away. `ManualImportData` gains `actionType`, set from
that decode, and `derivedActionType` is deleted.

**The carry is not a plain assignment, and that matters.** `DecodedAction` has four kinds and
`ActionType` has seven (`api/proposals.ts:12-13`), so the two are bridged by a small mapping — and a
mapping is exactly where the next person writes `default: return 'multisig_update'` and puts the
guess back. It is written as an exhaustive `Record<DecodedAction['kind'], ActionType>` with no
default arm, so a fifth decoded kind is a compile error rather than a silent `multisig_update`, and
it is a pure function with its own assertion (§6, test 5).

This fixes `vk_update`'s cousin too: the heuristic returned `multisig_update` for
`operator_set_update` and `sequencer_key_update` as well. Those are not reachable through this path
today (`decodeActionHex` still returns `unknown` for them, so the import is refused before the
label matters) — noted so the change is understood as removing a guess, not as widening support.

### 4.5 The clipboard export is refused by the screen it is meant for (AC 15)

AC 15 asks that the copy-signatures action put every collected signature on the clipboard *"in the
format the manual path consumes"*. The app has the export — `Copy bundle` (`proposal-detail.tsx:102`)
writes the whole `Proposal` JSON, signatures included. Two importers can be handed it, and they
disagree:

| Importer | Accepts a bundle object? | Where |
|---|---|---|
| `ImportBundleModal` — paste into an existing proposal | **Yes.** An object with a `signatures` array is unwrapped, and `broadcastStatus`/txids/`status` are read off it too | `import-bundle-modal.tsx:64-68` |
| `PasteSignaturesModal` — paste into the `/manual` flow at step 2 | **No.** A non-array object is treated as a single signature row and fails with *"missing signerPubkey or signatureHex"* | `paste-signatures-modal.tsx:20-30` |

So a signer who follows the copy button to the offline screen — the exact sequence AC 15 and AC 15a
describe, and the one that matters when the orchestrator is unreachable — is told their own export is
malformed. The path that does work is *Download bundle* → drop the file on `/manual` step 1, because
`processBundle` reads the four fields it needs and ignores the rest (`use-manual-proposal.ts:241`).

**`parseSignaturesInput` unwraps `{ signatures: [...] }`,** which is what its sibling already does.
Four lines, in the same pure function the modal already routes every input through, and it makes the
two paste boxes in the app agree about what a bundle is.

Not Defcon-specific, and that is the point: AC 15 is Phase 6's to close, the export is the fallback
the whole slice leans on for an irreversible action, and the failure is one branch away from the
code that already handles it correctly. Widening the import is not the same as building the missing
*Enter manually* entry point (§7), which is a new affordance rather than a fix to an existing one.

### 4.6 The offline route drops the escape hatch it is the escape hatch for (AC 15b)

AC 15b: *"a composed Defcon 1 commit/reveal pair on the manual route ... the clipboard contains the
raw transaction hex, which is accepted by any external Bitcoin RPC"*.

The app builds that panel already. When every broadcaster fails, the Tauri layer returns a
structured error — `{ code: "broadcast_unavailable", commitTxHex, revealTxHex }`
(`src-tauri/src/commands/proposals.rs:549-563`, pinned by
`all_broadcasters_failed_maps_to_broadcast_unavailable_with_hexes`) — `deriveBroadcastError` parses
it (`broadcast-proposal/model/broadcast-proposal.ts:65-81`), and `BroadcastPhaseProgress` renders
**Send manually**: both raw hexes, each with a copy button, under a `sendrawtransaction` instruction
(`broadcast-phase-progress.tsx:138-147`).

`/manual` gets the same structured error — `proposals_broadcast_manual` ends in the same
`.map_err(map_broadcast_error)` (`src-tauri/src/commands/proposals.rs:1072`) — and then renders it as
a raw string (`manual-proposal-screen.tsx:332`). So the one route built for the case where the
orchestrator is gone is the one route that discards the hex a signer would need.

**The fix is frontend-only:** the manual hook parses the error with `deriveBroadcastError` instead of
storing a string, and the screen renders the existing recovery block. No new IPC, no `src-tauri`
change, no second copy of the panel.

Worth stating plainly: the first draft of this document marked AC 15b *"No code — the manual
broadcast step already offers it"*, with no `file:line` behind it. It was the only row in §3 without
evidence, and it was the only row that was wrong. The rule that produced the error is the one Phase 5
wrote down about `default:` arms — a claim with nothing under it is a claim nobody checked.

### 4.7 A decoded view can outlive the action it decoded

`useDecodedProposal` sets `signerSetChange` only when the decode is a `multisig_update`
(`use-decoded-proposal.ts:49`), and never clears it otherwise. The effect resets state when the
proposal is `null` (`:31-35`) and on no other path, so a hook instance that sees a signer update and
then a Defcon 1 keeps the first one's before/after table — and `deriveProposalTitle`
(`proposal-detail.tsx:52-69`) would title the Defcon 1 *"Add 2 signers"*.

Not reachable through today's navigation: every route into the detail screen mounts it fresh. It is
listed because it is the same class of defect as §4.4 — a view rendering an action it did not decode
— on the one action where a wrong title is worst, and because the fix is the `else` the effect is
missing.

## 5. Migration — six commits, each atomic

Each leaves the tree green and none repairs the one before it. A and B both edit the dashboard and
the detail screen, and are ordered so the badge lands before the affordance — they fail differently
and revert independently. C, D and E touch the offline path and are independent of both and of each
other. F is documentation.

**Every commit that adds a test script adds it to `.github/workflows/ci.yml` in the same commit.**
CI enumerates the frontend test scripts one by one (`:170-197`); a script in `package.json` alone
never runs there. Phase 5 nearly shipped that way.

**Commit A — the states after quorum read as Defcon 1's own.** AC 9, and §4.2.

| File | Change |
|---|---|
| `lib/proposal-status.ts` | `quorum_reached` in `DisplayStatus` and in `PROPOSAL_STATUS_STYLE`; `proposalDisplayStatus`; `showsActivationCountdown` |
| `lib/__tests__/proposal-display-status.test.ts` (new) | §6, tests 1–3 |
| `domain/proposals-dashboard/components/proposals-dashboard.tsx` | the **badge** reads `proposalDisplayStatus`. `awaitingEnactment` stays: it also selects the card-footer branch (`:543`), and deleting it would change the footer for every proposal in the app |
| `domain/proposal-detail/components/proposal-detail.tsx` | same, replacing the inline conditional |
| `screens/proposal-detail-screen.tsx` | the countdown reads `showsActivationCountdown` |
| `package.json`, `.github/workflows/ci.yml` | `test:proposal-display-status` |

**Commit B — cancel is refused by the action, not by the authority.** AC 10.

| File | Change |
|---|---|
| `domain/proposal-detail/model/derive-proposal-actions.ts` | `CANCELABLE_AUTHORITIES` moves here, once; `authority` and `actionType` on the input; `canCancel` on the output |
| `domain/proposal-detail/model/__tests__/derive-proposal-actions.test.ts` | §6, test 4 |
| `domain/proposals-dashboard/components/proposals-dashboard.tsx` | read `canCancel`; the local constant goes |
| `screens/proposal-detail-screen.tsx` | same, for *Cancel this proposal* |
| `screens/cancel-proposal-screen.tsx` | same, for the redirect guard |

**Commit C — no screen shows an action it did not decode.** AC 15a, and §4.7.

| File | Change |
|---|---|
| `domain/manual-proposal/model/manual-proposal.types.ts` | `actionType` on `ManualImportData` |
| `domain/manual-proposal/model/action-type-from-decoded.ts` (new) | the exhaustive `DecodedAction['kind'] → ActionType` map, no default arm |
| `domain/manual-proposal/model/__tests__/action-type-from-decoded.test.ts` (new) | §6, test 5 |
| `domain/manual-proposal/hooks/use-manual-proposal.ts` | carry the decoded kind into `importData` on both import paths |
| `domain/manual-proposal/components/manual-sign-collect.tsx` | read it; delete `derivedActionType` |
| `domain/proposal-detail/hooks/use-decoded-proposal.ts` | the missing `else`: clear `signerSetChange` when the decode is not a signer update |
| `package.json`, `.github/workflows/ci.yml` | `test:action-type-from-decoded` |

**Commit D — the offline path accepts the export the app told the signer to make.** AC 15.

| File | Change |
|---|---|
| `domain/manual-proposal/model/parse-pasted-signatures.ts` (new) | `parseSignaturesInput` moves out of the modal so it can be tested without mounting; gains the bundle-object branch |
| `domain/manual-proposal/components/paste-signatures-modal.tsx` | import it |
| `domain/manual-proposal/model/__tests__/parse-pasted-signatures.test.ts` (new) | §6, test 6 |
| `package.json`, `.github/workflows/ci.yml` | `test:parse-pasted-signatures` |

**Commit E — the offline route keeps the raw transactions.** AC 15b, §4.6.

| File | Change |
|---|---|
| `domain/manual-proposal/hooks/use-manual-proposal.ts` | store `deriveBroadcastError(raw)` instead of the raw string |
| `screens/manual-proposal-screen.tsx` | render the existing *Send manually* block on `broadcast_unavailable` |

**Commit F — back-propagate the corrections into the contract.** Documentation only.

`security-council-defcon.md` names *"Quorum reached — ready to broadcast"* in three places, not one:
AC 9, the State Model's display-label list, and the *Lifecycle Display* wireframe. Correcting AC 9
alone would leave the contract contradicting itself twice. All three are corrected to the shipped
wording, with the reason recorded inline the way Phase 3 corrected AC 3 — and the non-negotiable
half, *never the word "Approved"*, is quoted unchanged in each.

The same commit marks Phase 6 ✅ in the build plan and records that the *Enacted* wireframe's block
number and safe-harbour line are not built (§8).

## 6. Tests

Six assertions, all pure, each pinned to a claim that can regress and none of them re-pinning generic
lifecycle machinery.

| # | Claim | Assertion | Where |
|---|---|---|---|
| 1 | AC 9: a Defcon 1 never reads *Approved* | `proposalDisplayStatus` on an approved `defcon_1` is `quorum_reached`, and the label of the resolved status contains no *Approved* in any of the four states | `lib/__tests__/proposal-display-status.test.ts` |
| 2 | The carve-out is Defcon 1's alone, and `awaiting_enactment` still wins | an approved `multisig_update` still resolves to `approved`; an approved `defcon_1` at `reveal_confirmed` resolves to `awaiting_enactment` | same file |
| 3 | §4.2: a depth-0 action shows no countdown | `showsActivationCountdown` is `false` for `defcon_1` with a height, `true` for another action with the same height, `false` with no height | same file |
| 4 | AC 10, against the V5 future | `canCancel` is `false` for a `defcon_1` **whose authority is in the allow-list**, and `true` for a `multisig_update` on the same authority | extend `derive-proposal-actions.test.ts` |
| 5 | §4.4: the decoded kind maps to an action type without guessing | every `DecodedAction['kind']` maps to its own `ActionType`, `unknown` included | `domain/manual-proposal/model/__tests__/action-type-from-decoded.test.ts` |
| 6 | AC 15: the copied bundle is accepted where the signer is told to paste it | `parseSignaturesInput` on a full proposal bundle yields its signatures; on a bare array it is unchanged; on an object with no `signatures` field it still errors | `domain/manual-proposal/model/__tests__/parse-pasted-signatures.test.ts` |

Tests 3, 4 and 5 exist because the reviewer of this document asked what would catch their deletion,
and `npm run build` was the honest answer in all three cases.

**Deliberately not tested, and why:**

- **The four "no code" criteria (6, 7, 13, 16) and 15.** They are properties of code this phase does
  not touch, and they are already exercised by the existing suite and by the flow walk in §10. A test
  written here would assert that `proposalSendState` still works, which
  `derive-proposal-actions.test.ts` and the broadcast model tests already do.
- **No render test of the dashboard or the detail screen.** Both are large components with router,
  session and Tauri dependencies; the claims that matter (which label, which affordance) were pushed
  down into pure functions precisely so they can be tested without mounting anything. Mounting them
  would pin the mock.
- **Commit E has no test.** It replaces one string with a parse whose parser is already pinned
  (`test:derive-broadcast-error`) and reuses a rendering block that is already on screen elsewhere.
  What is left is wiring, and a wiring failure is a failing `npm run build` or a visibly missing
  panel in §10 step 7.
- **§4.7 has no test.** It is the `else` an effect is missing, on a path today's navigation cannot
  reach; a test would have to simulate a route transition the router does not perform.
- **No WebDriver spec.** Same reasoning as Phase 5 §8 — the suite is run one spec at a time by hand
  and covers the wallet; the first proposal-lifecycle spec is its own piece of work.

## 7. Blast radius

- **`DisplayStatus` gains a member that no backend status maps to.** `quorum_reached` joins
  `awaiting_enactment` as UI-only. Anything doing an exhaustive `switch` over `DisplayStatus` becomes
  a compile error until it handles it — there is one such place (`PROPOSAL_STATUS_STYLE`) and it is
  in the same commit.
- **Commit A changes what *every* proposal's badge is derived from**, not only Defcon 1's. The
  resolution is behaviour-preserving for the other four action types by construction, and test 2
  pins that; it is called out because a regression here is visible on every card in the app.
- **`ProposalActionInput` gains two required fields.** There are two production callers, both
  passing a real `Proposal` (`proposals-dashboard.tsx:423`, `proposal-detail.tsx:89`), plus the test
  fixtures, which widen. The manual path reaches them indirectly: `manual-sign-collect.tsx:50-69`
  builds a synthetic `Proposal` that `ProposalDetail` then passes in, so once Commit C gives that
  synthetic proposal a real `actionType` it answers `canCancel` correctly rather than by accident.
- **No orchestrator-be change and no `src-tauri` change.** The backend half shipped in Phases 1–4;
  the IPC half shipped in Phase 5. Commit E in particular is frontend-only: the structured error it
  starts reading is already emitted (§4.6).
- **Commit B collapses three declarations into one import.** That is the fix, not a side effect — but
  it means a regression in `deriveProposalActions` now reaches three screens instead of one. Test 4
  is what stands under it.
- **`onManualExecute` stays a dead prop.** `ProposalDetail` receives it, destructures it as
  `_onManualExecute` and renders no control — so the `/manual` prefill the detail screen builds
  (`proposal-detail-screen.tsx:149-161`) is unreachable. This predates the slice: the manual spec
  asked for an *Enter manually* button on the dashboard
  ([`manual-execution-flow.md`](./manual-execution-flow.md):177) and it was never built either. AC 15a
  is satisfied without it — *Download bundle* → `/manual` from the connect screen → drop the file —
  so wiring a new control is a change to the manual flow's UX for all five authorities, which
  belongs to that spec and not to a Defcon phase. Recorded so the next reader knows it was seen.

## 8. Out of scope

- **Phase 7** — the safe-harbour banner, the per-proposal enactment predicate, AC 18–20.
- **A route guard on `/proposals/:id/cancel`** (§4.3).
- **The `Enter manually` entry point** (§7).
- **The contract's *Enacted* wireframe panel.** *Lifecycle Display* draws an enacted proposal as
  `Status: Enacted · Block: 850,123` with a `Safe harbour activated: ✓` line
  (`security-council-defcon.md:265-270`). Neither fact is rendered today and neither is required by
  an acceptance criterion — AC 8 is about the backend detecting enactment, not about the screen
  reporting how. The safe-harbour half is unbuildable here in any case: `grep -rn "safe_harbour" desktop-app/`
  returns nothing until Phase 7 adds the read, and Phase 7 spends it on the dashboard banner and the
  create-form warning, which are the two places a signer can still act on it. The block number is a
  standalone addition to the detail screen with no criterion behind it. Recorded as a known
  divergence between the wireframe and the shipped screen rather than silently dropped.
- **The raw authority string on two cards.** `proposal-detail.tsx:125` and
  `broadcast-details-card.tsx:114` print `proposal.authority` verbatim — `security_council`, not
  *Security Council*. AC 14 is about the header badge, which both screens render correctly
  (`proposal-detail-screen.tsx:91-94`, `broadcast-proposal-screen.tsx:68-71`), so the criterion is
  met; the raw string is a pre-existing inconsistency across all five authorities and its fix is a
  `authorityLabel` pass, not a Defcon change.
- **Removing `CANCELABLE_AUTHORITIES` entirely.** It is display data whose replacement is the live
  per-action depth, which the desktop cannot read. V5 owns that; this phase only stops it from being
  the *only* term.

## 9. Verification

Per commit, the full [`AGENTS.md`](../../AGENTS.md) pre-commit checklist:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd desktop-app && npm run format:check && npm run lint && npm run build
```

plus this phase's scripts:

```bash
cd desktop-app
npm run test:proposal-display-status
npm run test:derive-proposal-actions
npm run test:action-type-from-decoded
npm run test:parse-pasted-signatures
npm run test:derive-broadcast-error
```

Structural evidence, chosen so it can actually fail. A grep for `'Approved'` would not: the string
appears once before this phase and once after, in `PROPOSAL_STATUS_STYLE`. What must hold instead is
that no screen decides a badge or a cancel affordance for itself:

```bash
grep -rn "StatusBadge status=" desktop-app/src        # every hit passes proposalDisplayStatus(...)
grep -rn "CANCELABLE_AUTHORITIES" desktop-app/src     # one declaration, in derive-proposal-actions.ts
grep -rn "derivedActionType" desktop-app/src          # nothing
```

## 10. Manual walk (what §3 leaves to the flow)

Against the local stack, continuing from the proposal Phase 5 §11 created:

```bash
./scripts/local-stack.sh --clean   # only if the genesis predates Phase 5's council key
cd desktop-app && npm run tauri dev
```

1. **AC 6.** Sign the Defcon 1 with the second council mnemonic (`…jar`) until quorum. The card moves
   to the *Quorum reached* group, the badge reads **Quorum reached** and never *Approved*, and *Send*
   appears.
2. **AC 7.** Send. The commit and reveal go out through the existing pipeline; the badge moves to
   *Awaiting enactment* once the reveal confirms, and **no activation countdown is rendered**.
3. **AC 10.** In every state above, and on the detail screen, there is no Cancel control and no
   cancellation-signatures block.
4. **AC 16.** After the ASM applies it, the proposal reads *Enacted* and appears under **Past**.
5. **AC 15/15a.** *Download bundle* on the detail screen, then `/manual` from the connect screen and
   drop the file: the imported proposal reads **Defcon 1**, not *Signer update*, and its signatures
   are the ones that were collected. Then *Copy bundle* and paste it into the same screen's
   **Paste signatures** box: it is accepted, where before it read as malformed.
6. **AC 15b.** Stop the Electrum and Bitcoin endpoints, then send from `/manual`: the failure shows
   **Send manually** with the commit and reveal raw hex and a copy button on each, instead of a bare
   error line.
7. **AC 13.** Not walkable in a session — the 7-day expiry is wall-clock. Covered by the backend's
   expiry handling, which is action-agnostic and predates this slice.
