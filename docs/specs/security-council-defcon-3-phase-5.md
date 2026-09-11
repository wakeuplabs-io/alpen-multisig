# Security Council — Defcon 3 (V2), Phase 5: Create and sign

**Functional contract:** [`security-council-defcon-3.md`](./security-council-defcon-3.md) — SSOT for
*what* V2 must do. This document never overrides it.

**Build plan:** [`security-council-defcon-3-implementation.md`](./security-council-defcon-3-implementation.md)
§4 Phase 5. This document is that phase at implementation detail.

**Closes:** [AC 1](./security-council-defcon-3.md#1-a-council-signer-can-create-a-defcon-3),
[AC 1a](./security-council-defcon-3.md#1a-no-other-authority-can-reach-it),
[AC 2](./security-council-defcon-3.md#2-only-a-council-session-can-create-a-defcon-3),
[AC 3](./security-council-defcon-3.md#3-a-duplicate-defcon-3-is-rejected),
[AC 4](./security-council-defcon-3.md#4-the-signing-message-is-the-four-canonical-lines),
[AC 5](./security-council-defcon-3.md#5-the-type-to-confirm-gate-is-exact-and-mutually-exclusive) and
[AC 15](./security-council-defcon-3.md#15-the-safe-harbour-note-appears-with-its-own-wording);
[Constraint 5](./security-council-defcon-3.md#5-defcon-3-is-destructive-but-it-is-not-irreversible).

## 1. The change in one sentence

Defcon 3 becomes creatable and signable: one Tauri builder command, one entry in the council's action
menu, one validator, and a Defcon form that is **parameterized by level** rather than duplicated — so
the confirmation string, the destructive copy and the safe-harbour wording are the only three things
that differ, and the signing-message wiring that carries the safety is shared.

## 2. What this phase is not

It is not the queued lifecycle (Phase 6), nor the cancel (Phase 7). It adds no protocol rule and no
enactment logic — Phase 4 owns that. It writes **no signing-message code**: the four canonical lines
resolve through the same Rust renderer the device signs over, and a second renderer is precisely the
defect this phase must not introduce.

One thing it does that the build plan does not name: the broadcast screen's safe-harbour note was
keyed to `defcon_1`, and a Defcon 3 is reachable there the moment this phase ships. Leaving it would
mean the last screen before the commit and reveal fees are spent says nothing, so it reads its level
from `DEFCON_COPY` like every other surface.

## 3. Spec traceability audit

| Document | What Phase 5 takes from it |
|---|---|
| [`security-council-defcon-3.md`](./security-council-defcon-3.md) § Frontend Contract | Menu order, form field order, the mutual-exclusion requirement |
| [`security-council-defcon-3.md`](./security-council-defcon-3.md) Constraint 5 | No Defcon 3 surface may reuse Defcon 1's *Irreversible* copy |
| [`security-council-defcon-3.md`](./security-council-defcon-3.md) Constraint 1 | The delay is a live depth, so no copy may name a block count or an hour count |
| [`security-council-defcon-phase-5.md`](./security-council-defcon-phase-5.md) (V1) | Pattern: builder command, validator entry, form fields, authority-keyed menu |
| [`security-council-defcon-3-phase-1.md`](./security-council-defcon-3-phase-1.md) | `defcon_3` already readable end to end; this phase lifts its "nothing can create one" barrier |

## 4. Why the form is parameterized and the validators are not

The two Defcon variants differ in exactly three things: the confirmation string, the destructive
paragraph, and the safe-harbour note's wording. Everything else in
[`defcon-form-fields.tsx`](../../desktop-app/src/domain/create-proposal/components/defcon-form-fields.tsx)
is the safety-critical half — the action-hex resolve, the `useDeviceSigningMessage` call, the mirror
of the resolved message into `defconMessage`, and the CTA gate that depends on it. Duplicating the
component forks that wiring, and a fork is a place where one copy gets fixed and the other does not.

The **validators stay separate** (`validators/defcon-1.ts`, `validators/defcon-3.ts`), because the
registry in `validators/index.ts` is keyed by action type and its exhaustiveness is what makes a new
action type a compile error rather than a silent no-op. Both delegate to one `validateDefcon(level)`.

### 4.1 The three copies become data

The *Irreversible* paragraph is currently written out three times — in the form fields, in
[`create-proposal-preview.tsx:125`](../../desktop-app/src/domain/create-proposal/components/create-proposal-preview.tsx),
and in `Defcon1Details` inside
[`sign-proposal-view.tsx:121`](../../desktop-app/src/domain/sign-proposal/components/sign-proposal-view.tsx).
Adding a second level to three hand-written copies is how the two levels come to disagree.

```ts
// desktop-app/src/lib/defcon-copy.ts
export type DefconLevel = 'defcon_1' | 'defcon_3'

export type DefconCopy = {
	confirmation: string
	menuTitle: string
	menuDescription: string
	calloutTitle: string
	calloutBody: string
	signCalloutBody: string
	safeHarbourNote: string
	signSafeHarbourNote: string
}

export const DEFCON_COPY: Record<DefconLevel, DefconCopy>
```

`DefconLevel` is declared **in `lib`**, not imported from the create-proposal domain: the module is
read by two domains, and the import direction is `domain → lib`, never the reverse. The component
that renders it, `DefconCallout`, goes to `src/components/` for the same reason
`src/components/safe-harbour-note.tsx` lives there.

### 4.2 Mutual exclusion becomes true by construction

```ts
export function matchesDefconConfirmation(level: DefconLevel, input: string): boolean {
	return input.toUpperCase() === DEFCON_COPY[level].confirmation
}
```

Case-insensitive and nothing else — no `trim()`, because the contract's Edge Cases pin a trailing
space as a rejection, and that rejection is the gate's only evidence that the signer read the form.

The residual risk parameterizing introduces is not that the strings collide; it is that
`validators/defcon-3.ts` passes `'defcon_1'` to the factory, which no test of the pure matcher can
catch. Test 5 below goes through the schema for exactly that reason.

## 5. The two defects the build plan did not anticipate

Both are introduced *by* the parameterization, and both are closed in code rather than by a test.

### 5.1 A stale action hex under the other level's heading

[`use-defcon-action-hex.ts`](../../desktop-app/src/domain/create-proposal/hooks/use-defcon-action-hex.ts)
never resets its state: the effect has empty deps and only ever writes in the `.then()`. Parameterized
to `useDefconActionHex(level)` with `level` in the deps, it keeps the **previous level's hex** for the
duration of the refetch. The pairing guard in
[`use-device-signing-message.ts`](../../desktop-app/src/hooks/use-device-signing-message.ts) matches
*message against hex*, not *hex against level*, so during that window a form labelled DEFCON 3 renders
`Action: Defcon 1`, and `defconMessage` is non-empty — the validator would call the other action's
message resolved.

Closed twice, deliberately: the hook clears its state at the top of the effect, and the dispatch site
mounts `<DefconFormFields key={actionType} …>` so the switch remounts. The typed confirmation is
cleared on the same switch, because it is evidence that the signer read *this* form.

### 5.2 A Defcon 3 built as a VK update

`resolveActionHex` in
[`use-create-proposal.ts:95`](../../desktop-app/src/domain/create-proposal/hooks/use-create-proposal.ts)
is an `if` chain whose final `else` builds a `vk_update`. A missing arm therefore does not fail to
compile and does not fail loudly — it makes the signer sign a `vk_update` sighash under a form
labelled DEFCON 3. It becomes a `switch` with an exhaustiveness guard, following the precedent in
[`action-type-from-decoded.ts`](../../desktop-app/src/domain/manual-proposal/model/action-type-from-decoded.ts).
The two cosmetic ternaries (preview, form dispatch) are left as they are; only this one is a safety
failure.

## 6. The copy

Shorter and markedly less severe than Defcon 1's, because Defcon 3 **is** cancelable until it
activates, and overstating that trains signers to discount the same warning where it is true
([Constraint 5](./security-council-defcon-3.md#5-defcon-3-is-destructive-but-it-is-not-irreversible)).
No block count and no hour count appears anywhere: the delay is a live depth
([Constraint 1](./security-council-defcon-3.md#1-the-delay-is-always-the-live-depth-never-a-constant)).

| Field | Defcon 3 |
|---|---|
| `confirmation` | `DEFCON 3` |
| `menuTitle` | `DEFCON 3` |
| `menuDescription` | `Sweep bridge funds to the Safe Harbor after a delay. Cancelable until it activates.` |
| `calloutTitle` | `Delayed and cancelable` |
| `calloutBody` | `DEFCON 3 sweeps bridge funds to the Safe Harbor, but not immediately. Once the approved proposal confirms, it is queued for the delay this deployment configures. Until it activates, the council can cancel it. From activation on it cannot be undone.` |
| `signCalloutBody` | `Signing this approves a delayed Safe Harbor sweep. Until it activates, the council can cancel it. From activation on it cannot be undone.` |
| `safeHarbourNote` | `The bridge is already in safe harbour. A DEFCON 3 does not change that — it consumes a council sequence number, costs fees, needs a full quorum, and waits out its full delay before changing nothing.` |
| `signSafeHarbourNote` | `The bridge is already in safe harbour. Signing this does not change that — it waits out its full delay before changing nothing.` |
| `broadcastSafeHarbourNote` | `The bridge is already in safe harbour. Sending this does not change that — it costs the commit and reveal fees, then waits out its full delay before changing nothing.` |

Defcon 1's fields keep their current wording verbatim, including the `Irreversible` callout title —
this phase moves those strings, it does not rewrite them.

## 7. The signing message needs no code

`SigningMessage::for_action` renders four lines from the action itself, and Defcon 1 and Defcon 3
differ in exactly one of them (`Action: Defcon 1` / `Action: Defcon 3`); neither produces an
`Action Details:` block, because both payloads are empty. Upstream owns and freezes those strings, so
asserting them here would pin upstream's phrasing twice. What this side can lose is the *distinction*:
an upstream change that rendered the two identically would be discovered on a signer's screen. That
is the one tripwire, and it is the first test `infrastructure/signing.rs` has for
`render_signing_message`.

## 8. Migration — five commits, each atomic

| # | Commit | Why it is safe on its own |
|---|---|---|
| 0 | This spec, linked from the build plan's traceability table | Docs only |
| 1 | `build_defcon_3_action_hex` + both `invoke.rs` lists + three Rust tests | Registered but unreachable: no TS caller exists |
| 2 | `defcon-copy.ts`, `buildDefcon3ActionHex`, the `ActionType`/schema enum, the menu **option**, and the two validators | The exhaustive `Record`s force the option and validator into this same commit; `ACTION_TYPES_BY_AUTHORITY` is untouched, so nothing can create one yet |
| 3 | `DefconCallout` + the parameterized form fields, hook, preview and sign view | Behaviour-preserving for Defcon 1; the sign view gains a `defcon_3` arm it lacked |
| 4 | The authority menu, the form dispatch, `isDestructive`, the exhaustive `switch`, and `5 ✅` in the build plan | The one commit where a Defcon 3 becomes creatable — and the only one where the phase is honestly shipped |

Commit 2 before commit 4 is what keeps the barrier
[Phase 1](./security-council-defcon-3-phase-1.md) put up — `defcon_3` legal to write down, impossible
to produce — intact until the flow is complete behind it.

## 9. Tests

| # | Layer | Claim |
|---|---|---|
| 1 | `src-tauri` `action_builder.rs` | `build_defcon_3_action_hex` round-trips to `DecodedAction::Defcon3` (a **rewrite** of `decode_defcon_3_names_the_action`, whose "no builder command yet" comment stops being true) |
| 2 | `src-tauri` `signing.rs` | AC 4 — at one and the same `seqno`, the Defcon 3 message is non-empty and differs from Defcon 1's |
| 3 | `orchestrator-be` `asm_role_membership.rs` | AC 2 — a Defcon 3 is authorized for the council and refused for every other role, naming the role it requires |
| 4 | TS `defcon-confirm-gate.test.ts` | AC 5 — per level, through the schema: case variants accepted; near-misses rejected (`defcon1`, trailing space, leading space, `DEFCON`, empty). The pure matcher stays in `defcon-copy.test.ts` |
| 5 | TS `defcon-confirm-gate.test.ts` | AC 5 — mutual exclusion, **through the schema**: `DEFCON 1` typed into a `defcon_3` draft raises a `defconConfirm` issue, and vice versa |
| 6 | TS `defcon-confirm-gate.test.ts` | AC 1 / 1a — `getActionTypeOptions('security_council')` is `['defcon_1', 'defcon_3']` **in that order** (the order is the default), no other authority is offered either, including the unknown-authority fallback, and the schema itself refuses a `defcon_3` drafted under another authority |
| 7 | TS `defcon-copy.test.ts` | Constraint 5 — every Defcon 3 field differs from its Defcon 1 counterpart |

Test 7 asserts difference and nothing stronger. Forbidding the word "irreversible" in the Defcon 3
body would be wrong — it legitimately says the sweep cannot be undone once it activates — and
searching for "cancelable" would pin phrasing. What it guards is the real regression: someone
"deduplicating" the copy by collapsing the two levels onto one string.

**Not tested:** the form component, the preview and the sign view — there is no DOM runner, and a
`readFileSync` test pins a phrasing rather than a behaviour. `useDefconActionHex`, which would need
`tauriCall` mocked to say anything; §5.1 is closed in code. The four exact message lines (AC 4), which
upstream owns and freezes. An HTTP round trip for AC 2 — `handlers/proposals.rs` calls
`require_authorized_for_action` before `create_update_action` and has no test module. And AC 3 with a
Defcon 3 hex: `create_update_action` never inspects the action, and
`test_create_duplicate_action_rejected_naming_the_existing_proposal` already proves the rejection, the
named `ActionId` and the untouched original. The contract's own Test Plan lists neither.

The honest substitute for the untested surfaces is the manual walk in §11.

## 10. Blast radius

- **The council's action menu becomes a two-card grid.** `create-proposal-form.tsx` switches to
  `grid-cols-2` at more than one option. Cosmetic, and the first visible change a council signer sees.
- **A Defcon 3 reaching the sign view now explains itself.** Since Phase 1 it already rendered with the
  destructive palette and no copy at all; this is a fix, not a new surface.
- **Defcon 1 is unchanged everywhere**, including its `data-testid`s: they are derived from the level,
  so `e2e-defcon-1-confirm` and `e2e-defcon-1-signing-message` resolve exactly as before and the
  `defcon-1-create.e2e.js` wdio spec keeps working.
- **No backend code changes.** AC 2 and AC 3 are closed by evidence, not by new behaviour.

## 11. Verification

```bash
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
cd desktop-app && npm run format:check && npm run lint && npm run build && npm run test:unit
git grep -n "e2e-defcon-1-confirm" desktop-app/          # still resolves
git grep -ln "cannot be canceled" desktop-app/src/       # only lib/defcon-copy.ts
```

Manual walk on regtest (`./scripts/local-stack.sh --clean`), the parts of the build plan's §5 this
phase owns:

1. A council signer sees both cards, Defcon 1 first and selected; every other authority sees neither.
2. Selecting Defcon 3 renders its own callout and its own safe-harbour note, and switching back and
   forth never shows one level's message under the other's heading.
3. The rendered four-line message matches the signer's screen, has no `Action Details:` block, and
   differs from Defcon 1's.
4. Typing `DEFCON 1` into the Defcon 3 gate leaves the CTA disabled, and the reverse.
