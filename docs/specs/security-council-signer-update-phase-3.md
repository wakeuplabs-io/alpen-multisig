# V3 Phase 3 — The form targets the council

> **Functional contract:** [`security-council-signer-update.md`](./security-council-signer-update.md) — SSOT
> for *what* V3 must do. This document never overrides it.
> **Build plan:** [`security-council-signer-update-implementation.md`](./security-council-signer-update-implementation.md)
> §4 Phase 3. This document is that phase at implementation detail, and §9 records where it supersedes it.
> **Closes:** AC 1, 1a, 2, 3, 3a, 3b, 4, 11 and 12; Constraints 2 and 3.

## 1. The change in one sentence

A Strata Administrator can author a Security Council rotation, and every value that form is populated from,
validated against and previewed with is read from the **council's** config — never the session's.

## 2. What this phase is not

It is not the cancel and it is not the e2e — those are Phase 4, and `e2e-tests/` has **zero diff** here.

It is not a new signing message. The message resolves through the same Rust renderer the device already
signs over, and gets a tripwire, not code (§7.2).

It is not a new validator. `council_signer_update` is validated by `validateSignerUpdate`, **reused, not
duplicated**: the rules are identical and only the signer set they answer against changes.

It is not a new form component. `SignerUpdateFormFields` already takes `currentSigners` / `currentThreshold`
as props and is reused verbatim.

Nothing in `orchestrator-be/src/` changes behaviour: this phase only adds the tests that pin AC 2 and AC 12,
both of which already hold by construction (§7.3, §7.4).

## 3. The problem, and why it is one commit's worth of care

`multisigConfig` is loaded in `use-create-proposal.ts:134-150` from `authorityFromRole(selectedRole)` — the
**session**. Constraint 2 says the target is decided by the **action**. And the `actionType` that names the
action lives inside react-hook-form, not in the hook and not in the screen.

The obvious split — "add the menu entry", then "read from the target" — leaves a state where the form offers
a council rotation while validating against the administrator's signer set. Two of `signer-update.ts`'s rules
decide from that set (`:98-111` and `:113-157`), so that state accepts a key already on the council, allows a
threshold that will exceed the resulting council, and sends both to a hardware signer. **Commits 3 and 4 of
§8 must never be split across pull requests.** Inside one branch the intermediate state is a review artifact;
in `develop` it is a form that lies to a signer about a signer set.

## 4. Design

### 4.1 One pure function carries Constraint 2

`desktop-app/src/domain/create-proposal/model/multisig-target.ts`:

```ts
export function multisigTargetAuthority(
	actionType: ActionType,
	sessionAuthority: MultisigTargetAuthority,
): MultisigTargetAuthority {
	return actionType === 'council_signer_update' ? 'security_council' : sessionAuthority
}
```

It lives in `domain/create-proposal/model/` and not next to its read-side twin
(`lib/multisig-update-target.ts`) because it needs `ActionType` from the domain, and `lib → domain` would
invert the layering. Both files carry a comment naming the other; unifying them later is a deliberate move,
not a discovery.

The same value feeds three consumers, which is the whole point: the config read, the builder's `role`, and
(through the config) the schema, the validator context, the threshold reset and the preview.

### 4.2 The wire type is narrowed at the source, so no cast replaces the one removed

`use-create-proposal.ts:94` casts because `authorityFromRole` returns `string`. Widening the builder's union
and then casting into it would trade one cast for another. Instead:

- `api/action-builder.ts` exports
  `type MultisigTargetAuthority = 'strata_admin' | 'sequencer_manager' | 'alpen_admin' | 'security_council'`,
  and `BuildAdminMultisigUpdateHexInput.role` becomes that type. This is the blast radius Constraint 2 accepts
  by name. `payout_admin` is **not** in the union: it has no ASM role
  (`orchestrator-be/src/infrastructure/asm_role_membership.rs:277-280`) and dies in the codec
  (`desktop-app/src-tauri/src/infrastructure/action_codec.rs:154-156`).
- `api/orchestrator-auth.ts` narrows `authorityFromRole(role: AuthRole): MultisigTargetAuthority`. Its switch
  is already exhaustive over `AuthRole` and already returns exactly those four strings — this is an
  annotation, not a behaviour change, and the compiler checks the body.

