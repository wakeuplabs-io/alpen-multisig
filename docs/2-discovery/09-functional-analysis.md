# Functional Analysis — Alpen Multisig Application

> **Status:** In progress — for validation and review

## 1. System Overview

The Alpen Multisig App is a **desktop application** that enables authorized signers to manage governance multisigs for the Strata and Alpen protocols. It coordinates the creation, signing, and broadcasting of administrative update transactions to the Bitcoin network.

```
┌─────────────────────────────────────────────────────────────────────┐
│                        BITCOIN NETWORK                              │
│  (Settlement layer — stores admin transactions, provides finality)  │
└──────────────────────┬──────────────────────────────────────────────┘
                       │ reads blocks / broadcasts tx
                       ▼
┌──────────────────────────────────────────────────────────────────────┐
│                    STRATA NODE (ASM)                                  │
│  Anchor State Machine — reads Bitcoin, validates admin txs,          │
│  maintains canonical signer sets and governance state                 │
│  (Source of truth for who can sign what)                              │
└──────────────────────┬───────────────────────────────────────────────┘
                       │ RPC (read signer sets, state)
                       ▼
┌──────────────────────────────────────────────────────────────────────┐
│                  ORCHESTRATOR BACKEND                                 │
│  Offchain coordination — proposals, signatures, lifecycle tracking   │
│  (NOT authoritative — just coordination)                             │
└──────────────────────┬───────────────────────────────────────────────┘
                       │ HTTP API
                       ▼
┌──────────────────────────────────────────────────────────────────────┐
│                  DESKTOP APPLICATION (Tauri)                          │
│  ┌──────────────┐  ┌──────────────────────────┐                      │
│  │  React UI    │◄─┤  Tauri Rust Backend      │                      │
│  │  (WebView)   │  │  - Signing (SPS-65)      │                      │
│  │              │──►│  - HWI (hardware wallet) │                      │
│  │              │  │  - HTTP to orchestrator   │                      │
│  └──────────────┘  └──────────────────────────┘                      │
└──────────────────────────────────────────────────────────────────────┘
                       │
                       ▼
              ┌─────────────────┐
              │ HARDWARE WALLET │
              │ (Ledger, etc.)  │
              └─────────────────┘
```

---

## 2. Entities

### 2.1 Authorities (Multisig Groups)

An **authority** is a group of signers that governs a specific aspect of the Strata/Alpen protocol. Each authority has its own signer set, threshold, and sequence number.

| # | Authority | What it governs | Signer set source |
|---|---|---|---|
| 1 | **Alpen Administrator** | Alpen protocol parameters (VK, signers) | Alpen consensus protocol |
| 2 | **Strata Administrator** | Strata protocol parameters (VK, signers, operators, bridge, safe harbor) | Strata ASM state |
| 3 | **Strata Sequencer Manager** | Sequencer configuration (signers, sequencer pubkey) | Strata ASM state |
| 4 | **Security Council** | Emergency actions (Defcon 1/3) | Strata ASM state |
| 5 | **Payout Administrator** | Bridge payout spending (`block_payout`) | Bridge multisig script |

```
                    ┌──────────────────────┐
                    │   AUTHORITIES        │
                    └──────────┬───────────┘
         ┌──────────┬──────────┼──────────┬──────────┐
         ▼          ▼          ▼          ▼          ▼
   ┌──────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐
   │  Alpen   │ │ Strata │ │  Seq   │ │Security│ │ Payout │
   │  Admin   │ │ Admin  │ │Manager │ │Council │ │ Admin  │
   └──────────┘ └────────┘ └────────┘ └────────┘ └────────┘
    2 updates   7 updates  2 updates  2 updates  block_payout
                                                  (different
                                                   protocol)
```

### 2.2 Signers

A **signer** is a person who holds a private key listed in an authority's canonical signer set. Properties:

- Identified by a **compressed secp256k1 public key** (33 bytes)
- Key derived from hardware wallet at path `m/86'/0'/73'/0/n` (first 20 addresses)
- A signer can be on **multiple** authorities but interacts with one at a time
- A signer of one authority is a **non-signer** with respect to all others (strict isolation)

