# Broadcast Implementation Audit — 2026-05-09

> **Resolution (2026-06):** Historical audit. `orchestrator-be/.../broadcast_tx.rs` was removed; desktop executes commit/reveal; orchestrator records metadata via `claim_broadcast` / PATCH only. See [ADR-006](../../architecture/adrs/006-backend-coordination-boundary.md) and [`proposal-broadcast-commit-reveal.md`](../../specs/proposal-broadcast-commit-reveal.md). Findings below are not current architecture.

**Scope**: Broadcast commit/reveal implementation and post-broadcast signer update flow.  
**Files reviewed**:
- `orchestrator-be/src/infrastructure/broadcast_tx.rs`
- `orchestrator-be/src/application/proposals.rs`
- `orchestrator-be/src/handlers/proposals.rs`
- `orchestrator-be/src/infrastructure/asm_role_membership.rs`
- `orchestrator-be/src/infrastructure/bitcoin_rpc.rs`
- `orchestrator-be/src/infrastructure/postgres_repo.rs`
- `orchestrator-be/src/domain/proposal.rs`
- `desktop-app/src-tauri/src/infrastructure/broadcast_tx.rs`
- `desktop-app/src-tauri/src/application/proposals.rs`
- `desktop-app/src-tauri/src/commands/proposals.rs`

**Reference**: PRD `docs/0-prd/01-multisig-ui.md`, `docs/0-prd/02-multisig-backend.md`, SPS-50, SPS-51, SPS-65.

**Overall assessment**: The SPS-50/51 commit/reveal cryptographic structure is correctly implemented (P2TR taproot, tapscript envelope, OP_RETURN SPS-50 tag, `SignedPayload` SSZ encoding). Issues are in fee management, orchestration state consistency, and authority access control.

---

## CRITICAL

### C1 — Hardcoded 2000-sat fee in orchestrator-be `build_reveal_tx`

**File**: `orchestrator-be/src/infrastructure/broadcast_tx.rs:208`

```rust
let fee = Amount::from_sat(2000);
```

The commit output is funded with `COMMIT_DUST_SATS + fee_rate * REVEAL_TX_VBYTES` (computed dynamically), but the reveal transaction always deducts exactly 2000 sats as fee regardless of the current fee rate.

**Failure scenario**: At `fee_rate = 10 sat/vb`, the commit receives 5000 sats, but the reveal pays only 2000 sats for ~350 vbytes (~5.7 sat/vb). The reveal tx will not confirm under moderate mempool congestion. The commit UTXO is permanently stranded.

**Divergence with Tauri**: `desktop-app/src-tauri/src/infrastructure/broadcast_tx.rs:173` accepts `fee_sats: u64` as a parameter and uses it correctly. The two implementations diverge here.

**Fix**: Add `fee_sats: u64` parameter to `build_reveal_tx` in orchestrator-be (matching the Tauri signature) and pass `fee_rate * REVEAL_TX_VBYTES` from `do_broadcast`.

---

## HIGH

### H1 — Race condition: broadcast idempotency guard is not atomic

**File**: `orchestrator-be/src/application/proposals.rs:253`

```rust
if proposal.broadcast_status != BroadcastStatus::Idle {
    return Err(...)
}
```

The SELECT that loads the proposal and the subsequent guard check are not inside a transaction with `SELECT FOR UPDATE`. Two concurrent requests can both pass the check. Additionally, `update_broadcast_status` in Postgres does not include `broadcast_status = 'idle'` as a WHERE precondition — it always overwrites.

**Failure scenario**: Two concurrent `execute_broadcast` calls fund two separate commit UTXOs. The second call overwrites `commit_txid` in the repo. One UTXO is permanently stranded.

**Fix**: Use an optimistic update as the locking mechanism:
```sql
UPDATE proposals
SET broadcast_status = 'commit_broadcasted'
WHERE action_id = $1 AND broadcast_status = 'idle'
RETURNING *
```
Return `Conflict` if zero rows are updated.

---

### H2 — Tauri broadcast does not update orchestrator proposal state

**File**: `desktop-app/src-tauri/src/application/proposals.rs:109`