`buildActionHex` then shares one arm between `signer_update` and `council_signer_update`: the payload is
byte-for-byte the same shape, and only `role` differs, derived rather than chosen.

### 4.3 The config read is keyed by the target, and its freshness is derived

`desktop-app/src/domain/create-proposal/hooks/use-multisig-config.ts` is the effect that lives in
`useCreateProposal` today, lifted out and keyed by an authority the caller supplies. It is called from
`create-proposal-form.tsx` as:

```ts
const targetAuthority = multisigTargetAuthority(actionType, authority)
const { multisigConfig, multisigConfigVersion, isLoadingConfig } = useMultisigConfig(targetAuthority)
```

where `authority` is the prop the screen already passes (the session authority), **not** a fresh
`useSession()` call.

**Why the hook is called from the component and not from the screen.** `react-frontend-patterns.md` puts
async effects in domain hooks and route composition in screens, and this satisfies both: the fetch is in a
`domain/create-proposal/hooks/` hook, and the component consumes it. What it does not do is hoist
`actionType` to the screen — that would mean a `useState` mirroring react-hook-form for the single value
Constraint 2 says decides the target, with desync paths that already exist (`reset()` at `:139`,
`defaultValues` at `:112`). Two sources of truth for the target is the failure Constraint 2 exists to
prevent. `create-proposal-form.tsx` is already a stateful component (`isPreviewMode`, `frozenAtPreview`,
`trigger`); this adds no new kind of responsibility to it.

**Freshness is derived, never stored.** `useEffect` runs *after* render. In the render immediately following
the click on the council card, a hook that only stored `config` would still hold the administrator's signers
while `isLoadingConfig` was still `false` — one frame of a lie, with both buttons enabled. So the hook stores
which authority the config it holds belongs to:

```ts
type LoadedConfig = { authority: string; config: MultisigConfigSnapshot }
// ...
const isFresh = loaded !== null && loaded.authority === authority
return {
	multisigConfig: isFresh ? loaded.config : null,
	multisigConfigVersion: version,
	isLoadingConfig: inFlight || !isFresh,
}
```

Through the whole loading window `currentMultisigSigners` is `null`, so the two rules in `signer-update.ts`
that decide from the set skip themselves through guards that already exist (`:98`, `:129`), and both buttons
are disabled by `isLoadingConfig` (`:429`, `:454`).

**Cancellation** uses the same `let cancelled = false` flag the current effect uses; the cleanup sets the flag
and nothing else. It must **not** lower `isLoadingConfig`: doing so in cleanup re-opens exactly the lying
frame the gate closes. Under StrictMode the double invocation is idempotent — same target, same result, one
extra version bump, and the reset it triggers writes the same values.

### 4.4 What the retarget reaches, for free

`multisigConfig` already feeds four places. Changing where it comes from retargets all four at once, which is
why this is one change and not four:

| Consumer | Site |
|---|---|
| the schema, hence the validator context | `create-proposal-form.tsx:97-104` |
| the threshold reset | `create-proposal-form.tsx:136-144` |
| the form fields | `create-proposal-form.tsx:384-388` |
| the Before/After preview | `create-proposal-form.tsx:315-316` |

### 4.5 The reset distinguishes a changed target from a refetch

`keysToRemove` and `threshold` are already reset on every version bump. `keysToAdd` currently survives — keys
a signer chose against a different multisig, silently reinterpreted as adds against the new one. A
`useRef<string | null>` holding the last applied target separates the two cases, and both lists are cleared
only when the target actually changed.

Going `council_signer_update → signer_update` refetches the administrator and resets. Going
`signer_update → vk_update → signer_update` does **not** refetch — the hook's dependency is a primitive
string that never changed — so no spurious reset. `reset()` also clears `formState.isDirty`, so the
navigation guard (`:241`) stops warning once per target switch; this already happens on mount and is recorded
in §10, not fixed here.

### 4.6 The unknown-authority fallback stops being the administrator's menu

`action-type-config.ts:61` falls back to `ACTION_TYPES_BY_AUTHORITY.strata_admin` for any authority not in
the map. Once `council_signer_update` joins that list, the fallback offers a council rotation to an authority
nobody enumerated — in practice `payout_admin`, the one authority with no ASM role. And the schema's own gate
reads the **same function** (`create-proposal.schema.ts:84`), so it would authorize it too. That is a hole in
AC 1a, opened by this phase and closed in the same commit: the fallback gets its own conservative list rather
than borrowing the administrator's.