### 2.3 Proposals (Updates)

A **proposal** is a governance action that requires a threshold of signatures to execute. Properties:

- Identified by `ActionId = hash(MultisigAction, SeqNo)`
- `SeqNo` is a `u64` — strictly greater than the last executed seqno, can skip values
- Proposals are **authority-scoped** — only visible to signers of that authority
- Duplicate `(MultisigAction, SeqNo)` are rejected

### 2.4 Update Types

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         MultisigAction                                   │
├─────────────────────────────┬───────────────────────────────────────────┤
│  Update(UpdateAction)       │  Cancel(CancelAction)                     │
├─────────────────────────────┤  - target_id: u32 (update to cancel)      │
│                             │  - consumes a seqno                       │
│  ┌────────────────────────┐ └───────────────────────────────────────────┤
│  │ Multisig(MultisigUpdate)│                                            │
│  │ - role: Role            │  Signer set changes:                       │
│  │ - add_keys, remove_keys │  add/remove keys, change threshold         │
│  │ - new_threshold         │                                            │
│  ├────────────────────────┤                                             │
│  │ OperatorSet             │  Bridge operator add/remove                 │
│  │ (OperatorSetUpdate)     │                                            │
│  ├────────────────────────┤                                             │
│  │ Sequencer               │  Change sequencer public key               │
│  │ (SequencerUpdate)       │  (executes immediately, not queued)        │
│  ├────────────────────────┤                                             │
│  │ VerifyingKey            │  Update proof verification keys             │
│  │ (PredicateUpdate)       │  (OL STF or ASM STF)                       │
│  └────────────────────────┘                                             │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.5 Proposal Lifecycle States

```
                    propose
                       │
                       ▼
                  ┌──────────┐
                  │ PENDING  │ ◄── offchain, visible only to signers
                  │          │     has expiry (7 days)
                  └────┬─────┘
                       │
          ┌────────────┼────────────┐
          ▼            ▼            ▼
    ┌──────────┐ ┌──────────┐ ┌──────────┐
    │ APPROVED │ │ EXPIRED  │ │(skipped) │
    │          │ │          │ │ never    │
    │ on-chain │ │ 7 days   │ │ reached  │
    │ confirmed│ │ no quorum│ │ quorum   │
    └────┬─────┘ └──────────┘ └──────────┘
         │
    ┌────┼──────────┐
    ▼    ▼          ▼
┌────────┐ ┌────────────┐
│ENACTED │ │ CANCELED   │
│        │ │            │
│ after  │ │ cancel tx  │
│ 2016   │ │ before     │
│ blocks │ │ activation │
└────────┘ └────────────┘
```

**Key behaviors:**
- **Pending → Approved**: Quorum of signatures reached + confirmed on Bitcoin
- **Pending → Expired**: 7 days without quorum (offchain only)
- **Approved → Enacted**: Activation height reached (current + 2016 blocks ≈ 2 weeks)
- **Approved → Canceled**: Cancel transaction confirmed before activation
- **SequencerUpdate**: Exception — executes immediately, never queued
- **SeqNo skipping**: Proposals can be abandoned without explicit cancellation; a higher seqno can execute without resolving earlier ones

---

## 3. Update Types by Authority

### 3.1 Alpen Administrator

| Update Type | Description | Execution |
|---|---|---|
| Alpen verification key update | Change the proof verification key for Alpen protocol | Queued (2016 blocks) |
| Alpen Administrator Signer update | Add/remove signers or change threshold on Alpen Admin multisig | Queued (2016 blocks) |

### 3.2 Strata Administrator

| Update Type | Description | Execution |
|---|---|---|
| Strata Administrator Signer update | Add/remove signers or change threshold on Strata Admin multisig | Queued (2016 blocks) |
| Security Council Signer update | Add/remove signers or change threshold on Security Council multisig | Queued (2016 blocks) |
| Strata verification key update | Change the OL STF proof verification key | Queued (2016 blocks) |
| Operator update | Add/remove bridge operators | Queued (2016 blocks) |
| "Soft" bridge update | TBD — semantics unclear, not defined in Alpen crates | Queued (2016 blocks) |
| "Hard" bridge update | TBD — semantics unclear, not defined in Alpen crates | Queued (2016 blocks) |
| Safe Harbor address update | TBD — not defined in Alpen crates | Queued (2016 blocks) |

