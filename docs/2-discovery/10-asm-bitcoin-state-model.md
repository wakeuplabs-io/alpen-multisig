# ASM & Bitcoin L1 — State Representation and Transition Model

## Overview

This document explains, at a conceptual level, how the Anchor State Machine (ASM) represents governance state using Bitcoin L1 as its foundation. It covers how state is derived (not stored) from Bitcoin, how governance actions produce state transitions, and how the commit-reveal transaction pattern works.

### Sources

- **POC-1 findings** — [`03-poc1-findings.md`](./03-poc1-findings.md)
- **Conceptual overview** — [`01-conceptual-overview.md`](./01-conceptual-overview.md)
- **Architecture overview** — [`../architecture/overview.md`](../architecture/overview.md)
- **Alpen crate source** — `crates/asm/subprotocols/admin/src/` (handler, state, authority, subprotocol)

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

### Two distinct signature layers

The ECDSA signatures inside the `SignedPayload` (from governance signers) are **not** the Schnorr signature on the Bitcoin transaction. They serve different purposes:

| Signature | Type | Purpose |
|-----------|------|---------|
| Governance signatures (inside envelope) | ECDSA (recoverable, secp256k1) | Authorize the governance action |
| Transaction signature (in witness) | Schnorr (Taproot script-path) | Authorize the Bitcoin UTXO spend |

---

## 4. End-to-End State Transition Example

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

## 5. Key Conceptual Takeaways

1. **Bitcoin is the transport and immutable log.** State lives exclusively as derived computation in each Strata node.
2. **Every action is an "event" recorded on Bitcoin** that, when replayed, produces a deterministic state transition.
3. **The commit-reveal pattern is a Bitcoin constraint**, not a protocol design choice. It exists because Taproot witness data requires spending an output committed to that specific script.
4. **Both transactions are generated by one actor** — the signer who broadcasts. The governance signatures (ECDSA, from multiple signers) are collected off-chain beforehand; the Bitcoin transaction signature (Schnorr) is from the broadcaster alone.
5. **State can change even in blocks with no admin transactions** — queued updates automatically enact when their activation height is reached.
6. **The ASM is the sole arbiter.** The off-chain backend and desktop app are convenience layers. If they disappear, signers can still construct transactions manually and broadcast them. The ASM will process them identically.
