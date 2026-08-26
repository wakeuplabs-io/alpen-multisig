# Security Council — Defcon 1 (V1), Phase 5: Frontend create and sign

**Functional contract:** [`security-council-defcon.md`](./security-council-defcon.md) — SSOT for
*what* V1 must do. This document never overrides it.

**Build plan:** [`security-council-defcon-implementation.md`](./security-council-defcon-implementation.md)
§4 Phase 5. This document is that phase at implementation detail.

**Closes:** AC 1, AC 1a, AC 4, AC 5, AC 14.

## 1. The change in one sentence

`grep -rn "defcon" desktop-app/src` returns nothing; after this phase a Security Council signer
authenticates, reaches a visibly destructive Defcon 1 form, reads the four canonical signing lines
rendered by upstream's own renderer, types `DEFCON 1`, and signs — and no other authority is offered
any of it.

## 2. What this phase is not

It is not the lifecycle. The "Quorum reached — ready to broadcast" label, the absent cancel
affordance, the Send control, the Past list and the manual fallback are Phase 6 (AC 6, 7, 9, 10, 13,
15/15a/15b, 16). This phase stops at "the proposal exists and carries the creator's signature".

It is not a new screen or a new domain. See §4.

## 3. The prerequisite the build plan does not name

The build plan reduces Phase 5 to two bullets — register `defcon_1` in three places, add one
`*-form-fields.tsx`. Both are necessary and neither is sufficient, because **the desktop app has no
Security Council session at all.** Five places encode three-or-four authorities and none of them is
the council:

| Place | State today | Consequence |
|---|---|---|
| `desktop-app/src/types/auth-role.ts:1-6` | four `AuthRole` members, no council | nothing downstream can name it |
| `desktop-app/src/api/orchestrator-auth.ts:69` | `authorityFromRole` has `default: 'strata_admin'` | a council role would silently authenticate as the Strata Administrator |
| `desktop-app/src/lib/authority-label.ts:11` | `default: 'Unknown authority'` | AC 14 fails: the badge must read "Security Council" |
| `desktop-app/src/screens/wallet-connect-screen.tsx:14` | three `AUTHORITY_OPTIONS` | no way to select the council |
| `desktop-app/src-tauri/src/domain/auth.rs:8`, `infrastructure/asm_status_rpc.rs:59-70` | three `AuthRole` variants, three `role_to_keys` inserts | the membership check that marks an authority card *Available* cannot answer for the council |

The `default:` arms are the dangerous half. They are not compile errors and they are not runtime
errors — they are silent substitutions. A council enum member added to TypeScript without touching
`authorityFromRole` produces a session that authenticates, works, and is the wrong authority.

`orchestrator-be` already crossed this line in Phase 3 (`asm_role_membership.rs:225`, `:251-252`).
The desktop is the half that was left behind, and it is a **prerequisite of this phase**, not a
separate concern: AC 14 is unsatisfiable without the label, and AC 1 is unverifiable without an
authority that can be compared against the others.

### 3.1 The fallback that would defeat AC 1 on its own

`getActionTypeOptions` (`desktop-app/src/domain/create-proposal/model/action-type-config.ts:39-42`)
resolves an unknown authority to the Strata Administrator's menu:

```ts
const actionTypes = ACTION_TYPES_BY_AUTHORITY[authority] ?? ACTION_TYPES_BY_AUTHORITY.strata_admin
```

So a council session that reached `/proposals/create` before `security_council` is registered would
be offered *signer update, VK update, operator set update* — three actions its role cannot
authorize, which `require_authorized_for_action` (Phase 3) would then refuse at the backend. The
fallback is not removed here: it is the sane default for the authorities that legitimately have no
entry, and removing it would turn a wrong menu into a crash. It is **pinned by a test** instead
(§8, test 2), which is what makes AC 1 a claim about data rather than about a rendered screen.

## 4. Where Defcon 1 lives, and the contract text this contradicts

**Decision: Defcon 1 is one more action type inside `desktop-app/src/domain/create-proposal/`.**
No `/proposals/create/defcon-1` route, no `defcon-proposal-create-screen.tsx`.

