# Security Council — Defcon 1 (V1), Phase 4: Enactment detection

**Functional contract:** [`security-council-defcon.md`](./security-council-defcon.md) — SSOT for
*what* V1 must do. This document never overrides it.

**Build plan:** [`security-council-defcon-implementation.md`](./security-council-defcon-implementation.md)
§4 Phase 4. This document is that phase at implementation detail.

**Closes:** AC 8.

## 1. The change in one sentence

`asm_enactment.rs:102-104` answers `BadRequest("Defcon1 enactment detection is not implemented
yet")`; it becomes a real post-condition read — the safe harbour is activated and no Defcon 1
entry sits in the admin queue.

## 2. What this phase is not

It is not the end of V1. A council signer still has no screen: Phases 5 and 6 own the form, the
signing message and the lifecycle display. What changes here is invisible to anyone without an
HTTP client — a Defcon 1 proposal that today parks at `Approved` forever now reaches `Enacted`.

It is also not Defcon 3, and not the safe-harbour address update. Both keep their
"not implemented yet" arms (`:105-110`), and the verification in §9 pins that they do.

## 3. The two post-conditions

Upstream's admin handler routes a Defcon 1 through two decisions, and each one leaves a mark in
canonical state.

**The activation.** `handler.rs:167` sends both Defcon variants to the same `relay_bridge_defcon`,
which ends in `activate_safe_harbour()` → `SafeHarbour::set_activated(true)`
(`asm/crates/subprotocols/bridge-v1/subprotocol/src/state/bridge.rs:107-109`). So
`bridge.safe_harbour().is_activated()` is the post-condition, and it is a **value**, never an
edge — see §3.2.

**The queue bypass.** `handler.rs:81-92` enqueues any update whose confirmation depth is non-zero
and applies the rest immediately. Defcon 1 is hardcoded to depth `0` upstream (Phase 1 §4), so it
never enters the queue. `e2e_defcon_probe.rs:117` asserts exactly that against a live ASM.

Enacted is the conjunction: `is_activated() && no Defcon 1 queued`.

### 3.1 What the queue check actually distinguishes — the build plan is wrong about this

The build plan (`security-council-defcon-implementation.md:203`) justifies the queue check by
saying the activation flag "cannot distinguish a Defcon 1 from a Defcon 3 that has matured".

The premise is true — `handler.rs:167` gives both variants the same flag, so the flag alone says
"some Defcon executed", not which one. The conclusion does not follow. A **matured** Defcon 3 has
already left the queue (`e2e_defcon_probe.rs:165-171` asserts it), so a check for queued entries
does not catch it. And the check is for **Defcon 1** entries specifically; a Defcon 3 sitting in
the queue would not match it either — and a queued Defcon 3 has not activated anything, so the
first post-condition already excludes that case.

What the queue check is, honestly stated: **a tripwire against upstream drift.** It is the only
part of this arm that would notice if `ConfirmationDepths` ever grew a Defcon 1 field, or if
`handler.rs:84` changed how depth `0` is surfaced. Under that drift a Defcon 1 would start being
enqueued, and without the check a proposal would be marked `Enacted` while its update was still
pending and still cancellable. It costs one `matches!` over a slice the arm already decodes.

