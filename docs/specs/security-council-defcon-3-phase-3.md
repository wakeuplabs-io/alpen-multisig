# Security Council — Defcon 3 (V2), Phase 3: Cancelability travels on the proposal

**Functional contract:** [`security-council-defcon-3.md`](./security-council-defcon-3.md) — SSOT for
*what* V2 must do. This document never overrides it.

**Build plan:** [`security-council-defcon-3-implementation.md`](./security-council-defcon-3-implementation.md)
§4 Phase 3. This document is that phase at implementation detail.

**Closes:** [AC 13](./security-council-defcon-3.md#13-cancelability-travels-on-the-proposal), and
[Constraint 4](./security-council-defcon-3.md#4-cancelability-is-answered-by-the-backend-for-every-authority).

## 1. The change in one sentence

Whether a proposal offers a cancel affordance stops being guessed by the desktop from an authority
allow-list and becomes a boolean on every proposal response, derived from the same confirmation-depth
gate `create_cancel_proposal` already uses.

## 2. What this phase is not

It is not enactment detection (Phase 4), the Defcon 3 create flow (Phase 5), the queued lifecycle
pin (Phase 6), or the cancel e2e (Phase 7). It is not a refactor of the reconciliation loop's
N+1 `strata_asm_getStatus` reads — this phase adds one depth read per list/detail request and
records the broader fix as debt.

It does not change when cancel is *allowed* on chain — `create_cancel_proposal` is unchanged. It only
makes the read path honest so the desktop and the write gate cannot drift.

## 3. Why the backend must land before the desktop

Serde ignores unknown JSON fields on deserialize; Zod does not. The orchestrator must emit
`is_cancelable` before the desktop schema requires `isCancelable` — the inverse of Phase 1's
emitter-before-acceptor rule for `actionType`.

## 4. Design decisions

### 4.1 Response DTO, not persisted domain

[`Proposal`](../../orchestrator-be/src/domain/proposal.rs) is stored in Postgres and must not carry
computed live metadata. The wire shape is:

```rust
#[derive(Serialize)]
pub struct ProposalResponse {
    #[serde(flatten)]
    pub proposal: Proposal,
    pub is_cancelable: bool,
}
```

`GET /proposals` returns `Vec<ProposalResponse>`. `GET /proposals/:id` flattens the same fields via
`ProposalDetailResponse` plus `cancel_proposal`. **Every handler that returns a proposal** enriches
through the same resolver so Tauri always sees the field.

### 4.2 Unknown collapses to `false`

No `Option<bool>`. When the ASM cannot answer, `is_cancelable = false` — the honest failure is a
missing cancel button, not a button that cannot work. Same spirit as
[`live_last_seqno`](../../orchestrator-be/src/application/proposals.rs): a read never fails because
cleanup could not run.

### 4.3 Derivation = `depth_for_action > 0`

```rust
pub(crate) fn is_cancelable_for_action(
    action: &MultisigAction,
    depth_of: impl Fn(UpdateTxType) -> Option<u16>,
) -> bool {
    depth_for_action(action, depth_of) > 0
}
```

This is exactly the gate in `create_cancel_proposal` (`target_depth == 0` → refused). A
`MultisigAction::Cancel` resolves to depth `0` by construction and is never cancelable.

### 4.4 One ASM read per request

`ConfirmationDepthResolver::fetch(rpc_url)` does one `strata_asm_getStatus` per `list_proposals` or
`get_proposal` (and per write response that enriches). Each proposal then decodes `action_hex` and
calls `is_cancelable_for_action` with the cached table — no per-row RPC.

Mock URLs (`mock://asm-membership`) use the same `uniform_confirmation_depths` fixture as
`mock_lock_period`.

### 4.5 Wire format

| Layer | Field |
|---|---|
| orchestrator-be JSON | `is_cancelable` |
| Tauri domain `Proposal` | `is_cancelable` |
| Tauri `ProposalDto` | `is_cancelable` → JSON `isCancelable` |
| Zod `proposalSchema` | `isCancelable: z.boolean()` |

## 5. Frontend contract

[`derive-proposal-actions.ts`](../../desktop-app/src/domain/proposal-detail/model/derive-proposal-actions.ts)
deletes `CANCELABLE_AUTHORITIES`. `canCancelProposal` reads `proposal.isCancelable` only. Status,
terminal state, and `cancelProposal === null` gates stay at call sites — the backend field answers
only "does this action type have a non-zero confirmation depth right now".

## 6. Tests

| # | Layer | Claim |
|---|---|---|
| 1 | `asm_role_membership` | Defcon 1 → not cancelable |
| 2 | `asm_role_membership` | Defcon 3 with depth 7 → cancelable |
| 3 | `asm_role_membership` | Cancel action → not cancelable |
| 4 | `asm_role_membership` | Multisig update with depth 11 → cancelable |
| 5 | `ipc-schemas.test.ts` | `proposalSchema` requires `isCancelable` |
| 6 | `derive-proposal-actions.test.ts` | `isCancelable: true` → `canCancel: true` |
| 7 | `derive-proposal-actions.test.ts` | `security_council` + `defcon_3` + `isCancelable: true` — authority does not decide |
| 8 | `derive-proposal-actions.test.ts` | `defcon_1` + `isCancelable: false` — replaces authority-based AC 10 proxy |

**Not tested:** HTTP round-trip, ASM integration inside `orchestrator-be`, DOM/components.

## 7. Blast radius

- **Sequencer Manager** proposals gain a visible cancel affordance. The backend permitted this since
  V1 Phase 2; only the desktop hid it. Correct behaviour, not a regression.
- **Security Council + Defcon 1** stays without cancel (`depth = 0`).
- **Security Council + Defcon 3** (once creatable in Phase 5) gains cancel when `depth > 0` with no
  further UI gate change.
- **ASM down:** listing succeeds; no cancel affordance.

No product-visible Defcon 3 change until Phase 5 — nothing in this app can create one yet.

## 8. Verification

```bash
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
cd desktop-app && npm run format:check && npm run lint && npm run build && npm run test:unit
git grep -n "CANCELABLE_AUTHORITIES" -- desktop-app/   # must be empty
```

No manual walk required — see §7.
