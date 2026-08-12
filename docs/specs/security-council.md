# Security Council — Master Plan

**Status:** Stage 1 — discovery complete
**PRD:** [`05-prd-payout-admin-block-payouts-update.md`](../0-prd/05-prd-payout-admin-block-payouts-update.md) (latest revision) §3.1.4, §5.1, §5.2.2, §5.5
**Stories:** [`story-map.md`](../3-stories/story-map.md) US-E5, US-E7, US-E12, US-E13
**Blocker it closes:** issue #117 — *Pending definition of actions and roles*

This document is the SSOT for the scope, staging and slice status of the Security Council feature.
Per-slice functional contracts are separate siblings (`security-council-defcon.md`,
`security-council-signer-update.md`, `security-council-safe-harbour-address.md`) and are written
only once a slice is proven implementable — see [Stage 4](#6-stage-board).

All upstream claims below were read from the `asm` submodule at tags `v0.1-alpha.11` and `v0.3.1`,
with file and line references. Where the two tags differ, [§7](#7-upstream-version-notes) says so.

---

## 1. Why this was blocked, and why it no longer is

Security Council is the last of the five PRD multisig authorities with no implementation. It was
deferred for a good reason: upstream had nothing to build against.
[`08-alpen-crate-prd-coverage.md`](../2-discovery/08-alpen-crate-prd-coverage.md) recorded
*"Security Council — 0%, role not defined anywhere"* and *"Defcon 1 — Blocked, zero references to
'defcon' in the Alpen codebase"*, and
[`19-asm-bump-impact-assessment.md`](../2-discovery/19-asm-bump-impact-assessment.md) confirmed the
same after the last ASM bump. The 2026-05-22 comprehensive audit records Defcon 1/3 as **FAIL —
not implemented**.

**Those statements are now stale.** `alpenlabs/asm` PR #81 (*feat(admin): add Security Council and
Defcon actions*, merge commit `3d45351`, merged 2026-05-30) implemented the role and all four
actions this feature needs, complete. Our workspace is pinned at `e0461f8` (2026-05-11), 19 commits
before that merge, which is why none of it is visible from here yet.

The historical documents are not edited — they were true when written. They gain a superseded-by
pointer to this document in [Stage 6](#6-stage-board).

---

## 2. What the Security Council is

The administration subprotocol defines four roles
(`crates/params/src/subprotocols/admin/roles.rs:15-39`), of which the council is the fourth:

```rust
pub enum Role {
    StrataAdministrator,      // 0
    StrataSequencerManager,   // 1
    AlpenAdministrator,       // 2
    StrataSecurityCouncil,    // 3
}
```

The council exists to act during a security incident, and its only power is to **signal the bridge
to sweep all bridge funds to the safe harbour**. It has two levers — one immediate, one timelocked
— and nothing else.

### 2.1 The segregation invariant

The design centerpiece is that three different questions are answered by two different authorities.
The council controls **when** the sweep fires; the Strata Administrator controls **where** the funds
land and **who** sits on the council. Upstream states this in the code itself
(`crates/params/src/subprotocols/admin/updates.rs:51-73`):

> The safe harbour destination is rotated by the administrator, not the security council: the
> council can sweep funds to the safe harbour (via Defcon signals) but must not also pick where
> they land, otherwise the same authority could both trigger a sweep and steal the proceeds.

and (`roles.rs:35-38`):

> Its own membership is rotated by the `Role::StrataAdministrator`, not itself, so the council
> cannot lock itself out via self-rotation.

Every other role authorizes its own multisig-config update. The council is the only one that
cannot, and it holds no rotation power over anyone else. The invariant is even visible on the wire:
the SPS-50 byte bands are `10..=19` Administrator, `20..=29` Sequencer Manager, `30..=39` Alpen
Administrator, `40..=49` Security Council — and both `SafeHarbourAddressUpdate = 14` and
`StrataSecurityCouncilMultisigUpdate = 15` sit deliberately in the **Administrator** band.

**This is why this feature spans two authorities.** Delivering "Security Council" end-to-end means
delivering two council actions *and* two Strata Administrator actions; delivering only the council
half would ship the trigger without the safeguards that make it safe.

### 2.2 What happens when a Defcon fires

The admin handler relays a single message to the bridge for **both** Defcon levels
(`subprotocol/src/handler.rs:201`):

```rust
UpdateAction::Defcon1(_) | UpdateAction::Defcon3(_) => relay_bridge_defcon(relayer),
```

`DefconPayload` is an empty marker struct, and the bridge msg enum documents the consequence
(`bridge-v1/msgs/src/lib.rs:34-39`): the admin subprotocol distinguishes Defcon 1 from Defcon 3 on
the signing surface, but *the bridge response is identical*, so they collapse into one message.
**The bridge cannot tell them apart.** The only difference between the two levers is *when* the
message is emitted — immediately, or after a timelock.

The bridge sets `safe_harbour.set_activated(true)` (`bridge-v1/state/bridge.rs:108-110`). This is
idempotent and **nothing ever sets it back to `false`** — there is no de-escalation path in the
protocol. Once activated, safe-harbour address rotation is rejected and silently dropped, so bridge
nodes always observe a single destination (`bridge-v1/state/bridge.rs:114-123`).

---

## 3. Action inventory

Subprotocol id `0` (`admin/txs/src/constants.rs:4`); SPS-50 tag is `(0, tx_type_byte, [])` with
empty aux data — the SSZ `SignedPayload` travels in the taproot leaf witness envelope, not the
OP_RETURN (`admin/txs/src/parser.rs:10-12`).

| SPS-50 byte | `UpdateAction` variant | SSZ union selector | Payload | Authorizing role | Confirmation depth | Cancelable |
|---|---|---|---|---|---|---|
| **41** | `Defcon1` | 9 | `Defcon1Update` — unit struct | **Security Council** | hardcoded `0`, no config field | **never** |
| **43** | `Defcon3` | 10 | `Defcon3Update` — unit struct | **Security Council** | `confirmation_depths.defcon3` | **yes**, see [§5.1](#51-defcon-3-is-cancelable-upstream-the-prd-says-the-council-has-no-cancel) |
| **15** | `StrataSecurityCouncilMultisig` | 3 | `ThresholdConfigUpdate` | **Strata Administrator** | `strata_security_council_multisig_update` | yes if depth ≠ 0 |
| **14** | `SafeHarbourAddress` | 11 | `SafeHarbourAddress` (P2TR BOSD descriptor) | **Strata Administrator** | `safe_harbour_address_update` | yes if depth ≠ 0 |

Two traps worth stating plainly:

- **The SSZ union selector order is not the SPS-50 byte order**, and both are load-bearing.
  `StrataSecurityCouncilMultisig` is selector **3**, inserted before `OperatorSet` — which is what
  makes this pin bump wire-breaking for every action we already persist. See [§5.4](#54-the-pin-bump-invalidates-every-persisted-action_hex).
- **Defcon 1 has no `ConfirmationDepths` field at all.** `ConfirmationDepths::get` returns a
  hardcoded `0` for it (`confirmation_depth.rs:51-53`): *"Defcon1 is the emergency lever — by
  definition it applies immediately, so there is no per-deployment knob for it."* Depth `0` means
  "never enqueued", which is exactly why it can never be cancelled.

### 3.1 The signing message

Both Defcon actions carry no payload, so the rendered signing message is four lines with **no
`Action Details:` block** — the block is omitted when there are no detail lines
(`signing_message.rs:32`). Upstream pins this exact string in a test (`defcon1.rs:40-45`):

```
Strata ASM Administration v1
Action: Defcon 1
Authorized By: Strata Security Council
Sequence: 42
```

The role labels and action names are frozen by contract (`roles.rs:42-45`, `updates.rs:76-79`):
*"Must remain byte-stable: external signers (hardware wallets, signing services) hash the rendered
payload, so changing these labels invalidates already-signed messages."*

Because the action has no payload, **this string is the entire reviewable artifact** — there is
nothing else for a signer to inspect. That drives the signer-safety treatment in
[§8](#8-signer-safety-position).

### 3.2 Observable post-conditions

There is no RPC for the admin queue; admin state is reachable only through
`strata_asm_getAnchorState` plus `find_section(AdministrationSubprotocol::ID)`. The safe harbour
does have a dedicated method, `strata_asm_getSafeHarbour(block_hash) -> Option<SafeHarbour>`
(`crates/rpc/src/traits.rs:39-41`).

| Action | Read | Enacted when |
|---|---|---|
| Defcon 1 | bridge `.safe_harbour().is_activated()` | `true` **in the submission block**; admin `queued()` never grows |
| Defcon 3 | same | `false` until `submit_height + defcon3`, `true` after; entry leaves `queued()` |
| Safe Harbour address | bridge `.safe_harbour().address()` | equals the new address after depth; `is_activated()` **unchanged** |
| Council rotation | `state.authority(Role::StrataSecurityCouncil).config()` | keys/threshold changed after depth |
| any action accepted | `state.authority(role).last_seqno()` | advanced to the payload seqno |

Note that `is_activated()` alone cannot distinguish a Defcon 1 from a Defcon 3 that has matured, so
enactment detection for Defcon 3 additionally requires the update to be **absent from
`admin.queued()`**.

---

## 4. PRD ↔ upstream matrix

| PRD requirement | Story | Upstream action | Authorizing role | Slice |
|---|---|---|---|---|
| §5.5 *Security Council multisig: Defcon 1 transaction* | US-E12 | `UpdateTxType::Defcon1 = 41` | Strata Security Council | V1 |
| §5.5 *Security Council multisig: Defcon 3 transaction* | US-E13 | `UpdateTxType::Defcon3 = 43` | Strata Security Council | V2 |
| §5.5 *Strata Administrator: Security Council Signer update* | US-E7 | `StrataSecurityCouncilMultisigUpdate = 15` | **Strata Administrator** | V3 |
| §5.5 *Strata Administrator: Safe Harbor address update* | US-E5 | `SafeHarbourAddressUpdate = 14` | **Strata Administrator** | V4 |
| §3.1.4 *Strata Security Council multisig MUST be usable exclusively by all Strata Security Council Signers* | US-C1 | `Role::StrataSecurityCouncil` membership | — | V1 |
| §5.2.2 *…does not apply to the Strata Security Council multisig, because it does not produce update types that have an "Approved" or "Canceled" state* | — | contradicted upstream, see [§5.1](#51-defcon-3-is-cancelable-upstream-the-prd-says-the-council-has-no-cancel) | — | V5 |
| §5.5 *Strata Administrator: "Soft" bridge update / "Hard" bridge update* | US-E9, US-E10 | **none — concept withdrawn** | — | out of scope |

### 4.1 Requirement numbering

The PRD has been re-issued twice. Sections 1–5 of the latest revision are identical to
[`03-prd-update.md`](../0-prd/03-prd-update.md); only §6 (Payout Administrator) was rewritten.
Everything about the Security Council has been carried **unchanged** since the original
[`01-multisig-ui.md`](../0-prd/01-multisig-ui.md) of 2026-04-07, where the same requirements are
numbered §7.4, §12.2 and §15.4. This document uses the numbering of the latest revision (05).

---

## 5. Known discrepancies

Recorded rather than resolved silently. Each is either an open question for Alpen
([§9](#9-open-questions-for-alpen)) or a decision this feature must make explicitly.

### 5.1 Defcon 3 is cancelable upstream; the PRD says the council has no cancel

PRD §5.2.2 excludes the Security Council from the Approved/Canceled lifecycle *"because it does not
produce update types that have an 'Approved' or 'Canceled' state"*. Upstream, that is **half true**:

- **Defcon 1** — correct. Depth is hardcoded `0`, so it is never enqueued and a cancel targeting it
  fails with `UnknownAction`. Upstream says so in the type doc (`defcon1.rs:11-14`).
- **Defcon 3** — **incorrect**. It is enqueued like any other update and the cancel handler has no
  variant filter (`handler.rs:129-142`). Every deployment in the upstream tree sets
  `defcon3 ≠ 0` (functional tests use 144, the Rust harness 2), so the queued, cancellable window
  genuinely exists.

Worse for the PRD's framing: a cancel's authorizing role is *the role of the update being
cancelled* (`actions/mod.rs:62`). So a Defcon 3 cancel is signed by **the Security Council itself** —
the very authority the PRD says has no cancel. There is no cross-role veto and no separate canceller
role.

A deployment *could* collapse the two by setting `defcon3 = 0`, making Defcon 3 immediate and
uncancellable. Nothing upstream does that, and the tests assert the opposite. We therefore treat
"Defcon 3 has a queued, cancellable window" as ground truth and **drive it from the live
`confirmation_depths` rather than hardcoding it**.

Product decision required — this is [open question 1](#9-open-questions-for-alpen) and the entire
content of slice V5.

### 5.2 The story map says Defcon 3 executes immediately

[`story-map.md`](../3-stories/story-map.md) US-E13 describes Defcon 3 as *"Defcon 3 emergency action
(executes immediately)"*, and §5 of the same document says the Approved state does not apply to the
council. Both predate upstream and are wrong for Defcon 3. Corrected in Stage 6.

### 5.3 The 72-hour delay has no default in code

Alpen's public documentation describes the delayed sweep as a **72-hour** delay (≈432 blocks).
`ConfirmationDepths.defcon3` is a per-deployment parameter with **no default anywhere in the ASM**;
the only values in the tree are test fixtures (144 and 2). The production value must come from the
operator — [open question 2](#9-open-questions-for-alpen).

### 5.4 The pin bump invalidates every persisted `action_hex`

`StrataSecurityCouncilMultisig` is inserted at SSZ union selector **3**, shifting `OperatorSet` and
every later variant by one. Any `action_hex` persisted before the bump decodes to a *different
action* after it, and `ActionId = hash(MultisigAction, SeqNo)` values are not comparable across the
boundary. This makes the operational reset in Stage 3 mandatory, not optional.

### 5.5 Two PRD items have no upstream counterpart at any revision

- **"Soft" bridge update / "Hard" bridge update** (§5.5, US-E9/US-E10): zero references in the ASM
  at any tag. The client has since stated these are no longer relevant concepts. Out of scope here;
  flagged for formal removal from the PRD — [open question 4](#9-open-questions-for-alpen).
- **Payout Administrator**: no `Role::PayoutAdmin` and no corresponding `UpdateTxType` exists
  upstream, not even on `main`. Out of scope for this feature, but it means `Authority::PayoutAdmin`
  remains the one authority with no ASM mapping after this work lands.

---

## 6. Stage board

| Stage | Goal | Status |
|---|---|---|
| 0 | Branch off `develop`; triage the two prior branches | Done |
| 1 | High-level discovery; this document | Done |
| 2 | ASM pin decision, with compile evidence → ADR-007 | In progress |
| 3 | Upstream capability evaluation — **go/no-go gate** | Pending |
| 4 | Functional specs, one per slice | Pending |
| 5 | Vertical slices V1–V5 | Pending |
| 6 | Close-out: compliance audit, doc updates, issue #117 | Pending |

## 7. Slice board

| Slice | End-to-end path | Status |
|---|---|---|
| V1 — Defcon 1 | Authenticate as a council signer → create → sign → quorum → broadcast → Enacted | Pending |
| V2 — Defcon 3 | Same path, timelocked, with an activation countdown | Pending |
| V3 — Security Council signer update | Strata Admin rotates council membership | Pending |
| V4 — Safe Harbour address update | Strata Admin sets the sweep destination | Pending |
| V5 — Defcon 3 cancel | Conditional on [open question 1](#9-open-questions-for-alpen) | Pending |

V1 carries the shared spine (authority→role mapping, per-action lock period, enactment detection,
codec, action builder, authentication, signer-safety UX), so every later slice is cheap.

### 7.1 Upstream version notes

The entire role/action/segregation model is **byte-identical** between `v0.1-alpha.11` and `v0.3.1`
— PR #81 landed it complete and nothing since has changed it. The only API differences that reach
us are `ThresholdConfigUpdate::new` → `try_new` (now fallible, via the `strata-common` v0.3.0
cascade) and a split of `AdministrationTxParseError::MalformedTransaction` into two variants. Wire
format, signing-message bytes, role assignments, confirmation-depth semantics and cancel semantics
are the same at both tags.

### 7.2 Coverage upstream does not have

Upstream tests cover Defcon 1/3 propagation, the negative activation window, and wrong-role
rejection for all four actions. They do **not** cover: cancelling a queued Defcon 3, an end-to-end
`StrataSecurityCouncilMultisigUpdate`, or the council losing its signing ability after the
Administrator rotates it. Those three are the highest-value tests for us to write.

---

## 8. Signer-safety position

Defcon authorizes sweeping **all bridge funds**, and there is no de-escalation path in the protocol
— `is_activated()` is never set back to `false`. Combined with the fact that the actions carry no
payload, this drives the treatment:

- **The four-line signing message is the reviewable artifact.** Rendered verbatim in the form, it
  is byte-identical to what the hardware wallet displays.
- **Type-to-confirm** (`DEFCON 1` / `DEFCON 3`) before the sign CTA enables.
- **Distinct destructive visual treatment**, unmistakably different from every other action form.
- **Authority context on every step** — the council badge from create through broadcast.
- A non-council session can never reach these forms; a council session sees only Defcon actions.

Backend lifecycle stays uniform (`Pending → Approved → Enacted`); the **UI re-labels rather than
re-models**. A council proposal shows "Quorum reached — ready to broadcast", never "Approved", and
carries no cancel CTA (pending V5).

---

## 9. Open questions for Alpen

1. **Defcon 3 cancellation.** Upstream queues Defcon 3 in a cancellable window signed by the
   council itself, contradicting PRD §5.2.2. Should the application expose that cancel? If not, we
   land the "structurally unreachable" invariant plus its test instead. Blocks slice V5 only.
2. **Production `confirmation_depths.defcon3`.** Public docs say 72 hours (≈432 blocks); the code
   has no default. What value ships?
3. **Expiry.** Does the standard 7-day pending-proposal expiry (§5.3.4) apply to Defcon proposals,
   given their emergency nature?
4. **"Soft"/"Hard" bridge update** (§5.5, US-E9/US-E10) have no upstream presence at any revision
   and were described as no longer relevant. Confirm formal removal from the PRD.
5. **Payout Administrator** has no upstream role at any revision. Any timeline?

---

## 10. Related documents

| Topic | Document |
|---|---|
| Alpen crate dependency strategy | [`adrs/001-alpen-crate-dependencies.md`](../architecture/adrs/001-alpen-crate-dependencies.md) |
| ASM pin decision for this feature | `adrs/007-*.md` (Stage 2) |
| Cancel lifecycle (template and precedent) | [`cancel-approved-proposal.md`](./cancel-approved-proposal.md) |
| Signer safety model | [`signer-safety-model.md`](./signer-safety-model.md) |
| Commit/reveal broadcast pipeline | [`proposal-broadcast-commit-reveal.md`](./proposal-broadcast-commit-reveal.md) |
| Superseded upstream-gap analyses | [`08-alpen-crate-prd-coverage.md`](../2-discovery/08-alpen-crate-prd-coverage.md), [`19-asm-bump-impact-assessment.md`](../2-discovery/19-asm-bump-impact-assessment.md) |
