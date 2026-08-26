# Security Council — Defcon 1 (V1), Phase 3: Role, builder, creation

**Functional contract:** [`security-council-defcon.md`](./security-council-defcon.md) — SSOT for
*what* V1 must do. This document never overrides it.

**Build plan:** [`security-council-defcon-implementation.md`](./security-council-defcon-implementation.md)
§4 Phase 3. This document is that phase at implementation detail.

**Closes:** AC 2 (backend half), AC 3, AC 17.

## 1. The change in one sentence

The Security Council becomes a first-class authority on the backend — mapped to its ASM role, allowed
to create proposals, and refused when the action it posts belongs to somebody else — and the desktop
learns to *build* a Defcon 1 action hex.

## 2. What this phase is not

It is not the Defcon 1 product flow. When it merges, a council signer still has no screen, no form and
no entry point: `ACTION_TYPES_BY_AUTHORITY` is untouched, `decode_action_hex` still answers
`Unknown`, and nothing in the UI can reach the new builder command. What exists afterwards is an
action a client *can* construct and a backend that *will* accept it from the right session and refuse
it from every other one. Phases 5 and 6 turn that into a flow.

Enactment detection stays in Phase 4: a Defcon 1 proposal created after this phase reaches
`Approved` and stops there.

## 3. The four changes

| # | Change | Where | Closes |
|---|---|---|---|
| A | `Authority::SecurityCouncil` → `Role::StrataSecurityCouncil` | `orchestrator-be/src/infrastructure/asm_role_membership.rs` | prerequisite for all of V1 |
| B | The action's authorizing role must match the session's authority | `orchestrator-be` handler + `asm_role_membership.rs` | AC 17 |
| C | A duplicate `(action, seq_no)` is refused *by name* | `orchestrator-be/src/application/proposals.rs` | AC 3 |
| D | Defcon 1 action builder | `desktop-app/src-tauri` | AC 2 (backend half) |

A, B and C are backend and land together (§8, cycle 1). D is the desktop side and lands separately
(§8, cycle 2). Nothing in D compiles against A–C, and nothing in A–C needs D: the split is by blast
radius, not by dependency.

## 4. A — the role mapping

`authority_to_role_impl` (`asm_role_membership.rs:190-199`) maps three of the five authorities and
falls through to an error for the rest. The council gains its arm:

```rust
Authority::SecurityCouncil => Ok(Role::StrataSecurityCouncil),
```

**This is not the whole mapping.** `fetch_role_membership` (`:201-220`) builds a map of role → keys
by hand, with one `insert` per role, and `is_signer_member_for_authority` then looks its role up in
that map (`:47-53`) and errors when it is absent. Mapping the authority without adding the fourth
insert would move the failure from "not mapped to ASM role authorization yet" to "admin state missing
authority for role `StrataSecurityCouncil`" — a worse error for the same broken outcome. Both change,
or neither does.

The fourth insert cannot break the other three. `authority_keys_hex` fails the whole map when its
role is missing from the state, but the council is never missing: `AdministrationSubprotoState`
indexes its authorities by `Role` discriminant and genesis fills every one of the four from
`AdminConfig` (`asm/crates/params/src/subprotocols/admin/config.rs:68-78`). A state old enough not to
carry the council does not decode against this workspace's pin at all, and already reports that.

`Authority::PayoutAdmin` stays unmapped: it has no ASM role upstream, and the fall-through arm stays
for it.

### The dev mock learns the council too

`mock_membership`, `mock_last_seqno` and `mock_threshold`
(`asm_role_membership.rs:355-411`) each answer for three authorities and return `None` for the rest.
`None` means "not a mock answer", so after A the council falls through to the real RPC path and a
`mock://asm-membership` URL fails there. The three mocks gain a council arm, with the same shape and
the same key material the Strata admin mock already uses (`0279be66…`, plus signer B), a threshold of
`2` and a last-seqno of `0`.

Without this, no local-stack or e2e session can authenticate as the council, and Phases 5–6 would
have to add it as a prerequisite to their own work. It is three lines and it belongs with the mapping
it completes.

## 5. B — the action must match the session (AC 17)

### Why nothing refuses this today

`create_proposal` (`handlers/proposals.rs:74-104`) decodes `action_hex` only to discard the result —
the call is a validity check for SSZ hygiene, nothing more. The `Proposal` it then builds takes its
authority from the *session*, never from the action. So a Strata admin session that posts an
Alpen-admin action hex is persisted today, under the wrong authority, and collects signatures against
the wrong threshold.

