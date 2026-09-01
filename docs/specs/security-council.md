# Security Council — Master Plan

**Status:** V1 (Defcon 1) shipped end to end; V2–V5 pending, and none of them has a functional contract yet
**PRD:** [`06-prd-hardware-signer-and-block-payouts-update.md`](../0-prd/06-prd-hardware-signer-and-block-payouts-update.md) (current snapshot) §3.1.4, §5.1, §5.2.2, §5.5
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

**Those statements are stale.** `alpenlabs/asm` PR #81 (*feat(admin): add Security Council and
Defcon actions*, merge commit `3d45351`, merged 2026-05-30) implemented the role and all four
actions this feature needs, complete. They were invisible from here because the workspace was
pinned at `e0461f8` (2026-05-11), 19 commits before that merge. The pin has since moved to
`v0.1-alpha.11` ([ADR-007](../architecture/adrs/007-asm-pin-for-security-council.md)) and the
capability is proven against a regtest ASM — see [§3.3](#33-go-no-go-result).

The historical documents are not edited; they were true when written. They are routed around
instead, by the Security Council row in the SSOT table of [`docs/README.md`](../README.md).

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
| **43** | `Defcon3` | 10 | `Defcon3Update` — unit struct | **Security Council** | `confirmation_depths.defcon3` | **yes**, by the council itself — see [§5.1](#51-defcon-3-is-cancelable--resolved-the-prd-was-corrected) |
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

### 3.3 Go / no-go result

**GO.** `e2e-tests/tests/e2e_defcon_probe.rs` proves all of the above against a real regtest ASM,
through the real path — two council signatures, commit, reveal, worker processing the block — with
no product code involved. Defcon 1 activates the safe harbour inside the reveal block and never
enters the queue; Defcon 3 stays queued with the harbour off and activates exactly at its depth;
the signing message matches the four canonical lines with no details block.

Three things the probe settled that the source reading alone had left open:

- **The harness needs no explicit bridge config.** `AsmParams::arbitrary` always emits all three
  subprotocols, and `SafeHarbourAddress`'s `Arbitrary` impl derives a valid P2TR descriptor from a
  fresh keypair, so the default harness already carries a deactivated safe harbour.
- **The activation boundary is `activation_height <= tip`**, so exactly `depth` blocks after the
  reveal are required — not `depth + 1`. Upstream's doc comment on `process_queued` says "equals"
  while the code partitions on `<=`; harmless drift, but `e2e_enactment_predicate` was mining one
  block more than necessary. Corrected in Stage 3 close-out, and checked in both directions: the
  test passes at `depth` and fails at `depth - 1`.
- **Seqno is per-role**, so the council's counter is independent of the administrator's and a
  fresh council authority starts from the same baseline regardless of admin activity.

Not covered, and worth adding when the product surfaces a role-mismatch error: a Defcon signed by
a non-council role. Upstream covers it in `asm/tests/asm/admin_to_bridge.rs`.

---

## 4. PRD ↔ upstream matrix

| PRD requirement | Story | Upstream action | Authorizing role | Slice |
|---|---|---|---|---|
| §5.5 *Security Council multisig: Defcon 1 transaction* | US-E12 | `UpdateTxType::Defcon1 = 41` | Strata Security Council | V1 |
| §5.5 *Security Council multisig: Defcon 3 transaction* | US-E13 | `UpdateTxType::Defcon3 = 43` | Strata Security Council | V2 |
| §5.5 *Strata Administrator: Security Council Signer update* | US-E7 | `StrataSecurityCouncilMultisigUpdate = 15` | **Strata Administrator** | V3 |
| §5.5 *Strata Administrator: Safe Harbor address update* | US-E5 | `SafeHarbourAddressUpdate = 14` | **Strata Administrator** | V4 |
| §3.1.4 *Strata Security Council multisig MUST be usable exclusively by all Strata Security Council Signers* | US-C1 | `Role::StrataSecurityCouncil` membership | — | V1 |
| §5.2.2 *…subsection (b) does not apply to … Strata Security Council multisig **(Defcon 1 transaction)*** | — | Defcon 1 has no Approved/Canceled state; Defcon 3 does — see [§5.1](#51-defcon-3-is-cancelable--resolved-the-prd-was-corrected) | — | V5 |
| §5.5 *Strata Administrator: "Soft" bridge update / "Hard" bridge update* | ~~US-E9~~, ~~US-E10~~ | **none — confirmed withdrawn** | — | retired |

### 4.1 Requirement numbering

This document uses the numbering of the current PRD snapshot,
[`06-prd-hardware-signer-and-block-payouts-update.md`](../0-prd/06-prd-hardware-signer-and-block-payouts-update.md).

Nothing about the Security Council has ever moved. §1–5 were carried unchanged from
[`03-prd-update.md`](../0-prd/03-prd-update.md) through snapshots 05 and 06, and the requirements
themselves go back to the original [`01-multisig-ui.md`](../0-prd/01-multisig-ui.md) of 2026-04-07,
where they are numbered §7.4, §12.2 and §15.4. Between 05 and 06 the only substantive change
anywhere in §5 is the §5.2.2 carve-out described below.

> **The §5.2.2 amendment has landed in `0-prd/`.** It was agreed on 2026-08-12 and arrived in the
> repository with snapshot 06, which carries the amended wording verbatim — so the PRD, upstream and
> this document now agree, and there is no local copy running behind. Snapshot 05 keeps the
> superseded wording as history; read 06. `0-prd/` holds frozen client inputs and is never edited in
> place, so a correction always arrives as a new snapshot rather than a patch to an old one.

---

## 5. Known discrepancies

Recorded rather than resolved silently. Every one is now either an explicit decision this feature
makes, or a deferral with a stated assumption. **Nothing here is still open with Alpen** —
[§9](#9-questions-for-alpen) tracks the trail.

### 5.1 Defcon 3 is cancelable — resolved, the PRD was corrected

**Resolved 2026-08-12, and in `0-prd/` since snapshot 06. The PRD matches upstream, and the carve-out
applies to Defcon 1 only.**

The original PRD §5.2.2 excluded the whole Security Council from the Approved/Canceled lifecycle
*"because it does not produce update types that have an 'Approved' or 'Canceled' state"*. We raised
that this is only half true against the code, and the requirement was amended. Quoting
[`06-prd-hardware-signer-and-block-payouts-update.md`](../0-prd/06-prd-hardware-signer-and-block-payouts-update.md)
§5.2.2, which is the authority here:

> For the avoidance of doubt, this subsection (b) does not apply to the following multisigs /
> transaction type, because they do not produce proposals that have an "Approved" or "Canceled"
> state:
> - Strata Sequencer Manager multisig
> - **Strata Security Council multisig (Defcon 1 transaction)**

Two clarifications came with it: "update types" became "proposals", and the subsection was named
explicitly as **(b)** — the whole Approved-updates block, including viewing cancellation signatures,
cancelling, and the cancel broadcast flow.

So the split is now explicit in the requirement, and it is exactly what the code does:

- **Defcon 1** — carved out. Depth is hardcoded `0`, so it is never enqueued and a cancel targeting
  it fails with `UnknownAction` (`defcon1.rs:11-14`). No Approved state, no cancel, ever.
- **Defcon 3** — **in scope for §5(b)**. It is enqueued like any other update and the cancel handler
  has no variant filter (`handler.rs:129-142`). Every deployment in the upstream tree sets
  `defcon3 ≠ 0` (functional tests use 144, the Rust harness 2), so the queued, cancellable window
  genuinely exists and the application must expose it.

One consequence to carry into V5: a cancel's authorizing role is *the role of the update being
cancelled* (`actions/mod.rs:62`), so a Defcon 3 cancel is signed by **the Security Council itself**.
There is no cross-role veto and no separate canceller role — the same council that raised the alarm
is the one that stands it down.

A deployment *could* collapse the two by setting `defcon3 = 0`, making Defcon 3 immediate and
uncancellable. Nothing upstream does that, and the tests assert the opposite. We treat "Defcon 3 has
a queued, cancellable window" as ground truth and **drive it from the live `confirmation_depths`
rather than hardcoding it** — which also means a deployment that did set it to 0 degrades correctly
rather than showing a cancel affordance that cannot work.

**Slice V5 is confirmed in scope**, no longer conditional.

### 5.2 The story map said Defcon 3 executes immediately — corrected

[`story-map.md`](../3-stories/story-map.md) US-E13 used to describe Defcon 3 as *"executes
immediately"*, and the carve-out language elsewhere in that document excluded the whole council from
the Approved state. Both predated upstream and were wrong for Defcon 3 on two counts: it is
timelocked, and it *does* reach an Approved state with a cancel window
([§5.1](#51-defcon-3-is-cancelable--resolved-the-prd-was-corrected)).

**Corrected in Stage 3 close-out** rather than deferred: US-E13 now describes the timelock and the
cancel window, US-D3 and US-F2 narrow the carve-out to Defcon 1, and **US-E14** carries the Defcon 3
cancel that slice V5 delivers.

### 5.3 The Defcon 3 delay is read from live ASM state, never hardcoded

Alpen's public documentation describes the delayed sweep as a **72-hour** delay (≈432 blocks).
`ConfirmationDepths.defcon3` is a per-deployment parameter with **no default anywhere in the ASM**;
the only values in the tree are test fixtures (144 and 2).

**Decision: the application always resolves the depth from the live `confirmation_depths`** — never
a constant, never a UI default, never 432. The production value is
[deferred](#9-questions-for-alpen) rather than unknown-and-blocking: because the depth is
configurable, taking whatever the ASM reports is correct on every deployment without a code change,
and production is assumed to honour the documented 72 hours. We neither depend on that number nor
assert it.

Two consequences worth stating, since they are easy to get wrong later:

- This is why `lock_period_for_authority` has to become **`lock_period_for_action`** in V1. The
  council has two tx types with different depths — Defcon 1 fixed at 0, Defcon 3 configurable — so
  a per-authority mapping is wrong by construction, not merely imprecise.
- The Defcon 3 activation countdown shown to signers derives from the same live value. Upstream's
  own test harness does this too: its `activation_depth` helper reads
  `admin_state().confirmation_depth(tx_type)` rather than a constant, precisely so it cannot drift
  from how the deployment was configured.

### 5.4 The pin bump invalidates every persisted `action_hex`

`StrataSecurityCouncilMultisig` is inserted at SSZ union selector **3**, shifting `OperatorSet` and
every later variant by one. Any `action_hex` persisted before the bump decodes to a *different
action* after it, and `ActionId = hash(MultisigAction, SeqNo)` values are not comparable across the
boundary. This makes the operational reset in Stage 3 mandatory, not optional.

### 5.5 Two PRD items have no upstream counterpart at any revision — both resolved

Neither is an open question any more; both were settled while this document was being written.

- **"Soft" bridge update / "Hard" bridge update** (§5.5, US-E9/US-E10): zero references in the ASM
  at any tag. **Confirmed withdrawn** — they are no longer relevant concepts. US-E9 and US-E10 were
  retired in place in the story map during Stage 3 close-out; the rows stay, struck through, so the
  numbering other documents cite does not shift.
- **Payout Administrator**: no `Role::PayoutAdmin` and no corresponding `UpdateTxType` exists
  upstream, not even on `main`. **Not implemented for now**, by decision rather than by blocker. It
  is therefore expected — not a gap to chase — that `Authority::PayoutAdmin` remains the one
  authority with no ASM role mapping after this work lands. The enum variant stays; it just maps to
  nothing upstream.

---

## 6. Stage board

| Stage | Goal | Status |
|---|---|---|
| 0 | Branch off `develop`; triage the two prior branches | Done |
| 1 | High-level discovery; this document | Done |
| 2 | ASM pin decision, with compile evidence → [ADR-007](../architecture/adrs/007-asm-pin-for-security-council.md) | Done — `v0.1-alpha.11` |
| 3 | Upstream capability evaluation — **go/no-go gate** | Done — **GO**, see [§3.3](#33-go-no-go-result) |
| 3.5 | Close-out of 0–3: absorb `develop`, retire the "blocked on upstream" claims across the docs | Done |
| 4 | Functional specs — Defcon first (V1, V2, V5's cancel), then the rest | In progress — V1 done: [`security-council-defcon.md`](./security-council-defcon.md); V2–V5 unwritten |
| 5 | Vertical slices V1–V5 | In progress — V1 shipped; V2–V5 pending |
| 6 | Close-out: compliance audit, issue #117 | Pending |

Stage 3.5 was the documentation debt the earlier stages left behind. `develop` was absorbed into the
branch (28 commits: v0.2.6 and the Admin ID program), and every live document that still called the
Security Council *blocked on upstream* now says what is actually true — it is not built yet, and the
protocol support has been proven. What remains historical stays historical:
[`2-discovery/`](../2-discovery/) is Phase 1 evidence and is covered by a routing row in
[`docs/README.md`](../README.md) instead of being rewritten. Corrections that this document had
parked for Stage 6 — the story map, the retired stories — were made here, so Stage 6 is now only the
compliance audit and issue #117.

## 7. Slice board

| Slice | End-to-end path | Status |
|---|---|---|
| V1 — Defcon 1 | Authenticate as a council signer → create → sign → quorum → broadcast → Enacted | Spec written — [`security-council-defcon.md`](./security-council-defcon.md); build plan — [`security-council-defcon-implementation.md`](./security-council-defcon-implementation.md); **shipped**, all eight phases (PRs #505–#512) |
| V2 — Defcon 3 | Same path, timelocked, with an activation countdown | Pending |
| V3 — Security Council signer update | Strata Admin rotates council membership | Pending |
| V4 — Safe Harbour address update | Strata Admin sets the sweep destination | Pending |
| V5 — Defcon 3 cancel | Council cancels its own queued Defcon 3 (US-E14), reusing the existing cancel flow | Pending — **confirmed in scope** |

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
  is byte-identical to what the hardware signer displays.
- **Type-to-confirm** (`DEFCON 1` / `DEFCON 3`) before the sign CTA enables.
- **Distinct destructive visual treatment**, unmistakably different from every other action form.
- **Authority context on every step** — the council badge from create through broadcast.
- A non-council session can never reach these forms; a council session sees only Defcon actions.

Backend lifecycle stays uniform (`Pending → Approved → Enacted`); the **UI re-labels rather than
re-models**. But the two levers diverge here, and the divergence is a PRD requirement rather than a
styling choice — see [§5.1](#51-defcon-3-is-cancelable--resolved-the-prd-was-corrected):

- **Defcon 1** is carved out of PRD §5(b) entirely. It shows "Quorum reached — ready to broadcast",
  never the word "Approved", and carries **no cancel CTA anywhere**. `Canceled` is structurally
  unreachable for it: depth 0 means it is never enqueued, so a cancel would fail on-chain with
  `UnknownAction` even if the UI offered one. Worth an invariant test rather than trusting the
  absence of a button.
- **Defcon 3** is fully inside §5(b). It reaches a real **Approved** state, appears in the Approved
  list with its cancellation-signature count, and gets the standard cancel flow — copy signatures,
  build the cancel transaction, broadcast. Signed by the council itself.

The practical shape: the same authority that fires the alarm is the one that can stand it down, and
only for the timelocked lever. Defcon 1 is deliberately a one-way door.

**Expiry is not special-cased.** A Defcon proposal that fails to reach quorum expires on the
standard 7-day window like any other pending proposal. The emergency framing applies to how the
action is confirmed and how fast it enacts once broadcast — not to how long an unsigned proposal
lingers. Decided, not overlooked: nothing in the PRD or upstream asks for a different window.

---

## 9. Questions for Alpen

**None open.** Everything raised during discovery is answered or deferred with a stated assumption.

### Deferred until a testnet or production environment exists

Alpen has neither today, so both of these are answerable only once those environments are stood up.
Each records the assumption we build against, so the deferral stays auditable instead of becoming a
forgotten gap.

- **Production `confirmation_depths.defcon3`.** Public docs say 72 hours (≈432 blocks); the ASM has
  no default. *We build against:* whatever the live ASM reports, never a constant — see
  [§5.3](#53-the-defcon-3-delay-is-read-from-live-asm-state-never-hardcoded). Production is assumed
  to honour the documented value.
- **Production safe harbour address.** `BridgeV1InitConfig.safe_harbour_address` is required at
  genesis and must be a P2TR BOSD descriptor; ours is a deliberate regtest throwaway. *We build
  against:* the address being supplied when those environments are created. Nothing in the
  application hardcodes or validates a specific destination — the council triggers the sweep, the
  Strata Administrator owns where it lands ([§2.1](#21-the-segregation-invariant)).

### Answered

- **Defcon 3 cancellation.** Answered 2026-08-12 by amending the PRD: the §5(b) carve-out now
  applies to the Security Council **only for Defcon 1**, so Defcon 3 has a real Approved state and
  a real cancel. Slice V5 is confirmed in scope. See
  [§5.1](#51-defcon-3-is-cancelable--resolved-the-prd-was-corrected).
- **Expiry.** The standard 7-day pending-proposal window applies to Defcon proposals too — no
  emergency carve-out. Neither the PRD nor anything else states an exception, so this is a decision,
  not an oversight. See [§8](#8-signer-safety-position).
- **"Soft"/"Hard" bridge update** — confirmed no longer relevant concepts. US-E9 and US-E10 are
  retired in the story map.
- **Payout Administrator** — not implemented for now; see [§5.5](#55-two-prd-items-have-no-upstream-counterpart-at-any-revision--both-resolved).

---

## 10. Related documents

| Topic | Document |
|---|---|
| Alpen crate dependency strategy | [`adrs/001-alpen-crate-dependencies.md`](../architecture/adrs/001-alpen-crate-dependencies.md) |
| ASM pin decision for this feature | [`adrs/007-asm-pin-for-security-council.md`](../architecture/adrs/007-asm-pin-for-security-council.md) |
| Cancel lifecycle (template and precedent) | [`cancel-approved-proposal.md`](./cancel-approved-proposal.md) |
| Signer safety model | [`signer-safety-model.md`](./signer-safety-model.md) |
| Commit/reveal broadcast pipeline | [`proposal-broadcast-commit-reveal.md`](./proposal-broadcast-commit-reveal.md) |
| Superseded upstream-gap analyses | [`08-alpen-crate-prd-coverage.md`](../2-discovery/08-alpen-crate-prd-coverage.md), [`19-asm-bump-impact-assessment.md`](../2-discovery/19-asm-bump-impact-assessment.md) |
