# ASM & Bitcoin L1 — State Representation and Transition Model

## Overview

This document explains, at a conceptual level, how the Anchor State Machine (ASM) represents governance state using Bitcoin L1 as its foundation. It covers how state is derived (not stored) from Bitcoin, how governance actions produce state transitions, and how the commit-reveal transaction pattern works.

### Sources

- **POC-1 findings** — [`03-poc1-findings.md`](./03-poc1-findings.md)
- **Conceptual overview** — [`01-conceptual-overview.md`](./01-conceptual-overview.md)
- **Architecture overview** — [`../architecture/overview.md`](../architecture/overview.md)
- **Hardware wallet architecture** — [`06-hardware-wallet-architecture.md`](./06-hardware-wallet-architecture.md)
- **PRD** — [`../0-prd/01-multisig-ui.md`](../0-prd/01-multisig-ui.md) (§6 hardware wallet requirements)
- **Alpen crate source** — `crates/asm/subprotocols/admin/src/` (handler, state, authority, subprotocol)
- **Envelope builder source** — `strata-common/crates/l1proto/envelope-fmt/src/builder.rs`
- **Tx builder source** — `alpen/crates/btcio/src/writer/builder.rs`

---

## 1. Bitcoin Does Not Store State — It Stores History

This is the most important and counter-intuitive point if coming from Ethereum:

**There is no "state" stored on Bitcoin.** No storage slots, no contracts with variables. Bitcoin is simply an **append-only, ordered log of transactions**.

The governance state (who the current signers are, which updates are queued, what the last sequence number is) **does not exist anywhere on the Bitcoin blockchain**. It exists only as a **derived computation** in the memory of each Strata node.

### How state is "represented"

```
Block 0    Block 1    Block 2    ...    Block N
  [ ]        [tx₁]      [ ]              [tx₂]
              │                            │
              v                            v
State₀ ───> State₁ ────────────────────> State₂
```

Every Strata node, on startup, **replays all Bitcoin blocks from genesis**, scans transactions that carry the correct OP_RETURN magic tag, and applies them sequentially. Because the logic is deterministic and blocks are ordered, **all nodes arrive at the same state**.

State is not "read from Bitcoin" — it is **computed from Bitcoin**.

> **Analogy:** Think of a public ledger. Nobody writes "the current balance is X" in the ledger. Only movements are recorded ("A transferred 5 to B"). The current balance is calculated by reading all movements from the beginning. The ASM does exactly this with governance actions.

---

## 2. How an Action Changes State

A governance action (e.g., "update the signer set") materializes as a **regular Bitcoin transaction**. But it carries two special pieces of embedded data:

```
┌──────────────────────────────────────────────────┐
│  Bitcoin Transaction (reveal)                     │
│                                                   │
│  Output 0: OP_RETURN   ← "I am an admin tx,      │
│            [tag]          type MultisigUpdate"     │
│                           (label for filtering)   │
│                                                   │
│  Input 0:  Witness     ← actual payload:          │
│            [envelope]     {seqno, action, sigs}    │
│                           (the real data)          │
└──────────────────────────────────────────────────┘
```

When the Strata node processes the block containing this transaction:

1. Sees the OP_RETURN → "this tx belongs to the admin subprotocol"
2. Opens the witness → extracts the `SignedPayload`
3. Verifies signatures against the **current in-memory state**
4. If valid → **mutates its in-memory state** (queues update, cancels, or applies immediately)

**The Bitcoin transaction itself does not "contain" new state.** It contains an **instruction** that, when processed against the current state, produces a new state.

---

## 3. The Commit-Reveal Pattern

### Why two transactions?

To embed arbitrary data in a Taproot witness, you need to spend an output that is committed to a specific script. You cannot attach an arbitrary witness to a normal wallet spend — the witness must correspond to the script of the output being spent. The **commit** creates that output; the **reveal** spends it (and exposes the data).

### Transaction structure

