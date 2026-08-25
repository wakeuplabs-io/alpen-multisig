# Security Council — Defcon 1 (V1), Phase 1: Per-action lock period

**Functional contract:** [`security-council-defcon.md`](./security-council-defcon.md) — SSOT for
*what* V1 must do. This document never overrides it.

**Build plan:** [`security-council-defcon-implementation.md`](./security-council-defcon-implementation.md)
§4 Phase 1. This document is that phase at implementation detail.

**Closes:** AC 12, AC 12a, and
[Constraint 1](./security-council-defcon.md#1-lock-period-is-per-action-never-per-authority).

## 1. The change in one sentence

`lock_period_for_authority(rpc_url, authority)` becomes `lock_period_for_action(rpc_url, action_hex)`:
the confirmation depth is resolved from the action the proposal carries, not from the authority that
signed it.

## 2. Why the authority cannot answer

`authority_to_update_tx_type` (`orchestrator-be/src/infrastructure/asm_role_membership.rs:161`) maps
one authority to one `UpdateTxType`. The Strata Security Council has two, with opposite depths:

| Action | `UpdateTxType` | Depth |
|---|---|---|
| Defcon 1 | `Defcon1 = 41` | `0`, fixed upstream — no configuration field exists |
| Defcon 3 | `Defcon3 = 43` | `confirmation_depths.defcon3`, per-deployment |

A function keyed on the authority would have to return one number for both. The action is the only
input that separates them.

## 3. This is a bug fix, not a no-op refactor

The per-authority mapping is already wrong for authorities in production, and the phase corrects it.

`strata_admin` can create three action types today
(`desktop-app/src/domain/create-proposal/model/action-type-config.ts:34`): `signer_update`,
`vk_update`, `operator_set_update`. All three currently resolve to
`confirmation_depths.strata_admin_multisig_update`, because that is what the authority maps to. Their
real depths are three independently configurable fields
(`ConfirmationDepths`, `confirmation_depth.rs:22-33`):

| Proposal | Depth before | Depth after |
|---|---|---|
| `strata_admin` / `signer_update` | `strata_admin_multisig_update` | `strata_admin_multisig_update` — unchanged |
| `strata_admin` / `vk_update` | `strata_admin_multisig_update` | `ol_stf_vk_update` |
| `strata_admin` / `operator_set_update` | `strata_admin_multisig_update` | `operator_update` |
| `alpen_admin` / `vk_update` | `alpen_admin_multisig_update` | `ee_stf_vk_update` |
| `sequencer_manager` / `sequencer_key_update` | `strata_seq_manager_multisig_update` | `sequencer_update` |

Behaviour is unchanged **only** where the action's `UpdateTxType` is the authority's own multisig
update. Everywhere else the resolved depth changes — deliberately, to the correct value.

The change is invisible on the current fixtures because they set every depth to the same number
(`e2e-tests/src/fixtures/signer_update_enacted.rs:86-98` all `144`, `:129-141` all `5`), and
invisible in production because `activation_height` is currently write-only (see §8). Neither makes
it a no-op, and a regression test phrased as *"resolves to what it resolved to before"* would pin the
old, wrong mapping. See §7 for how it is phrased instead.

## 4. Deviation from the build plan: compose upstream's table, do not copy it

The build plan and Constraint 1 both describe the resolution as *"check the action type, return
hardcoded 0 for Defcon 1, query `confirmation_depths.defcon3` for Defcon 3"*. Upstream
(`alpenlabs/asm` @ `b84eb28`) already owns both halves of that table:

- `UpdateAction::update_tx_type()` — `crates/subprotocols/admin/txs/src/actions/updates/mod.rs:58` —
  maps every variant, `Defcon1` and `Defcon3` included, to its `UpdateTxType`. The `enum UpdateAction`
  doc comment (`:34-37`) states the invariant this rests on: *"the wire-format tx type, the variant
  identity, and the per-variant `RenderSigningMessage` impl are all in lockstep, so adding a new admin
  update kind forces matching arms across all dispatch sites."*
- `ConfirmationDepths::get()` — `crates/params/src/subprotocols/admin/confirmation_depth.rs:38` —
  already returns `None` (the "apply immediately" sentinel) for `UpdateTxType::Defcon1` and
  `self.defcon3` for `UpdateTxType::Defcon3`. `AdministrationSubprotoState::confirmation_depth`
  delegates to it.

This phase composes those two calls. **The observable behaviour is exactly what Constraint 1
mandates** — Defcon 1 resolves to `0`, Defcon 3 to the live `confirmation_depths.defcon3` — and AC 12
is asserted directly against it.

**The reason is duplication, not drift.** Neither `UpdateAction` nor `ConfirmationDepths` is
`#[non_exhaustive]`, so a local match would be compile-time exhaustive and an upstream variant would
break our build rather than slip past. What a local copy would risk is restating upstream's *values*
wrongly, and owning a second table that has to be kept honest by hand. The orchestrator stays
coordination-only: it reads canonical state, it does not decide depths.

**The exposure this creates runs the other way, and §7 covers it with a tripwire.** If upstream ever
gives Defcon 1 a configurable field, our composition would follow it silently and Constraint 1's
"hardcoded 0" would break with nothing failing. The arm we depend on is a comment away from being
editable (`confirmation_depth.rs:52-54`):

```rust
// Defcon1 is the emergency lever — by definition it applies immediately,
// so there is no per-deployment knob for it.
UpdateTxType::Defcon1 => 0,
```

## 5. Function contract

```rust
/// Confirmation depth (in blocks) before the update in `action_hex` activates.
///
/// Returns `0` for actions that bypass the queue and apply immediately.
pub(crate) async fn lock_period_for_action(
    rpc_url: &str,
    action_hex: &str,
) -> Result<u64, AppError>
```

Shape matches its neighbours `threshold_for_authority` and `update_id_in_queue_for_action`: `async`,
`&str` RPC URL, then `strata_asm_getStatus` → `AnchorState` → `AdministrationSubprotoState`.

**Order is decode → mock → RPC**, not the mock-first order of `lock_period_for_authority` — an
action-keyed mock cannot answer before the action is decoded. Consequence to accept knowingly: an
undecodable `action_hex` now errors on a mock URL, where the authority-keyed version returned `2016`.
That is the correct behaviour and no test depends on the old one.

| Input | Result |
|---|---|
| `MultisigAction::Update(u)` | `admin.confirmation_depth(u.update_tx_type()).unwrap_or(0)` |
| `MultisigAction::Cancel(_)` | `0` |
| Undecodable `action_hex` | `Err(AppError::BadRequest)`, message from `decode_multisig_action_hex` |
| RPC failure | `Err(AppError::BadRequest)`, unchanged from the function being replaced |

**Why `Cancel` returns `0`.** A cancel transaction is never itself enqueued — it applies when it
confirms — so zero is the true delay before it takes effect. `CancelAction` does embed the
`UpdateAction` it targets (`cancel.rs:15`), so returning *that* update's depth would compile and read
plausibly, but it would be a number that is wrong in every path that could ever consume it. `0` is
safe everywhere, which matters because the only thing keeping this value away from `activation_height`
today is the `!updated.is_cancel()` guard at `proposals.rs:700`.

This arm is not what Phase 2 consumes. Phase 2's gate lives in `create_cancel_proposal`
(`proposals.rs:544`), which already resolves the target proposal at `:554-557` and will pass the
**target's** `action_hex` — an `Update`.

### Testability seam

The lookup is passed in rather than reached for, so the decision is testable without an ASM:

```rust
fn depth_for_action(action: &MultisigAction, depth_of: impl Fn(UpdateTxType) -> Option<u16>) -> u64
```

Production passes `|t| admin.confirmation_depth(t)`. Tests pass `|t| depths.get(t)` over a
`ConfirmationDepths` built literally — every field is `pub`. No global mutable state, so no flaky
test, and the assertions still run through upstream's real `ConfirmationDepths::get`.

### Mock

`mock_lock_period` becomes action-keyed **by delegating to `depth_for_action`** over a
`ConfirmationDepths` with every field set to `2016`. It therefore cannot drift from the real dispatch,
and it gets the Defcon 1 answer right for free: upstream's `get` forces `0` regardless of the fixture.

A mock that returned `2016` unconditionally would store `activation_height = reveal + 2016` for an
action that applies immediately — precisely the value Constraint 1 exists to prevent, in the
environment Phases 3–6 are demoed in.

## 6. Migration (expand → migrate → contract, one commit)

1. Add `lock_period_for_action` and `depth_for_action`.
2. Point `compute_and_store_activation_height`
   (`orchestrator-be/src/application/proposals.rs:613`) at it. The function already holds the whole
   `Proposal`, so it passes `&proposal.action_hex` where it passed `proposal.authority`; the
   `activation_height = reveal_confirm_block + lock_period` arithmetic is untouched.
3. Delete `lock_period_for_authority`. `authority_to_update_tx_type` becomes unreachable and goes
   with it — its only caller was the deleted function.

All three steps land in **one atomic commit**: step 3 alone leaves the tree broken and step 1 alone
leaves dead code, so no intermediate commit is a valid stopping point.

## 7. Tests

Three inline unit tests in `asm_role_membership.rs`, all over `depth_for_action` with a literal
`ConfirmationDepths` fixture. **Every field in the fixture is non-zero and mutually distinct** — with
`defcon3 = 0`, `ConfirmationDepths::get` returns `None` for it too (`confirmation_depth.rs:59`:
`(depth != 0).then_some(depth)`), and the Defcon 1 / Defcon 3 test would pass while proving the
opposite of its name. AC 12 states this precondition itself.

| # | Claim | Assertion |
|---|---|---|
| 1 | AC 12 — resolved per action | On one authority, `Defcon1` resolves to `0` and `Defcon3` to the fixture's `defcon3`. The same test asserts `depths.get(UpdateTxType::Defcon1).is_none()` against the all-distinct fixture: the tripwire from §4, which fires if upstream ever gives Defcon 1 a knob. |
| 2 | §3 — resolution follows the action, not the authority | A Strata-admin signer update resolves to `strata_admin_multisig_update`; a Strata-admin VK update resolves to `ol_stf_vk_update`. The two fields differ in the fixture, so the old per-authority mapping cannot pass this. |
| 3 | Cancel is total and safe | A `Cancel` wrapping a non-zero-depth update resolves to `0`. |

### AC 12a is structural evidence, not a unit test

AC 12a asks that the depth be read live rather than captured at startup. The seam above deliberately
bypasses the RPC, which is the only place a cache could live, so a test over it would assert *"a pure
function of two arguments returns different results for different arguments"* — it cannot fail and it
says nothing.

The evidence is the shape of the code, and it is what review must check: `lock_period_for_action`
issues a fresh `strata_asm_getStatus` on every call, memoizes nothing, and captures nothing at
startup. No `static`, no `OnceCell`, no field on `AppState`.

**A wording mismatch worth recording:** the contract says the depth is resolved "during enactment
detection" / at "enactment-detection time". In the code there is no repeated resolution cycle —
`compute_and_store_activation_height` is reached from exactly one site (`proposals.rs:700-702`),
guarded by `broadcast_status == RevealConfirmed && !is_cancel()`, so it runs **once per proposal, at
reveal confirmation**, and `is_proposal_enacted_on_asm` never reads `activation_height`. The live-read
property holds; the "cycle" the contract describes does not exist yet. Phase 4 is where enactment
detection gets its Defcon arm, and if a repeated cycle is wanted, that is where it belongs.

## 8. Blast radius

`activation_height` is currently **write-only**: persisted (`postgres_repo.rs:401-407`,
`memory_repo.rs:155-165`), carried through the Tauri DTO
(`desktop-app/src-tauri/src/commands/proposals.rs:220`), and read by nothing — no consumer in
`desktop-app/src`, and no existing test asserts a value for it.

That cuts both ways, and both belong on the record: it is why this phase is safe to land, and it is
why the correction in §3 could otherwise have shipped unnoticed.

## 9. Out of scope

Everything the resolver does not need: `Authority::SecurityCouncil` → `Role::StrataSecurityCouncil`
mapping, the Defcon codec and action builder, proposal creation, enactment detection, and every
frontend change. Phases 3–6 own those. The cancel gate is Phase 2 and depends on this function.

No Defcon proposal can be created when this phase merges — the resolver knowing an action exists is
not the same as the product being able to create it.

## 10. Verification

`cargo test -p orchestrator-be`, then the full [`AGENTS.md`](../../AGENTS.md) pre-commit checklist.

Review must additionally confirm the structural claim in §7: no caching of the depth anywhere on the
path. End-to-end regtest verification belongs to the close-out of all six phases
([build plan §5](./security-council-defcon-implementation.md#5-verification)).