The hole predates Defcon 1 and A does not widen it: a Strata admin session can post a Defcon 1 hex on
today's `develop` and have it persisted, because `threshold_for_authority` reads the *session's*
authority and never looks at the action at all. A adds the legitimate path; B is the only thing in
this phase that closes the illegitimate one, and it would be worth doing even if the council did not
exist.

AC 17 asks for the gate the contract calls the Authorization Gate: PRD 06 §3.1.4's "usable
exclusively by" has to hold against a caller that never touches the UI.

### The gate

Upstream already owns the answer. `UpdateTxType::authorized_role()`
(`asm/crates/params/src/subprotocols/admin/updates.rs:52-75`) maps every update variant to the role
that may sign it, and `UpdateAction::update_tx_type()` gets there from the decoded action — the same
composition Phase 1 used for the depth, for the same reason: the table is upstream's and we do not
own a second copy.

```rust
/// Refuse an action the session's authority is not allowed to sign.
///
/// The mapping is upstream's (`UpdateTxType::authorized_role`), so a new update variant is
/// authorized correctly here the moment it exists, without a table of ours to update.
pub(crate) fn require_authorized_for_action(
    authority: Authority,
    action: &MultisigAction,
) -> Result<(), AppError>
```

It lives in `asm_role_membership.rs`, the module that owns the authority↔role translation, beside
`authority_to_role_impl`. It is **not** `async` and takes no RPC URL: unlike its neighbours it reads
nothing from the chain, only upstream's static table. The handler calls it with the action it already
decoded, before `threshold_for_authority` and before any `Proposal` exists.

**A cancel passes unconditionally.** `MultisigAction::Cancel` has no `update_tx_type` and so no
authorized role, and cancels do not reach this endpoint anyway — they are created through
`POST /proposals/:action_id/cancel`. Refusing them here would be a new rejection on a path this phase
has no business changing, in a slice whose Phase 2 already settled how cancels are gated. The arm is
explicit, with that reason, rather than a `_ =>` catch-all.

**The error is a `BadRequest` naming both roles**, e.g.

> action `Defcon 1` must be authorized by `Strata Security Council`, but the session is
> `Strata Administrator`

Both sides are named in upstream's vocabulary — `UpdateTxType::name()` and `Role::name()`, the same
strings the hardware signer renders — so the message reads in the terms the signer just saw, not in
ours.

Not `Unauthorized`: that variant renders as the bare string `unauthorized`
(`error.rs:44`), which would tell a signer nothing about which of the two halves is wrong. There is
no `Forbidden` variant today and this phase does not add one — Phase 2's depth gate set the precedent
that a governance-rule refusal on this path is a `BadRequest` whose *message* is the signal.

### This tightens every authority, not just the council

Deliberate. It is the same class of bug fix Phase 1 made: a rule that was expressed per-authority and
answered wrongly for actions. Any existing test that logs in as one authority and posts another's
action hex is pinning that bug and must move to a matching fixture — never be excused by weakening the
gate. The handler suite's `create_body` uses `test_fixture_action_hex()`, a Strata admin update, and
`login` authenticates as `strata_admin` (`handlers/mod.rs:141-148`, `:160-166`), so it is already
consistent; the expectation is that nothing needs to move, and any surprise found while implementing
is a real finding to report, not to paper over.

## 6. C — a duplicate creation is refused by name (AC 3)

`create_update_action` (`application/proposals.rs:42-84`) computes a stable `ActionId` from
`(seq_no, action_hex)` and hands the proposal to `repo.save_proposal`, which rejects a duplicate id
with `AppError::Conflict` in both repositories (`memory_repo.rs:33-35`,
`postgres_repo.rs:160-165`). The refusal is correct; what it cannot do is tell the second creator
*which* proposal already holds that id.

### The contract said the opposite, and the PRD wins

AC 3 read "the second signer's POST returns the existing proposal (idempotent)" until this phase.
That contradicts three things at once:

| Source | Says |
|---|---|
| [PRD 02](../0-prd/02-multisig-backend.md) §3.4.1 | "The backend MUST **reject** duplicate creation" |
| AC 3's own title | "ActionId is stable and **duplicate rejection** works" |
| [`story-map.md`](../3-stories/story-map.md), quoted by this contract's Requirements Alignment | "duplicate rejection" |