### 4.7 The no-op rule

Upstream **accepts** an empty update. All five rejection cases in `validate_update` pass with empty add and
remove sets; `apply_update` retains everything, extends nothing and re-sets the same threshold; and
`handle_action` advances `last_seqno` regardless.

The consequence is not that a sequence number is spent. It is that the on-chain result is a *successful*
no-op, so `multisig_update_post_conditions_met` finds keys and threshold matching and the proposal reports
**`Enacted`** — indistinguishable, on every surface this application has, from a rotation that actually
rotated. That is what makes AC 3b a safety rule rather than hygiene.

The rule is a set question, not a count question: counting members before and after cannot tell "nothing
changed" from "removed one, added another". `addKeyIndexes` and `removeKeyIndexes` (`signer-update.ts:37-77`)
are already maps keyed by normalized key with blank rows discarded — which is precisely the AC's wording
("once blank rows are discarded"). The rule goes last, after the threshold has parsed:

```ts
const changesKeys = addKeyIndexes.size > 0 || removeKeyIndexes.size > 0
if (!changesKeys && currentMultisigThreshold !== null && thN === currentMultisigThreshold) {
	ctx.addIssue({ code: 'custom', path: ['keysToAdd'], message: NO_OP_UPDATE_MESSAGE })
}
```

`path: ['keysToAdd']` and not `path: []`: a root issue renders nowhere, and that path is the slot
`signer-update-form-fields.tsx:185` already paints. A threshold-only change stays allowed, and is the
mandatory counter-case in §7.1.

This needs the target's *current* threshold, which the validator context does not carry. It gains
`currentMultisigThreshold: number | null`, twinned with `currentMultisigSigners`, threaded from one call site:

```ts
buildCreateProposalFormSchema({
	currentMultisigSigners: multisigConfig?.signers ?? null,
	currentMultisigThreshold: multisigConfig?.threshold ?? null,
	authority,
})
```

Both are null exactly together, because both come off one optional chain. When they are null the rule is off
— which is safe **only** because §4.8 blocks submission in that state. The two are load-bearing together, and
a refactor that relaxes the gate silently re-opens AC 3b.

The rule lives in `signer-update.ts`, which is shared, so it changes behaviour for the three authorities
already shipped. That is correct and desirable, and it is outside the slice's declared scope — recorded in §10.

### 4.8 An unavailable config stops enabling the buttons

Today, if `getMultisigConfig` fails, `isLoadingConfig` goes false, `multisigConfig` stays `null`, and both
buttons enable while the schema validates against nothing — which the contract forbids ("this read is
load-bearing: without it the form would validate against nothing"). The retarget makes it more reachable,
because the council's read can now fail on its own. The gate:

```ts
const isConfigUnavailable = isSignerUpdateActionType(actionType) && !isLoadingConfig && multisigConfig === null
```

joins the `disabled` of both buttons, with a message naming the target that could not be read.

### 4.9 The read side: what retargets and what must not

`useDecodedProposal` makes **one** `getMultisigConfig(proposal.authority)` call (`:42`) and feeds **two**
things with it: `setAllSigners` (`:47-49`) and the Before/After table (`:59+`). `allSigners` is the pending
signer roster `ApprovalsList` derives its rows from (`proposal-detail.tsx:233`,
`cancel-proposal-screen.tsx:142`, derivation pinned in `components/__tests__/approvals-list.test.tsx:37-38`).

Retargeting that one call would make an administrator proposal list the council's four members as its pending
signers — a false roster on the approval surface, worse than Phase 1's suppression. `use-manual-proposal.ts`
has the identical shape at `:99`, `:105`, `:107`.

| Keeps reading the **proposal's authority** | Reads the **target** |
|---|---|
| `allSigners` → `ApprovalsList` rows, and anything quorum-shaped | the Before/After rows, `thresholdBefore`, `thresholdAfter` |

The signer-membership gates in `use-manual-proposal.ts:216` and `:295` ("Your key is not a signer for X") also
stay on the declared authority: they ask who may sign, not what is being changed.

**Structure.** The target is only known after the decode, so one `Promise.all` can no longer answer for both.
The existing `Promise.all([decode, config(authority)])` is kept as-is — `allSigners` lands on exactly the same
path, at the same time, with no regression risk on the approval surface — and a **second**, conditional read
is issued for the table only when the target differs from the proposal's authority. That second read
re-checks `cancelled`.

Two behaviours to preserve deliberately:

- A failed decode, or a failed read of the **target's** config, falls back to suppression (Phase 1
  behaviour), never to rendering the table against the authority's config. The narrow
  `else if (actionRes.ok && !tableApplies)` at `:104-114` documents why it is narrow; `tableApplies`
  disappears, so that narrowing must be rewritten on purpose rather than deleted.