### 3.3 Strata Sequencer Manager

| Update Type | Description | Execution |
|---|---|---|
| Strata Seq Manager Signer update | Add/remove signers or change threshold on Seq Manager multisig | Queued (2016 blocks) |
| Sequencer update | Change the sequencer public key | **Immediate** (no queue) |

### 3.4 Security Council

| Update Type | Description | Execution |
|---|---|---|
| Defcon 1 transaction | Emergency action — TBD, not defined in Alpen crates | TBD |
| Defcon 3 transaction | Emergency action — TBD, not defined in Alpen crates | TBD |

### 3.5 Payout Administrator

| Update Type | Description | Execution |
|---|---|---|
| `block_payout` transaction | Spend bridge payout UTXOs — fundamentally different from admin subprotocol | Bitcoin native spend (not SPS-50/65) |

---

## 4. User Flows

### 4.1 Connection & Authentication Flow

```
┌─────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  START  │────►│ Connect HW   │────►│ Select       │────►│ Select       │
│         │     │ Wallet       │     │ Address      │     │ Multisig     │
└─────────┘     │              │     │ (m/86'/0'/   │     │ Authority    │
                │ HWI detect   │     │  73'/0/n)    │     │              │
                └──────────────┘     └──────────────┘     └──────┬───────┘
                                                                  │
                                                                  ▼
                                                         ┌──────────────┐
                                                         │ Sign Nonce   │
                                                         │ (auth)       │
                                                         │              │
                                                         │ HW wallet    │
                                                         │ signs nonce  │
                                                         └──────┬───────┘
                                                                │
                                          ┌─────────────────────┼──────────────┐
                                          ▼                     ▼              ▼
                                    Valid signer?          Invalid sig?    Not a signer?
                                          │                     │              │
                                          ▼                     ▼              ▼
                                   ┌──────────────┐      ┌──────────┐   ┌──────────┐
                                   │ MULTISIG     │      │ Error:   │   │ Error:   │
                                   │ DASHBOARD    │      │ invalid  │   │ not a    │
                                   │              │      │ signature│   │ signer   │
                                   └──────────────┘      └──────────┘   └──────────┘
```

**Authentication model (ephemeral session keys):**
1. Client generates ephemeral keypair
2. Signer signs structured message with admin key attesting to ephemeral pubkey
3. Message binds to: specific authority + nonce + expiry
4. Backend verifies against canonical signer set from ASM state
5. All subsequent requests signed with ephemeral private key

### 4.2 Proposal Creation Flow (Admin Updates)

```
Signer                    Desktop App                  Backend              Bitcoin
  │                           │                           │                    │
  │  1. Select update type    │                           │                    │
  │  2. Fill parameters       │                           │                    │
  │  (keys, threshold, etc.)  │                           │                    │
  │──────────────────────────►│                           │                    │
  │                           │                           │                    │
  │                           │  3. Build MultisigAction  │                    │
  │                           │  4. Assign SeqNo          │                    │
  │                           │  5. Compute sighash       │                    │
  │                           │     (SPS-65 tagged hash)  │                    │
  │                           │                           │                    │
  │  6. Review on HW screen   │                           │                    │
  │◄──────────────────────────│                           │                    │
  │                           │                           │                    │
  │  7. Sign on HW wallet     │                           │                    │
  │──────────────────────────►│                           │                    │
  │                           │                           │                    │
  │                           │  8. Create proposal       │                    │
  │                           │     (action + sig)        │                    │
  │                           │──────────────────────────►│                    │
  │                           │                           │                    │
  │                           │  9. ActionId returned     │                    │
  │                           │◄──────────────────────────│                    │
  │                           │                           │                    │
  │  Proposal is now PENDING  │                           │                    │
  │                           │                           │                    │
```

### 4.3 Signature Collection & Approval Flow