```
  COMMIT TX                            REVEAL TX
┌──────────────────┐                ┌──────────────────────┐
│                  │                │                      │
│ Input:           │    funds       │ Input:               │
│  Wallet UTXO(s)  │ ────────────>  │  Commit tx output    │
│                  │                │  (Taproot spend)     │
│ Output:          │                │                      │
│  Taproot address │                │ Output 0:            │
│  (derived from   │                │  OP_RETURN [tag]     │
│   reveal script) │                │                      │
│                  │                │ Witness:             │
└──────────────────┘                │  [Schnorr signature] │
                                    │  [reveal script with │
                                    │   envelope payload]  │
                                    │  [control block]     │
                                    └──────────────────────┘
```

**Commit tx:** "I place funds at a Taproot address that can only be spent by revealing the script containing the governance data." This is a normal Bitcoin transaction spending wallet UTXOs. The Taproot address is mathematically derived from the script that contains the payload.

**Reveal tx:** "I spend that Taproot output via script-path, and in doing so, expose the data in the witness." The witness is permanently recorded in the block, and that is where the ASM reads it.

### Taproot script tree

The tree is minimal — a single leaf at depth 0 containing the reveal script:

```
Taproot output
  └── Leaf 0: <pubkey> CHECKSIG OP_FALSE OP_IF <chunks...> OP_ENDIF
```

The spend is always **script-path** (not key-path). The internal key is ephemeral and has no separate spending path.

### Who generates each transaction?

**Both transactions are generated by the same actor** — the desktop app of the signer who broadcasts after quorum is reached:

1. Takes the `SignedPayload` (with all governance signatures already collected off-chain)
2. Builds the reveal script (envelope with Borsh-serialized payload)
3. Derives the Taproot address from the reveal script
4. Constructs the commit tx that funds that address
5. Constructs the reveal tx that spends it
6. Signs the commit tx with their Bitcoin wallet (to spend their UTXOs)
7. Broadcasts commit → waits for confirmation → broadcasts reveal

---

## 4. The Reveal Script — Opcode by Opcode

The reveal script is built by `EnvelopeScriptBuilder` (from `strata-l1-envelope-fmt`) and has this structure:

```
<pubkey_32_bytes>  OP_CHECKSIG  OP_FALSE  OP_IF  <chunk₀>  <chunk₁>  ...  <chunkₙ>  OP_ENDIF
```

For a payload of 1041 bytes (3 chunks: 520 + 520 + 1):

```
OP_PUSHBYTES_32 <x-only pubkey, 32 bytes>
OP_CHECKSIG
OP_FALSE                                    ← pushes 0 to the stack
OP_IF                                       ← sees 0, skips everything until OP_ENDIF
  OP_PUSHDATA2 0x0802 <520 bytes>           ← chunk 0
  OP_PUSHDATA2 0x0802 <520 bytes>           ← chunk 1
  OP_PUSHBYTES_1 <1 byte>                   ← chunk 2 (remainder)
OP_ENDIF
```

### Script execution semantics

When a Bitcoin node executes this script:

1. **`<pubkey> OP_CHECKSIG`** — verifies the Schnorr signature from the witness against the pubkey. If valid, pushes `1` (true) to the stack.
2. **`OP_FALSE OP_IF ... OP_ENDIF`** — a **dead branch**. `OP_FALSE` pushes `0`, `OP_IF` sees `0` and skips everything until `OP_ENDIF`. The envelope data is **never executed**.

Although the data is never executed, it is **included in the script hash**. The Schnorr signature signs the transaction sighash, which includes the `TapLeafHash`, which covers the **entire script including the dead branch**. The signature transitively commits to the envelope contents.

> **Analogy:** It is like signing a sealed envelope. You do not need to open the envelope for the signature to certify its contents — the signed envelope is an indivisible whole.

### Chunk encoding

Payloads are automatically chunked at `MAX_SCRIPT_ELEMENT_SIZE = 520` bytes (Bitcoin consensus limit):