- The `isEnacted` branch reconstructs `beforeSigners` from the *current* config minus the added keys
  (`:70-73`). Retargeted, it operates on the council's post-rotation config, which is correct — but only
  because `enacted` compares post-conditions against the **target's** config (Constraint 1, Phase 2). If
  enactment ever fell back to a seqno-shaped answer, this reconstruction would invert silently. Cross-phase
  dependency, recorded here.

### 4.10 The duplication the read side already carries

`use-decoded-proposal.ts:78-97` and `use-manual-proposal.ts:113-150` build the same `SignerRow[]` from the
same inputs, one with an `isEnacted` branch and one without. Touching both for the retarget is the moment to
extract it: a pure `buildSignerSetChange({ signers, threshold, addKeys, removeKeys, newThreshold, isEnacted })`
in `domain/proposal-detail/model/`, imported by both. It removes a real duplicate and makes the row
construction testable with no DOM — today nothing tests it.

## 5. Where the compiler insists, and where it does not

Widening the form's `ActionType` breaks compilation in three places, all of which are `Record<ActionType, _>`
or exhaustive: `buildActionHex`'s `never` default (`use-create-proposal.ts:127-130`), `ACTION_TYPE_OPTIONS`
(`action-type-config.ts:10`) and `actionValidators` (`validators/index.ts:10`).

It does **not** break in five places, two of which a signer sees:

| Site | If missed |
|---|---|
| `create-proposal-preview.tsx:170` — ternary whose `else` is the VK block | the signer reviews a council rotation and is shown **"New Verification Key"** |
| `create-proposal-form.tsx:383` — same chain, `else` is `VkUpdateFormFields` | the council form renders VK fields |
| `create-proposal-form.tsx:123` — `signerKeysDigest` | digest stays `''`, so the threshold is never re-validated as keys are edited |
| `create-proposal-form.tsx:132` — the trigger effect's guard | same |
| `currentSigners` / `currentThreshold` threaded into `CreateProposalPreview` (`:34-35`, wired at `:315-316`) | nothing type-checks that these came from the retargeted config — **this is the site AC 3 rests on** |

The first four are the same question asked four times, and the refactor in commit 8 replaces them with one
tested predicate, `isSignerUpdateActionType`, which is also §4.8's gate.

**Two unions are called `ActionType`.** The form's
(`domain/create-proposal/model/create-proposal.types.ts:4` — has `signer_update`, has no `multisig_update`)
is the one this phase widens. The proposal DTO's (`api/proposals.ts:18-24`) already carries
`council_signer_update` from Phase 1 and is **not touched**. Widening the wrong one compiles green with a
broken menu.

## 6. Copy

The new card's title is `'Security Council signer update'` — the same string `lib/proposal-type-label.ts:10`
already returns, so the menu, the dashboard and the detail view say one thing. The administrator's card keeps
the title `'Signer update'`; the two are disambiguated by description.

**The e2e constraint this creates.** Three WebDriver specs select the administrator's card with
`//button[.//p[contains(text(),"Signer update")]]` (`proposal-add-signer.e2e.js:24`,
`defcon-1-create.e2e.js:40`, `signing-no-sighash.e2e.js:30`), and `.//p` matches the **description**
paragraph too — `ActionTypeCard` renders title and description as two `<p>` inside the button
(`create-proposal-form-primitives.tsx:63-64`). So: neither the title nor the description of the new card may
contain the exact substring `"Signer update"`. `contains()` is case-sensitive, so
`'Security Council signer update'` does not match. A `data-testid` per action type would remove the
constraint and is a Phase 5 candidate, not this phase's business.