The PRD is the client's SSOT and is not ours to modify; the contract is ours, so the contract is
what moves. AC 3 is corrected in the same commit as this spec, with the citation inline. Both
readings already agreed on PRD 02 §3.4.2 — the existing proposal must not be mutated.

### The change

```rust
if repo.find_by_action_id(&action_id).await?.is_some() {
    return Err(AppError::Conflict(/* a message naming `action_id` */));
}
```

The lookup is what lets the message name the id; `save_proposal`'s own `Conflict` stays as the
backstop for the race where two creators both see `None`, and reports the same outcome in weaker
words. Mapping that error instead of pre-checking would have been one code path fewer and wrong:
`save_proposal` raises `Conflict` for a second reason — a duplicate signer inside the same insert
(`postgres_repo.rs:187`) — and relabelling that as a duplicate proposal would misreport it.

### The second signer's signature is dropped, deliberately

They signed on their device and the backend keeps nothing. That is the point of PRD 02 §3.4.2, and
it is the safe half of the trade: a creation call is not an approval, and quietly turning one into
the other is how a signature ends up on a proposal nobody deliberately approved. Naming the
`ActionId` in the refusal is what keeps this from being a dead end — the signer approves that
proposal instead, through the path that exists for approving. Phase 6 owns making the UI follow the
name instead of showing a raw conflict.

### What this does not check

That the existing proposal belongs to the caller's authority. It cannot differ in a way that
matters: the `ActionId` is a hash of `(seq_no, action_hex)`, and after B the same action implies the
same authority — with one exception, `MultisigAction::Cancel`, which B passes unconditionally
(§5) and which therefore *can* collide across authorities. The refusal leaks only that an id
exists, never the proposal, so the exception costs an id and no state. Closing it belongs to
whoever gives cancels their own gate on this endpoint; this phase records it rather than widening.

## 7. D — the Defcon 1 action builder

The desktop keeps its own `Action` domain and its own codec, deliberately independent of the
orchestrator's (`desktop-app/src-tauri/src/domain/action.rs`,
`src/infrastructure/action_codec.rs:1-6`). Defcon 1 is a payload-less unit struct upstream, so it is
the smallest possible addition to all three layers:

1. **Domain** — `Action::Defcon1`, a unit variant on the enum at `domain/action.rs:144-149`. No
   fields, no constructor, no validation: there is nothing to get wrong.
2. **Codec** — `to_strata_action` gains
   `Action::Defcon1 => MultisigAction::Update(UpdateAction::Defcon1(Defcon1Update))`, and
   `from_strata_action`'s existing `Defcon1` arm (`action_codec.rs:245-247`) stops returning
   `UnsupportedVariant` and returns `Ok(Action::Defcon1)`. That arm exists precisely so this slice
   would find it.
3. **Command** — `build_defcon_1_action_hex()` in `commands/action_builder.rs`, following its
   neighbours, registered in both invoke lists (`commands/invoke.rs:22`, `:92`). It takes **no input
   struct**: the action has no payload, and the sequence number is not part of the action — it is a
   separate field of the creation request everywhere else in this codebase
   (`CreateProposalRequest.seq_no`), and folding it into the builder here would be a shape no other
   builder has.

### `decode_action_hex` stays untouched — and the build plan is wrong about why