```
Other Signers              Desktop App                  Backend              Bitcoin
  │                           │                           │                    │
  │  1. View pending proposals│                           │                    │
  │◄──────────────────────────│◄──────────────────────────│                    │
  │     (shows collected/     │                           │                    │
  │      required sigs)       │                           │                    │
  │                           │                           │                    │
  │  2. Select proposal       │                           │                    │
  │  3. Review details        │                           │                    │
  │──────────────────────────►│                           │                    │
  │                           │                           │                    │
  │  4. Sign on HW wallet     │  5. Compute same sighash  │                    │
  │──────────────────────────►│──────────────────────────►│                    │
  │                           │     Submit signature      │                    │
  │                           │                           │                    │
  │                           │  6. Quorum reached?       │                    │
  │                           │◄──────────────────────────│                    │
  │                           │                           │                    │
  │                     ┌─────┴─────────────┐             │                    │
  │                     │ YES: Build Bitcoin │             │                    │
  │                     │ transaction        │             │                    │
  │                     │ (SPS-50 tag +      │             │                    │
  │                     │  SPS-51 envelope)  │             │                    │
  │                     └─────┬─────────────┘             │                    │
  │                           │                           │                    │
  │  7. Set fee rate          │                           │                    │
  │──────────────────────────►│                           │                    │
  │                           │                           │                    │
  │                           │  8. Broadcast reveal tx   │                    │
  │                           │───────────────────────────┼───────────────────►│
  │                           │                           │                    │
  │                           │                           │    9. Confirmed    │
  │                           │                           │◄───────────────────│
  │                           │                           │                    │
  │  Proposal is now APPROVED │  (queued for 2016 blocks) │                    │
  │                           │                           │                    │
```

### 4.4 Cancellation Flow

```
Signer                    Desktop App                  Backend              Bitcoin
  │                           │                           │                    │
  │  1. View APPROVED updates │                           │                    │
  │◄──────────────────────────│◄──────────────────────────│                    │
  │     (shows cancel sigs)   │                           │                    │
  │                           │                           │                    │
  │  2. Click "Cancel" on     │                           │                    │
  │     an approved update    │                           │                    │
  │──────────────────────────►│                           │                    │
  │                           │                           │                    │
  │                           │  3. Build CancelAction    │                    │
  │                           │     (target_id = update   │                    │
  │                           │      to cancel)           │                    │
  │                           │  4. Assign new SeqNo      │                    │
  │                           │  5. Compute cancel sighash│                    │
  │                           │                           │                    │
  │  6. Sign on HW wallet     │                           │                    │
  │──────────────────────────►│──────────────────────────►│                    │
  │                           │     Submit cancel sig     │                    │
  │                           │                           │                    │
  │          ... collect threshold of cancel signatures ...│                    │
  │                           │                           │                    │
  │                           │  7. Build + broadcast     │                    │
  │                           │     cancel tx             │                    │
  │                           │───────────────────────────┼───────────────────►│
  │                           │                           │                    │
  │  Update removed from queue│                           │                    │
  │                           │                           │                    │
```

**Note:** Cancellation only applies to Alpen Admin and Strata Admin updates. Seq Manager and Security Council updates do not have an "Approved" state that can be canceled.

### 4.5 Payout Administrator Flow (block_payout)

```
Payout Signer             Desktop App                  Backend              Bitcoin
  │                           │                           │                    │
  │  ┌──────── TWO MODES ────────────┐                    │                    │
  │  │                               │                    │                    │
  │  ▼                               ▼                    │                    │
  │  MANUAL                    AUTOMATIC                  │                    │
  │  User specifies            "Block payouts" button     │                    │
  │  block_payout inputs       auto-selects max UTXOs     │                    │
  │  manually                  within standardness limit  │                    │
  │  │                               │                    │                    │
  │  └───────────┬───────────────────┘                    │                    │
  │              ▼                                        │                    │
  │  1. Construct block_payout tx                         │                    │
  │     (Bitcoin PSBT, NOT SPS-50)                        │                    │
  │──────────────────────────►│                           │                    │
  │                           │  2. Store as PENDING      │                    │
  │                           │──────────────────────────►│                    │
  │                           │                           │                    │
  │  3. Sign with HW wallet   │                           │                    │
  │──────────────────────────►│──────────────────────────►│                    │
  │                           │     Submit spend sig      │                    │
  │                           │                           │                    │
  │          ... collect threshold of spend signatures ... │                    │
  │                           │                           │                    │
  │  4. Set fee rate           │                           │                    │
  │     (0.1 sat/vB increments│                           │                    │
  │      up to 10,000 sat/vB) │                           │                    │
  │──────────────────────────►│                           │                    │
  │                           │  5. Broadcast             │                    │
  │                           │───────────────────────────┼───────────────────►│
  │                           │                           │                    │
```