The entry goes **after** `signer_update` in `ACTION_TYPES_BY_AUTHORITY.strata_admin`, so the default selection
(`getDefaultActionType` = first entry) does not move.

## 7. Tests

### 7.1 TypeScript — `model/__tests__/council-signer-update-retarget.test.ts`

Top-level assertions with `node:assert/strict`, no runner, no mocks, no DOM — the house style of
`model/__tests__/defcon-confirm-gate.test.ts`. `scripts/run-unit-tests.mjs:19-28` discovers by filesystem
under `src/` and CI runs `npm run test:unit` (`.github/workflows/ci.yml:156`), so the file needs no
registration.

The fixture's two signer sets differ in **membership and cardinality**; with equal sizes half the assertions
would pass by accident:

```
ADMIN   = { signers: [A, B, C],    threshold: 3 }
COUNCIL = { signers: [X, Y, Z, W], threshold: 2 }
```

1. **AC 1** — `getActionTypeOptions('strata_admin')` is exactly
   `['signer_update', 'council_signer_update', 'vk_update', 'operator_set_update']`, in that order, and
   `getDefaultActionType('strata_admin')` is still `'signer_update'`. The order is the claim, not the
   membership: the default is the first entry.
2. **AC 1a** — no other authority is offered it: `security_council` (the case that matters — it must not be
   able to rotate itself), `sequencer_manager`, `alpen_admin`, and an authority not in the map, which is the
   path §4.6 fixes.
3. **AC 1a through the schema** — `strata_admin` may author it; the other three raise an `actionType` issue.
   This is the half the menu does not cover: stale form state, a direct route.
4. **Constraint 2** — for all four authorities, `multisigTargetAuthority('council_signer_update', a)` is
   `'security_council'`; for every other action type it is `a`. This is the test that fails if someone
   "simplifies" the function to read the session.
5. **AC 3** — one draft, two contexts: adding `X` against `COUNCIL.signers` raises "already exists" and
   against `ADMIN.signers` does not; and the mirror with `A`. This is the one property the retarget can break
   silently, and the only one reachable without a DOM.
6. **AC 3a** — removing two of the council's four with threshold `3` raises a threshold issue; the same
   numbers against `ADMIN` (three members) do not. The differing cardinalities are what make this
   discriminate.
7. **AC 3b** — blank rows with the council's threshold → refused; blank rows with a different threshold →
   allowed (threshold-only change, the mandatory counter-case); a real add with an unchanged threshold →
   allowed; and blank rows with a threshold equal to the **administrator's** but not the council's → allowed,
   which is what proves the rule reads the target's threshold and not some other one.
8. **AC 11** — `removesCurrentMembers` answers `true` for a removal row naming a current council member and
   `false` for one naming a key that is not on the council. Same fixture, second use.

### 7.2 The signing message tripwire

Upstream already pins the nine lines byte-for-byte
(`strata_security_council_multisig.rs:52-75`), and restating that here would test upstream's test. The claim
upstream cannot make is that **our** mapping `Authority::SecurityCouncil → UpdateAction::StrataSecurityCouncilMultisig`
(`action_codec.rs:149-153`) lands on the variant that renders tx type 15.

So the test runs the path the device actually signs over — out of the builder, not out of a hand-built
`Action`: `build_admin_multisig_update_hex({ role: "security_council", … })` → `render_signing_message(seqno, hex)`.
It asserts on `message.lines()`, not `contains()` over the whole string: a renderer that joined the two lines
with a space would pass a `contains` and fail a signer. And at the same seqno, with the same keys and
threshold, it differs from an administrator signer update's — same seqno for both, so "differs" can only mean
the action.

It lives in `desktop-app/src-tauri/src/commands/action_builder.rs`, next to the tests it makes redundant, and
not in `infrastructure/signing.rs`. `render_signing_message` is reachable from both — `signing.rs` compiles
into the `desktop_app` **library** crate (`lib.rs`) — but `build_admin_multisig_update_hex` lives in
`commands/`, which `main.rs` pulls in as `mod commands` and the library crate does not re-export. A test that
needs the builder can only run from the **binary** crate's own test tree.

