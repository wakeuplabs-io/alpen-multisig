# Security Council — Defcon 3 (V2), Phase 1: `defcon_3` is a readable type

**Functional contract:** [`security-council-defcon-3.md`](./security-council-defcon-3.md) — SSOT for
*what* V2 must do. This document never overrides it.

**Build plan:** [`security-council-defcon-3-implementation.md`](./security-council-defcon-3-implementation.md)
§4 Phase 1. This document is that phase at implementation detail.

**Closes:** no acceptance criterion. This is a prerequisite of Phases 2, 3, 5 and 6, each of which
needs `defcon_3` to be a legal `ActionType` merely to write a fixture.

## 1. The change in one sentence

`defcon_3` becomes a legal value everywhere a proposal is **read** — the Tauri domain action, the
codec, both IPC boundaries and the type label — while nothing in the application can produce a
Defcon 3 hex.

## 2. What this phase is not

It is not the create flow. No `build_defcon_3_action_hex`, no registration in `invoke.rs`, no menu
entry, no validator, no form. Those are Phase 5, and §6 shows that their absence is structural rather
than a matter of remembering.

It is not the lifecycle, the redundancy fix or cancelability — Phases 2, 3 and 6. §7 walks every site
that sees a `defcon_3` today and says which phase owns it.

## 3. Why the commit is atomic, and why it is not the compiler that says so

The build plan says emitter and acceptor "cannot be split across two PRs". That is true, but not for
the reason a reader would assume: **no compile error crosses the Rust↔TypeScript border.** A
Rust-only commit builds, passes `cargo test --workspace`, and leaves `npm run build` green.

The coupling is on the wire, and it is asymmetric:

| Order | Consequence |
|---|---|
| Rust before TypeScript | **Broken.** `actionType` is a closed `z.enum` (`ipc-schemas.ts:48-56`) and the listing parses with `z.array(proposalSchema)` (`api/proposals.ts:110`), so one `defcon_3` row fails the parse of **every proposal in the same list**, not just its own. |
| TypeScript before Rust | Inert, but unverifiable — the enum only widens, and nothing emits the value yet. |

