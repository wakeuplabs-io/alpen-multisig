# Security Council — Safe Harbour Address Update (V4) Implementation Plan

**Functional contract:** [`security-council-safe-harbour-address.md`](./security-council-safe-harbour-address.md)
— the SSOT for *what* V4 must do. This document is only *how* it gets built, and never overrides it.

**Master plan:** [`security-council.md`](./security-council.md) §6 Stage board, §7 Slice board.

**Story:** [`story-map.md`](../3-stories/story-map.md) US-E5.

**Status:** All three phases implemented, automated checks green. The Phase 1 manual walk produced
two findings, both fixed in Phase 2; the walk of 2026-09-10 over Phase 2's surfaces produced seven,
five of which Phase 3 closes — the remaining two are recorded in
[Phase 3 §8](./security-council-safe-harbour-address-phase-3.md#8-what-the-walk-found-and-this-phase-does-not-take).
The reserve is spent.

A phase marked ✅ means the engineering step shipped, not that every acceptance criterion in the
contract is satisfied — the contract's `## Acceptance Criteria` section stays the measure.

## 1. Purpose and scope

V4 is the last slice of the feature, and it is the first one whose *shape* differs from its
predecessors rather than only its payload. V1 carried the shared spine, V2 and V3 spent it. What is
left here is genuinely new in two places and free everywhere else:

- **A payload the application has to construct**, from an address to a BOSD descriptor, with the
  conversion exposed for verification.
- **A post-condition in another subprotocol**, which the bridge can accept and silently refuse.

Everything else — depth, cancelability, authorization, countdown, lifecycle, manual bundle — already
answers for tx type 14 with no new branch
([Constraint 7](./security-council-safe-harbour-address.md#7-everything-generic-already-answers-and-gets-tests-rather-than-changes)).

**In scope**

- `UpdateTxType::SafeHarbourAddressUpdate = 14` end to end, authorized by the Strata Administrator
  (US-E5).
- The standard cancel, which per PRD §5.2.2 applies here in full.
- The end-to-end test of a rotation submitted after the harbour is up, which exists nowhere.

**Not in scope**

- Blocking a rotation the chain will swallow. See
  [Constraint 1](./security-council-safe-harbour-address.md#1-a-rotation-with-the-harbour-already-activated-is-accepted-and-changes-nothing).
- De-escalating the safe harbour. No such action exists upstream.
- Any protocol validity rule. The orchestrator stays coordination-only.

## 2. Traceability

| Phase | Name | Closes (contract) | Touches |
|---|---|---|---|
| 1 ✅ | From the screen to `Enacted` — [phase spec](./security-council-safe-harbour-address-phase-1.md) | AC 1, 1a, 2, 3, 3a, 3b, 3c, 4, 5, 6, 7, 7a, 7b, 8, 11, 12; Constraints 1-7 | `Cargo.toml`, `src-tauri`, `desktop-app`, `orchestrator-be` |
| 2 ✅ | The cancel, the detail, the message panel and the e2e — [phase spec](./security-council-safe-harbour-address-phase-2.md) | AC 6 (countdown), 7a, 9, 10, 13; the two manual-walk findings | `e2e-tests`, `desktop-app`, `orchestrator-be`, `src-tauri` |
| 3 ✅ | What the manual walk exposed — [phase spec](./security-council-safe-harbour-address-phase-3.md) | The five findings of the 2026-09-10 walk that belong to this slice | `desktop-app` |

Each phase is its own pull request against `develop`, branched from a freshly pulled `develop`.
Phases are sequential, not parallel.

## 3. Architecture

### What already exists and is reused

The list is the point of the slice: nothing below needs a line of new code.

| Piece | Location | Why it matters |
|---|---|---|
| `require_authorized_for_action` | `orchestrator-be/src/infrastructure/asm_role_membership.rs:243-270` | Generic over upstream's `authorized_role()`, which returns `StrataAdministrator` for tx type 14. A council session is refused with no council-specific code — the segregation invariant enforced by the generic mechanism. |
| `lock_period_for_action` / `depth_for_action` | same module, `:112-148` | Resolves through `update_tx_type()`, so tx type 14 reaches `confirmation_depths.safe_harbour_address_update` with no new branch. |
| `ConfirmationDepthResolver::is_cancelable_for_hex` | same module, `:187-193` | Derived from the live depth, so the action is cancelable exactly when its depth is non-zero. |
| `decode_bridge_state` | `orchestrator-be/src/infrastructure/asm_enactment.rs` (used at `:99,118,139`) | The bridge half of the post-condition is already decoded at the call site for the Defcon arms. |
| `create_cancel_proposal` | `orchestrator-be/src/application/proposals.rs` | Stores the cancel under the target's authority (`strata_admin`) and requires the session to match. |
| `render_signing_message` / `compute_sighash` | `desktop-app/src-tauri/src/infrastructure/signing.rs:57-67,138-145` | Delegate to `SigningMessage::for_action`. The six canonical lines come out for free. |
| `network_from_env` | `desktop-app/src-tauri/src/infrastructure/network_env.rs:31-34` | The process-wide network resolution this repository already treats as canonical. `NodeConfig` deliberately carries only endpoints. |
| `SafeHarbourNote`, `useSafeHarbourActivated` | `desktop-app/src/components`, `src/hooks` | The already-in-harbour note is a component, not a Defcon detail. |
| `VkUpdateFormFields` | `desktop-app/src/domain/create-proposal/components` | The shape the new fields component follows: current value from chain, then one input. |
| `showsActivationCountdown` | `desktop-app/src/lib/proposal-status.ts` | Already correct — it excludes only `defcon_1`. |
| `e2e_council_rotation.rs` | `e2e-tests/tests/` | The shape Phase 2's e2e follows, including the exhaustive `ConfirmationDepths` literal that already carries `safe_harbour_address_update`. |

### The three breaking points

**The domain does not learn BOSD.** `action_codec.rs` is by module contract the only place that
imports protocol crates, and that holds. The domain gains a newtype holding the **32-byte x-only
key**, constructed from a bech32m address (via `bitcoin::Address`, already used at
`domain/action.rs:45`) or from hex, exposing the BOSD bytes as `[0x04] ++ key`. The codec is what
calls `Descriptor::new_p2tr` and `SafeHarbourAddress::try_from`.

**Two dependencies, and one trap.** `strata-asm-proto-bridge-v1` is declared but does **not**
re-export `SafeHarbourAddress` (`bridge-v1/subprotocol/src/lib.rs:31-34`), so
`strata-asm-proto-bridge-v1-types` and `bitcoin-bosd` are both needed. The trap: `Cargo.lock`
already holds **two** `bitcoin-bosd` — 0.9.0 from crates.io via `strata-btc-types`, and 0.11.0 from
alpenlabs' git via the bridge types crate. The new declaration goes in `[workspace.dependencies]`
pinned to the **same tag `v0.11.0`**; against the crates.io one the types do not unify and the error
surfaces only when the codec compiles.

**Enactment leaves the administration subprotocol.** `is_proposal_enacted_on_asm` answers
`BadRequest("not implemented yet")` at `asm_enactment.rs:169-171` today. It becomes a predicate over
the bridge's address and the administrator's seqno
([Constraint 2](./security-council-safe-harbour-address.md#2-enactment-is-read-from-the-bridge-and-the-seqno-from-the-administrator)),
applied to **both** copies of the module.

### Where V4 lives in the frontend

Same answer V1, V2 and V3 settled: it extends `desktop-app/src/domain/create-proposal/` and gets no
route of its own. The domain dispatches by action type, so this is one more entry in
`ACTION_TYPES_BY_AUTHORITY`, one more validator, one more fields component.

**The fields component is new, not parameterized.** Unlike the two Defcon levels — which differ only
in copy — and unlike the two signer updates — which differ only in which config feeds them — this
action has a different field, a different validation and a different preview. There is nothing to
share but the frame.

### Tripwires

Four sites **fail to compile** if missed, and are a free checklist: `validators/index.ts:10`
(`Record<ActionType, _>`), `action-type-from-decoded.ts:19` (`Record<DecodedAction['kind'], _>`),
the `const unhandled: never` at `use-create-proposal.ts:126-129`, and Rust's exhaustive matches.

Four do **not**, and have to be carried by hand:

1. `lib/proposal-type-label.ts` — an `if` chain ending in `return 'Unknown'`.
2. `api/ipc-schemas.ts:48` — a closed `z.enum` that fails at runtime, not at compile time, and takes
   the parse of *every proposal in the same list* with it. This is why TypeScript precedes Rust in
   the commit order below, exactly as in V3 Phase 1 §3.
3. `desktop-app/src-tauri/src/commands/invoke.rs` — **two** handler lists (`:22-28` production,
   `:95-101` dev-signing). Registering in one makes the command fail in one mode only.
4. `screens/__tests__/safe-harbour-note-gating.test.ts:30-48` — it scans `SafeHarbourNote` render
   sites and requires each to call `useSafeHarbourActivated(` behind the guard. Phase 1 confirms
   whether its glob reaches `domain/**` or only `screens/*`, and widens it if not.

## 4. Phased plan

### Phase 1 — From the screen to `Enacted`, in one vertical

The screen exists from the first day, and the action reaches `Enacted` in the same pull request. It
is the largest phase of the slice and that is the intended outcome: a `develop` carrying the menu
entry without enactment detection is a form that lets a signer create a proposal which can only park
at Approved — the state V1, V2 and V3 each refused to ship.

Commits, in order. Each is atomic; none repairs the one before it.

| # | Contents |
|---|---|
| 1 | `bitcoin-bosd` (tag `v0.11.0`) and `strata-asm-proto-bridge-v1-types` in `[workspace.dependencies]` and in `desktop-app/src-tauri/Cargo.toml`. No behaviour change. |
| 2 | TypeScript vocabulary: both `ActionType` unions, the `z.enum` in `ipc-schemas.ts`, the `decodedActionSchema` member, the `DecodedAction` type, `ACTION_TYPE_BY_KIND`, and the label. Inert — nothing emits the value yet. |
| 3 | The Rust domain newtype and its parsing table, red then green. |
| 4 | The codec in both directions, the builder command registered in **both** `invoke.rs` lists, `action_type_from_hex`, and the `DecodedAction` variant. |
| 5 | The bridge's current address on the read path: `asm_status_rpc` returns it alongside the activation flag, `SafeHarbourStatusDto` carries both, and the TS API follows. |
| 6 | The menu entry, the validator (including the no-op rule), the fields component, the preview arm and the sign-view arm, with the safe-harbour note on all three surfaces. |
| 7 | Enactment in `orchestrator-be` and in the desktop's copy, plus the stale comment at `asm_enactment.rs:113-116` that still says the slice is pending. |

**Why the preview arm cannot be forgotten.** `create-proposal-preview.tsx` ends its action-type
ternary with the VK block, so a missing arm shows a signer **"New Verification Key"** above a safe
harbour rotation. The same shape exists in `create-proposal-form.tsx`. Neither is a compile error;
both are in commit 6 by name. This is the trap V3 Phase 3 §5 documented.

**Tests.**
- The address → descriptor table: valid P2TR, P2WPKH, wrong network, valid hex, off-curve x-only.
- Codec round-trip both directions, with a tripwire that the variant still encodes
  `UpdateTxType::SafeHarbourAddressUpdate`.
- The signing-message tripwire, run out of the builder and asserted on `lines()`.
- The enactment truth table, with AC 7b and AC 8 as two tests carrying their own names.
- Authorization over one tx-14 action: administrator accepts, council refuses.
- Depth for tx type 14 against tx type 10 — the pair an authority-shaped mapping cannot separate.
- Pure TS: the per-authority menu and its default; the validator over the five address forms; the
  no-op rule with a genuine change as its counter-case.

**Not tested:** the form wiring. There is no DOM runner; the honest substitute is the manual walk.

### Phase 2 — The cancel, the detail and the e2e

Mostly verification plus the test upstream does not have.

- Confirm `create_cancel_proposal` and `build_cancel_action_hex` admit the action with no new
  backend code. If the phase discovers otherwise, that discovery is its most valuable output.
- The detail view: current versus proposed destination, both with their descriptor hex. It is the
  analogue of V3's Before/After over a single value instead of a set.
- `e2e-tests/tests/e2e_safe_harbour_address.rs`, a **new file**, following `e2e_council_rotation.rs`:
  - **Enacted path** — submit an administrator-signed rotation, assert it is queued with the
    bridge's address unchanged, mine exactly `depth`, assert the address changed, the
    administrator's `last_seqno` advanced, and `is_activated()` is unchanged.
  - **Cancelled path** — cancel inside the window, mine `depth`, assert the queue is empty and the
    address is still unchanged.
  - **Swallowed path** — fire a Defcon 1 first, then submit the rotation: accepted, seqno advanced,
    queue drained, **address unchanged**. This is the only automated proof of Constraint 1 against a
    real chain, and it exists nowhere, upstream included.

  The fixtures need no work: every exhaustive `ConfirmationDepths` literal in the repository already
  carries `safe_harbour_address_update`.

**Anti-flake:** reuse the existing `bitcoind`-availability skip and mine an exact depth. Never sleep.

### Phase 3 — What the manual walk exposed

V1 needed two phases nobody planned plus four close-out PRs; V2 and V3 budgeted one each. Budgeted
one here, expecting copy — how "safe harbour" reads to someone who has not read the PRD — and how the
destination is displayed. Both halves were right, and one of them was not cosmetic: the dashboard
never read the harbour for the authority that rotates it, so a rotation the bridge swallowed told its
signer another action had taken its sequence number and to build a replacement that would be
swallowed too. See the [phase spec](./security-council-safe-harbour-address-phase-3.md).

## 5. Verification

Per phase, the `AGENTS.md` checklist plus `npm run test:unit`, which is absent from that checklist
and present in CI:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd desktop-app && npm run format:check && npm run lint && npm run build && npm run test:unit
```

**Every commit that adds a frontend test file confirms CI picks it up.** CI globs
`src/**/*.test.ts(x)` rather than enumerating scripts — V1 shipped a phase where 21 of 62 test
scripts never ran, twice — so the check is that the new file falls inside the pattern.

End to end, once both phases land, the manual walk in the contract's `## Verification`.

## 6. Known debt this slice does not take

- **Acceptance is not application**, at the bridge this time. Recorded as Constraint 1 and covered by
  the post-condition rather than repaired; there is nothing to repair, since the behaviour is
  upstream's and deliberate.
- **Two rotations to the same destination** satisfy each other's post-conditions. The same ambiguity
  every config-carrying action has; the seqno term bounds it.
- **An `activation_height` that fails to compute is never retried**, and a stored one can go stale if
  the deployment changes the depth while an update is queued. Recorded in V2's plan §6 and unchanged.
- **N+1 `strata_asm_getStatus` reads in the reconciliation loop**, recorded at V1 close-out and
  revisited but not fixed in V2 Phase 3. Unchanged.
- **`Authority::PayoutAdmin` still maps to no ASM role.** Expected, not a gap — see
  [`security-council.md` §5.5](./security-council.md#55-two-prd-items-have-no-upstream-counterpart-at-any-revision--both-resolved).

## 7. Close-out

Four places do not update themselves, and V1, V2 and V3 each needed a follow-up PR for exactly this
drift: the `Status:` header of the contract, the `Status:` header of this document, and in
[`security-council.md`](./security-council.md) the Stage board (§6) and the Slice board (§7). With
V4 closed, Stage 6 — the compliance audit and issue #117 — is the only remaining work on the
feature.
