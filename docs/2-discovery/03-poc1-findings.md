# POC 1 Findings — Admin / Subprotocol Integration

## Overview

This document captures findings from POC 1: understanding the Alpen Administration Subprotocol — its topology, how the layers connect, and how an admin action flows through the system end-to-end.

### Sources

- **Admin crate source code** — all files at [`crates/asm/subprotocols/admin/src/`](https://github.com/alpenlabs/alpen/tree/main/crates/asm/subprotocols/admin/src): `authority.rs`, `handler.rs`, `queued_update.rs`, `state.rs`, `subprotocol.rs`, `error.rs`
- **Supporting crates** — `asm/params` (Role, AdministrationInitConfig), `asm/txs/admin` (MultisigAction, SignedPayload, Sighash), `asm/common` (Subprotocol trait, MsgRelayer)
- **PRD documents** — [`docs/0-prd/01-multisig-ui.md`](../0-prd/01-multisig-ui.md) (includes full text of SPS-50, SPS-51, and SPS-65 transaction processing), [`docs/0-prd/02-multisig-backend.md`](../0-prd/02-multisig-backend.md)
- **Conceptual overview** — [`docs/2-discovery/01-conceptual-overview.md`](01-conceptual-overview.md)

---

## 1. System Topology — What Lives Where?

The system has three layers that communicate through well-defined interfaces.

### Layer 1: Bitcoin (BTC) — The Settlement Layer

Bitcoin is the foundation. It does **not** run smart contracts or understand Alpen/Strata logic. Its role in the admin flow:

- **Stores admin transactions.** When signers approve a governance change, they construct a standard Bitcoin transaction and broadcast it. It gets mined like any normal BTC payment.
- **Provides finality and ordering.** Once a transaction has enough confirmations, it is irreversible.
- **Carries protocol data** via two mechanisms defined in SPS-50 and SPS-51:
  - **OP_RETURN output (SPS-50):** A tag in output position 0 that identifies the transaction as a protocol transaction. Format: `OP_RETURN <magic(4 bytes) | subprotocol_id(1 byte) | tx_type(1 byte) | aux(≤74 bytes)>`. The admin subprotocol has ID `0`. Transaction types are enumerated in `AdminTxType` (Cancel=0, StrataAdminMultisigUpdate=10, StrataSeqManagerMultisigUpdate=11, OperatorUpdate=20, SequencerUpdate=21, OlStfVkUpdate=30, AsmStfVkUpdate=31).
  - **Witness envelope (SPS-51):** The actual payload (the `SignedPayload`) is Borsh-serialized and embedded in the Taproot witness using an `OP_FALSE OP_IF <chunk_0> ... <chunk_n> OP_ENDIF` envelope. Payloads larger than 520 bytes are chunked into 520-byte segments due to Bitcoin's stack element size limit. Total payload must be under 395 KB (standardness limit). The envelope is placed inside a spend script that starts with `<pubkey> CHECKSIG`, so the input signature transitively signs the envelope contents.

> **Analogy:** Bitcoin is like a public bulletin board. You pin a notice (transaction) on it, and once it has enough thumbtacks (confirmations), nobody can remove it. The notice has a colored tag (OP_RETURN) so the system knows which department it belongs to, and the detailed instructions are in an envelope pinned behind it (witness data).

### Layer 2: The Anchor State Machine (ASM) — The Governance Brain

The ASM is the critical middle layer. It is a **deterministic state machine** that reads Bitcoin blocks, finds admin transactions, validates them, and updates governance state.

Key facts:
- **Runs inside the Strata node.** It is not a separate chain or process. Every Strata full node runs the ASM as part of block processing.
- **Single source of truth.** The ASM state says who the current signers are, what updates are pending, and what the sequence numbers are. No other component is authoritative.
- **Implemented as a "subprotocol."** The ASM framework supports multiple subprotocols (admin, bridge, checkpoint). Each has its own state, transaction types, and can send messages to each other via `MsgRelayer`.

The admin subprotocol state ([`AdministrationSubprotoState`](https://github.com/alpenlabs/alpen/blob/main/crates/asm/subprotocols/admin/src/state.rs)) contains:

| Field | Type | Description |
|-------|------|-------------|
| `authorities` | `Vec<MultisigAuthority>` | One per Role (currently 2: StrataAdministrator, StrataSequencerManager) |
| `queued` | `Vec<QueuedUpdate>` | Pending updates waiting for activation height |
| `next_update_id` | `u32` | Monotonically increasing counter for identifying queued updates |
| `confirmation_depth` | `u16` | Default 2016 blocks (~2 weeks) |
| `max_seqno_gap` | `NonZero<u8>` | Default 10 — maximum allowed gap between last_seqno and new seqno |

Each [`MultisigAuthority`](https://github.com/alpenlabs/alpen/blob/main/crates/asm/subprotocols/admin/src/authority.rs) tracks:

| Field | Type | Description |
|-------|------|-------------|
| `role` | `Role` | Which authority this is (StrataAdministrator or StrataSequencerManager) |
| `config` | `ThresholdConfig` | Public keys + threshold (e.g., 2-of-3) |
| `last_seqno` | `u64` | Last successfully executed sequence number |

### Layer 3: Alpen (EVM Rollup) — The Execution Layer

For the admin subprotocol, Alpen is mostly a **consumer** of governance changes, not a producer. When the ASM enacts a change (e.g., new verifying key or sequencer update), it relays messages to other subprotocols:

- `UpdateSequencerKey(Buf32)` → checkpoint subprotocol (sequencer changed)
- `UpdateCheckpointPredicate(PredicateKey)` → checkpoint subprotocol (rollup verifying key changed)
- Operator set changes → bridge subprotocol (add/remove operator)

### How the Layers Connect

```
SIGNERS (humans with hardware wallets)
    |
    | 1. Construct + sign admin transaction (offline/desktop app)
    v
BITCOIN (L1)
    |
    | 2. Transaction gets mined into a block
    v
STRATA NODE (runs ASM)
    |
    | 3. ASM scans Bitcoin blocks, finds admin txs by OP_RETURN tag
    | 4. Parses witness envelope to extract SignedPayload
    | 5. Validates signatures against current authority config
    | 6. Queues or executes the update
    | 7. Relays messages to other subprotocols if needed
    v
ALPEN (EVM rollup) — receives relayed config changes
```

**Key insight:** There is no direct RPC call from the desktop app to the ASM. The communication is **indirect through Bitcoin**. The app constructs a Bitcoin transaction, broadcasts it, and the ASM picks it up when it processes that Bitcoin block.

---

## 2. Admin Action Flow — End-to-End Lifecycle

### Concrete Example: Updating the Strata Administrator Signer Set

#### Phase 1: Proposal Creation (Off-chain)

1. A signer opens the desktop app, connects their hardware wallet, selects the Strata Administrator multisig.
2. They create a proposal: "Add key X, remove key Y, new threshold = 3."
3. The app computes `ActionId = hash(MultisigAction, SeqNo)` where SeqNo > last confirmed sequence number.
4. The proposal is stored in the **off-chain backend** (Axum + Postgres). The backend is purely a coordination tool — it holds the proposal and collects signatures. Proposals expire after 7 days.

#### Phase 2: Signature Collection (Off-chain)

5. Other signers open the app, see the pending proposal, review details.
6. Each approving signer uses their hardware wallet to sign. The sighash is computed as (SPS-65):

```
sighash = SHA256(SHA256(tag) || seqno_be_bytes || sighash_payload)
```

Where:
- `tag` = `"strata/admin/<type_name>"` (e.g., `"strata/admin/strata_admin_multisig_update"`) — provides **domain separation** so signatures for one action type cannot be replayed against another
- `seqno_be_bytes` = 8-byte big-endian sequence number — prevents **replay attacks**
- `sighash_payload` = action-specific Borsh-serialized bytes

7. Signatures are **ECDSA** (chosen for hardware wallet compatibility per BIP-137).
8. The backend collects signatures. Once the threshold is met (e.g., 2-of-3), the proposal reaches quorum.

#### Phase 3: Transaction Construction and Broadcast (Off-chain → On-chain)

9. A signer constructs a Bitcoin transaction containing:
   - **OP_RETURN** output (position 0) with: `magic | subprotocol_id(0) | tx_type(10 for StrataAdminMultisigUpdate)`
   - **Taproot witness** with the Borsh-serialized `SignedPayload { seqno, action, signatures }` in an SPS-51 envelope
10. The signer sets a fee rate (in sat/vB increments of 0.1) and broadcasts to the Bitcoin network.

#### Phase 4: ASM Processing (On-chain, deterministic)

11. The Strata node processes the next Bitcoin block. The ASM's [`process_txs`](https://github.com/alpenlabs/alpen/blob/main/crates/asm/subprotocols/admin/src/subprotocol.rs) is called.
12. **First**, `handle_pending_updates` runs — checks if any previously queued updates have reached their activation height and enacts them.
13. **Then**, for each transaction tagged for the admin subprotocol, `parse_tx` extracts the `SignedPayload` from the witness envelope.
14. [`handle_action`](https://github.com/alpenlabs/alpen/blob/main/crates/asm/subprotocols/admin/src/handler.rs) processes it:
   - Determines the required role from the action type
   - Gets the `MultisigAuthority` for that role
   - Calls `verify_action_signature` which checks:
     - `payload.seqno > authority.last_seqno` (replay protection)
     - `payload.seqno <= authority.last_seqno + max_seqno_gap` (bounded gap, default 10)
     - Threshold signatures are valid against the current key set (ECDSA recovery)
   - If valid, computes `activation_height = current_bitcoin_height + confirmation_depth`
   - Creates a `QueuedUpdate { id, action, activation_height }` and enqueues it
   - Advances the authority's `last_seqno`

#### Phase 5: Waiting Period (On-chain, ~2016 blocks / ~2 weeks)

15. The update sits in the queue. During this window, it can be **canceled** by submitting a `MultisigAction::Cancel(CancelAction { target_id })` transaction signed by the same authority with a valid seqno.

#### Phase 6: Enactment (On-chain, automatic)

16. When a Bitcoin block arrives at `activation_height`, `handle_pending_updates` fires.
17. Ready updates are partitioned from the queue and executed in order.
18. For a `MultisigUpdate`: updates the `ThresholdConfig` (add/remove keys, change threshold).
19. For a `VerifyingKey` update: relays `UpdateCheckpointPredicate` to the checkpoint subprotocol.
20. For an `OperatorSet` update: relays add/remove operator to the bridge subprotocol.

#### Exception: Sequencer Updates

Sequencer updates are the **one exception** to the queuing pattern. They skip the queue entirely and take effect immediately because sequencer rotation needs to be fast.

### Lifecycle State Diagram

```
                    +-----------+
                    |  PENDING  |  (off-chain, 7-day expiry)
                    |  (backend)|
                    +-----+-----+
                          |  quorum reached + broadcast to BTC
                          v
                    +-----------+
                    |  APPROVED |  (on-chain, in ASM queue)
                    |  (queued) |
                    +-----+-----+
                     /         \
          cancel tx /           \ activation_height reached
                   v             v
             +-----------+  +-----------+
             |  CANCELED |  |  ENACTED  |
             +-----------+  +-----------+

  Exception: Sequencer updates go directly from broadcast → ENACTED (no queue)
```

### Safe Model vs. Strata/Alpen Model

Unlike Ethereum's Safe multisig where proposal N+1 cannot execute until N is executed or canceled:

- A proposal that doesn't reach quorum **may be skipped**.
- A proposal with a higher SeqNo **may** be executed without explicitly rejecting earlier proposals.
- The backend **must not** enforce strict ordering between sequence numbers.
- Strict ordering, if desired, must be coordinated voluntarily by signers.

---

## 3. Integration Points

### 3.1 Crate-Level APIs (Rust)

| Crate | Key Types | Purpose |
|-------|-----------|---------|
| `strata_asm_txs_admin` | `MultisigAction`, `UpdateAction`, `CancelAction`, `SignedPayload`, `SignatureSet`, `Sighash` trait, `AdminTxType` | Constructing the payload for Bitcoin transactions |
| `strata_asm_params` | `Role`, `AdministrationInitConfig`, `ThresholdConfig` | Understanding the authority model and configuration |
| `strata_crypto` | `ThresholdConfig`, `verify_threshold_signatures`, `CompressedPublicKey`, `SignatureSet` | Signature verification and key management |
| `strata_l1_txfmt` | `TagData`, `SubprotocolId` | Constructing the OP_RETURN tag (SPS-50) |
| `strata_l1_envelope_fmt` | `parse_envelope_payload` | Parsing/constructing the witness envelope (SPS-51) |
| `borsh` | serialization | All payloads are Borsh-serialized (not JSON, not protobuf) |

**Transaction construction flow for the app:**

1. Build a `MultisigAction` (Update or Cancel)
2. Compute sighash: `SHA256(SHA256(tag) || seqno_be || borsh(action_payload))`
3. Have each signer sign the sighash with their ECDSA key
4. Pack into `SignedPayload { seqno, action, signatures: SignatureSet }`
5. Borsh-serialize the `SignedPayload`
6. Embed in a Taproot witness using the SPS-51 envelope format (chunk into 520-byte segments if needed)
7. Add an OP_RETURN output at position 0 with SPS-50 header (magic + subprotocol ID 0 + tx type)
8. Broadcast the Bitcoin transaction

### 3.2 RPC Interfaces

| Interface | Purpose | Notes |
|-----------|---------|-------|
| **Bitcoin RPC** | Broadcast transactions, monitor confirmations | Must support local Strata node or trusted remote endpoint on `stratabtc.org` |
| **Strata Node RPC** | Read current ASM state (signer sets, sequence numbers, queued updates) | **Exact RPC methods not yet verified** — this is the primary integration unknown |
| **Off-chain Backend API** | Proposal CRUD, signature aggregation | Built by us (Axum + Postgres). See `MultisigBackend` trait in PRD |

### 3.3 Backend Storage Model

From the PRD, the backend needs three maps per authority:

```rust
// SeqNo -> Vec<ActionId>
actions_by_seqno: Map<SeqNo, Vec<ActionId>>

// ActionId -> MultisigAction
action_by_id: Map<ActionId, MultisigAction>

// ActionId -> Vec<Signature>
sigs_by_id: Map<ActionId, Vec<Signature>>
```

### 3.4 Hardware Wallet Interface

- **HWI** (Hardware Wallet Interface) for Taproot inputs, message signing, on-device display
- Derivation path: `m/86'/0'/73'/0/n` (BIP-86 Taproot, custom account `73'`)
- Signatures must be ECDSA
- The user must be able to clearly read and verify each message on the hardware wallet screen

### 3.5 Authority Roles and Their Actions

| Role | Actions | Queued? |
|------|---------|---------|
| **Strata Administrator** | Strata Admin Signer update, Security Council Signer update, Operator update, Verifying key updates (OL STF, ASM STF), Safe Harbor address update, Bridge updates (soft/hard) | Yes (2016 blocks) |
| **Strata Sequencer Manager** | Sequencer Manager Signer update, Sequencer update | Signer update: Yes. Sequencer update: **No** (immediate) |
| **Alpen Administrator** | Alpen verification key update, Alpen Admin Signer update | Yes (2016 blocks) |
| **Security Council** | Defcon 1 transaction, Defcon 3 transaction | TBD |
| **Payout Administrator** | `block_payout` transactions | Different flow (UTXO spending, not ASM updates) |

---

## 4. Feasibility — Blockers and Unknowns

### Confirmed Feasible (Low Risk)

1. **The admin crate API is well-structured and usable.** Types are clean, well-documented, and have comprehensive tests. `Sighash` trait, `SignedPayload`, `MultisigAction` are ready for integration.
2. **The lifecycle model is clear.** Pending → Approved/Queued → Enacted/Canceled is well-defined in both the spec pseudocode and Rust implementation.
3. **Borsh serialization is deterministic.** Important for reproducible sighash computation across signers.
4. **The `SeqNoToken` pattern prevents misuse.** The Rust type system ensures you cannot advance the sequence number without verifying signatures first.
5. **SPS-50 and SPS-51 are fully documented** in the PRD with clear format specifications.

### Known Gaps / Unknowns (Medium Risk)

1. **Strata Node RPC for reading ASM state.** We could not find or verify the RPC methods that expose the current admin subprotocol state. The app needs to read: current signer sets per role, current sequence numbers, queued updates, confirmation depth. **This is the biggest integration unknown.**
2. **Roles not yet in the codebase.** The `Role` enum currently only has `StrataAdministrator` and `StrataSequencerManager`. The PRD lists 5 authorities (+ Alpen Admin, Security Council, Payout Admin). The Security Council and Payout Admin may live in separate subprotocols or may not be implemented yet. Needs clarification from the Alpen team.
3. **Alpen Administrator role.** The conceptual overview mentions this as a separate authority on the Alpen protocol (not Strata). It likely uses the same crate/logic but with a different initialization config. Needs confirmation.
4. **Transaction construction tooling.** Building the actual Bitcoin transaction (Taproot witness, OP_RETURN, fee estimation, signing) requires bitcoin transaction building libraries. The Alpen codebase uses the `bitcoin` crate (rust-bitcoin). The `chunked_envelope` module in `crates/btcio/src/writer/` handles envelope construction.
5. **Missing subprotocol features (TODOs in code).** The handler has TODO items for `OperatorSet` relay to Bridge (STR-1721) and ASM verifying key logging. These are ASM-internal and should not block the multisig app.

### Potential Blockers (Low Probability, High Impact)

6. **Hardware wallet compatibility with the custom sighash format.** The sighash is `SHA256(SHA256(tag) || seqno || payload)`. Hardware wallets typically sign Bitcoin transactions or BIP-322 messages. Whether this custom sighash can be presented to a hardware wallet for signing (and displayed meaningfully on the device screen) needs validation in POC 2/3. The PRD explicitly requires on-device readability.

---

## 5. Crate & API Usage Per Step

### Flow Diagram with Crate Annotations

```
 ┌─────────────────────────────────────────────────────────────┐
 │  Step 1: READ ASM STATE                                     │
 │  crate: strata-asm-subprotocols-admin                       │
 │  types: AdministrationSubprotoState, MultisigAuthority       │
 └──────────────────────────┬──────────────────────────────────┘
                            v
 ┌─────────────────────────────────────────────────────────────┐
 │  Step 2: BUILD MultisigAction                               │
 │  crate: strata-asm-txs-admin, strata-asm-params, strata-crypto │
 │  types: MultisigAction, UpdateAction, CancelAction, Role     │
 └──────────────────────────┬──────────────────────────────────┘
                            v
 ┌─────────────────────────────────────────────────────────────┐
 │  Step 3: COMPUTE SIGHASH                                    │
 │  crate: strata-asm-txs-admin                                │
 │  trait: Sighash::compute_sighash(seqno) -> Buf32            │
 └──────────────────────────┬──────────────────────────────────┘
                            v
 ┌─────────────────────────────────────────────────────────────┐
 │  Step 4: COLLECT ECDSA SIGNATURES (hardware wallets)        │
 │  crate: secp256k1                                           │
 │  types: strata-crypto -> IndexedSignature (65 bytes)        │
 └──────────────────────────┬──────────────────────────────────┘
                            v
 ┌─────────────────────────────────────────────────────────────┐
 │  Step 5: PACK INTO SignedPayload                            │
 │  crate: strata-asm-txs-admin, strata-crypto                 │
 │  types: SignedPayload, SignatureSet                          │
 └──────────────────────────┬──────────────────────────────────┘
                            v
 ┌─────────────────────────────────────────────────────────────┐
 │  Step 6: BORSH-SERIALIZE                                    │
 │  crate: borsh                                               │
 │  call:  borsh::to_vec(&signed_payload)                      │
 └──────────────────────────┬──────────────────────────────────┘
                            v
 ┌─────────────────────────────────────────────────────────────┐
 │  Step 7: BUILD SPS-51 WITNESS ENVELOPE                      │
 │  crate: strata-l1-envelope-fmt (from strata-common repo)    │
 │  types: EnvelopeScriptBuilder                               │
 └──────────────────────────┬──────────────────────────────────┘
                            v
 ┌─────────────────────────────────────────────────────────────┐
 │  Step 8: BUILD SPS-50 OP_RETURN TAG                         │
 │  crate: strata-l1-txfmt (from strata-common repo)           │
 │  types: TagData, ParseConfig, MagicBytes                    │
 └──────────────────────────┬──────────────────────────────────┘
                            v
 ┌─────────────────────────────────────────────────────────────┐
 │  Step 9: CONSTRUCT BITCOIN TX (commit + reveal)             │
 │  crate: strata-btcio (writer/builder)                       │
 │  types: EnvelopeConfig, create_envelope_transactions        │
 └──────────────────────────┬──────────────────────────────────┘
                            v
 ┌─────────────────────────────────────────────────────────────┐
 │  Step 10: BROADCAST TO BITCOIN                              │
 │  crate: bitcoind-async-client                               │
 │  calls: sign_raw_transaction_with_wallet, send_raw_tx       │
 └──────────────────────────┬──────────────────────────────────┘
                            v
 ┌─────────────────────────────────────────────────────────────┐
 │  Step 11: ASM PICKS UP TX (on-chain, automatic)             │
 │  crate: strata-asm-subprotocols-admin                       │
 │  funcs: parse_tx -> handle_action -> enqueue/cancel/apply   │
 └─────────────────────────────────────────────────────────────┘
```

### Step-by-Step API Details

#### Step 1 — Read ASM State

```rust
// crate: strata-asm-subprotocols-admin
use strata_asm_subprotocols_admin::state::AdministrationSubprotoState;
use strata_asm_subprotocols_admin::authority::MultisigAuthority;
use strata_asm_params::Role;

let auth = state.authority(Role::StrataAdministrator).unwrap();
let last_seqno = auth.last_seqno();
let config: &ThresholdConfig = auth.config();  // keys + threshold
let queued = state.queued();                    // pending updates
```

> **Note:** The RPC method to fetch this state from a Strata node is not yet verified — this is the primary integration unknown.

#### Step 2 — Build MultisigAction

```rust
// crate: strata-asm-txs-admin, strata-asm-params, strata-crypto
use strata_asm_txs_admin::actions::{MultisigAction, UpdateAction};
use strata_asm_txs_admin::actions::updates::MultisigUpdate;
use strata_crypto::ThresholdConfigUpdate;
use strata_asm_params::Role;

// Example: update signer set
let config_update = ThresholdConfigUpdate::new(
    add_keys,                       // Vec<CompressedPublicKey>
    remove_keys,                    // Vec<CompressedPublicKey>
    NonZero::new(2).unwrap(),       // new threshold
);
let update = MultisigUpdate::new(config_update, Role::StrataAdministrator);
let action = MultisigAction::Update(UpdateAction::Multisig(update));

// Example: cancel a queued update
let action = MultisigAction::Cancel(CancelAction::new(target_update_id));
```

#### Step 3 — Compute Sighash

```rust
// crate: strata-asm-txs-admin (Sighash trait)
use strata_asm_txs_admin::actions::Sighash;

let seqno: u64 = last_seqno + 1;
let sighash: Buf32 = action.compute_sighash(seqno);
// formula: SHA256(SHA256("strata/admin/<tx_type>") || seqno_be || action_payload)
```

Tag strings per action type:
- `"strata/admin/strata_admin_multisig_update"`
- `"strata/admin/strata_seq_manager_multisig_update"`
- `"strata/admin/operator_update"`
- `"strata/admin/sequencer_update"`
- `"strata/admin/ol_stf_vk_update"`
- `"strata/admin/asm_stf_vk_update"`
- `"strata/admin/cancel"`

#### Step 4 — Collect ECDSA Signatures

```rust
// crate: secp256k1, strata-crypto
use secp256k1::{Secp256k1, Message, SECP256K1};
use strata_crypto::IndexedSignature;

let msg = Message::from_digest_slice(&sighash.0).unwrap();
let sig = SECP256K1.sign_ecdsa_recoverable(&msg, &secret_key);
let (recid, compact) = sig.serialize_compact();

// 65 bytes: recovery_id(1) || r(32) || s(32)
let mut bytes = [0u8; 65];
bytes[0] = recid.to_i32() as u8;       // raw 0-3 or BIP-137 27-42 both accepted
bytes[1..].copy_from_slice(&compact);

let indexed = IndexedSignature::new(my_signer_index, bytes);
```

#### Step 5 — Pack into SignedPayload

```rust
// crate: strata-asm-txs-admin, strata-crypto
use strata_asm_txs_admin::parser::SignedPayload;
use strata_crypto::SignatureSet;

let sig_set = SignatureSet::new(vec![sig0, sig1]).unwrap();
let payload = SignedPayload::new(seqno, action, sig_set);
```

#### Step 6 — Borsh-Serialize

```rust
// crate: borsh (SignedPayload derives BorshSerialize)
let envelope_payload: Vec<u8> = borsh::to_vec(&payload).unwrap();
```

#### Step 7 — Build SPS-51 Witness Envelope

```rust
// crate: strata-l1-envelope-fmt (external, from strata-common repo)
use strata_l1_envelope_fmt::EnvelopeScriptBuilder;

let reveal_script = EnvelopeScriptBuilder::with_pubkey(&xonly_pk.serialize())?
    .add_envelope(&envelope_payload)?   // auto-chunks at 520 bytes
    .build()?;

// Use as taproot leaf script
let spend_info = TaprootBuilder::new()
    .add_leaf(0, reveal_script.clone())?
    .finalize(SECP256K1, pubkey)?;
```

#### Step 8 — Build SPS-50 OP_RETURN Tag

```rust
// crate: strata-l1-txfmt (external, from strata-common repo)
use strata_l1_txfmt::{TagData, ParseConfig};

let tag: TagData = action.tag();  // subproto_id=0, tx_type=AdminTxType as u8, aux=[]
let op_return_script = ParseConfig::new(magic_bytes)
    .encode_script_buf(&tag.as_ref())?;
```

#### Step 9 — Construct Bitcoin TX

```rust
// crate: strata-btcio (writer/builder)
use strata_btcio::writer::builder::{EnvelopeConfig, create_envelope_transactions};

let config = EnvelopeConfig::new(magic, addr, network, fee_rate, 546);
let (commit_tx, reveal_tx) = create_envelope_transactions(&config, &envelope_payload, utxos)?;
// reveal_tx has: output[0] = OP_RETURN tag, witness = signature + reveal_script + control_block
```

> **Note:** This uses a commit-reveal pattern. The commit tx funds a Taproot output. The reveal tx spends it via script-path, exposing the envelope in the witness.

#### Step 10 — Broadcast

```rust
// crate: bitcoind-async-client
// Commit tx needs wallet signing (spends wallet UTXOs)
let signed_commit = wallet.sign_raw_transaction_with_wallet(&commit_tx).await?;
wallet.send_raw_transaction(&signed_commit).await?;

// Reveal tx is pre-signed (taproot script-path spend)
wallet.send_raw_transaction(&reveal_tx).await?;
```

#### Step 11 — ASM Processing (Automatic, On-chain)

This step happens inside the Strata node — we don't call it, but it's useful to understand:

```rust
// crate: strata-asm-subprotocols-admin
// 1. Parse tx from Bitcoin block
let payload = parse_tx(&tx_input)?;

// 2. Process action (verify sigs, check seqno, enqueue or cancel)
handle_action(&mut state, payload, current_height, &mut relayer)?;

// 3. At each block, enact ready updates
handle_pending_updates(&mut state, &mut relayer, block_height);
```

### Key External Dependencies

| Crate | Source | Purpose |
|-------|--------|---------|
| `strata-l1-txfmt` | [strata-common](https://github.com/alpenlabs/strata-common) repo | SPS-50 OP_RETURN construction |
| `strata-l1-envelope-fmt` | [strata-common](https://github.com/alpenlabs/strata-common) repo | SPS-51 witness envelope construction |
| `strata-btcio` | [alpen](https://github.com/alpenlabs/alpen) repo | Bitcoin transaction building (commit+reveal) |
| `bitcoind-async-client` | external | Bitcoin RPC client |
| `borsh` | external | Deterministic serialization |
| `secp256k1` / `bitcoin` | external | ECDSA signing, Bitcoin primitives |

### Implementation Notes

1. **Signature format:** 65-byte recoverable ECDSA. Header byte accepts both raw recovery ID (0-3) and BIP-137 format (27-42). Verification normalizes automatically.
2. **Seqno rules:** Must be strictly greater than `last_seqno` and within `max_seqno_gap` (default 10). First valid seqno is 1 (initial `last_seqno` is 0).
3. **TagData has no aux data** for admin txs: `TagData::new(0, tx_type_u8, vec![])`.
4. **Sequencer updates skip the queue:** Applied immediately in `handle_action`. All other update types are queued with `activation_height = current_height + confirmation_depth`.
5. **strata-l1-txfmt and strata-l1-envelope-fmt** come from the [strata-common](https://github.com/alpenlabs/strata-common) repo (tag `v0.1.0-alpha-rc11`), not from the main alpen repo.
6. **Test utilities** in `crates/asm/txs/test-utils/src/lib.rs` contain `create_reveal_transaction_stub` which shows the full SPS-50 + SPS-51 assembly as a reference implementation.

---

## Key Takeaways

1. **The architecture is indirect:** the desktop app never talks to the ASM directly. It constructs Bitcoin transactions, broadcasts them, and the ASM picks them up from Bitcoin blocks.
2. **The off-chain backend is purely for coordination** — proposal storage and signature collection. The protocol does not depend on it. Signers can always construct and broadcast transactions manually.
3. **The primary unknowns are:** (a) Strata node RPC methods for reading ASM state, (b) whether the Role enum will be extended to cover all 5 PRD authorities, and (c) hardware wallet compatibility with the custom sighash format.
4. **The crate APIs are ready for integration.** The types, serialization, and validation logic are clean and well-tested.