This is the exact inverse of the rule Phase 3 will follow ("the backend serves the field before the
desktop reads it"): there serde ignores an unknown field, here zod **rejects** an unknown value. Both
phases order their commits by which side tolerates the other's absence.

**What this phase does *not* fix, contrary to a tempting reading:** nothing is broken today.
`actionType` never travels from the orchestrator — it is computed locally by `action_type_from_hex`
(`src-tauri/src/commands/proposals.rs:168`), whose `Err(_)` arm answered `"unknown"` for a Defcon 3
hex, and `"unknown"` is a value the closed enum accepts. An externally created Defcon 3 listed as an
*Unknown* row with its raw payload; it never emptied a list. The wire argument above is about the
order of *this* phase's commits, not about a live outage.

The reason this phase goes first is the one the build plan gives: Phases 2, 3, 5 and 6 each need
`defcon_3` to be a legal `ActionType` merely to write a fixture, and landing it later would force a
commit that repairs the one before it.

## 4. Inside each language, the compiler does insist

Adding `Action::Defcon3` is an `E0004` in four places, which is the design working:

| Site | Match |
|---|---|
| `src-tauri/src/infrastructure/action_codec.rs:105-155` | `to_strata_action`, no wildcard |
| `src-tauri/src/commands/action_builder.rs:42-59` | `decode_action_hex`, no wildcard |
| `src-tauri/src/commands/proposals.rs:174-184` | `action_type_from_hex`, no wildcard |
| `src-tauri/src/domain/action.rs:194-200` | the panic arm of `test_action_builds` |

And widening `DecodedAction` in `api/signing.ts` is a `tsc` error in two more:
`manual-proposal/model/action-type-from-decoded.ts:16-21` (an exhaustive `Record`, written that way
on purpose) and `sign-proposal/components/sign-proposal-view.tsx:196` — see §5.

The one site with **no compiler net at all** is `lib/proposal-type-label.ts`: a chain of `if`s ending
in `return 'Unknown'`. It is what the dashboard, the detail view and the sign screen header display,
so it is the one change in this phase that gets a test written for it first (§8).

## 5. `sign-proposal-view.tsx` is in this phase, and gets the minimum

`decodedAction.rawHex` is read in the final `else` of the details chain (`:189-197`). Once the union
carries `{ kind: 'defcon_3' }`, that arm narrows to `unknown | defcon_3` and `rawHex` no longer
exists on it. The fix is to stop using `unknown` as an `else`:

```tsx
) : decodedAction.kind === 'unknown' ? (
	<UnknownActionDetails rawHex={decodedAction.rawHex} />
) : null
```

**A Defcon 3 therefore renders with no details panel in this phase.** Reusing `Defcon1Details`
(`:121-149`) would be a lie — its copy reads *"activates the Safe Harbor sweep immediately… it cannot
be canceled, and is therefore irreversible"*, which is the opposite of Defcon 3 on both counts and
which [Constraint 5](./security-council-defcon-3.md#5-defcon-3-is-destructive-but-it-is-not-irreversible)
forbids. Writing an honest `Defcon3Details` is destructive copy plus the safe-harbour note, which is
Phase 5's by assignment.

`UnknownActionDetails` is not the fallback either, and cannot be: it prints `decodedAction.rawHex`,
which `DecodedAction::Defcon3` does not carry — for the same reason `Defcon1` does not, the payload
is an empty unit struct. The hex of a payload-less action is four bytes of union selector, which
tells a signer strictly less than the word *Defcon 3* the header already prints through
`inferProposalTypeLabel`. The screen loses no information it used to hold; it gains the action's
name and postpones the paragraph that explains it.

**`isDestructive` (`:177`) *is* extended, and this reverses the phase's first decision.** It read
`decodedAction?.kind === 'defcon_1'`, which would have put the second bridge-sweeping lever behind a
neutral CTA on the screen where a signer commits. Both levers relay the same message to the bridge
and move the same funds; only the delay differs. The line between this phase and Phase 5 is drawn at
copy, not at palette: a boolean that selects an existing danger token is not the destructive callout,
and leaving it for later would mean shipping a phase in which a Defcon 3 is signable with no visual
signal at all.

## 6. Two unions are called `ActionType`, and this phase touches one

| Union | Members | Answers |
|---|---|---|
| `src/api/proposals.ts:18-19` | `multisig_update`, `vk_update`, `operator_set_update`, `sequencer_key_update`, `defcon_1`, `cancel`, `unknown` | what a proposal **read from the backend** is |
| `src/domain/create-proposal/model/create-proposal.types.ts:3` | `vk_update`, **`signer_update`**, `operator_set_update`, `sequencer_key_update`, `defcon_1` | what a signer **can author** |

They are different vocabularies — the second does not even contain `multisig_update`. Phase 1 widens
only the first.

That is what makes "there is no way to create a Defcon 3" structural rather than disciplinary:
`ACTION_TYPES_BY_AUTHORITY` (`model/action-type-config.ts:37-43`) still answers `['defcon_1']` for
`security_council`, the `Record<ActionType, ActionValidator>` in `model/validators/index.ts:8-14`
stays satisfied, and `create-proposal.schema.ts:20`'s enum still refuses the value. Nothing has to be
remembered.

The corresponding hazard: adding `defcon_3` to the *creation* union during this phase would turn that
validator `Record` into a compile error and drag half of Phase 5 in with it. §9's checklist greps for
exactly that.

## 7. Every site that sees a `defcon_3`, and the phase that owns it

Each one degrades safely today — the failure is a missing affordance or a missing warning, never a
false one.

| Site | Behaviour with `defcon_3` | Phase |
|---|---|---|
| `sign-proposal-view.tsx:189-197` details panel | no panel; the header names the action | 5 (§5) |
| `create-proposal-preview.tsx` | unaffected — it is typed by the *other* union (§6) | 5 |
| `broadcast-proposal-screen.tsx:54,111` | no safe-harbour note; only reachable after quorum | 5 |
| `proposal-status.ts:78` `proposalDisplayStatus` | `'approved'`, which is already what the contract wants | 6 (pinning) |
| `proposal-status.ts:95-97` `showsActivationCountdown` | `true`, already correct — the predicate excludes only `defcon_1` | 6 (pinning) |
| `derive-proposal-actions.ts:44` `canCancelProposal` | `false`: `security_council` is not in `CANCELABLE_AUTHORITIES` | 3 |
| `lib/redundant-defcon-1.ts:25` (now `lib/safe-harbour-redundancy.ts`) | an enacted Defcon 3 is not considered as the harbour activator | 2 |
| `manual-proposal/hooks/use-manual-proposal.ts:202,280` | the `kind === 'unknown'` guard lets a Defcon 3 bundle through — **correct, do not "fix"** | — |
| `proposal-detail/hooks/use-decoded-proposal.ts:93` | clears the signer table for a non-`multisig_update` — correct | — |

`orchestrator-be` does not participate: `action_type_from_hex` lives only in the desktop.
`src-tauri/src/infrastructure/signing.rs` does not participate either — it matches on upstream's
`MultisigAction`, and upstream's renderer already produces Defcon 3's four canonical lines
(`asm/crates/subprotocols/admin/txs/src/actions/updates/defcon3.rs:33-46`).

## 8. Tests

Five claims, each pinned where it lives. All of them are pure functions or the real codec — no mocks,
no I/O, no clock.

| # | Claim | Assertion | Where |
|---|---|---|---|
| 1 | The bytes are Defcon **3**'s, not a codec that merely agrees with itself | round-trip; `update_tx_type() == UpdateTxType::Defcon3`; and `encode(Defcon1) != encode(Defcon3)` | `action_codec.rs`, beside `defcon_1_round_trips_and_encodes_upstreams_defcon_1_tx_type:316-332` |
| 2 | The decoded-action boundary names it | `decode_action_hex(hex)` is `DecodedAction::Defcon3` | `action_builder.rs`, beside `:222-231` |
| 3 | The DTO boundary names it | `action_type_from_hex(&None, &hex) == "defcon_3"` | `proposals.rs`, beside `:1093-1104` |
| 4 | Both zod boundaries accept it, **and one row does not take the list down** | `proposalSchema` and `decodedActionSchema` parse; `z.array(proposalSchema)` parses a mixed list | `api/ipc-schemas.test.ts`, extending `:152-160` |
| 5 | The label reads `Defcon 3` | not `'Unknown'` (missing arm) and not `'Defcon 1'` (copied arm); `defcon_1` unchanged; `kind === 'cancel'` still wins | `src/lib/__tests__/proposal-type-label.test.ts` (new) |

Test 1's third assertion is the one with real value. Both payloads are empty unit structs, separated
only by the SSZ union selector, so a codec with the two encode arms crossed would round-trip happily
and sign the wrong lever. The Defcon 1 test's own comment anticipated this exact test.

Test 4's mixed-list assertion is not in the build plan's list. It is added because it pins the very
claim the build plan uses to order the phases (§3), and nothing pins it today.

Test 5 is a new file, added because `inferProposalTypeLabel` has no test at all and is the only
change here with no compiler net (§4). One cheap assertion joins
`manual-proposal/model/__tests__/action-type-from-decoded.test.ts` for the same reason: the `Record`
is exhaustive by type, but `defcon_3: 'defcon_1'` would compile.

**Not tested, deliberately:** anything end to end — there is no producer yet, and hand-writing a hex
fixture to assert one would restate what test 1 owns. No render test of the sign view: the repo has
no DOM runner for this path, and what §5 changes is a narrowing, which `tsc` already enforces.

## 9. Verification

The full [`AGENTS.md`](../../AGENTS.md) checklist:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd desktop-app && npm run format:check && npm run lint && npm run build && npm run test:unit
```

`npm run test:unit` discovers `src/**/*.test.ts(x)` from disk (`scripts/run-unit-tests.mjs`), which is
what CI runs (`.github/workflows/ci.yml:155-156`), so the new file needs no registration — confirm
the final `N/N test files passed` count went up by one. No `test:*` script is added to
`package.json`: that enumerated list is the debt the runner exists to kill.

Two structural checks that the phase stayed inside its scope:

```bash
grep -rn "build_defcon_3" desktop-app/          # nothing can create a Defcon 3
git diff --stat -- desktop-app/src/domain/create-proposal/   # empty (§6)
```

No manual walk. There is nothing to walk until a producer exists in Phase 5.

## 10. Blast radius

- **`Action::Defcon3` and `DecodedAction::Defcon3` are not dead code.** Both are `pub`, and `decode`
  constructs the first in production. No `#[allow]` is needed; if clippy ever seems to ask for one,
  the answer is a missing arm somewhere, not an attribute.
- **No test pinned `UnsupportedVariant("Defcon3")`** as expected behaviour, so nothing goes red for
  behavioural reasons. The four `E0004`s are mechanical and fixed in the same commit.
- **A Defcon 3 created outside the app** — by another orchestrator, or pasted through `/manual` —
  used to list as an *Unknown* row showing its raw hex. It now lists, labels and decodes as itself.
  That is the only user-visible change the phase makes.
- **`/manual` stops refusing a Defcon 3 bundle.** `use-manual-proposal.ts:202,280` rejects a decode
  of kind `unknown` with *"Unknown action kind — cannot decode this hex"*, which is what a Defcon 3
  hex produced until now. It imports from here on. That is the intended behaviour — the manual
  fallback is required for both levers ([AC 14](./security-council-defcon-3.md#14-the-manual-fallback-works-for-both))
  — and it is listed here because it is a behaviour change the diff does not show.