**Key differences from admin updates:**
- No SPS-50 tag, no SPS-51 envelope, no sighash — this is a standard Bitcoin multisig spend
- Expiry is 7 days (offchain), expired transactions are **deleted** (not kept like admin updates)
- Fee rate is user-specified with 0.1 sat/vB granularity
- Automatic mode uses greedy UTXO selection within standardness limit (< 400 KB tx)

### 4.6 Manual Fallback Flow (Backend Unavailable)

```
Signer A                  Signer B                  Bitcoin
  │                           │                        │
  │  1. Build MultisigAction  │                        │
  │     locally               │                        │
  │  2. Compute sighash       │                        │
  │  3. Sign with HW wallet   │                        │
  │                           │                        │
  │  4. Copy signature        │                        │
  │     to clipboard          │                        │
  │──── (paste/email/chat) ──►│                        │
  │                           │                        │
  │                           │  5. Paste signatures   │
  │                           │  6. Verify threshold   │
  │                           │  7. Build Bitcoin tx   │
  │                           │  8. Broadcast          │
  │                           │───────────────────────►│
  │                           │                        │
```

The application MUST work without the backend. Signers can construct, sign, aggregate, and broadcast manually.

---

## 5. Bitcoin Transaction Structure

Every admin update produces a Bitcoin transaction with this structure:

```
┌─────────────────────────────────────────────────────┐
│                 REVEAL TRANSACTION                    │
├─────────────────────────────────────────────────────┤
│                                                      │
│  INPUT 0:                                            │
│  ┌────────────────────────────────────────────────┐  │
│  │ Witness (Taproot):                             │  │
│  │   <signature>                                  │  │
│  │   <spend_script>:                              │  │
│  │     <pubkey> CHECKSIG                          │  │
│  │     OP_FALSE OP_IF        ◄── SPS-51 envelope  │  │
│  │       <chunk_0>  (≤520 bytes)                  │  │
│  │       <chunk_1>  (≤520 bytes)                  │  │
│  │       ...                                      │  │
│  │       <chunk_n>  (remaining)                   │  │
│  │     OP_ENDIF                                   │  │
│  │                                                │  │
│  │   Envelope contains Borsh-serialized:          │  │
│  │     SignedPayload {                            │  │
│  │       seqno: u64,                              │  │
│  │       action: MultisigAction,                  │  │
│  │       signatures: SignatureSet                 │  │
│  │     }                                          │  │
│  └────────────────────────────────────────────────┘  │
│                                                      │
│  OUTPUT 0 (OP_RETURN):         ◄── SPS-50 header     │
│  ┌────────────────────────────────────────────────┐  │
│  │ OP_RETURN                                      │  │
│  │   magic (4 bytes, e.g. "ALPN")                 │  │
│  │   subprotocol_id (1 byte, 0 = admin)           │  │
│  │   tx_type (1 byte, see AdminTxType)            │  │
│  │   aux (≤74 bytes, type-specific)               │  │
│  └────────────────────────────────────────────────┘  │
│                                                      │
│  OUTPUT 1+: change (if any)                          │
│                                                      │
└─────────────────────────────────────────────────────┘
```

### Sighash Computation (SPS-65)

```
sighash = SHA256(
    SHA256(tag)           ◄── 32 bytes, tag = "strata/admin/<type_name>"
    ║ seqno_be            ◄── 8 bytes, big-endian u64
    ║ sighash_payload     ◄── variable, Borsh-encoded action-specific data
)
```

Each signer signs this 32-byte sighash with ECDSA (recoverable signature). The SignatureSet contains indexed signatures: `(signer_index, signature)`.

---

## 6. Data Model

### 6.1 Core Types