This follows the build plan §3 ("Where Defcon 1 lives in the frontend") and contradicts the
contract's *Create Form Layout* (`security-council-defcon.md:195`) and *Critical Files*
(`:467-470`), which describe a dedicated route and screen. The contract anticipates the conflict and
defers to this document: "Stage 5 decides; this contract does not mandate a new abstraction"
(`:467`).

The reason is duplication. The creation flow is not a form — it is a two-step machine with a frozen
preview (`create-proposal-form.tsx:150-159`, `:177-210`), a session-expiry re-authentication modal
with action retry (`:212-230`), a navigation guard (`:232`), a sighash pre-flight
(`use-create-proposal.ts:218-240`) and a signature-collected success state
(`create-proposal-preview.tsx:90-116`). A sibling screen for Defcon 1 would reimplement all of it to
change a colour scheme and add one input.

**AC 1a survives this unchanged, and is worth being precise about.** It requires that direct
navigation to `/proposals/create/defcon-1` by a non-council session render no form and redirect to
`/`, "matching the existing catch-all behaviour in `desktop-app/src/App.tsx`". The route is never
registered, so `App.tsx:131` — `{ path: '*', element: <Navigate to="/" replace /> }` — answers it
for *every* session, council included. AC 1a's letter is met and its spirit is exceeded: the form is
not merely guarded at that path, it does not exist at that path.

What replaces the route guard is the authority-keyed menu (§3.1) plus the backend gate AC 17 already
pins. The contract's *Create Form Layout* and *Critical Files* are corrected in the
back-propagation commit, the way Phase 3 corrected AC 3.

## 5. The four-line message needs no new code, and must not get any

AC 4 requires exactly:

```
Strata ASM Administration v1
Action: Defcon 1
Authorized By: Strata Security Council
Sequence: <seq_no>
```

with no `Action Details:` block. **Every character of that already comes from Rust.**
`SigningMessage::for_action` (`asm/crates/subprotocols/admin/txs/src/signing_message.rs:22-36`)
builds the four lines and appends `Action Details:` *only if* `render_details` produced lines;
`Defcon1Update::render_details` is empty
(`asm/crates/subprotocols/admin/txs/src/actions/updates/defcon1.rs:23`), and upstream's own test
`defcon1_renders_signing_message` (`:35-46`) asserts the exact four-line string.

The path from there to the screen exists and is already used by the review step:
`render_signing_message` (Tauri) → `renderSigningMessage` (`desktop-app/src/api/signing.ts`) →
`useDeviceSigningMessage` (`desktop-app/src/hooks/use-device-signing-message.ts`).

So AC 4 is a **placement** requirement, not a formatting one. The rule this phase adopts:

> The Defcon 1 form renders `useDeviceSigningMessage(...).message` verbatim in a `<pre>`. It never
> constructs the message text in TypeScript.

A hand-written template in TSX would compile, look identical in review, and silently diverge the
moment upstream bumps `ADMIN_SUBPROTOCOL_VERSION` (`signing_message.rs:9`) or renames a role —
turning the signer-safety artifact, the one thing the signer compares against their device, into a
lie. This is the single most important constraint in the phase.

Rendering it requires the action hex, which for a payload-less action is a constant.
`build_defcon_1_action_hex` (`desktop-app/src-tauri/src/commands/action_builder.rs:168`, shipped in
Phase 3) takes no input, so a small hook resolves it once and feeds `useDeviceSigningMessage`
together with the seq-no the signer is typing — which the form already auto-detects from chain via
`getNextSeqNo` (`use-create-proposal.ts:133-144`) and lets the signer override. Until the hex
resolves, `useDeviceSigningMessage` returns `null` and the box shows a placeholder; it never shows a
stale message, because the hook clears before every resolve
(`use-device-signing-message.ts:24-27`). The message therefore updates live with
`Sequence:` — which is what the contract's wireframe (`security-council-defcon.md:204-211`) draws.

## 6. The gate, and why it lives in the validator