The build plan assigns `decode_action_hex` to Phase 5 ([§4 Phase 5](./security-council-defcon-implementation.md#phase-5--frontend-create-and-sign)),
and it is right, but not for the reason it gives. Moving that arm earlier is not merely premature: it
would be a **regression**. `decodedActionSchema` (`desktop-app/src/api/ipc-schemas.ts:128-142`) is a
zod *discriminated union*, so a `kind` it does not list is a parse failure, not an `unknown`
fallback. Emitting `kind: "defcon_1"` from the command before Phase 5 registers it in that union
turns today's graceful "unknown action" rendering into a thrown parse error on any screen that
touches a Defcon 1 proposal — and after §5 and §6 the backend does accept one.

So the IPC boundary keeps answering `Unknown` for Defcon 1 until the TypeScript side is ready, and it
moves in one step with the union that reads it. The round trip is still proven in this phase, one
layer below, through `action_codec::encode_hex`/`decode_hex`.

## 8. Migration

Two commits, each atomic, each leaving the tree green.

**Cycle 1 — backend (A + B + C).** A alone opens council sessions against a gate (B) that does not
exist yet, and B alone is a tightening whose motivating action nothing can build. C is the third
because B is what makes it safe (§6). One commit, `orchestrator-be` only.

**Cycle 2 — desktop (D).** Independent of the backend commit and of everything before it; separate
because it is a different crate, a different blast radius, and a different review.

## 9. Tests

Minimal and behavioural. Every one of these asserts a rule that can actually break; none of them
restate a language guarantee or a fixture.

| # | Claim | Where | Assertion |
|---|---|---|---|
| 1 | AC 17 — a non-council session cannot create a Defcon 1 proposal | `handlers/mod.rs` | A `strata_admin` session POSTs `test_fixture_defcon_1_action_hex()`; the response is a refusal, and the subsequent `GET /proposals` holds no proposal for that `(action, seq_no)`. The second half is the half AC 17 actually asks for. |
| 2 | B — the gate reads the action, not the authority | `asm_role_membership.rs` | `require_authorized_for_action` accepts the Defcon 1 fixture for `SecurityCouncil` and rejects it for `StrataAdmin`, with a message naming the required role. One test, both directions: split in two they would be halves of one claim. |
| 3 | AC 3 — a duplicate creation is refused and names the existing proposal | `application/proposals.rs` | Two `create_update_action` calls with the same `(action_hex, seq_no)` and *different* signers: the second is a `Conflict` whose message contains the first's `action_id`, and the stored proposal still holds exactly the first signature. The differing signer is what makes it a test of the rule rather than of equality. |
| 4 | D — the Defcon 1 action round-trips | `src-tauri/src/infrastructure/action_codec.rs` | `Action::Defcon1` → hex → `Action::Defcon1`, and the hex equals the orchestrator's fixture bytes. Both halves matter: the round trip alone would pass on a codec that agreed with itself and with nobody else. |

Test 3 **rewrites** `test_create_duplicate_action_rejected` (`application/proposals.rs:933-948`),
which asserted only that *a* `Conflict` came back. It keeps that assertion, gains the `ActionId` in
the message, gains `sig_b()` — already in the fixtures at `:785-791` — as a second creator, and
gains the read-back that proves the stored signatures did not change.

Test 1 needs a council **session** only if it tests the positive path; it does not — AC 17 is the
refusal, and the positive path is already covered by test 2 at the level where the decision is made.
The mock's council arms (§4) exist for Phases 5–6 and for the local stack, not for these tests.

`test_fixture_defcon_1_action_hex()` already exists under `#[cfg(test)]`
(`action_codec.rs:29-41`), added by Phase 2.

**No test asserts the role mapping itself.** `authority_to_role_impl` returning
`Role::StrataSecurityCouncil` for `Authority::SecurityCouncil` is a one-line table; tests 1 and 2 both
fail if it is wrong, and a test that restated it would only pin the line to itself.

## 10. Blast radius

- **Every authority's proposal creation** now refuses a mismatched action (§5). Intended, and the
  reason this is one of the two phases that touch a shared path.
- **Duplicate creation stays a `409`** and only its message changes (§6), so no client changes and
  no behaviour change for any existing proposal type. The correction landed in the contract, not in
  the code.
- **The council becomes an authenticatable authority.** `POST /auth/challenge` with
  `authority: "security_council"` now succeeds for a member. There is still nothing a council session
  can reach in the UI.
- **`Authority::PayoutAdmin` is unchanged** and still unmapped.

## 11. Out of scope

Enactment detection (Phase 4), every frontend change including `ACTION_TYPES_BY_AUTHORITY`, the
create form, the type-to-confirm gate and the `decodedActionSchema` union (Phases 5–6). The
`create_defcon_proposal` function the functional contract sketches under Backend Contract → Proposal
Creation is **not** written: creation on this backend is generic over `action_hex`
(`create_update_action`), a Defcon-specific constructor would fork a shared path to add nothing, and
the contract's own logic for it — a stable `ActionId` and a duplicate that changes nothing — is
what the generic path already does, plus C's naming of the id.

## 12. Verification

`cargo test -p orchestrator-be` for cycle 1, `cargo test -p desktop-app` for cycle 2, then the full
[`AGENTS.md`](../../AGENTS.md) pre-commit checklist on each.

Review must additionally confirm:

- No proposal-creation path reads the session's authority to decide *what* it may sign other than
  through `require_authorized_for_action`.
- The desktop can build a Defcon 1 hex and still cannot render one: `grep -rn "defcon" desktop-app/src`
  returns nothing.

End-to-end regtest verification belongs to the close-out of all six phases
([build plan §5](./security-council-defcon-implementation.md#5-verification)).