The ambiguity the check does **not** remove, and which this phase accepts: a Defcon 3 that matured
between a Defcon 1's reveal confirmation and the next reconcile poll activates the safe harbour,
and the Defcon 1 proposal would read as enacted on the strength of somebody else's action. This is
the same class of risk the module already declares at `asm_enactment.rs:1-4` ("concurrent
overlapping updates may produce ambiguous post-condition matches"), and it is out of reach for V1:
Defcon 3 has no product flow until V2, so no orchestrator-tracked proposal can produce it.

### 3.2 A value, not a transition

The contract's Edge Cases (`security-council-defcon.md:454`) require that a Defcon 1 whose safe
harbour was **already** activated before the broadcast still reaches `enacted` — upstream's
`set_activated(true)` is idempotent. Any design that watched for the flag *flipping* would leave
such a proposal stuck at `Approved`. Reading the flag's current value satisfies the requirement by
construction, and the conjunction with the queue check does not weaken it.

### 3.3 One RPC call, and why not `strata_asm_getSafeHarbour`

Upstream exposes `strata_asm_getSafeHarbour(block_hash)`
(`asm/bin/asm-runner/src/rpc_server.rs:142`), which this repo does not use and will not start
using here. The `strata_asm_getStatus` call at `asm_enactment.rs:41` already ran before the
`match` and already produced the whole `AnchorState`, from which both the bridge section and the
administration section decode — **from the same block**. A second call for the safe harbour would
read a different tip than the queue check, tearing a post-condition that is by definition a
conjunction over one state.

### 3.4 Reveal-block ordering needs no new code

The contract says the activation must be observed "in the same block as or after the reveal was
confirmed" (`security-council-defcon.md:163`). The existing call sites already enforce the only
half of that which this architecture expresses: `reconcile_enacted_for_authority`
(`proposals.rs:359-364`) and `reconcile_enacted_for_action` (`:461-466`) both skip any proposal
that is not `broadcast_status == RevealConfirmed` with a `reveal_txid` present, and
`report_broadcast_progress` (`:672-677`) refuses the transition outright otherwise.

Stated plainly, because it is a V1 constraint a later maintainer should not have to rediscover:
**no height comparison happens anywhere in `asm_enactment.rs`**, and no arm performs one. What the
call sites give is "the reveal is confirmed"; what they do not give is "the activation happened at
or after that block". In V1 the gap is unreachable — Defcon 1 is the only orchestrator-tracked
action that can set this flag, and Edge Cases (`security-council-defcon.md:454`) already rules
that a pre-activated safe harbour must still enact — so closing it would mean building a
per-block-height read for one arm, which nothing in V1 can exercise. If Defcon 3 ever gains a
product flow (V2), this is the assumption that has to be revisited, together with §3.1.

## 4. Function contract

The dispatch arm, replacing `asm_enactment.rs:102-104`:

```rust
MultisigAction::Update(UpdateAction::Defcon1(_)) => {
    let bridge = decode_bridge_state(&anchor).map_err(AppError::BadRequest)?;
    let admin = decode_admin_state(&anchor).map_err(AppError::BadRequest)?;
    let safe_harbour_activated = bridge.safe_harbour().is_activated();
    let defcon1_queued = admin
        .queued()
        .iter()
        .any(|q| matches!(q.action(), UpdateAction::Defcon1(_)));
    Ok(defcon1_enacted(safe_harbour_activated, defcon1_queued))
}
```

The two locals are named for the parameters they fill, and that is deliberate. Both are `bool`, so
a swapped call site compiles, passes the predicate's test — which never exercises the caller — and
inverts the post-condition in production. Matching names is the cheapest defence that does not cost
a newtype; the alternative, inlining `bridge.safe_harbour().is_activated()` into the call, hides
the mismatch instead of showing it.

and the decision it delegates to, beside `ee_stf_vk_enacted`:

```rust
/// Defcon 1 executes at depth 0: it activates the safe harbour in the reveal block and never
/// enters the admin queue. A queued Defcon 1 means upstream changed that depth, not that this
/// proposal enacted.
fn defcon1_enacted(safe_harbour_activated: bool, defcon1_queued: bool) -> bool {
    safe_harbour_activated && !defcon1_queued
}
```

| Input | Result |
|---|---|
| Safe harbour activated, no Defcon 1 queued | `Ok(true)` — the proposal is promoted to `Enacted` |
| Safe harbour not activated | `Ok(false)` — reveal confirmed but ASM has not applied it yet |
| Safe harbour activated, a Defcon 1 queued | `Ok(false)` — upstream drift (§3.1); refuse rather than promote |
| `AnchorState` has no bridge or no administration section | `Err(BadRequest)` — retried next poll (§5) |

Nothing is added to the signature of `is_proposal_enacted_on_asm`: `authority` and `seq_no` stay
unused by this arm, as they are by the `OlStfVk`, `Sequencer` and `OperatorSet` arms. In
particular the arm does **not** need `Role::StrataSecurityCouncil`, so the module-local
`authority_to_role` (`:287`) — which still maps only three authorities — is untouched.

## 5. Blast radius

- **`asm_enactment.rs` only.** No call site changes: all three (`proposals.rs:365`, `:475`,
  `:678`) already pass `action_hex` and already handle both `Ok` and `Err`.
- **Error handling is unchanged and correct.** A decode failure keeps returning `BadRequest`; the
  two reconcilers log it and move on, so the proposal stays `Approved` and is retried on the next
  poll — which is what Edge Cases (`security-council-defcon.md:453`) requires of an unavailable
  ASM. `report_broadcast_progress` keeps propagating, which is right for a client-asserted status.
- **`extract_multisig_config_update` is untouched.** Its `Defcon1(_)` arm (`:227`) is unreachable
  from the dispatch and stays as `Ok(None)`: it is there for match exhaustiveness over upstream's
  enum, not as a second dispatch.
- **A pre-existing warning becomes routine.** `report_broadcast_progress` logs "update not found
  in ASM queue after RevealConfirmed" (`proposals.rs:735`) whenever `update_id_in_queue_for_action`
  returns `None`. For every Defcon 1 that is the correct and permanent state — depth 0 never
  enqueues — so the line is expected noise, not a symptom. (`reconcile_update_id_in_queue:444`
  handles the same `Ok(None)` silently, so only the broadcast-progress path is loud.) Left alone
  here; if it becomes a nuisance it belongs to Phase 6's lifecycle work, not to this arm.
- **`mock_is_enacted` (`:323`) stays URL-keyed and action-blind.** Under `mock://asm-enacted` a
  Defcon 1 enacts vacuously, exactly as every other action does. Phase 1's precedent for making a
  mock action-aware does not transfer: `mock_lock_period` delegates to the real `depth_for_action`
  because a depth is a pure lookup, whereas enactment is a fact about the world that no in-process
  mock can derive. The local-stack behaviour this mock exists for — a broadcast proposal reaching
  `Enacted` — is the behaviour we want.

## 6. Migration

One atomic commit: the test, the predicate and the arm. There is nothing to expand or contract —
the arm being replaced returns an error, so no caller can regress and no intermediate state
exists. Written test-first: `defcon1_enacted` is asserted before it is defined.

## 7. Tests

One test, on the extracted predicate, in the module's existing `mod tests`.

| # | Claim | Assertion |
|---|---|---|
| 1 | Both post-conditions are required, and together they suffice | `defcon1_enacted(false, false)` and `(true, true)` are false; `(true, false)` is true |

Named `defcon1_enacted_requires_safe_harbour_active_and_queue_clear`, mirroring
`ee_stf_vk_enacted_requires_seqno_consumed_and_not_queued` (`:533-537`) — the same shape of
decision, tested the same way.

**No test drives `is_proposal_enacted_on_asm` for Defcon 1.** Doing so needs a real SSZ
`AnchorState` carrying both a bridge and an administration section; `orchestrator-be` has no such
fixture and hand-building one would pin SSZ layout we do not own, in a test that duplicates
coverage that already exists against a real ASM. That coverage is
`e2e-tests/tests/e2e_defcon_probe.rs::e2e_defcon1_activates_safe_harbour_in_the_reveal_block`,
which asserts `bridge.safe_harbour().is_activated()` (`:110`) and `admin.queued().is_empty()`
(`:117`) — precisely the two reads this arm performs.

**No test asserts the `matches!` over the queue.** It has no branch of its own to get wrong that
the predicate's second argument does not already cover, and asserting it would require the same
missing fixture.

**What this leaves undetectable, stated rather than glossed:** a unit test of the predicate cannot
catch a fault in the caller — the `matches!` naming the wrong variant, the queue boolean inverted,
or the two arguments swapped. All three need the SSZ fixture this crate does not have. The
mitigations are the naming discipline in §4, review, and the e2e probe. That is the honest cost of
the fixture decision, and it is the same cost every other arm in this module already pays.

## 8. Out of scope

- **Defcon 3 (V2) and SafeHarbourAddress (V4)** keep their error arms. Defcon 3's post-condition
  is genuinely different — it *does* pass through the queue — and belongs with its own slice.
- **Phases 5 and 6** own everything a signer can see: the creation form, the four-line message,
  the "Quorum reached" label, the absent cancel affordance and the Past list.
- **The contract's stale Critical Files row** (`security-council-defcon.md:462`) attributes the
  dispatch to `application/proposals.rs`; it lives in `infrastructure/asm_enactment.rs`, as the
  build plan says. Not corrected here — it changes no behaviour and the contract's AC 8 wording,
  which is what this phase is measured against, is accurate.

## 9. Verification

```bash
cargo test -p orchestrator-be
```

then the full [`AGENTS.md`](../../AGENTS.md) pre-commit checklist (`cargo fmt --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`). The frontend
is untouched by this phase.

Structural evidence that the phase stayed inside its scope:

```bash
grep -n "not implemented yet" orchestrator-be/src/infrastructure/asm_enactment.rs
```

must no longer report the Defcon 1 line, and must still report the Defcon 3 and
SafeHarbourAddress ones.

End-to-end regtest verification belongs to the close-out of all six phases, not to this one. The
upstream capability this arm reads is already proven by `e2e-tests/tests/e2e_defcon_probe.rs`.