AC 5: the sign CTA stays disabled unless the confirm field matches `DEFCON 1`, case-insensitively.
Edge Cases (`security-council-defcon.md:452`) add that `"defcon1"` and `"DEFCON 1 "` must **not**
match, and name the message: `Type must match 'DEFCON 1' exactly (case-insensitive).`

```ts
export function matchesDefconConfirmation(input: string): boolean {
	return input.toUpperCase() === DEFCON_1_CONFIRMATION
}
```

| Input | Result |
|---|---|
| `'DEFCON 1'`, `'defcon 1'`, `'Defcon 1'` | `true` |
| `'defcon1'` (no space) | `false` |
| `'DEFCON 1 '` (trailing space) | `false` |
| `''` | `false` |

No `trim()`. The contract's own rule is `input.toUpperCase() === "DEFCON 1"` (`:226`) and its edge
case pins the trailing space as a rejection; trimming would be a kindness that deletes the gate's
only evidence that the signer was reading.

**The gate is a zod issue in `model/validators/defcon-1.ts`, not a disabled-prop in the component.**
`create-proposal-form.tsx:409` and `:430` already disable both CTAs on `!formState.isValid`, and
`buildCreateProposalFormSchema` (`create-proposal.schema.ts:79-80`) already dispatches to a
per-action validator through an exhaustive `Record`
(`model/validators/index.ts:8-13`). Adding `'defcon_1'` to the action-type union makes that `Record`
a compile error until the validator exists — the type system asks for the gate rather than a
reviewer having to. Putting the rule in the component instead would add a second source of "can this
be submitted", and the preview step's frozen-snapshot comparison (`:196`) would not see it.

## 7. Migration — three commits, each atomic

Ordered so that no commit repairs the one before it and every commit leaves the tree green.

**Commit A — Defcon 1 in the creation form.** Inert on merge: no session can select
`security_council` yet, so no existing authority's menu, form or preview changes. Test 2 (§8) is what
proves it landed.

| File | Change |
|---|---|
| `model/create-proposal.types.ts:3` | `'defcon_1'` in the `ActionType` union |
| `model/create-proposal.schema.ts:19,29` | `'defcon_1'` in the `z.enum`, and a `defconConfirm: z.string()` field |
| `model/validators/defcon-1.ts` (new) | `matchesDefconConfirmation` + the zod issue (§6) |
| `model/validators/index.ts:8-13` | register it — the `Record` is exhaustive, so this is a compile error until done |
| `model/action-type-config.ts:9,33` | the `ACTION_TYPE_OPTIONS` entry and `security_council: ['defcon_1']` |
| `components/defcon-1-form-fields.tsx` (new) | warning box, the rendered four lines, the confirm input |
| `components/create-proposal-form.tsx:51,365` | `defconConfirm: ''` default; the branch in the fields chain; the destructive CTA treatment |
| `api/action-builder.ts` | `buildDefcon1ActionHex()` over the existing no-arg command |
| `hooks/use-create-proposal.ts:72` | the `buildActionHex` branch |
| `components/create-proposal-preview.tsx:17,57,130` | the widened prop union, the label, the details branch |

**Commit B — the council can hold a session.** This is the switch: the entry commit A registered
becomes reachable. Closes AC 1, AC 2 (frontend half), AC 4, AC 5, AC 14.

| File | Change |
|---|---|
| `src-tauri/src/domain/auth.rs:8,17,24` | `AuthRole::StrataSecurityCouncil`, `→ Role::StrataSecurityCouncil`, wire `"strata_security_council"` |
| `src-tauri/src/application/authentication.rs:266` | `role_wire` → `"security_council"`, **matching the orchestrator's `authority_wire`** (`orchestrator-be/src/handlers/auth.rs:180`) — the doc-comment above it records why the two must not drift |
| `src-tauri/src/infrastructure/asm_status_rpc.rs:58` | the fourth membership read |
| `src/types/auth-role.ts` | `StrataSecurityCouncil = 'strata_security_council'` |
| `src/api/orchestrator-auth.ts:69` | `authorityFromRole` → `'security_council'` |
| `src/lib/authority-label.ts` | → `'Security Council'` |
| `src/screens/wallet-connect-screen.tsx:14` | one `AUTHORITY_OPTIONS` entry (`AUTHORITY_ICONS` needs nothing: `authority-selection-phase.tsx:115` falls back to the shield) |

