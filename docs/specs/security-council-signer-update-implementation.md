# Security Council — Signer Update (V3) Implementation Plan

**Functional contract:** [`security-council-signer-update.md`](./security-council-signer-update.md) —
the SSOT for *what* V3 must do. This document is only *how* it gets built, and never overrides it.

**Master plan:** [`security-council.md`](./security-council.md) §6 Stage board, §7 Slice board.

**Story:** [`story-map.md`](../3-stories/story-map.md) US-E7.

**Status:** Not started. Four phases planned, plus one held in reserve.

A phase marked ✅ means the engineering step shipped, not that every acceptance criterion in the
contract is satisfied — the contract's `## Acceptance Criteria` section stays the measure.

## 1. Purpose and scope

V3 is the cheapest slice of the feature, and it is cheap for a reason worth stating: the application
has shipped `ThresholdConfigUpdate` end to end for three authorities since V1, and V1 and V2 between
them made the remaining generic machinery action-shaped rather than authority-shaped. What is left is
**one correctness rule, one form retarget, and their tests.**

**In scope**

- `UpdateTxType::StrataSecurityCouncilMultisigUpdate = 15` end to end, authorized by the Strata
  Administrator (US-E7).
- The standard cancel, which per PRD §5.2.2 applies here in full.
- The end-to-end test upstream does not have
  ([`security-council.md` §7.2](./security-council.md#72-coverage-upstream-does-not-have)).

**Not in scope**

- Safe Harbour address update (V4).
- Repairing [Constraint 4](./security-council-signer-update.md#4-acceptance-is-not-application-and-upstream-does-not-say-so).
  Pre-existing, applies to all four authorities, belongs to a slice that owns multisig-update
  correctness as a whole.
- Any protocol validity rule. The orchestrator stays coordination-only.

## 2. Traceability

| Phase | Name | Closes (contract) | Touches |
|---|---|---|---|
| 1 | `council_signer_update` is a readable type | AC 5; [Constraint 2](./security-council-signer-update.md#2-the-target-comes-from-the-action-never-from-the-session) (the Rust half) | `src-tauri`, `desktop-app/src/api` |
| 2 | Enactment reads two roles | AC 7, AC 7a; [Constraint 1](./security-council-signer-update.md#1-enactment-reads-two-roles-not-one) | `orchestrator-be`, `src-tauri` |
| 3 | The form targets the council | AC 1, 1a, 2, 3, 3a, 3b, 4, 11, 12; [Constraints 2](./security-council-signer-update.md#2-the-target-comes-from-the-action-never-from-the-session) and [3](./security-council-signer-update.md#3-the-form-validates-against-the-targets-config-never-the-sessions) | `desktop-app`, `src-tauri` |
| 4 | The cancel and the e2e | AC 6, 7b, 8, 9, 10, 13 | `e2e-tests`, `desktop-app` |
| 5 | Reserve — what the manual walk exposes | — | — |

## 3. Architecture

### What already exists and is reused

The list is long, and that is the point of the slice. Nothing below needs a line of new code.

| Piece | Location | Why it matters |
|---|---|---|
| `require_authorized_for_action` | `orchestrator-be/src/infrastructure/asm_role_membership.rs:243-264` | Generic over upstream's `authorized_role()`, which returns `StrataAdministrator` for tx type 15 (`asm/.../updates.rs:64`). A council session is refused with no council-specific code. |
| `lock_period_for_action` / `depth_for_action` | same module, `:112-149` | Resolves through `update_tx_type()`, so tx type 15 reaches `confirmation_depths.strata_security_council_multisig_update` with no new branch. This is what V1's Constraint 1 bought. |
| Cancelability on the DTO | `asm_role_membership.rs:152-193`, shipped in V2 Phase 3 (#525) | Derived from the live depth, not from an authority allow-list. A council rotation is cancelable exactly when its depth is non-zero. |
| `create_cancel_proposal` | `orchestrator-be/src/application/proposals.rs:755-800` | Stores the cancel under the target's authority (`strata_admin`) and requires the session to match — the correct rule here. |
| `multisig_update_post_conditions_met` | `orchestrator-be/src/infrastructure/asm_enactment.rs:358-390` | Takes keys, threshold, `last_seqno` and the update as four arguments with no opinion about where they came from. Phase 2 changes the callers, not this. |
| `render_signing_message` / `compute_sighash` | `desktop-app/src-tauri/src/infrastructure/signing.rs:57-67, 138-145` | Delegate entirely to `SigningMessage::for_action`. The nine canonical lines come out for free. |
| `get_multisig_config(authority)` | `desktop-app/src-tauri/src/commands/asm_state.rs:34-69` | Already accepts any authority and already maps `SecurityCouncil → Role::StrataSecurityCouncil`. The council's config is readable from the desktop **today**. |
| `signer-update-form-fields.tsx`, `validators/signer-update.ts` | `desktop-app/src/domain/create-proposal/` | The fields and the rules are identical; only the config they read against changes. |
| `showsActivationCountdown` | `desktop-app/src/lib/proposal-status.ts` | Already correct — it excludes only `defcon_1`. |
| `e2e_cancel_proposal.rs` | `e2e-tests/tests/` | The shape Phase 4's cancelled path follows, and it already initialises a `strata_security_council` authority and a depth for tx type 15 (`:53-72`). |
| `multisig_config_update(role, …)` | `asm/tests/harness/admin.rs:251-277` | Upstream's helper already supports `Role::StrataSecurityCouncil`. No test upstream ever passes it — Phase 4 is the first. |

### Where V3 lives in the frontend

Same answer V1 and V2 settled: **it extends `desktop-app/src/domain/create-proposal/`** and gets no
route of its own. The domain dispatches by action type, so a council rotation is one more entry in
`ACTION_TYPES_BY_AUTHORITY`, one more validator registry entry, and a retargeted config read.

**The fields component is reused, not parameterized and not duplicated.** Unlike Defcon 1 vs Defcon 3
— which differ in copy and needed a parameter — a council rotation and an administrator rotation
render the *same* inputs. What differs is which config feeds them, and that is a prop the component
already takes.

### The two breaking points

Everything else in V3 is additive.

**Enactment stops deriving one role.** `is_proposal_enacted_on_asm`
(`asm_enactment.rs:173-207`) resolves a single role from the proposal's authority at `:183` and reads
`keys`, `threshold` and `last_seqno` from it at `:192-199`. For tx type 15 the first two belong to
the council and the third to the administrator. Both terms would be wrong, in opposite directions,
and neither failure raises an error — see
[Constraint 1](./security-council-signer-update.md#1-enactment-reads-two-roles-not-one). The rule
`extract_multisig_config_update` encodes at `:327-337` — "action variant does not match proposal
authority" — stays true for the three self-rotating updates and stops being universal.

**The target stops coming from the session.** `use-create-proposal.ts:80` casts the session authority
into the builder's `role`, excluding `security_council` in the cast itself. Widening
`api/action-builder.ts:6` is what makes the fourth value expressible; the doc comment at
`desktop-app/src-tauri/src/domain/action.rs:112-113` is what has to stop claiming a role can only
modify its own config.

## 4. Phased plan

Every phase: its own branch off `develop`, one atomic commit (never a commit that repairs the one
before it), and the full [`AGENTS.md`](../../AGENTS.md) pre-commit CI checklist green before pushing.
The phases are **sequential, not parallel**.

**Phase 3 cannot start until V2's Phase 5 has merged.** It touches
`desktop-app/src/domain/create-proposal/`, which V2 is editing for Defcon 3's create flow. Phases 1,
2 and 4 do not overlap V2 at all — Phase 4 deliberately writes a new e2e file rather than extending
`e2e_defcon_probe.rs`, which V2's Phase 7 will edit.

### Phase 1 — `council_signer_update` is a readable type, end to end

Make a council rotation a legal value everywhere it is *read*, with no way to create one yet: the two
codec arms (`action_codec.rs:105-126` encoding, `:244-246` decoding), `action_type_from_hex`
(`commands/proposals.rs:169-187`) reporting it distinctly instead of collapsing every multisig update
to `"multisig_update"`, the corrected doc comment on `MultisigUpdate.role`, and on the TypeScript side
the `ActionType` union, the IPC schemas, the decoded-action schema and the type label.

**Why emitter and acceptor cannot split across two PRs.** `actionType` is a **closed** Zod enum. A
Tauri that emits a value the schema does not accept fails the parse of *every proposal in the same
list*, not just the new one. V2 learned this in its own Phase 1 and the test in
`src-tauri/src/commands/proposals.rs` exists because of it. Beyond that, Phases 2, 3 and 4 each need
the value to be legal merely to **write a fixture**.

It is a prerequisite, not a product step: nothing in the application can produce a council rotation
hex when it merges.

**Tests.** Codec round-trip in both directions plus a tripwire that the variant still encodes
`UpdateTxType::StrataSecurityCouncilMultisigUpdate`; `action_type_from_hex` distinguishing a council
rotation from an administrator one; the IPC schema contract test accepting the new `actionType` and
the new decoded-action kind.
**Not tested:** anything end to end. There is no producer yet, and hand-writing a hex fixture to
assert one would only restate what the codec test owns.

### Phase 2 — Enactment reads two roles

The target role is derived from the action variant; the authorizing role keeps coming from the
proposal's authority. `keys` and `threshold` are read from the target's `AuthorityConfig`,
`last_seqno` from the authorizing role's. `multisig_update_post_conditions_met` is untouched.

Applied to **both** copies — `orchestrator-be/src/infrastructure/asm_enactment.rs` and
`desktop-app/src-tauri/src/infrastructure/asm_enactment.rs`. They move together or the desktop and
the backend disagree about whether a rotation enacted.

Also in this phase: the private `authority_to_role` at `asm_enactment.rs:424-434` maps three
authorities while `asm_role_membership.rs:266-281` maps four. Two functions with the same name and
different answers is how the council reaches a wrong arm silently, and this is the phase that would
otherwise depend on the wrong one.

**This precedes the create flow**, for the same reason V1's and V2's Phase 4 did: never let a signer
create a proposal that can only park at Approved forever.

**Pick up while here:** `scripts/asm-params.example.json` no longer deserializes against the current
pin — `ConfirmationDepths` has no `serde(default)` and the example omits
`strata_security_council_multisig_update`, `defcon3` and `safe_harbour_address_update`.
`scripts/asm-params.json` and `staging/asm-params.template.json` already carry all three. V2's plan
(§6) parked this for "a phase that touches params", and this is the phase that reads that depth.

**Tests.** A truth table with one row per meaningful case, and the two substitution failures as tests
carrying their own names: one where the administrator's signer set changes and the answer must not,
one where the council's `last_seqno` advances and the answer must not. A test called
`council_rotation_ignores_the_councils_own_seqno` is the documentation.
**Not tested:** an ASM-backed integration test inside `orchestrator-be`. It would be the flakiest test
in the repository, and Phase 4's e2e proves the chain behaviour this predicate encodes.

### Phase 3 — The form targets the council

`council_signer_update` in `ACTION_TYPES_BY_AUTHORITY.strata_admin`, after the administrator's own
entry so the default selection does not change; the widened `role` union in
`api/action-builder.ts:6`; the builder call passing `role: 'security_council'`; the validator registry
entry; and the config read retargeted through the schema, the validator context, the form's threshold
reset and the preview.

**Why this is one commit and not two.** The obvious split — "add the menu entry", then "read from the
target" — leaves a PR where the form offers a council rotation while validating against the
administrator's signer set. Two of `signer-update.ts`'s rules make decisions from that set
(`:98-111` and `:113-157`), so the intermediate state accepts a key already on the council and allows
a threshold that will exceed the resulting council, and sends both to a hardware signer. An
intermediate state that is wrong in the signer-safety dimension costs more than the review burden of
one 300-line PR. See [Constraint 3](./security-council-signer-update.md#3-the-form-validates-against-the-targets-config-never-the-sessions).

**The signing message needs no code and must not get any.** It resolves through the same Rust renderer
the device signs over. It gets one Rust tripwire: the rendered message names both roles on separate
lines and **differs** from an administrator signer update's — an upstream change that collapsed them
would otherwise be discovered on a signer's screen.

**Also in this phase: the no-op update.** The validator requires one *row* in each of add and remove
(`validators/signer-update.ts:6-11`), but blank rows are discarded downstream
(`use-create-proposal.ts:81-82`), so an update that changes nothing is buildable today and would be
accepted on chain as a no-op. The rule belongs with the retarget because both are about what the
validator compares against
([AC 3b](./security-council-signer-update.md#3b-the-update-must-be-a-real-change)). A threshold-only
change stays allowed.

**Tests.** The builder (build → decode → `StrataSecurityCouncilMultisig`); the signing-message
tripwire; pure TS for the per-authority menu and its default; the validator answering against a
supplied signer set with a fixture where the council's and the administrator's sets disagree — which
is the one property the retarget can break silently; and the no-op refusal, with a threshold-only
change as its counter-case.
**Not tested:** the form component and the sign view. No DOM runner. The honest substitute is the
manual walk, budgeted below.

### Phase 4 — The cancel and the e2e

Mostly verification plus the test. `build_cancel_action_hex` requires a non-null queue `UpdateId`,
which a queued rotation has; `create_cancel_proposal` stores the cancel under the target's authority
and requires the session to match, which here is the Strata Administrator. No new backend code is
expected — and if the phase discovers otherwise, that discovery is its most valuable output.

The deliverable is `e2e-tests/tests/e2e_council_rotation.rs`, a **new file**, following the shape of
`e2e_cancel_proposal.rs`:

- **Enacted path** — submit an administrator-signed council rotation, assert it is queued with the
  council's config unchanged, mine exactly `depth` blocks, assert the council's config changed and
  the **administrator's** `last_seqno` advanced. This is the only automated proof of
  [Constraint 1](./security-council-signer-update.md#1-enactment-reads-two-roles-not-one) against a
  real chain.
- **Cancelled path** — cancel inside the window, mine `depth`, assert the queue is empty **and the
  council's config is still unchanged**.
- **Membership effect** — after the enacted path, submit a Defcon signed by a quorum of the *new*
  council and assert it is accepted, then one carrying a removed member's signature and assert it is
  not ([AC 7b](./security-council-signer-update.md#7b-the-new-council-can-act-and-the-removed-signers-cannot)).
  A rotation only means something if it changes who can pull the emergency lever, and this is the
  case §7.2 names as untested anywhere.

This is also the first end-to-end exercise of tx type 15 anywhere, upstream included.

**Anti-flake:** reuse the existing `bitcoind`-availability skip and mine an exact depth. Never sleep.
**Not tested:** the desktop cancel journey. Manual walk.

### Phase 5 — Reserve

V1 needed two phases nobody planned (#511, #512) plus four close-out PRs, all of them born from
running the flow by hand rather than from reviewing it; V2 budgeted one for the same reason. Budget
one here and expect it to be about copy — this is the first action whose title must disambiguate two
similar entries in one menu.

### On splitting further, and on not splitting

Recorded because the trade-off was made deliberately rather than by default:

- **Phases 1 and 2 could merge.** Both are Rust, ~350 lines together, and V2 shipped its contract,
  plan and first two phases in one PR (#524). They stay separate because they close different things
  — one is vocabulary, the other a correctness rule — and Phase 2 is the only part of this slice that
  needs careful review. Bundling it with renames and enum widenings dilutes that review, which is the
  cost the split is buying down.
- **Phase 3 does not split**, for the signer-safety reason above. It is the largest PR of the slice
  and that is the right outcome.
- **Vertical vs horizontal.** Phase 1 crosses Rust↔TypeScript because the closed Zod enum forces it;
  Phases 2 and 4 stay in one layer because nothing forces them out of it. A uniformly vertical
  slicing would force exactly the cut Phase 3 forbids, so the shape follows the data contracts rather
  than a rule about slice geometry.

## 5. Verification

Per phase: the `AGENTS.md` checklist, plus evidence that the acceptance criteria the phase claims to
close are covered by tests.

**Every commit that adds a frontend test file confirms CI picks it up.** CI globs
`src/**/*.test.ts(x)` rather than enumerating scripts — V1 shipped a phase where 21 of 62 test
scripts never ran, twice — so the check is that the new file falls inside the pattern.

End to end, once all four land, on regtest with the local stack
(`./scripts/local-stack.sh --clean` if any state predates the ASM pin bump). The stack already carries
a four-key council at threshold 2 and a depth of 30 for tx type 15
(`scripts/asm-params.json`), so no fixture work is needed:

1. A Strata Administrator signer sees two signer-update entries and reaches the council one; a
   Security Council signer sees neither entry and cannot navigate to it.
2. The form shows the council's four signers and threshold 2 — not the administrator's.
3. The rendered message matches the signer's screen, names both roles on separate lines, and carries
   the `Action Details:` block.
4. Quorum, broadcast — Approved, then Awaiting enactment with a countdown to `reveal + 30`, and the
   council's config unchanged.
5. Path A: mine 30 blocks → `Enacted`, and the council's new config is visible in
   `strata_asm_getAnchorState`.
6. Path B: cancel inside the window → the target reads `Canceled`, the council's config is unchanged,
   and nothing reads `Enacted`.
7. An administrator signer update created in the same session still validates against the
   administrator's own signers.
8. `cargo test -p alpen-multisig-e2e-tests` green, including `e2e_council_rotation`.

## 6. Known debt this slice does not take

- **Acceptance is not application.** `apply_multisig`
  (`asm/crates/subprotocols/admin/subprotocol/src/handler.rs:174-182`) logs and swallows a failed
  `validate_update` after the seqno is consumed and the queue entry drained, so a rotation can be
  accepted and apply nothing. Our post-conditions compare the real config, so no false `Enacted` is
  reported — the proposal resolves as `Superseded`. Pre-existing and identical for the three signer
  updates shipped since V1. See
  [Constraint 4](./security-council-signer-update.md#4-acceptance-is-not-application-and-upstream-does-not-say-so).
- **Signatures collected offchain for a Defcon 3 cancel do not survive a council rotation.** They are
  the old council's and will not verify. Out of band for the application, which holds no notion of a
  quorum spanning a membership change. Recorded because this is the slice that can cause it, and
  because it is the concrete shape of the untested case
  [`security-council.md` §7.2](./security-council.md#72-coverage-upstream-does-not-have) names.
- **`Authority::PayoutAdmin` still maps to no ASM role.** Expected, not a gap — see
  [`security-council.md` §5.5](./security-council.md#55-two-prd-items-have-no-upstream-counterpart-at-any-revision--both-resolved).
- **An `activation_height` that fails to compute is never retried**, and a stored one can go stale if
  the deployment changes the depth while an update is queued. Both recorded in V2's plan §6 and
  unchanged here; a council rotation inherits them like every other queued action.
- **N+1 `strata_asm_getStatus` reads in the reconciliation loop**, recorded at V1 close-out and
  revisited but not fixed in V2 Phase 3. Unchanged.

## 7. Close-out

Four places do not update themselves, and both V1 and V2 needed a follow-up PR for exactly this
drift: the `Status:` header of
[`security-council-signer-update.md`](./security-council-signer-update.md), the header of
[`security-council.md`](./security-council.md), its §6 Stage board and its §7 Slice board.