Its doc comment says why literals are pinned here when the neighbouring Defcon tripwire
(`signing.rs:453-458`) explicitly declines to pin upstream's: the new coverage is the *pair* of lines naming
two different roles, which is the wire-level expression of the segregation invariant.

`decode_council_signer_update_names_the_target_role` (`action_builder.rs:279`) becomes redundant once this
test exists — it builds the `Action` by hand and asserts a subset of what this asserts.

### 7.3 AC 2, both directions

`require_authorized_for_action` already refuses a tx type 15 from a non-administrator session, by comparing
against upstream's `authorized_role()` — and `create_proposal` calls it before persisting anything
(`handlers/proposals.rs:104-105`, `create_update_action` only at `:110`), so "no proposal is persisted" holds
literally. No test covers it: the only authorization tests are Defcon's (`asm_role_membership.rs:604-631`).

One test, both halves, over the **same** tx 15 action: `StrataAdmin` accepts, `SecurityCouncil` refuses. The
second half is the segregation invariant — the action that rotates the council must not be reachable by the
council — and nothing pins it today.

### 7.4 AC 12, with the discriminating pair

There is exactly one `depth_for_action` (`asm_role_membership.rs:139-148`); it dispatches on
`update.update_tx_type()` into `ConfirmationDepths::get`, which maps tx 15 to
`strata_security_council_multisig_update`. True by construction. The desktop resolves no depth at all:
`activationHeight` is a field copied from the backend's DTO (`domain/proposal.rs:35`,
`commands/proposals.rs:234`). The frontend holds no depth constant — `AVG_BLOCK_SECONDS = 600`
(`activation-countdown.tsx:7`) is a block-time estimate for the "~2d 3h" label, not a depth.

The test therefore lives only in `orchestrator-be`, and its pair is **tx 10 against tx 15**: both are created
by the administrator, which makes them exactly the pair an authority-shaped mapping cannot separate. It is
modelled on `two_actions_of_one_authority_resolve_to_their_own_depths` (`:655+`), not on the Defcon pair.

### 7.5 Not tested

That the form hands the council's config to the schema. That is component wiring and there is no DOM runner
in this repository. The mitigation is structural — the decision lives in a pure, tested function and the
wiring is one line — and the honest substitute is the manual walk in §11. A WebDriver spec modelled on
`e2e-webdriver/test/specs/proposal-add-signer.e2e.js` is a Phase 5 candidate.

## 8. Migration — eleven commits, each atomic

None of them repairs the one before it. Every one compiles, lints and leaves the suite green.

> **Commits 3 and 4 ship together.** Between them the menu offers a council rotation while the form validates
> against the administrator — Constraint 3, violated. They belong to one pull request, and this phase ships
> as one.

| # | | Contents |
|---|---|---|
| 0 | 📄 | This document. The phase is designed before it is built, and reviewed before it is implemented. |
| 1 | 🟢 | `MultisigTargetAuthority` in `action-builder.ts`; `role` widened; `authorityFromRole`'s return type narrowed; the cast at `use-create-proposal.ts:94` removed. No behaviour change. |
| 2 | 🔴 | The test file of §7.1 in full. Red: neither the action type nor `multisigTargetAuthority` exists. |
| 3 | 🟢 | The vocabulary: the form's `ActionType`, the schema's `z.enum`, `ACTION_TYPE_OPTIONS`, `ACTION_TYPES_BY_AUTHORITY.strata_admin` **and its fallback (§4.6)**, the registry entry reusing `validateSignerUpdate`, `multisig-target.ts`, the shared arm in `buildActionHex` — and the five sites of §5 by hand. Claims 1-6 green. |
| 4 | 🟢 | `use-multisig-config.ts` with derived freshness; the wiring in the form; the config effect and three props out of `useCreateProposal` and the screen; the target-keyed reset (§4.5). |
| 5 | 🔴🟢 | AC 3b: `currentMultisigThreshold` through the context, the args and the call site, then the rule. Claim 7 green. |
| 6 | 🟢 | The unavailable-config gate (§4.8), `removesCurrentMembers`, the AC 11 callout, and the card copy. Claim 8 green. |
| 7 | 🟢 | Rust: the four tests of §7.2, §7.3 and §7.4, and the removal of the now-redundant `decode_council_signer_update_names_the_target_role`. |
| 8 | 🟢 | The read side (§4.9) in both hooks, with the extraction of §4.10 and its tests. |
| 9 | ♻️ | `isSignerUpdateActionType` replacing the four literals; the cross-references between `multisig-target.ts` and `lib/multisig-update-target.ts`. |
| 10 | 📄 | The four close-out edits of §11. |