| Payload size | Chunk sizes |
|---|---|
| 520 | `[520]` |
| 521 | `[520, 1]` |
| 1040 | `[520, 520]` |
| 2000 | `[520, 520, 520, 440]` |

Maximum envelope payload: **395,000 bytes** (Bitcoin's 400 KB standardness limit).

---

## 5. Script-Path Spend — The Witness Stack

When the reveal tx spends the Taproot output from the commit tx, the witness contains **exactly 3 elements**:

```
witness[0] = <schnorr_signature>    (64 bytes)
witness[1] = <reveal_script>        (the full script bytes)
witness[2] = <control_block>        (33 bytes for single-leaf)
```

### What each element does

**Schnorr signature (64 bytes):** A BIP-341 taproot script-spend signature. Signs the transaction sighash which includes the `TapLeafHash` of the script. Verified against the **untweaked** pubkey embedded in the script (not the output key `Q`).

**Reveal script:** The complete script as-is. The Bitcoin node needs it to reconstruct the leaf hash and verify that it belongs to the Taproot tree.

**Control block (33 bytes for single-leaf):**

```
control_block = leaf_version(1 byte) || internal_key(32 bytes)
                    0xC0                  <x-only pubkey>
```

For a single-leaf tree there are no merkle path hashes, so only 33 bytes. The Bitcoin node uses this to reconstruct the output key `Q` and verify it matches the scriptPubKey of the output being spent.

### How a Bitcoin node validates the spend

```
1. Parse witness → extract signature, script, control_block

2. From control_block extract:
   - leaf_version (0xC0 = TapScript v1)
   - internal_key (P)

3. Compute leaf_hash = tagged_hash("TapLeaf", 0xC0 || len(script) || script)

4. Compute merkle_root (= leaf_hash for single-leaf)

5. Compute output_key = P + tagged_hash("TapTweak", P || merkle_root) × G

6. Verify: output_key == scriptPubKey of the spent output
   (if mismatch → tx invalid)

7. Execute the script with the signature on the stack:
   - <sig> <pubkey> OP_CHECKSIG → verifies, pushes 1
   - OP_FALSE OP_IF ... OP_ENDIF → skip (no-op)
   - Stack result: [1] → VALID
```

---

## 6. Output Types — OP_RETURN and Dust

### Output 0: OP_RETURN (provably unspendable)

The reveal tx's first output is an `OP_RETURN` carrying the SPS-50 tag. These outputs are known as **provably unspendable outputs** (or **null data outputs** in Bitcoin Core terminology).

`OP_RETURN` immediately aborts script execution. No witness or signature can ever make it valid to spend. Any Bitcoin node will reject a transaction attempting to spend an OP_RETURN output.

The key design consequence: **Bitcoin nodes do not add OP_RETURN outputs to the UTXO set.** Before OP_RETURN existed, people embedded data in fake P2PKH outputs, which polluted the UTXO set permanently (nodes cannot prove they are unspendable). OP_RETURN was introduced specifically to provide a clean way to embed data without inflating the UTXO set.

### Output 1: Dust output (spendable)

The reveal tx's second output carries 546 sats to the broadcaster's address. This is called a **dust output** — an output whose value is so small that spending it alone would cost more in fees than the output is worth.

**Why 546 sats?** Bitcoin Core defines a dust limit below which transactions are considered non-standard and relay nodes will not propagate them. The formula is:

```
dust_limit = 3 × (output_size + input_size_to_spend_it) × dust_relay_fee / 1000
```

For P2PKH this yields **546 sats** — the classic threshold. For P2TR (Taproot) the actual dust limit is **330 sats**, but Alpen uses 546 as a conservative value that passes for any output type.

**Why does the reveal tx need it?** Without this output, the difference between the input value and the OP_RETURN (value 0) would be entirely consumed as miner fee. The dust output captures the remaining sats after fees.

**The address type is not fixed by the protocol.** Alpen's `EnvelopeConfig` has a `sequencer_address: Address` field — a generic `bitcoin::Address`. The builder calls `.script_pubkey()` on it, producing whatever script type the address encodes (P2TR, P2WPKH, etc.). The same address is used for both commit tx change and reveal tx dust output.

In practice, this dust output either gets consolidated into a future transaction by the broadcaster's wallet, or remains unspent.

---

## 7. Three Signature Layers

There are three distinct layers of signatures, each serving a different purpose:

| # | Signature | Type | Authorizes | Signer |
|---|-----------|------|-----------|--------|
| 1 | Commit tx input | Depends on wallet (Schnorr if P2TR, ECDSA if P2WPKH) | Spending wallet UTXOs to fund the Taproot output | The broadcaster (their Bitcoin wallet) |
| 2 | Reveal tx input | **Schnorr** (Taproot script-path) | Spending the Taproot output, revealing the envelope | The **ephemeral keypair** (same key embedded in the script) |
| 3 | Inside `SignedPayload` | **ECDSA** recoverable (secp256k1) | The governance action (multisig quorum) | The **N signers** of the authority (hardware wallets), collected off-chain |

### How they nest

```
Signature 3 (ECDSA × N signers)
   inside →  SignedPayload { seqno, action, signatures }
      Borsh-serialized inside →  chunks of the envelope
         inside →  reveal script
            signed by →  Signature 2 (Schnorr, ephemeral keypair)
               in the witness of →  reveal tx
                  which spends the output of →  commit tx
                     signed by →  Signature 1 (broadcaster's wallet)
```

### Independence and timing

The three signatures are **independent**: the ECDSA governance signatures (3) are collected off-chain over days. When the broadcaster decides to broadcast, they generate the ephemeral keypair, sign the reveal (2), and sign the commit with their wallet (1) — all at broadcast time.

Signature 2 (Schnorr on the reveal) cannot be forged because the ephemeral keypair is generated locally and discarded after broadcast. However, it adds no governance security — the real security is in signature 3 (the ECDSA quorum that the ASM verifies on-chain). Signatures 1 and 2 are purely Bitcoin mechanics for moving funds and revealing data.

---

## 8. Hardware Wallet Scope

### What the hardware wallet signs

Hardware wallets **only produce signature 3** — the ECDSA governance signature over the SPS-65 sighash. They sign a 32-byte hash, not a Bitcoin transaction:

```
sighash = SHA256( SHA256("strata/admin/<type_name>") || seqno_be(8) || payload )
```

The sighash is computed in the Rust backend and sent directly to the device. The hardware wallet never sees or constructs a Bitcoin transaction for governance signing.

Signatures 1 and 2 are the **broadcaster's responsibility** — handled by the desktop app at broadcast time, using the broadcaster's own wallet and the ephemeral keypair.

### Why the PRD requires "Taproot inputs" support

The PRD (§6.1) requires hardware wallets to support "Taproot inputs" as a device compatibility filter, **not** because the governance signature is a Taproot signature. The reasons are:

1. **Key derivation path is BIP-86** (`m/86'/0'/73'/0/n`) — purpose `86'` is the Taproot derivation standard. A hardware wallet that does not support Taproot cannot derive keys on this path at all.
2. **Address display** — the address shown to the user and verified on-device (PRD §6.5) is a P2TR address derived from the BIP-86 key. The device must understand Taproot addresses to display them.
3. **Possible broadcaster role** — if a signer also acts as broadcaster and their Bitcoin wallet UTXOs are on the hardware wallet, they would need Taproot input signing for the commit tx (signature 1).

### Summary

| Need | Requires Taproot? | Requires ECDSA message signing? |
|------|-------------------|--------------------------------|
| Derive keys on `m/86'/0'/73'/0/n` | **Yes** — BIP-86 is Taproot | N/A |
| Display address on device screen | **Yes** — the address is P2TR | N/A |
| Sign governance action (signature 3) | **No** | **Yes** |
| Broadcast (signatures 1 and 2) | Not necessarily | No — signature 2 uses ephemeral keypair |

Governance signers **do not need a funded Bitcoin wallet**. They only sign a message. The broadcaster is a separate role requiring UTXOs to fund the commit tx.

---

## 9. End-to-End State Transition Example

```
OFF-CHAIN                           ON-CHAIN (Bitcoin)             DERIVED (Strata node)
─────────                           ──────────────────             ──────────────────────

Proposal created
     │
Signatures collected                                               Current state:
     │                                                             ┌─────────────────┐
Quorum reached                                                     │ signers: [A,B,C]│
     │                                                             │ threshold: 2    │
     v                                                             │ last_seqno: 4   │
Desktop app builds                                                 │ queued: []      │
  commit + reveal      ──broadcast──>  Block N:                    └─────────────────┘
                                       [commit tx]                        │
                                       Block N+1:                         │
                                       [reveal tx]  ──ASM reads──> handle_action()
                                                                         │
                                                                         v
                                                                   New state:
                                                                   ┌─────────────────┐
                                                                   │ signers: [A,B,C]│
                                                                   │ threshold: 2    │
                                                                   │ last_seqno: 5   │
                                                                   │ queued: [{id:1, │
                                                                   │  "add D, rm A", │
                                                                   │  activates: N+  │
                                                                   │  2017}]         │
                                                                   └─────────────────┘
                                                                         │
                                       Block N+2017:                      │
                                       (no admin txs)  ──────────> handle_pending()
                                                                         │
                                                                         v
                                                                   Final state:
                                                                   ┌─────────────────┐
                                                                   │ signers: [B,C,D]│
                                                                   │ threshold: 2    │
                                                                   │ last_seqno: 5   │
                                                                   │ queued: []      │
                                                                   └─────────────────┘
```

Key observations:

1. **Block N (commit)** has no governance meaning — it just funds the Taproot output.
2. **Block N+1 (reveal)** carries the governance instruction. The ASM validates it and queues the update with `activation_height = N+1 + 2016`.
3. **Block N+2017** has no admin transactions at all, yet state changes — because `handle_pending_updates` runs at every block and enacts matured updates automatically.
4. During the ~2016 block window, a **cancel transaction** (another commit+reveal cycle) can remove the queued update.

---

## 10. Key Conceptual Takeaways

1. **Bitcoin is the transport and immutable log.** State lives exclusively as derived computation in each Strata node.
2. **Every action is an "event" recorded on Bitcoin** that, when replayed, produces a deterministic state transition.
3. **The commit-reveal pattern is a Bitcoin constraint**, not a protocol design choice. It exists because Taproot witness data requires spending an output committed to that specific script.
4. **Both transactions are generated by one actor** — the signer who broadcasts. The governance signatures (ECDSA, from multiple signers) are collected off-chain beforehand; the Bitcoin transaction signatures (wallet + Schnorr) are from the broadcaster alone.
5. **State can change even in blocks with no admin transactions** — queued updates automatically enact when their activation height is reached.
6. **The ASM is the sole arbiter.** The off-chain backend and desktop app are convenience layers. If they disappear, signers can still construct transactions manually and broadcast them. The ASM will process them identically.
7. **Three independent signature layers exist** — governance ECDSA (quorum, collected over days), Schnorr reveal (ephemeral, at broadcast), and wallet commit (broadcaster's funds). Only the governance signatures carry protocol security.
8. **Hardware wallets only produce governance signatures** — they sign a 32-byte ECDSA sighash, not a Bitcoin transaction. Taproot support is needed for key derivation (BIP-86) and address display, not for the governance signing itself.
9. **OP_RETURN outputs are provably unspendable** — they do not enter the UTXO set. The reveal tx's OP_RETURN exists solely as a filtering tag for the ASM; the dust output is a Bitcoin mechanical requirement.
