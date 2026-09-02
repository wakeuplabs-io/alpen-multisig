# Security Council — Defcon 3 (V2), Phase 4: Enactment detection

**Functional contract:** [`security-council-defcon-3.md`](./security-council-defcon-3.md) — SSOT for
*what* V2 must do. This document never overrides it.

**Build plan:** [`security-council-defcon-3-implementation.md`](./security-council-defcon-3-implementation.md)
§4 Phase 4. This document is that phase at implementation detail.

**Closes:** [AC 6](./security-council-defcon-3.md#6-a-broadcast-defcon-3-is-queued-not-enacted),
[AC 8](./security-council-defcon-3.md#8-it-enacts-at-exactly-its-depth), and the in-band half of
[AC 12](./security-council-defcon-3.md#12-a-cancelled-defcon-3-never-activates-the-harbour);
[Constraints 2](./security-council-defcon-3.md#2-defcon-3-enactment-cannot-reuse-defcon-1s-seqno-equality)
and [3](./security-council-defcon-3.md#3-a-cancelled-defcon-3-must-never-be-reported-as-enacted).

## 1. The change in one sentence

The `Defcon3` arm of `is_proposal_enacted_on_asm` stops returning `BadRequest` and becomes a pure
predicate over four observations — seqno consumed, gone from the queue, tip past the stored
activation height, harbour active — shaped like `defcon1_enacted` so its truth table is testable
without an ASM.

## 2. What this phase is not

It is not the create flow (Phase 5), the lifecycle UI pin (Phase 6), or the cancel e2e (Phase 7).
It is not a repair path for null `activation_height` (debt §6). It does not change the Defcon 1
predicate. No Tauri, no desktop, no new e2e.

## 3. Spec traceability audit

| Document | What Phase 4 takes from it |
|---|---|
| [`security-council-defcon-3.md`](./security-council-defcon-3.md) § Enactment detection | Four terms and their sources |
| [`security-council-defcon-phase-4.md`](./security-council-defcon-phase-4.md) (V1) | Pattern: free function beside dispatch arm; mock stays URL-keyed |
| [`proposal-lifecycle-seqno-truth.md`](./proposal-lifecycle-seqno-truth.md) §4 | Enactment before supersede; Defcon 1 uses `==`, Defcon 3 uses `>=` |
| [`security-council-defcon-3-phase-3.md`](./security-council-defcon-3-phase-3.md) | Cancelability done — not revisited |
| [`security-council-defcon-3-implementation.md`](./security-council-defcon-3-implementation.md) §6 | Null/stale `activation_height` recorded as debt, not patched here |

### Defcon 1 vs Defcon 3 — the seqno term must not be confused

| | Defcon 1 | Defcon 3 |
|---|---|---|
| Seqno term | `last_seqno == seq_no` | `last_seqno >= seq_no` |
| Height term | none (depth 0) | `bitcoin_tip >= activation_height` |
| Queue term | no Defcon 1 queued (tripwire) | this action not in queue |
| Why different | Never queued; equality pins *this* proposal | Accepted at reveal; later actions jump seqno; maturity needs height |

Using `==` on Defcon 3 marks a successfully enacted proposal as `Superseded`
([Constraint 2](./security-council-defcon-3.md#2-defcon-3-enactment-cannot-reuse-defcon-1s-seqno-equality)).
Omitting the height term marks a cancelled proposal as `Enacted` when the harbour was already on
([Constraint 3](./security-council-defcon-3.md#3-a-cancelled-defcon-3-must-never-be-reported-as-enacted)).

## 4. Function contract

### 4.1 Pure predicate

```rust
fn defcon3_enacted(
    last_seqno: u64,
    seq_no: u64,
    still_queued: bool,
    safe_harbour_activated: bool,
    bitcoin_tip: u64,
    activation_height: u64,
) -> bool {
    last_seqno >= seq_no
        && !still_queued
        && safe_harbour_activated
        && bitcoin_tip >= activation_height
}
```

### 4.2 Dispatch arm

Replace `asm_enactment.rs:136-138`. Read council `last_seqno`, bridge harbour flag, and whether
**this** `UpdateAction::Defcon3` is still in `admin.queued()` — match the decoded action, not any
Defcon 3 entry (contract edge case: byte-identical payloads).

### 4.3 Extended signature

V1 left `is_proposal_enacted_on_asm` unchanged because Defcon 1 needs no stored height. Defcon 3
does:

```rust
pub(crate) struct EnactmentObservations {
    pub activation_height: Option<u64>,
    pub bitcoin_tip: Option<u64>,
}
```

| Observation | Source |
|---|---|
| `activation_height` | `Proposal.activation_height` from [`compute_and_store_activation_height`](../../orchestrator-be/src/application/proposals.rs) |
| `bitcoin_tip` | new `BitcoinRpcClient::get_chain_tip()` via `getblockcount` |

### 4.4 Degradation

| Missing observation | Result |
|---|---|
| `activation_height == None` | `Ok(false)` — retry when reveal facts persist |
| `bitcoin_tip == None` | `Ok(false)` — retry next reconcile poll |
| ASM decode error | `Err(BadRequest)` — unchanged |

### 4.5 Call sites

Both in [`proposals.rs`](../../orchestrator-be/src/application/proposals.rs): `reconcile_one` and
`report_broadcast_progress`. Shared helper `enactment_observations(proposal, btc_client)`.

### 4.6 Known limit (recorded, not solved)

Cancel broadcast entirely outside this app, tip past activation height, harbour already active — no
observable ASM state distinguishes cancelled from enacted. In-band cancels write `Canceled` and
terminal proposals are not re-evaluated. Phase 7 e2e pins the in-band path.

## 5. Tests

Eight unit tests on `defcon3_enacted` in `asm_enactment.rs`, including named rows for Constraints 2
and 3. **Not tested:** HTTP round-trip, ASM integration inside `orchestrator-be` — `run_defcon3` in
`e2e_defcon_probe.rs` proves chain behaviour.

## 6. Blast radius

- **`orchestrator-be` only** — no desktop or Tauri diff.
- **`bitcoin_rpc.rs`** — +1 trait method.
- **`mock_is_enacted`** — unchanged; action-blind URL mock per V1 precedent.
- **Defcon 1 arm** — untouched.

No product-visible change until Phase 5 creates Defcon 3 proposals.

## 7. Verification

```bash
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
git grep -n "Defcon3 enactment detection is not implemented" orchestrator-be/   # empty
git diff --stat -- desktop-app/ desktop-app/src-tauri/   # empty
```
