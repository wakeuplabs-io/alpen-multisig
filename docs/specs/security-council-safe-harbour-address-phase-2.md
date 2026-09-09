# V4 Phase 2 — The cancel, the detail view, the message panel and the e2e

> **Functional contract:** [`security-council-safe-harbour-address.md`](./security-council-safe-harbour-address.md)
> — SSOT for *what* V4 must do. This document never overrides it.
> **Build plan:** [`security-council-safe-harbour-address-implementation.md`](./security-council-safe-harbour-address-implementation.md)
> §4 Phase 2. This document is that phase at implementation detail, and §9 records where it
> supersedes it.
> **Ticket:** [#547](https://github.com/wakeuplabs-io/alpen-multisig/issues/547).
> **Predecessor:** [Phase 1](./security-council-safe-harbour-address-phase-1.md), merged in #548.
> **Closes:** AC 6 (the countdown half), AC 7a, AC 9, AC 10, AC 13; and the two findings of the
> Phase 1 manual walk (§4, §5).

## 1. The change in one sentence

A queued safe harbour rotation can be cancelled; **every signer who has to approve one can see which
destination it installs**, not only the one who typed it; the signing-message panel stops shouting a
parser error at a half-typed address; and three paths run against a real regtest chain — including
the one where the bridge accepts the rotation and discards it.

## 2. What this phase is not

It is not new backend behaviour. §3 is a verification, and if it turns into code that discovery is
the phase's most valuable output.

It is not a change to the create flow's *logic*, the codec, the builder or the enactment predicate —
Phase 1 shipped those, and `action_codec.rs` and `asm_enactment.rs` have zero diff here. §5 changes
how a resolution failure is *presented*, and where.

It is not a protocol rule, and it does not make the resolved message a submission gate — Phase 1 §4.4
decided that deliberately and the manual walk did not disturb it.

## 3. The cancel is generic, and this phase proves it rather than writing it

Every gate a cancel passes through resolves from the action or from the target proposal, never from
a list of authorities. Read at the pin:

| Gate | Location | Why tx type 14 passes |
|---|---|---|
| `build_cancel_action_hex` | `commands/action_builder.rs:574-597` | Works at the upstream `MultisigAction` layer: `find_update_id_in_queue` matches the queued `UpdateAction` and `encode_cancel_hex_for_target` wraps it. Neither goes through the domain `Action`, so neither has a variant list. |
| `create_cancel_proposal` | `orchestrator-be/src/application/proposals.rs:765-810` | Stores the cancel under the **target's** authority (`strata_admin`) and requires the session to match. Correct here: the council must not be able to cancel a destination change it cannot author. |
| the depth gate | same, `:797-803` | Refuses a zero-depth action because it is never enqueued. Tx 14 carries depth 30 on the local stack, so the affordance exists exactly when the queue entry does. |
| cancel enactment | `asm_enactment.rs`, `MultisigAction::Cancel` arm | `admin.find_queued(target_id).is_none()` — no variant knowledge at all. |
| `isCancelable` on the DTO | `asm_role_membership.rs:187-193` | Derived from the live depth since V2 Phase 3. |

**So the deliverable here is tests, not code.** Two, both in `orchestrator-be`: a cancel of a tx-14
target is admitted for a `strata_admin` session, and refused for a `security_council` one
([AC 10](./security-council-safe-harbour-address.md#10-the-cancel-is-signed-by-the-strata-administrator)).
The second is the one that matters — it is the segregation invariant on the cancel path, where V3
found nothing pinned either.

## 4. The detail view has to say what changes, because that is where the quorum decides

**Found in the Phase 1 manual walk on regtest, 2026-09-09.** The proposal detail screen shows the
header, the signature progress and the approvals list. The destination appears nowhere — not the one
being installed, not the one it replaces.

Phase 1 put the destination in front of the signer who *creates* the proposal, in the form and in
the sign view. That is one signer. **Every other member of the quorum reaches this action through
the detail screen, and none of them ever saw the create form.** A second approver is being asked to
authorize where every bridge satoshi sweeps to, from a screen that does not name it. That is the
finding, and it is what turns this section from tidiness into the phase's most important frontend
work.

### 4.1 Why nothing is there today

`useDecodedProposal` resolves a `signerSetChange` and suppresses it when
`multisigUpdateTargetAuthority` answers `null` — which it does for tx 14, correctly: a safe harbour
rotation changes no signer set. Nothing is broken; there is simply nothing in its place. Three
surfaces inherit the gap:

1. `proposal-detail.tsx:159-166` renders no change section.
2. `deriveProposalTitle` (`:50-72`) falls through to `Proposal #<seqNo>` for an untitled rotation.
3. `cancel-target-summary.tsx:12` reads the same value, so the **cancel** screen — where an
   administrator decides to stand a rotation down — says nothing about what it was going to install.

### 4.2 The shape, following #543

The signer-set change is already split the way this needs to be split: a pure builder in
`domain/signer-set-change/model/build-signer-set-change.ts` and a Before/After table in
`components/`, unified across the three surfaces in #543. This mirrors it:
`domain/safe-harbour-change/model/build-safe-harbour-change.ts` and a matching component, consumed
by the detail view and the cancel summary from one place.

Each side carries **both forms of its destination — the address and its descriptor hex** — for the
reason [Constraint 3](./security-council-safe-harbour-address.md#3-the-reviewable-artifact-is-the-descriptor-hex-not-the-address)
gives: the device displays the descriptor, so the descriptor is what a signer can actually compare
against the screen in front of them. Both values come from Rust — the proposed pair from
`decode_action_hex`, the installed pair from `get_safe_harbour_status` — and neither is composed in
TypeScript.

**When the installed destination cannot be read, the section is not rendered.** Not a fallback that
shows the proposed value alone: on a screen whose whole purpose is *this replaces that*, a single
address with no stated role invites being read as either one. Suppression is what the signer-set
path already does when its config read fails, and it is the honest answer — the comparison is
unavailable, so it is not offered.

**The enacted case renders differently, and has to.** `from` is the bridge's **live** destination,
so for an enacted rotation it already *is* the value the proposal installed; a naive Before/After
would print the same address twice and read as a no-op. Two shapes, keyed on enactment:

- not yet enacted → *Replacing `<from>`* → `<to>`;
- enacted → *Destination is now `<to>`*, with no before.

`build-signer-set-change.ts` has an `isEnacted` branch for exactly this reason.

### 4.3 The title

`deriveProposalTitle` gains one arm, so an untitled rotation is named by what it does rather than by
its sequence number. The authored title still wins, as for every other action.

## 5. The signing-message panel lies to a signer who is still typing

**Also found in the manual walk.** With a half-typed address in the field, the panel turns red and
says:

> The signing message could not be resolved, so there is nothing to compare against your signer.
> Reconnect and try again. (not a valid Bitcoin address: legacy address base58 string)

Three defects in one block, and the same text is duplicated in the Defcon form, so they are fixed
together.

### 5.1 What is actually wrong

1. **It proposes a wrong cause.** "Reconnect and try again" names a connection problem. The cause is
   an address that is not finished.
2. **It leaks the parser's own words.** `legacy address base58 string` comes from `bitcoin`'s
   `Address` parser through `SafeHarbourDescriptorError::Address(String)`, which passes the inner
   message through verbatim. That string is written for a developer reading a stack trace.
3. **It is red over a field that is mid-edit.** A half-typed address is a *not yet*, not an error,
   and red is this repository's colour for errors.

### 5.2 Why Phase 1's guard did not prevent it

`safe-harbour-address-form-fields.tsx` only resolves when `errors.newSafeHarbourAddress` is
undefined. That guard filters nothing, because **the address is validated in Rust, not in the Zod
validator** — the TypeScript rules cover emptiness, a pasted descriptor hex and the no-op case, and
say nothing about whether the string is a valid taproot address. So no error is registered, the
builder is called on every keystroke that clears those three rules, and its rejection lands in the
panel.

That is the real defect, and it is mine: the guard was written as if the validator answered a
question it never answered.

### 5.3 The fix

**The builder's rejection is the address field's error, and belongs under the address field.** It is
the answer to "is this a destination I can use", which is what the field asks. Under the input, in
the field's own error slot, in the signer's language.

**The panel shows the message or a neutral placeholder, and nothing else.** While there is no
resolved message it reads *"Enter a destination address to resolve the signing message."* — the
placeholder it already has for an empty field. No red, no cause, no parser text.

**A real resolution failure keeps one red line**, without a diagnosis it cannot make: *"The signing
message could not be resolved, so there is nothing to compare against your signer."* Reachable when
the address parses and the IPC still fails, which is the case the sentence is actually true for.

**The panel becomes one shared component.** `defcon-form-fields.tsx:62-96` and
`safe-harbour-address-form-fields.tsx` render the same block with the same copy; unifying them is
what makes "fix both together" a property of the code rather than of this document. Defcon's action
hex is a constant, so its only failure mode is infrastructural — the shared component's red line is
correct for it unchanged, and it keeps its `data-testid` so the WebDriver specs that read it are
unaffected.

**And the Rust error stops leaking.** `SafeHarbourDescriptorError::Address` renders its own sentence
instead of the parser's. The detail stays available in `Debug` for logs; what reaches a signer is
one line that names the field, not the library.

**No submission gate is added.** Phase 1 §4.4 decided the address field is the gate and the message
is for comparison. The manual walk did not disturb that, and this phase leaves it alone.

## 6. The e2e — the only automated proof against a chain

`e2e-tests/tests/e2e_safe_harbour_address.rs`, a new file following `e2e_council_rotation.rs`: the
same `AsmTestHarnessBuilder`, the same `submit_action` / `mine_to` helpers, the same `bitcoind`
availability skip, and exact block counts rather than sleeps.

The action is composed through the desktop codec — `SafeHarbourDescriptor` plus
`action_codec::encode_hex` — exactly as the council rotation is, because that wire mapping is part
of what is under test and because it keeps `e2e-tests` free of the two protocol crates Phase 1 added
to the desktop.

| Path | Asserts |
|---|---|
| **Enacted** | Queued at `reveal + depth` with the bridge's destination unchanged; at exactly `depth`, the destination is the proposed one, the administrator's `last_seqno` advanced, and **`is_activated()` is unchanged** ([AC 7a](./security-council-safe-harbour-address.md#7a-activation-is-untouched)) |
| **Cancelled** | Cancel inside the window, mine past the original activation height: the queue is empty and the destination is still the original one |
| **Swallowed** | Fire a Defcon 1 first, then submit the rotation: it is accepted, the seqno advances, the queue drains — and the destination **does not change** |

The swallowed path is the only automated proof of
[Constraint 1](./security-council-safe-harbour-address.md#1-a-rotation-with-the-harbour-already-activated-is-accepted-and-changes-nothing)
against a real chain, and it exists nowhere — upstream included. It needs both key sets, since the
Defcon is council-authorized and the rotation is administrator-authorized; the fixture in
`e2e_council_rotation.rs` already carries that shape and is the model.

**One ordering detail worth stating.** Defcon 1 has depth 0, so it activates the harbour in its own
reveal block — the rotation must be submitted *after* that block, or it would be queued against a
deactivated harbour and the test would prove the enacted path instead. The assertion that separates
the two is the destination itself, not the queue.

## 7. Tests

| What | Where | Why it exists |
|---|---|---|
| Cancel of a tx-14 target: admitted for `strata_admin`, refused for `security_council` | `orchestrator-be` | AC 10, and the segregation invariant on the cancel path — nothing pins it today |
| The three e2e paths | `e2e-tests` | AC 7a, AC 9, and the only chain-level proof of Constraint 1 |
| `buildSafeHarbourChange`: both shapes, and `null` when the installed destination is unread | frontend, pure | The one piece of §4 that is logic rather than markup, including the enacted branch that would otherwise print one address twice |
| `SafeHarbourDescriptorError::Address` renders its own sentence and not the parser's | `src-tauri` | §5.1's second defect. A one-line assertion, and the only kind of test that can catch a message written for the wrong reader |

**Not tested:** the detail, cancel and create screens themselves. No DOM runner; the manual walk
covers them — which is precisely how both findings in this phase were discovered. No unit test of
`build_cancel_action_hex` against a live queue: that is what the e2e's cancelled path does with a
real one.

## 8. Migration — seven commits

| # | | Contents |
|---|---|---|
| 0 | 📄 | This document. |
| 1 | 🟢 | The two cancel authorization tests. No production diff — if one appears, §3 was wrong and this document is corrected before the phase continues. |
| 2 | 🔴🟢 | `buildSafeHarbourChange` and its tests: both shapes, and the unread case. |
| 3 | 🟢 | `useDecodedProposal`'s conditional second read, the change component, the detail section, the cancel summary and the derived title. |
| 4 | 🔴🟢 | §5: the shared signing-message panel, the address error moved to its field, and the Rust message that stops leaking the parser's. Both forms in one commit — they are one duplicated block. |
| 5 | 🟢 | `e2e_safe_harbour_address.rs`, all three paths. |
| 6 | 📄 | Close-out: three status lines, plus the V4 rows in `security-council.md`'s stage and slice boards. |

## 9. Where this phase departs from the build plan, and from its own first draft

**9.1 The contract-gap correction is already done.** The build plan asks this phase to fix the
post-condition row in `security-council.md` that omitted the freeze. That landed with the contract
in #545.

**9.2 The cancel-target summary is in scope.** The build plan names only "the detail view". The
cancel screen reads the same suppressed value and is where a decision is taken; leaving it blank
while fixing the detail view would be fixing the less important half.

**9.3 The signing-message panel is not in the build plan at all.** It comes from the manual walk,
and it is a defect Phase 1 introduced by copying Defcon's block along with a guard that could not
work (§5.2).

**9.4 An unreadable installed destination suppresses the section rather than degrading to the
proposed one.** This document's first draft said the opposite. The walk's framing is what changed
it: on a screen whose purpose is the comparison, a lone address with no stated role is worse than no
section, because a second approver has no way to tell which of the two it is.

## 10. Verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd desktop-app && npm run format:check && npm run lint && npm run build && npm run test:unit
```

`orchestrator-be` has no lib target; scope its runs with a filter, not `--lib`. `npm run build` is
not the same check as a bare type-check (Phase 1 §6).

The e2e needs `bitcoind` on `PATH`; without it the tests skip rather than fail.

**The manual walk**, which is where this phase's scope came from:

1. Type a partial address: the panel stays neutral and the field explains, in one line, what is
   wrong — with no parser text and nothing red until there is something to be wrong about.
2. The same half-typed state in the Defcon form still behaves as before.
3. A second signer opens a pending rotation from the dashboard and can see, without leaving the
   detail screen, which destination it installs and which one it replaces — each with its hex.
4. An enacted rotation shows the installed destination once, not twice.
5. A queued rotation offers a cancel; the cancel screen names the destination it was going to
   install; cancelling leaves the bridge unchanged and the proposal reads `Canceled`.
6. With the node unreachable, the change section is absent rather than half-drawn.
7. A Security Council session sees none of it.
