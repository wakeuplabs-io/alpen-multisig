# Security Council — Defcon 1 (V1), Phase 2: Cancel gate by depth

**Functional contract:** [`security-council-defcon.md`](./security-council-defcon.md) — SSOT for
*what* V1 must do. This document never overrides it.

**Build plan:** [`security-council-defcon-implementation.md`](./security-council-defcon-implementation.md)
§4 Phase 2. This document is that phase at implementation detail.

**Closes:** AC 11, and
[Constraint 2](./security-council-defcon.md#2-cancelability-is-decided-per-action-and-per-live-depth-never-by-authoritysecuritycouncil).

## 1. The change in one sentence

The authority allow-list in `create_cancel_proposal`
(`orchestrator-be/src/application/proposals.rs:565-573`) becomes a call to Phase 1's
`lock_period_for_action` over the **target's** `action_hex`, rejecting when the depth is zero and
saying so.

## 2. Why the authority cannot answer

The current gate asks which authority signed the target:

```rust
if !matches!(target.authority, Authority::AlpenAdmin | Authority::StrataAdmin) {
    return Err(AppError::BadRequest(format!(
        "cancel is only supported for AlpenAdmin and StrataAdmin (got: {:?})",
        target.authority
    )));
}
```

That question has no answer for the Strata Security Council, because one authority signs both of
these:

| Target action | Depth | Cancellable on-chain? |
|---|---|---|
| Defcon 1 | `0`, hardcoded upstream | No — never enqueued, a cancel fails with `UnknownAction` |
| Defcon 3 | `confirmation_depths.defcon3`, per-deployment | Yes, while it sits in the queue (V5) |

Adding `SecurityCouncil` to the allow-list would open cancel for Defcon 1; leaving it out would close
it for Defcon 3. AC 11 goes further than "reject Defcon 1": it requires the rejection to *name the
depth*, which an authority-shaped condition cannot do because it never reads one.

## 3. The depth that decides is the target's, never the cancel's

Non-obvious and load-bearing. `depth_for_action` returns `0` for every `MultisigAction::Cancel`
(`orchestrator-be/src/infrastructure/asm_role_membership.rs:147`) — correct, because a cancel is never
itself enqueued ([Phase 1 §5](./security-council-defcon-phase-1.md#5-function-contract)). Passing the
cancel's own `action_hex` — the `action_hex` parameter this function already receives — would compile,
read plausibly, and reject **every** cancel in the system.

The gate must therefore resolve `target.action_hex`. `create_cancel_proposal` already loads the target
at `proposals.rs:553-557`, so the value is in hand and no signature changes.

## 4. Function contract

The order of checks after this phase:

| # | Check | Outcome when it fails |
|---|---|---|
| 1 | Target exists | `AppError::NotFound` |
| 2 | Target is `Approved` | `BadRequest`, unchanged |
| 3 | **Target's depth is not zero** | `BadRequest` naming the depth — this phase |
| 4 | A cancel for this target already exists | Returns it, unchanged (idempotent) |
| 5 | `threshold_for_authority` | `BadRequest`, unchanged |

The gate itself is the whole of the change:

```rust
let lock_period = lock_period_for_action(asm_rpc_url, &target.action_hex).await?;
if lock_period == 0 {
    return Err(AppError::BadRequest(/* a message naming the depth */));
}
```

**The gate stays where the allow-list stood** — step 3 of the table above, ahead of the idempotent
return. Moving it after step 4 would let an already-created cancel for a zero-depth target keep being
returned as valid, which is the exact state AC 11 exists to make unreachable. Ordering it before also
means the *reason* a Defcon 1 cancel is refused is its depth, not the unmapped
`Authority::SecurityCouncil` role that step 5 would otherwise report
(`asm_role_membership.rs:190-199`) — which is what AC 11 asks for, and it survives Phase 3 mapping
that role.

**Cost accepted knowingly:** one extra ASM RPC round-trip per cancel attempt.
`lock_period_for_action` reads live on every call and caches nothing, by AC 12a
([Phase 1 §7](./security-council-defcon-phase-1.md#ac-12a-is-structural-evidence-not-a-unit-test)).
Step 5 already makes an ASM call on the success path, so the added cost lands on the *rejected* path,
where it buys the live answer the constraint requires. Caching it to save the round-trip would
reintroduce exactly what Phase 1 removed.

**The rejection message.** Naming the depth is the requirement, per AC 11; the exact wording is not
normative. It says depth `0`, why that means never enqueued, and what would happen on-chain:

> `cancel is not possible: this action has a confirmation depth of 0, so it is never enqueued and an on-chain cancel would fail with UnknownAction`

It stays an `AppError::BadRequest` → HTTP 400 (`orchestrator-be/src/error.rs:42-58`), the same status
the allow-list returned. No handler change (`orchestrator-be/src/handlers/proposals.rs:210-232`), no
signature change, no new error variant: the rejection changes its *reason*, not its shape.

## 5. Blast radius

**Sequencer Manager becomes cancellable on the backend, deliberately.** Its updates are enqueued with
a configurable depth (`confirmation_depths.sequencer_update`), so the protocol has always allowed
cancelling them while they sit in the queue; the refusal was an artifact of the allow-list, not a
rule. Constraint 2 makes the depth the only question, and Sequencer Manager answers it non-zero. This
is a knowingly accepted widening, not a side effect.

Nothing user-visible changes with it, because the desktop hides the affordance independently — see
below. `threshold_for_authority` already maps `SequencerManager`
(`asm_role_membership.rs:190-199`), so the path is complete rather than half-open.

**Defcon 1 becomes structurally incancellable**, from the moment Phase 3 can create one. No later
phase has to remember to special-case it.

**Defcon 3 will pass the gate on its own** when V5 arrives, with no gate to reopen — the outcome
Constraint 2 was written to secure.

**`AlpenAdmin` and `StrataAdmin` are unchanged.** Their targets carry configurable, non-zero depths
and pass step 3.

**The frontend keeps its own allow-list, and this phase does not touch it.**
`CANCELABLE_AUTHORITIES = ['alpen_admin', 'strata_admin']` is duplicated across three files
(`desktop-app/src/domain/proposals-dashboard/components/proposals-dashboard.tsx:21`,
`desktop-app/src/screens/cancel-proposal-screen.tsx:21`,
`desktop-app/src/screens/proposal-detail-screen.tsx:21`). It is the same authority-shaped condition
Constraint 2 forbids, on the other side of the wire. It is left alone here for two reasons: the build
plan scopes Phase 2 to `orchestrator-be`, and for Defcon 1 it happens to produce the right answer
(no cancel CTA, which is what AC 10 demands anyway). It becomes wrong the moment Defcon 3 needs a CTA,
so **V5 owns replacing it** — recorded here so that phase does not have to rediscover it.

## 6. The test debt this uncovers

`ACTION_HEX = "deadbeef"` (`proposals.rs:770`) is not valid SSZ. While the gate was an allow-list,
nothing on the cancel path decoded the target's `action_hex`, so the fixture was never exercised.
Resolving the depth makes decoding mandatory, and the existing cancel tests would start failing on
*decode* instead of on the rule they are written to prove — a false green turning into a misleading
red.

They migrate to `action_codec::test_fixture_action_hex()` (`action_codec.rs:13`), the valid Strata
admin update already used by the handler tests (`handlers/mod.rs:144`). `ACTION_HEX` stays for the
tests that never decode it; replacing it everywhere would be churn unrelated to this phase.

## 7. Migration (one atomic commit)

1. Replace the allow-list (`proposals.rs:565-573`) with the depth gate of §4.
2. Add `test_fixture_defcon_1_action_hex()` to `action_codec.rs`, beside `test_fixture_action_hex()`
   (`:11-27`) and in the same shape. It encodes
   `MultisigAction::Update(UpdateAction::Defcon1(Defcon1Update))` — the payload-less construction
   Phase 1's tests already use (`asm_role_membership.rs:500`).
3. Rewrite the authority-rejection test as the depth-rejection test, and point the four cancel tests
   that build a target (`proposals.rs:1582`, `:1639`, `:1762`, `:1786`) at
   `test_fixture_action_hex()`.

One commit: step 1 without step 3 leaves the suite red, step 3 without step 1 pins the gate being
retired.

## 8. Tests

Three tests in `proposals.rs`, one rewritten and two migrated. No new test file, no new mock: the mock
already produces the pair this phase needs. `mock_lock_period`
(`asm_role_membership.rs:441-449`) dispatches through the real `depth_for_action`, so it answers
`2016` for a Strata admin update and `0` for Defcon 1 without being told to.

| # | Claim | Assertion |
|---|---|---|
| 1 | AC 11 — the gate is the depth, and the reason names it | A Defcon 1 target is rejected with a `BadRequest` whose message contains both `depth` and `0`. The retired gate cannot pass this: it rejected the same target for its authority, with a message that never mentions a depth. |
| 2 | Regression — a non-zero depth still cancels | The happy path still creates the cancel for a Strata admin target. Only its fixture changes. |
| 3 | Regression — idempotency survives behind the new gate | A second cancel for the same target returns the first. Only its fixture changes. |

Test 1 replaces `test_create_cancel_proposal_rejects_unsupported_authority`
(`proposals.rs:1847-1900`) rather than deleting it, as the build plan requires. Its two-case loop over
`SequencerManager` and `SecurityCouncil` goes with it: one of those two answers changes on purpose
(§5), and the case that still rejects now rejects for a different reason.

**No Sequencer Manager test is added.** Asserting that it now cancels would pin a *consequence* of the
rule rather than the rule, and it would have to be rewritten if the deployment's
`sequencer_update` depth were ever zero — which the gate should then honour. The depth resolution
itself is already covered by Phase 1's three `depth_for_action` tests — including the tripwire that
fires if upstream ever gives Defcon 1 a configurable depth, which is where that assertion belongs.

**The Defcon 1 fixture is test-only.** `test_fixture_defcon_1_action_hex()` joins
`test_fixture_action_hex()` in `action_codec.rs` under `#[cfg(test)]`. This is not Phase 3's codec
work: `decode_multisig_action_hex` is generic SSZ and already decodes every `MultisigAction` variant
(`action_codec.rs:6-9`), so nothing in production learns about Defcon 1 here. What Phase 3 adds is the
*builder* — the desktop's ability to produce that hex.

## 9. Out of scope

`Authority::SecurityCouncil` → `Role::StrataSecurityCouncil`, the Defcon 1 action builder, proposal
creation and its authorization gate, enactment detection, and every frontend change — Phases 3–6. The
frontend's own cancel allow-list — V5 (§5).

When this phase merges, no Defcon 1 proposal can exist to be refused. The gate is correct before the
action it protects against is buildable, which is the whole reason the two shared refactors are
ordered first.

## 10. Verification

`cargo test -p orchestrator-be`, then the full [`AGENTS.md`](../../AGENTS.md) pre-commit checklist.

Review must additionally confirm the structural claim of Constraint 2: no authority-shaped condition
remains anywhere on the backend cancel path — `grep -rn "cancel is only supported for" orchestrator-be`
returns nothing, and the only `Authority` value the path still reads is the one it copies onto the
cancel proposal and passes to `threshold_for_authority`.

End-to-end regtest verification belongs to the close-out of all six phases
([build plan §5](./security-council-defcon-implementation.md#5-verification)).
