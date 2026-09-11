# Security Council — Signer Update (V3), Phase 2: Enactment reads two roles

**Functional contract:** [`security-council-signer-update.md`](./security-council-signer-update.md) —
SSOT for *what* V3 must do. This document never overrides it.

**Build plan:** [`security-council-signer-update-implementation.md`](./security-council-signer-update-implementation.md)
§4 Phase 2. This document is that phase at implementation detail, and §7 records where it supersedes
it.

**Closes:** [AC 7](./security-council-signer-update.md#7-enactment-compares-the-councils-config-against-the-administrators-sequence-number)
and [AC 7a](./security-council-signer-update.md#7a-neither-role-is-substituted-for-the-other);
[Constraint 1](./security-council-signer-update.md#1-enactment-reads-two-roles-not-one), including
the `authority_to_role` divergence it names as *"known divergence to fix while here"*.

## 1. The change in one sentence

`keys` and `threshold` stop being read from the proposal's authority and are read from the role the
**action variant** names, while `last_seqno` keeps being read from the role the proposal's authority
names — and the enactment module stops owning a fourth answer to "which role is this authority".

## 2. What this phase is not

It is not the create flow: no menu entry, no widened builder union, no validator, no form retarget,
no no-op rule. Those are Phase 3, and `desktop-app/src/` has **zero diff** in this phase. It is not
the cancel or the e2e — Phase 4, and `e2e-tests/` has zero diff. It adds no protocol validity rule:
whether a rotation is legal is the ASM's answer, not ours.

It does not repair [Constraint 4](./security-council-signer-update.md#4-acceptance-is-not-application-and-upstream-does-not-say-so).
It does not repair `AsmStfVk`, which rides in the multisig arm by inheritance and answers `Ok(false)`
forever (§10.2).

**Nothing product-visible changes.** No path in the application can author a council rotation until
Phase 3, so this phase's whole effect is on a proposal that arrived from outside — and on the three
authorities already shipped, for which it is behaviour-preserving by construction (§4.3).

## 3. Spec traceability audit

| Document | What Phase 2 takes from it |
|---|---|
| [`security-council-signer-update.md`](./security-council-signer-update.md) § Enactment detection | The five bullets: target from the variant, authorizing from the authority, keys/threshold from the target, `last_seqno` from the authorizing, `multisig_update_post_conditions_met` unchanged |
| Same, [Constraint 1](./security-council-signer-update.md#1-enactment-reads-two-roles-not-one) | Both terms wrong in opposite directions; both failures silent; the `authority_to_role` divergence closed here |
| Same, § Test Plan | "a truth table with AC 7a as two named tests"; no ASM-backed integration test inside `orchestrator-be` |
| Same, § Verification | *"Only one `authority_to_role` answers for the backend, and it maps four authorities"* — §4.1 closes it by deletion, which is stronger (§7.1) |
| [`security-council-defcon-3-phase-4.md`](./security-council-defcon-3-phase-4.md) §4.4 | `Ok(false)` falls through to supersession, so inconclusive must be `Err`. Reused verbatim as §6 |
| [`security-council-defcon-phase-1.md`](./security-council-defcon-phase-1.md) | The closure seam pattern: `depth_for_action(action, depth_of)` |
| [`security-council-signer-update-phase-1.md`](./security-council-signer-update-phase-1.md) §11.1 | *"The honest fix derives the authorizing role from the decoded action … that derivation is exactly what Phase 2 builds"* — §4.2 is it |
| [`proposal-lifecycle-seqno-truth.md`](./proposal-lifecycle-seqno-truth.md) §4 | Enactment is decided before supersession; the seqno term stays `>=` for config-carrying arms |

## 4. Design

### 4.1 The enactment module stops answering "which role is this authority"

`asm_enactment.rs:424-433` holds a private `authority_to_role` that maps **three** authorities, while
`asm_role_membership.rs:270-280` maps four. Two functions with the same name and different answers is
how the council reaches a wrong arm silently — which is why Constraint 1 names it.

**It is deleted, and nothing replaces it.** Upstream already publishes the answer this call site
needs:

```rust
// asm/crates/subprotocols/admin/txs/src/actions/updates/mod.rs:79-82
/// The role authorized to enact this update.
pub fn required_role(&self) -> Role { self.update_tx_type().authorized_role() }
```

So the authorizing role is `update.required_role()`, guarded by `require_authorized_for_action`
(§4.2). The Verification checkbox is satisfied by **elimination** rather than by sharing: after this
phase `orchestrator-be` has exactly one `authority_to_role`, in `asm_role_membership.rs`, and the
enactment module has none. Deleting the class of bug beats deduplicating one instance of it.

**The Defcon arms stay literal.** `Role::StrataSecurityCouncil` at `:128` and `:154` is not an
indirection worth adding: an arm that has already matched `UpdateAction::Defcon1(_)` knows its role
with type-level certainty, and all three of a Defcon's terms belong to the council, which is there
both target and authorizer. What must change is the **comment** at `:125-126`, whose first half —
*"resolved through `authority_to_role`, which does not map the council"* — becomes false the moment
the function is gone. Its second half is the real and sufficient reason and is what survives.

### 4.2 The authority↔variant rule is delegated, not rewritten

`extract_multisig_config_update` (`:303-356`) matches on the pair `(authority, action)` and encodes
"authority == variant" as a data-integrity error at `:327-337`. That rule stays true for the three
self-rotating updates and stops being universal.

It is **not rewritten with a fourth pair**. `require_authorized_for_action`
(`asm_role_membership.rs:243-264`) already answers the general form of the question from upstream's
own table, and it is `pub(crate)`, synchronous, reads no chain state and returns the same `AppError`.
Reusing it deletes ~40 lines of a table that could drift from the protocol, and upgrades the message
from *"action variant does not match proposal authority for enactment check"* to one naming all three
terms: the action, the role required, and the session's.

Once that guard passes, `authority_to_role(authority)` and `update.required_role()` are **equal by
construction** — the guard is exactly the proof of that identity. So Constraint 1's *"the authorizing
role comes from the proposal's authority"* is honoured: the authority is what is checked, and it
fails loudly when a stored row disagrees with its own hex.

What remains is a match on the variant alone:

```rust
/// The role a multisig-config update *modifies*, and the config it installs.
///
/// The target belongs to the action variant and to nothing else — see Constraint 2. Upstream applies
/// tx type 15 to `Role::StrataSecurityCouncil` (`handler.rs:145-147`) while authorizing it with
/// `Role::StrataAdministrator` (`updates.rs:64`); for the three self-rotating updates the two
/// coincide, which is why nothing needed this distinction before V3.
///
/// `None` for every action that is not a multisig config update — the caller answers `Ok(false)`,
/// which is what `AsmStfVk` has always relied on.
fn multisig_config_update_target(action: &MultisigAction) -> Option<(Role, &ThresholdConfigUpdate)>
```

**The order of the two is load-bearing**, and §10.3 says why: the target lookup runs first, so an
`AsmStfVk` still answers `Ok(false)` instead of becoming an `Err`.

### 4.3 The seam that makes AC 7a testable without an ASM

AC 7a asks for two named tests: one where the administrator's signer set changes and the answer must
not, one where the council's `last_seqno` advances and the answer must not. Neither is expressible
against `multisig_update_post_conditions_met`, because the risk AC 7a guards is **not the
computation — it is the wiring**: which role each of the three terms was read from. A test that
cannot see the wiring cannot fail when the wiring breaks.

Nor can it be expressed against the real state. `MultisigAuthority::update_last_seqno` is
`pub(crate)` upstream and demands a `SeqNoToken` that cannot be constructed outside that crate
(`asm/crates/subprotocols/admin/subprotocol/src/authority.rs:19-30,100`), so **no `MultisigAuthority`
with a non-zero `last_seqno` can be built from this repository**, and no `AnchorState` fixture exists
here at all. The choice is a closure seam or no unit test.

The repository already made that choice once, for the same reason, in `depth_for_action(action,
depth_of)` (`asm_role_membership.rs:134-149`):

```rust
/// The three terms an enactment check reads off one role, at one instant.
///
/// `keys` is hex of `CompressedPublicKey::serialize()` — 33 bytes, compressed. Not x-only: the
/// `OperatorSet` arm next door uses 32-byte x-only hex, and the two are not interchangeable.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AuthoritySnapshot {
    keys: Vec<String>,
    threshold: u8,
    last_seqno: u64,
}

/// `keys` and `threshold` come from the **target** role; `last_seqno` from the **authorizing** role.
/// Neither term is derived from the other, and collapsing the two roles into one is the regression
/// AC 7a exists to catch.
///
/// `snapshot_of` is a parameter for the same reason `depth_for_action` takes its lookup: it is the
/// only seam at which the two-role wiring is observable without a chain.
///
/// A role the state does not carry is `Err`, never `Ok(false)` — see §6.
fn multisig_update_enacted(
    target_role: Role,
    authorizing_role: Role,
    seq_no: u64,
    config: &ThresholdConfigUpdate,
    snapshot_of: impl Fn(Role) -> Option<AuthoritySnapshot>,
) -> Result<bool, String>
```

It delegates to `multisig_update_post_conditions_met`, **which does not change** — it already takes
the four terms as scalars and has no opinion about where they came from, exactly as the contract
says.

`Result<bool, String>` rather than `AppError` on purpose: it is what lets the desktop copy (§4.4) be
textually identical, so "both copies moved together" is checkable in review rather than asserted.

### 4.4 The desktop copy moves with it

`desktop-app/src-tauri/src/infrastructure/asm_enactment.rs` carries the same shape and is behind:
`extract_multisig_config_update:79-103` maps only `StrataAdmin` and `SequencerManager`. An
`AlpenAdminMultisig` update under `Authority::AlpenAdmin` matches neither of the two real arms nor
the mismatch arm beside them — it falls through to `(_, MultisigAction::Update(_)) => Ok(None)`,
and **the predicate answers `Ok(false)` for it, permanently and without complaint**.

`Ok(false)` is the worse of the two failures available, for §6's reason: an `Err` is logged and
retried, while `Ok(false)` is an answer a caller acts on. It is the same silent failure mode
Constraint 1 describes for the council, reached by a different route — which is a live bug for an
authority shipped long ago, not merely a missing council arm, and it is why that commit reads `fix`.

> Corrected after implementation. This section first said the update "falls into the mismatch arm
> and errors". It does not: the mismatch arm only ever listed `StrataAdminMultisig` and
> `StrataSeqManagerMultisig`. The distinction matters because it is the one §6 is about.

`authority_to_role` there (`:139-153`) is already exhaustive and already maps the council. Nothing to
unify on this side; the guard is written inline against it, since this crate has no
`require_authorized_for_action`.

`is_multisig_update_enacted_in_admin_state`'s signature does **not** change, so
`e2e-tests/tests/e2e_enactment_predicate.rs:167,190` needs no edit.

**On the duplication.** `orchestrator-be` and `desktop_app` are separate crates with no dependency
between them; extracting a shared crate for ~60 lines would cost more than the copy. The copies are
kept textually identical instead, which is what makes a review able to see them diverge.

## 5. Where the compiler insists, and where it does not

| Site | Net | Owner |
|---|---|---|
| `multisig_config_update_target` — variants | **`E0004`.** `UpdateAction` has no wildcard here if written exhaustively; a new upstream variant stops the build | T7 |
| The outer `match action` arm at `:173-179` | **`E0004`**, and it must stay an explicit variant list. Collapsing it to `MultisigAction::Update(update)` to pass a `&UpdateAction` would trade that net for nothing | — |
| Deleting `authority_to_role` | **`E0425`** at its one call site | — |
| The order of the guard vs the target lookup | **none** — both compile either way, and the wrong order silently changes `AsmStfVk`'s answer | T10 |
| Which role feeds which term inside `multisig_update_enacted` | **none** — both roles are `Role`, so a swap type-checks | T1, T2, T3 |

The last row is the whole phase. Two arguments of the same type, and the compiler is indifferent to
which one reaches which term.

## 6. Degradation: `Ok(false)` is an answer, `Err` is an absence

`reconcile_one` (`application/proposals.rs:589-591`) sends every `Ok(false)` to
`supersede_if_seq_no_consumed`. So a missing role in the decoded state must be `Err` — the cycle then
`warn!`s and retries, per proposal, and nothing else in the sweep is poisoned. `Ok(false)` there
would mark a live rotation `Superseded`.

| Observation | Result |
|---|---|
| The variant is not a multisig config update (`AsmStfVk`) | `Ok(false)` — preserved exactly |
| The session's authority may not authorize this variant | `Err(BadRequest)`, naming the required role |
| The state carries no authority for the **target** role | `Err(BadRequest)` |
| The state carries no authority for the **authorizing** role | `Err(BadRequest)` |
| Post-conditions not met | `Ok(false)` |

## 7. Where this phase departs from the build plan, and why

### 7.1 The divergence is closed by deletion, not by sharing

The build plan (§4 Phase 2) says the private `authority_to_role` *"maps three authorities while
`asm_role_membership.rs:266-281` maps four"* and calls it *"the phase that would otherwise depend on
the wrong one"*. The obvious reading is to share the four-mapping one.

§4.1 does not. It removes the need for either, because upstream's `required_role()` answers the
question the call site actually has, and `require_authorized_for_action` supplies the proof that the
answer matches the proposal's authority. Deduplicating leaves two call sites that could drift again;
deleting leaves one function in the crate that can be wrong at all.

Recorded here rather than left as drift: V1 and V2 each needed a close-out PR for exactly this kind
of silent divergence. [`docs/specs/README.md`](./README.md) sets the order — the functional contract
outranks the build plan — and the contract asks only that *"only one `authority_to_role` answers for
the backend, and it maps four authorities"*, which this satisfies.

### 7.2 One claim in the build plan is false, and it is worth correcting

The plan says the two copies *"move together or the desktop and the backend disagree about whether a
rotation enacted."* They cannot disagree today: `grep` for `asm_enactment` across
`desktop-app/src-tauri/src` returns exactly one hit, `infrastructure/mod.rs:42 pub mod
asm_enactment;`. **No Tauri command invokes it.** Its only consumer is
`e2e-tests/tests/e2e_enactment_predicate.rs`.

The copy is still worth fixing in this phase, for two reasons the plan does not give: the live
`AlpenAdmin` bug (§4.4), and that it is the vehicle Phase 4's `e2e_council_rotation.rs` runs against
a real chain. Fixing it there would mean Phase 4 discovering it while writing a test.

## 8. Tests

Eleven claims. **No mocks, no I/O, no clock** — each is a pure function, a two-entry lookup, or an
`AdministrationSubprotoState` built through upstream's own constructors.

| # | Claim | Assertion | Where |
|---|---|---|---|
| T1 | AC 7 — the happy path, and only that | **both** snapshots satisfy every term, so this answers `true` under the real wiring and under either substitution. A control, not a third detector | `orchestrator-be/.../asm_enactment.rs` |
| T2 | **AC 7a, half one** — the administrator's signer set is not the target's | the administrator's keys and threshold disagree with `config`; both roles share a `last_seqno` → the **only** test red when keys/threshold are read off the authorizing role | same |
| T3 | **AC 7a, half two** — the council's seqno is not the authorizing one | council `last_seqno` far ahead, administrator's below `seq_no` → the **only** test red when the seqno is read off the target | same |
| T4 | The three shipped authorities read one role for all three terms | an administrator rotation where target == authorizing still answers as before | same |
| T5 | A missing role is an absence, not a negative | `snapshot_of` answers `None` for the council → `Err`, never `Ok(false)` (§6) | same |
| T6 | `AsmStfVk` is not a multisig config update | `multisig_config_update_target` answers `None`, so the arm still answers `Ok(false)` | same |
| T7 | Every multisig variant names its own target role | the four-row truth table of `multisig_config_update_target`, council included | same |
| T8 | The desktop predicate reads the same two roles | T1's and T3's claims against the shared `multisig_update_enacted` seam | `src-tauri/.../asm_enactment.rs` |
| T9 | An Alpen Administrator rotation stops answering `Ok(false)` for ever | end to end through `is_multisig_update_enacted_in_admin_state` against a real `AdministrationSubprotoState` (§4.4) | same |
| T10 | The target lookup answers before the guard | an `AsmStfVk` under an authority that does not authorize it is `Ok(false)`, not `Err` — the only test that holds §10.3's ordering in place | same |
| T11 | A cancel is an absence, not a negative | a cancel hex through the same entry point is `Err` | same |

**T3 is the test that fails if a future refactor collapses the two roles back into one**, which is
the shape the code had before this phase. Its name says which role each term came from, and that name
is the documentation.

**T1's fixture is the load-bearing part of all three.** Give the two roles anything to disagree
about and T1 alone goes red under both substitutions — at which point T2 and T3 assert something
already proven one test above them, and their names become claims about coverage they do not own.
So T1's administrator snapshot agrees with `config` on every term and both roles stand at the same
`last_seqno`. Each of T2 and T3 then reddens for exactly one substitution and stays green for the
other, which is what lets a failure name which one happened.

> **Verified by mutation, not by reading.** Reading keys and threshold off the authorizing role
> reddens T2 and nothing else; reading the seqno off the target reddens T3 and nothing else;
> swapping the guard and the target lookup reddens T10 and nothing else. Three one-line edits, three
> single failures. The claim in this paragraph is the kind that is wrong by default, and it was
> wrong here on the first attempt — see §9's sixth commit.

**T7 is not restating the enum.** A codec with two arms crossed would round-trip happily — Phase 1's
own finding, for the same reason: a council rotation and an administrator rotation carry a
byte-identical `ThresholdConfigUpdate` and are separated only by the SSZ union selector.

**T10 and T11 run through the entry point, not the helper.** `is_multisig_update_enacted_in_admin_state`
takes an already-decoded `&AdministrationSubprotoState`, so the desktop copy can be tested end to
end without a chain — hex to action, action to target, guard, and the lookup off real state. The
backend has no such seam (§10.2), which is why its equivalent of T10 does not exist.

**Not tested, deliberately:**

- **No ASM-backed integration test inside `orchestrator-be`.** The contract's Test Plan excludes it:
  it would be the flakiest test in the repository and would re-prove what Phase 4's e2e proves.
- **No test of the `require_authorized_for_action` refusal itself.** It is already covered at
  `asm_role_membership.rs:598-625`, against its own message format, and restating it here would pin a
  delegation rather than a behaviour. AC 2 belongs to that test, not to a copy of it.
- **No chain-level test of tx type 15.** It arrives in Phase 4 and cannot arrive earlier: nothing
  produces a council rotation until Phase 3.
- **Nothing about `multisig_update_post_conditions_met`.** It does not change, and its seven existing
  tests stay green untouched — which is itself the evidence that this phase moved the wiring and not
  the arithmetic.

## 9. Migration — five commits, each atomic

| # | Commit | Why it is safe on its own |
|---|---|---|
| 0 | This spec | Docs only |
| 1 | `refactor(orchestrator-be)`: enactment stops carrying its own `authority_to_role` | Behaviour-preserving for the three shipped authorities: the guard is exactly as strict as the mismatch arm it replaces, and tx type 15 still hits the "not implemented" `Err` one line later. No test of its own, deliberately — the delegated function is already covered (§8) |
| 2 | `feat(orchestrator-be)`: enactment reads the target role's config and the authorizing role's seqno | The behaviour change, alone, with T1–T7. Arrives with no renames or table moves to dilute its review |
| 3 | `fix(desktop-app)`: the desktop predicate reads two roles, and stops silently refusing Alpen Admin | Different crate, no consumer in production (§7.2), signature unchanged so the e2e is untouched. T8, T9 |
| 4 | `refactor(orchestrator-be)`: one `authority_to_role`, and it stops at a catch-all | Found reviewing commit 1. Behaviour-constant: `PayoutAdmin` was the only authority the wildcard ever caught (§10.4) |
| 5 | `test(orchestrator-be)`: each role substitution reddens exactly one test | Found reviewing commit 2 — T1's fixture made T2 redundant (§8) |
| 6 | `fix(desktop-app)`: a cancel is an absence again, and the guard order has a test | Found reviewing commit 3: a cancel had become `Ok(false)`, and §10.3's ordering had no net (§10.5) |
| 7 | `docs`: the corrections, the debt, the phase board and the master plan | Docs only. By V2's precedent this is the commit that ships the phase |

Commit 1 precedes commit 2 as a **rule**: the deletion and the delegation are behaviour-constant, and
commit 2 is the only one that changes an answer. Bundling them would put the phase's one reviewable
decision behind a diff of moved code — the cost the split exists to buy down.

## 10. Blast radius

- **`orchestrator-be` and `src-tauri` only.** `desktop-app/src/` and `e2e-tests/` have zero diff.
- **No product-visible change.** No path in the application can author a council rotation until
  Phase 3; a broadcast one stops parking at Approved, which is the point, but nothing can broadcast
  one yet.
- **One live bug fixed on the way**: an `AlpenAdmin` multisig update errored in the desktop predicate
  (§4.4).
- **`multisig_update_post_conditions_met` is untouched** in both copies, and so are its tests.
- **`mock_is_enacted` is untouched.** Still URL-keyed and action-blind, per V1 precedent, so every
  test in `application/proposals.rs` short-circuits before any of this runs.
- **The Defcon 1 and Defcon 3 arms are untouched** but for one corrected comment (§4.1).

### 10.1 Every other role derived from `proposal.authority` — audited, all correct

The contract claims this single divergence is the whole of the slice. Checked, and it holds: every
other site wants the **authorizing** role, and for a council rotation the proposal's authority is
`strata_admin`, which is exactly that.

| Site | What it reads | Correct for tx 15? |
|---|---|---|
| `supersede_if_seq_no_consumed` (`application/proposals.rs:457`) | the authorizing role's `last_seqno`, plus queue presence by action | Yes — the seqno is consumed on the administrator (`handler.rs:114-119`) |
| `last_seqno_for_authority` (`asm_role_membership.rs:59`) | the authorizing role | Yes — every caller wants the authorizer |
| `threshold_for_authority` (`:82`) | the authorizing role's threshold | Yes, and load-bearing: `handlers/proposals.rs:107-108` means *how many administrator signatures*, never the council's threshold |
| `update_id_in_queue_for_action` (`:206`) | the action only | Yes, by construction |
| `create_cancel_proposal` (`application/proposals.rs:754`) | `target.authority`, and requires the session to match | Yes — AC 9: the council has no veto over its own rotation |
| `is_cancelable_for_hex` (`:187`) | the action's depth only | Yes |
| `list_proposals` (`handlers/proposals.rs:157`) | the session's authority | Yes — AC 10 depends on it |
| desktop `ordered_keys_for_authority(…, proposal.authority)` | the **authorizing** role's canonical key order, to map signatures to indices | Yes — and a trap for Phase 3/4: feeding it `MultisigUpdate.role` would index a broadcast against the council's signers |

### 10.2 Debt this phase records rather than fixes

- **`AsmStfVk` rides in the multisig arm and answers `Ok(false)` forever.** It has no enactment
  detection at all, so an ASM STF VK update never reaches `Enacted`. Pre-existing, unrelated to the
  council, and out of this slice — but this is the phase that made the arm's shape explicit, so it is
  recorded here rather than left in the shape of the code.
- **The desktop copy has no production consumer** (§7.2). Whether it should exist at all is a
  question for whichever slice owns the desktop's relationship with the ASM; deleting it here would
  break `e2e_enactment_predicate.rs` for no gain.
- **Tx type 15 has no chain-level coverage until Phase 4.** `e2e_enactment_predicate.rs` exercises
  `StrataAdmin` only.
- **The backend has no synchronous seam, and the desktop does.** `is_proposal_enacted_on_asm`
  (`asm_enactment.rs:41`) makes its RPC call before the `match`, so the whole multisig arm — the
  guard/target ordering, the target-role resolution, the integration with
  `require_authorized_for_action` — is untestable without a chain. The desktop copy exposes
  `is_multisig_update_enacted_in_admin_state(&admin, …)`, which is exactly why T9, T10 and T11 can
  exercise the real path there and have no backend counterpart. Extracting the equivalent would
  close that gap and give tx type 15 real coverage before Phase 4. Not done here: it is a change to
  a function every arm shares, and this phase's one reviewable decision should not arrive beside it.
- **The variant→applied-role table has no tripwire against upstream changing the mapping.**
  `multisig_config_update_target` (both copies) restates what upstream does inside `apply_multisig`
  (`handler.rs:145-147`), which publishes no function for it. The `E0004` net catches a **new**
  variant; it does not catch a **changed** mapping. If upstream ever applied tx type 15 to another
  role, both copies would compile and answer wrongly in silence. The honest tripwire is chain-level
  and belongs to Phase 4's e2e, which observes which role's config actually moved.

### 10.3 One ordering that must not be "simplified"

`multisig_config_update_target` runs **before** `require_authorized_for_action`. Reversed, an
`AsmStfVk` under a non-administrator authority goes from `Ok(false)` to `Err`, which
`reconcile_one` turns into a per-proposal warning that never resolves. The code says so in a comment
at both call sites, because otherwise it is a review comment twice.

A comment is not a net, and this one is carried by T10 in the desktop copy. The backend has no
equivalent, and cannot have one until it grows the synchronous seam §10.2 records.

### 10.4 `authority_to_role` had two more leftovers than Constraint 1 names

Found reviewing commit 1, fixed in commit 4. `asm_role_membership.rs` held `authority_to_role` as a
one-line wrapper over `authority_to_role_impl` — a split introduced by a rustfmt-and-clippy sweep
(`a707120`) rather than by a decision, and enough to make the contract's *"only one
`authority_to_role` answers for the backend"* false to `git grep`, which is the form a later reader
checks it in.

It also caught its fifth authority with `_`. Its desktop twin lists all five and says why, in a
comment written after the fact: *"a catch-all is how the council reached the error arm here long
after `orchestrator-be` had mapped it."* This phase is the second instance of that same sentence —
the enactment module's own copy mapped three authorities behind a wildcard and would have compared a
council rotation against the administrator's signer set. Both times the missing arm was invisible
because the wildcard answered for it. `PayoutAdmin` is now named, and the next authority upstream
adds stops the build.

### 10.5 Two things the desktop commit got wrong about `Ok(false)` vs `Err`

Found reviewing commit 3, fixed in commit 6. Replacing the old pair-match with a let-else turned a
cancel from an explicit `Err` into `Ok(false)` — the direction §6 exists to forbid. And §10.3's
ordering was carried by a comment alone: swapping the two lines compiled and passed every test.

Both are recorded rather than quietly fixed because they are the same mistake in two costumes, and
it is the mistake this module is most exposed to: `Ok(false)` and `Err` are both plausible-looking
answers, and only one of them is an answer at all.

## 11. Verification

The full [`AGENTS.md`](../../AGENTS.md) checklist:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd desktop-app && npm run format:check && npm run lint && npm run build && npm run test:unit
```

Structural checks that the phase stayed inside its scope:

```bash
# The blocker is gone, and not merely shadowed.
git grep -n "Security Council multisig update enactment is not implemented yet" orchestrator-be/   # nothing

# One authority_to_role answers for the backend, and it is not in the enactment module.
git grep -n "fn authority_to_role" orchestrator-be/                                    # one hit, in asm_role_membership.rs
git grep -n "authority_to_role" orchestrator-be/src/infrastructure/asm_enactment.rs    # nothing

# The comment that justified the literal Defcon roles no longer names a function that is gone.
git grep -n "which does not map the council" orchestrator-be/                          # nothing

# The create flow and the e2e are other phases.
git diff --stat develop -- desktop-app/src/ e2e-tests/ orchestrator-be/src/handlers/   # empty

# The arithmetic did not move.
git diff develop -- orchestrator-be/src/infrastructure/asm_enactment.rs | \
  grep -n "fn multisig_update_post_conditions_met" -A2                                 # unchanged
```

The predicate the desktop copy feeds still holds against a real chain:

```bash
cargo test -p alpen-multisig-e2e-tests e2e_enactment_predicate    # needs bitcoind in PATH
```

**No manual walk.** There is no UI path to a council rotation until Phase 3, and this phase adds no
surface a signer can reach. The honest substitute is Phase 4's e2e, which is the first chain-level
exercise of tx type 15 anywhere, upstream included.