`broadcast_commit_then_reveal` in the Tauri application layer executes the full commit/reveal sequence but never calls any orchestrator endpoint to update the proposal status. The command response hardcodes the strings:

```rust
// desktop-app/src-tauri/src/commands/proposals.rs:297
proposal_status: "enacted".to_string(),
broadcast_status: "reveal_confirmed".to_string(),
```

These only exist in the DTO returned to the frontend — they are never persisted.

**Consequence**: The orchestrator keeps the proposal as `Approved` with `broadcast_status = Idle`. A second signer can call `execute_broadcast` on the orchestrator, pass the Idle guard, fund a second commit, and broadcast a second reveal. The ASM will reject the second reveal (seqno already consumed), but the commit and reveal fees are lost.

**Fix**: Either (a) add a `PATCH /proposals/{id}/broadcast-status` endpoint on the orchestrator and call it from Tauri after success, or (b) enforce that `execute_broadcast` on the orchestrator is the only broadcast path.

---

### H3 — `list_proposals`, `get_proposal`, and broadcast handlers lack authority scoping

**File**: `orchestrator-be/src/handlers/proposals.rs:92, 102, 140, 164`

All four handlers accept `_auth: AuthenticatedSession` (note the underscore — unused) and do not filter by authority:

```rust
pub async fn list_proposals(..., _auth: AuthenticatedSession, ...) {
    let proposals = proposals::list_proposals(state.repo.as_ref(), query.status).await?;
    // no authority filter
}
```

**PRD §3.2–3.4**: "A signer of one multisig authority MUST be treated as a non-signer with respect to all other multisig authorities." / "A non-signer MUST NOT be able to view any pending proposals."

**Contrast**: `approve_action` in the application layer does check authority (`proposal.authority != session.authority → Unauthorized`). Read and broadcast handlers do not.

**Fix**: Pass `auth.authority` into `list_proposals` and `get_update_action`; add `AND authority = $N` to the SQL queries. For broadcast handlers, verify `proposal.authority == auth.authority` before proceeding.

---

### H4 — Three of five authorities cannot broadcast (unmapped to ASM Role)

**File**: `orchestrator-be/src/infrastructure/asm_role_membership.rs:83`

```rust
fn authority_to_role(authority: Authority) -> Result<Role, String> {
    match authority {
        Authority::StrataAdmin => Ok(Role::StrataAdministrator),
        Authority::SequencerManager => Ok(Role::StrataSequencerManager),
        _ => Err(format!("authority `{authority:?}` is not mapped to ASM role authorization yet")),
    }
}
```

`AlpenAdmin`, `SecurityCouncil`, and `PayoutAdmin` return `Err`. `ordered_keys_for_authority` (called in `broadcast_commit_then_reveal`) propagates this as `AppError::BadRequest`. Broadcast is completely broken for these three authorities.

**PRD §7**: All five multisigs must be supported.

---

## MEDIUM

### M1 — 65-byte signature path not verified against expected pubkey

**File**: `orchestrator-be/src/infrastructure/broadcast_tx.rs:108` (same in Tauri equivalent)

The 64-byte path tries all four recovery IDs and verifies the recovered pubkey matches `sig.signer_pubkey`. The 65-byte path does not:

```rust
65 => {
    let mut buf = [0u8; 65];
    let recid_byte = sig_bytes[64];  // taken at face value, no bounds check
    buf[0] = recid_byte;
    buf[1..65].copy_from_slice(&sig_bytes[..64]);
    buf  // never verified against signer_pubkey
}
```

A mnemonic wallet producing a 65-byte signature with an incorrect `recid` byte will be accepted by the orchestrator, built into a `SignedPayload`, and broadcast. The ASM will reject the tx silently. The operator loses commit + reveal fees with no actionable error message.

**Fix**: After rearranging the bytes, attempt `ecdsa_recover` and verify the result matches `sig.signer_pubkey`, same as the 64-byte path.

---

### M2 — Signer index cast to `u8` without bounds check

**File**: `orchestrator-be/src/infrastructure/broadcast_tx.rs:57` (same in Tauri equivalent)

