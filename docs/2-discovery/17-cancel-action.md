# Cancel Action — Behavior and Implementation

## How Cancel Works

A **cancel** removes a queued update from the ASM before it activates. It uses the same commit-reveal Bitcoin transaction pattern as any other governance action — the only difference is the `MultisigAction` variant it carries.

### Lifecycle context

When an update proposal reaches quorum and its approval tx is confirmed on Bitcoin, the ASM queues the update with an `activation_height = confirmation_block + 2016`. During that ~2-week window, any signer of the same authority can propose a cancel. If the cancel tx is confirmed before `activation_height`, the ASM removes the update from the queue and it never takes effect.

```
PENDING ──(quorum + confirmed)──► APPROVED (queued, waiting 2016 blocks)
                                        │
                              ┌─────────┴──────────┐
                              ▼                    ▼
                           CANCELED             ENACTED
                    (cancel tx confirmed     (activation height
                     before activation)       reached, no cancel)
```

Enforcement is **protocol-level**, not Bitcoin script-level. The ASM derives state by replaying Bitcoin blocks; it simply stops recognizing a cancel tx that arrives after the target's activation height has already been processed.

### Cancel as a signed payload

```
SignedPayload {
    seqno:      u64,                          // new seqno, > authority.last_seqno
    action:     MultisigAction::Cancel(
                    CancelAction { target_id: UpdateId }
                ),
    signatures: SignatureSet,                 // quorum of ECDSA sigs from authority
}
```

The sighash uses a **distinct domain tag**: `strata/admin/cancel` — never shared with any update tag. The required signers are derived from the target update's authority, not hardcoded.

### Validation rules

- `target_id` must exist in the queue (reject if already activated, canceled, or never existed).
- Signature set must meet the threshold of the authority that owns the target update.
- Standard seqno rules apply: `seqno > last_seqno` and within `max_seqno_gap`.
- On success: advance `authority.last_seqno`. Do **not** increment `next_update_id`.

---

## Implementation

Cancel is not yet implemented in the ASM admin subprotocol crates. The sections below describe the changes required.

### Files to change

| File | Change |
|---|---|
| `crates/txs/admin/src/actions/mod.rs` | Add `Cancel(CancelAction)` variant to `MultisigAction` |
| `crates/txs/admin/src/actions/cancel.rs` | New file — `CancelAction { target_id: UpdateId }` struct |
| `crates/txs/admin/src/actions/sighash.rs` | `Sighash` impl for `CancelAction`: type = `AdminTxType::Cancel`, payload = `target_id.to_be_bytes()` |
| `crates/txs/admin/src/constants.rs` | Add `AdminTxType::Cancel` discriminant and `strata/admin/cancel` tag hash constant |
| `crates/txs/admin/src/parser.rs` | Verify `SignedPayload` decode handles the new enum variant (likely no change if decode is enum-driven) |
| `crates/subprotocols/admin/src/handler.rs` | Route `Cancel` in action dispatch; derive role from target; remove queued entry |
| `crates/subprotocols/admin/src/state.rs` | Add method to remove a queued update by `target_id` |
| `crates/subprotocols/admin/src/subprotocol.rs` | Confirm block processing order: pending activations first, then incoming txs |

### Step-by-step

**1. Add `CancelAction` struct**

```rust
// crates/txs/admin/src/actions/cancel.rs
pub struct CancelAction {
    target_id: UpdateId,
}

impl CancelAction {
    pub fn new(target_id: UpdateId) -> Self { Self { target_id } }
    pub fn target_id(&self) -> &UpdateId { &self.target_id }
}
```

**2. Add `Cancel` to `MultisigAction`**

```rust
pub enum MultisigAction {
    Update(UpdateAction),
    Cancel(CancelAction),   // new
}
```

**3. Add cancel sighash domain**

```rust
// crates/txs/admin/src/constants.rs
// AdminTxType::Cancel discriminant + tag hash for "strata/admin/cancel"
```

```rust
// Sighash impl
fn tx_type(&self) -> AdminTxType { AdminTxType::Cancel }
fn sighash_payload(&self) -> Vec<u8> { self.target_id.to_be_bytes().to_vec() }
```

**4. Handle cancel in the admin runner**

```rust
// handler.rs — action dispatch
match action {
    MultisigAction::Update(u) => {
        let role = u.required_role();
        verify_sigs(role, &sigs, state)?;
        enqueue_or_apply(u, state)?;
    }
    MultisigAction::Cancel(c) => {
        let target = state.queued_update(c.target_id())
            .ok_or(AdminError::UnknownAction)?;
        let role = target.action.required_role();
        verify_sigs(role, &sigs, state)?;
        state.remove_queued(c.target_id());
    }
}
state.advance_seqno(authority, seqno);
```

**5. Block processing order** — confirm this order is preserved in `subprotocol.rs`:

```
1. process_pending_activations(current_height)   // enact matured updates first
2. process_incoming_txs(block_txs)               // then handle new actions
```

This ensures a cancel arriving in the same block as its target's activation height is correctly rejected.

### Test matrix

| Test | Type |
|---|---|
| Cancel removes target from queue | Unit |
| Cancel nonexistent target → `UnknownAction` | Unit |
| Duplicate cancel → `UnknownAction` | Unit |
| Cancel increments `last_seqno`, not `next_update_id` | Unit |
| Role for cancel derived from target (not hardcoded) | Unit |
| Queue update → cancel → verify queue empty | Integration |
| Cancel before activation prevents enactment | Integration |
| Cancel after activation has no effect | Integration |
| Cancel tag hash = `SHA256("strata/admin/cancel")` | Crypto |
| Cancel tag distinct from all update tags | Crypto |
| `AdminTxType` roundtrip includes `Cancel` | Codec |

### Pitfalls

- Do not reuse any update sighash domain tag for cancel.
- Derive cancel role from the target action — do not hardcode.
- Do not increment `next_update_id` on cancel.
- Do not reorder block processing (txs before activations) — it breaks cancel timing semantics.