```
ThresholdConfig
├── keys: Vec<CompressedPublicKey>    (33-byte compressed secp256k1)
├── threshold: NonZero<u8>            (minimum signatures required)

MultisigAuthority
├── role: Role                        (which authority)
├── config: ThresholdConfig           (current signer set)
├── last_seqno: u64                   (last executed sequence number)

AdministrationSubprotoState
├── authorities: Vec<MultisigAuthority>
├── queued: Vec<QueuedUpdate>         (pending activation)
├── next_update_id: u32
├── confirmation_depth: u16           (default 2016 blocks)
├── max_seqno_gap: NonZero<u8>        (default 10)
```

### 6.2 Backend Data Model

```
Proposal
├── action_id: ActionId               (= hash(MultisigAction, SeqNo))
├── authority: Role
├── seqno: u64
├── action: MultisigAction            (Borsh-serialized)
├── signatures: Vec<IndexedSignature>
├── status: ProposalStatus
├── created_at: DateTime
├── expires_at: DateTime              (created_at + 7 days)

ProposalStatus
├── Pending                           (collecting signatures)
├── Approved                          (confirmed on-chain, waiting activation)
├── Enacted                           (activation height reached)
├── Canceled                          (cancel tx confirmed before activation)
├── Expired                           (7 days without quorum)
```

---

## 7. Authority × Update Type Matrix

This matrix shows which authority can perform which update, and the execution behavior:

```
                          │ Alpen │ Strata │  Seq   │Security│ Payout │
         Update Type      │ Admin │ Admin  │Manager │Council │ Admin  │
──────────────────────────┼───────┼────────┼────────┼────────┼────────┤
Alpen VK update           │   Q   │        │        │        │        │
Alpen Admin signer update │   Q   │        │        │        │        │
Strata VK update          │       │   Q    │        │        │        │
Strata Admin signer update│       │   Q    │        │        │        │
Security Council signer   │       │   Q    │        │        │        │
Operator update           │       │   Q    │        │        │        │
"Soft" bridge update      │       │   Q    │        │        │        │
"Hard" bridge update      │       │   Q    │        │        │        │
Safe Harbor address update│       │   Q    │        │        │        │
Seq Manager signer update │       │        │   Q    │        │        │
Sequencer update          │       │        │   I    │        │        │
Defcon 1                  │       │        │        │   ?    │        │
Defcon 3                  │       │        │        │   ?    │        │
Cancel                    │   Q   │   Q    │        │        │        │
block_payout              │       │        │        │        │   BTC  │

Q = Queued (2016 blocks confirmation depth)
I = Immediate execution (no queue)
? = Execution model TBD (not defined in crates)
BTC = Bitcoin native spend (not admin subprotocol)
```

---

## 8. Key Constraints and Rules

### 8.1 Protocol Rules (enforced on-chain by ASM)

1. **Threshold**: `valid_signatures >= authority.config.threshold`
2. **SeqNo ordering**: `payload.seqno > authority.last_seqno`
3. **SeqNo gap**: `payload.seqno <= authority.last_seqno + max_seqno_gap` (default 10)
4. **SeqNo skipping**: Proposals with lower seqno can be abandoned — no explicit rejection needed
5. **Confirmation depth**: Queued updates activate after `current_height + 2016` blocks
6. **Cancel window**: Updates can be canceled any time before activation height

### 8.2 Backend Rules (offchain coordination only)

1. **Authority isolation**: Signers only see proposals for their authority
2. **Idempotent proposals**: `ActionId = hash(MultisigAction, SeqNo)` — duplicates rejected
3. **No protocol enforcement**: Backend does hygiene checks only, not canonical validation
4. **Offline survivability**: Backend unavailability must not prevent signing/broadcasting

### 8.3 Signer Safety Rules (UI/UX)

1. **Private keys never leave hardware wallet** — signing happens on device
2. **Explicit review**: Payload displayed on HW screen before signing
3. **Authority labeling**: Every action form shows which authority is being acted on
4. **Copy/paste signatures**: Signers can export/import signatures for manual aggregation
5. **Fee control**: User sets fee rate explicitly (0.1 sat/vB increments)