Two wire strings, deliberately different, because they answer different questions: the Tauri
`as_wire_str` (`"strata_security_council"`) names the **ASM role** for the membership read, while
`role_wire` (`"security_council"`) names the **orchestrator authority** inside the challenge the
signer reads. The three existing roles already carry the same split.

**The order is deliberate and was corrected during review.** The session first reads as the natural
prerequisite, but it is the broken half: §3.1's fallback means a council session that exists before
`security_council` is registered is offered the *Strata Administrator's* three actions — actions
`require_authorized_for_action` (Phase 3) would refuse at the backend. Registering the menu entry
first makes both commits correct in isolation, which is what atomic is supposed to mean.

**Commit C — Defcon 1 renders as itself across the IPC boundary.** Phase 3 parked two boundaries on
`Unknown` and named this phase as the one that unparks them
(`action_builder.rs:53-59`, `proposals.rs:180-184`). Each Rust arm moves **in the same commit as its
zod counterpart**: `decodedActionSchema` is a discriminated union and `actionType` is a closed enum,
so a Rust arm emitted ahead of its schema is a parse failure — and for `actionType` it takes down
the parse of every proposal in the same list, not just the Defcon 1 one. That coupling is the reason
these are one commit and not two.

Splitting B from C matters for a second reason: B is only reachable by a council signer, while C
changes what *every* session sees in the proposal list and on the sign screen. They fail
differently, so they are reverted differently.

## 8. Tests

Minimal, pure, and each pinned to a claim that could plausibly regress.

| # | Claim | Assertion | Where |
|---|---|---|---|
| 1 | The gate accepts only `DEFCON 1`, case-insensitively, and rejects the near-misses the contract names | `matchesDefconConfirmation` over `'DEFCON 1'`, `'defcon 1'`, `'defcon1'`, `'DEFCON 1 '`, `''` | `domain/create-proposal/model/__tests__/defcon-1-confirm-gate.test.ts` |
| 2 | AC 1 as data: the council is offered Defcon 1 and nothing else, and no other authority is offered it | `getActionTypeOptions('security_council')` is exactly `['defcon_1']`; the other three authorities' menus do not contain it | same file |
| 3 | Both IPC boundaries accept the new value | `proposalSchema` parses `actionType: 'defcon_1'`; `decodedActionSchema` parses `{ kind: 'defcon_1' }` | extend `src/api/ipc-schemas.test.ts` |
| 4 | The Rust side emits what those schemas now accept | `decode_action_hex` on a Defcon 1 hex is `DecodedAction::Defcon1`; `action_type_from_hex` is `"defcon_1"` | beside the existing tests in each module |

Test 1 is written **red first** — it is the only piece of genuinely new logic in the phase. The rest
is wiring, and a failing wire is a failing `npm run build` or a failing parse, not a subtle bug.

**What is deliberately not tested, and why:**

- **No render test of the form.** The repo has React component tests (`approvals-list.test.tsx`,
  `broadcast-details-card.test.tsx`), so the capability exists — but a test that mounted
  `CreateProposalForm` would need react-hook-form, a zod resolver, four Tauri commands and a wallet
  adapter stubbed, and would assert that a `<pre>` contains text produced by a mock. It would pin the
  mock, not AC 4. The claim that matters — the message comes from Rust, never from TypeScript — is a
  structural property, verified by review and by §10's grep.
- **No test of the destructive styling.** `src/lib/__tests__/color-tokens.test.ts` already fails the
  build if a red hex appears outside `styles.css`, which is the only mechanical part of "looks
  dangerous". The rest is a design judgement a test cannot hold.
- **No e2e / WebDriver spec.** The regtest stack does seed a council (§10), so a spec is
  *possible* — but the WebDriver suite is run one spec at a time by hand
  (`desktop-app/e2e-webdriver/README.md`) and covers the wallet, not proposal creation. Adding the
  first creation-flow spec is its own piece of work, not a rider on this phase.