```rust
.position(|k| k.eq_ignore_ascii_case(&sig.signer_pubkey))
.ok_or_else(|| { ... })? as u8;
```

If the signer set ever exceeds 255 keys, the index silently truncates. While unlikely given protocol limits, this should be a checked cast.

**Fix**: `u8::try_from(pos).map_err(|_| AppError::BadRequest(format!("signer index {} exceeds u8 range", pos)))?`

---

### M3 — `Enacted` in orchestrator does not match PRD "Enacted" semantics

**File**: `orchestrator-be/src/domain/proposal.rs:65`

The orchestrator transitions a proposal to `Enacted` when the reveal tx is confirmed. But SPS-65 queues non-sequencer updates for ~2016 blocks before applying them. The signer set is not updated at reveal confirmation — it updates after the queue executes.

**PRD §12**: "Approved" = "confirmed onchain, but has not yet been enacted." This maps to what the orchestrator calls `Enacted`. The missing state is post-enactment.

**Impact**: The frontend showing `Enacted` may mislead signers into thinking the update (e.g., a new signer) is already active, when it won't be for ~2 weeks on mainnet.

---

## Post-broadcast signer update: will it work?

**Yes — with a ~2016-block delay (~2 weeks on mainnet) for non-sequencer updates.**

The flow:

1. Reveal tx confirmed → orchestrator marks proposal `Enacted`.
2. ASM STF processes the tx: `handle_update_action` → `queue_update(state, action, current_height)`, `authority.last_seqno = payload.seqno`.
3. After ~2016 blocks: `handle_pending_updates` calls `apply_multisig_update` → modifies `authority.config.keys`.
4. `ordered_keys_for_authority` and `is_signer_member_for_authority` are fetch-on-demand (no caching). The next request after step 3 will return the updated signer set automatically.

**The backend is correctly designed for this**: no key set caching, always queries live ASM state.

**Risks that break this**:
- ASM RPC unavailable at request time → `ordered_keys_for_authority` fails with `BadRequest`, no fallback (acceptable per PRD §2 manual fallback requirement).
- Proposal is `AlpenAdmin`, `SecurityCouncil`, or `PayoutAdmin` → broadcast fails before reaching this point (H4).
- Signatures built with incorrect signer index (M1) → ASM rejects the reveal tx silently, update never queued.

---

## Priority order

| ID | Severity | Description |
|----|----------|-------------|
| C1 | Critical | Hardcoded 2000-sat reveal fee in orchestrator-be — tx won't confirm in congested mempool |
| H1 | High | Race condition on broadcast idempotency guard |
| H2 | High | Tauri broadcast does not notify orchestrator → stale state → double-broadcast risk |
| H3 | High | `list_proposals`, `get_proposal`, broadcast handlers have no authority scoping (PRD violation) |
| H4 | High | AlpenAdmin / SecurityCouncil / PayoutAdmin cannot broadcast (unmapped Role) |
| M1 | Medium | 65-byte signature path accepted without pubkey verification |
| M2 | Medium | Signer index truncated to `u8` without bounds check |
| M3 | Medium | `Enacted` status semantics diverge from PRD (~2016-block enactment delay not reflected) |

---

## Resolution status (2026-06)

| ID | Status | Notes |
|----|--------|-------|
| C1 | **Obsolete** | Orchestrator no longer builds reveal txs (`broadcast_tx` module removed) |
| H1 | **Partially addressed** | In-flight guards exist on desktop path; see [`proposal-broadcast-commit-reveal.md`](../../specs/proposal-broadcast-commit-reveal.md) |
| H2 | **Resolved** | Desktop broadcast + coordinator `claim_broadcast` / PATCH ([ADR-006](../../architecture/adrs/006-backend-coordination-boundary.md)) |
| H3 | **Partially addressed** | Authority scoping improved (Wave 1 P-002); remaining gaps in [`deferred-backlog.md`](../deferred-backlog.md) |
| H4 | **Deferred** | AlpenAdmin / SecurityCouncil / PayoutAdmin — upstream crate gaps |
| M1–M3 | **Open / tracked** | See [`deferred-backlog.md`](../deferred-backlog.md) and lifecycle specs |