## 9. Where this phase departs from the build plan, and why

**9.1 Eleven commits, not one.** The build plan §4 asks for one atomic commit per phase, and gives the reason:
no intermediate state may offer the menu entry while validating against the session's config. That reason is
about what reaches `develop`, and a single pull request satisfies it exactly as well while making a ~600-line
change reviewable in ten steps. §8 carries the constraint that actually matters — commits 3 and 4 are
inseparable — rather than the proxy for it.

**9.2 The read side is restored here, not in Phase 4.** The build plan's Phase 3 names only the create form's
preview. But Phase 1 suppressed the Before/After table on the detail view and in `/manual` and left the
retarget to "a later phase", and this is the phase that makes a council rotation *creatable*: the first one
anybody opens would show a suppressed table. Restoring it in the same phase that makes it reachable is the
smaller inconsistency. §4.9 is the part the build plan's one-line description underestimates.

**9.3 The fallback fix (§4.6) is not in the build plan at all.** It is a hole this phase would open, so this
phase closes it.

**9.4 The extraction in §4.10** is refactoring under the batch that touches both copies, per the repository's
red-green-refactor discipline, not new scope.

## 10. Blast radius, and debt recorded rather than fixed

1. **The wiring is untested** (§7.5) — the same debt V1 and V2 accepted; the manual walk covers it.
2. **A refetch on every action-type switch.** A local IPC; while it is in flight the form says "loading"
   rather than lying. If the skeleton flicker proves distracting in the manual walk, the alternative is to
   load both configs in the hook — a change local to `use-multisig-config.ts`.
3. **`reset()` clears `isDirty`** once per target switch, so the navigation guard stops warning. Pre-existing
   on mount, widened here.
4. **`currentMultisigSigners` and `currentMultisigThreshold` are two fields, not one snapshot.** They can
   diverge by type and cannot in practice — one optional chain, one call site. Collapsing them into
   `currentMultisigConfig: MultisigConfigSnapshot | null` touches all six validators and the Defcon test; not
   worth the diff in this phase.
5. **AC 3b changes behaviour for the three authorities already shipped**, because `signer-update.ts` is
   shared. Correct and desirable, and outside the slice's declared scope.
6. **Constraint 4 stays alive at the protocol level**, but for a rotation built by this form all five
   `validate_update` errors are now unreachable client-side: three because the form validates against the
   target's config, two because the duplicate rules (`signer-update.ts:47-96`) already caught them. This is a
   property of the form, **not** a protocol rule, and the manual/imported bundle path keeps the gap.
7. **The `isEnacted` branch depends on Phase 2's post-condition semantics** (§4.9).
8. **Three WebDriver specs select the action-type card by text** (§6). The copy rule avoids the collision
   today; a `data-testid` per action type is the durable fix, and Phase 5's.

## 11. Verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd desktop-app && npm run format:check && npm run lint && npm run build && npm run test:unit
```

`npm run test:unit` is absent from `AGENTS.md`'s checklist and present in CI; run it.

**The manual walk** (build plan §5, steps 1-3 and 7), with the backend and a local node up:

1. A Strata Administrator sees two signer-update entries and reaches the council one; a Security Council
   signer sees neither and cannot navigate to it.
2. The form shows the council's signers and threshold — not the administrator's.
3. The rendered message matches the signer's screen, names both roles on separate lines, and carries the
   `Action Details:` block.
4. An administrator signer update created in the same session still validates against the administrator's own
   signers. This is the regression that proves the retarget did not stick.
5. An administrator proposal's approvals list still shows the administrator's signers (§4.9).

**Close-out** — four edits across three files, none of which update themselves; both V1 and V2 needed a
follow-up pull request for exactly this drift:

- `security-council-signer-update-implementation.md`: the `Status:` header, and `| 3 ✅ |` in §2.
- `security-council-signer-update.md`: the `Status:` header.
- `security-council.md`: §6 Stage board and §7 Slice board.