- **No test that a non-council session cannot reach the form.** There is no route to reach. Test 2
  covers the menu, and AC 17's backend test (Phase 3) covers the caller that never touches the UI.
- **No test that `build_defcon_1_action_hex` refuses a non-council caller.** It has no auth context
  and no other builder has one either — a Tauri command is callable from any frontend code. Encoding
  an action is not authorising it; the gate is `require_authorized_for_action` at `POST /proposals`,
  which AC 17 pins. Stated here so a later reader does not mistake the absence for an oversight.

## 9. Blast radius

- **The `default:` arms in `authorityFromRole` and `authorityLabelForRole` stay.** They are reached
  only by `PayoutAdministrator`, which has no orchestrator session by design
  (`domain/connect-wallet/hooks/use-authority-membership.ts:7` exempts it from the ASM check).
  Narrowing them to exhaustive `switch`es is a separate cleanup and would change behaviour for that
  role.
- **`CreateProposalFormValues` gains one field for every action type.** `defconConfirm` joins
  `newSequencerKeyHex`, `newVkHex` and the rest as a field the other actions ignore — the shape the
  form already has (`create-proposal.schema.ts:18-30`). A per-action discriminated union would be
  the better model and is a refactor this phase does not attempt.
- **`create-proposal-preview.tsx` widens its `actionType` prop** (`:17`) and gains a label
  (`:57-64`) and a details branch (`:130-230`). The existing chain ends in `vk_update` as its
  `else`, so a Defcon 1 without a branch would render the VK preview — a silent wrong-action
  display, which is why the branch is not optional.
- **`sign-proposal-view.tsx` gains a Defcon 1 branch.** Without it a Defcon 1 renders through
  `UnknownActionDetails` (`:118-127`) as a raw hex blob — legal, but the opposite of signer safety
  for the one action that sweeps the bridge.
- **`operator_set_update` and `sequencer_key_update` also fall to `Unknown`** in `decode_action_hex`
  (`action_builder.rs:56-57`). Pre-existing, unrelated to the council, and **out of scope** — fixing
  them here would hide the Defcon 1 change inside an unrelated diff.
- **No orchestrator-be change.** The backend half shipped in Phases 1–4.

## 10. Out of scope

- **Phase 6** owns everything after the creator's signature.
- **The `/proposals/create/defcon-1` route** (§4).
- **A discriminated-union form model** (§9).
- **Provisioning the local regtest stack.** Nothing to do: `scripts/asm-params.json:38-44` already
  seeds `strata_security_council` with three keys at threshold 2, and its first key
  (`02300dc4…f4df7`) is also the Strata Administrator's — so one connected signer can walk both
  authorities and check AC 1 by switching roles rather than by swapping devices.

## 11. Verification

Per commit, the full [`AGENTS.md`](../../AGENTS.md) pre-commit checklist:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd desktop-app && npm run format:check && npm run lint && npm run build
```

plus the phase's own scripts:

```bash
cd desktop-app
npm run test:defcon-1-confirm-gate
npm run test:ipc-schemas
npm run test:color-tokens
```

Structural evidence that §5's rule held — the message is rendered, never written:

```bash
grep -rn "Strata ASM Administration" desktop-app/src
```

must return nothing.

Manual, against the local stack, once all three commits are in:

1. Connect a council key → the Security Council card reads *Available*; authenticate; land on
   `/proposals`.
2. `/proposals/create` offers Defcon 1 and nothing else, and the form is unmistakably destructive.
3. Submit stays disabled until seq-no is set **and** the confirm field matches; `defcon1` keeps it
   disabled with the contract's message.
4. The form shows the four lines with no `Action Details:` block, and they match the review step and
   the device; the header badge reads *Security Council* and no other.
5. Reconnect as Strata Administrator → no Defcon 1 entry anywhere; `/proposals/create/defcon-1`
   redirects to `/`.
6. The created proposal lists as *Defcon 1*, not *Unknown*, and the sign screen renders it as
   Defcon 1.

End-to-end regtest verification of enactment belongs to the close-out of all six phases, not to this
one.
