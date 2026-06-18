# Reference — Snapshot: Governance Analogy & Learning Resources

> **Status:** Reference — standalone mental model for engineers coming from an Ethereum background. The Snapshot/SafeSnap/EIP-712/Reality.eth comparison in this document is not duplicated elsewhere. For the direct description of the Alpen/Strata system itself, see [`docs/architecture/overview.md`](../architecture/overview.md), [`docs/3-stories/story-map.md`](../3-stories/story-map.md), and [`docs/3-stories/non-functional-items.md`](../3-stories/non-functional-items.md). Off-chain coordination persistence: Postgres when `DATABASE_URL` is set (see [`architecture/overview.md`](../architecture/overview.md)).

## Purpose

This document explains how Snapshot works (proposal creation, voting, and multisig integration), maps it against the Alpen/Strata multisig system, and identifies useful references for developers working on this project.

---

## 1. What Is Snapshot?

Snapshot is a gasless, off-chain governance platform used by Ethereum-based DAOs. Its core idea mirrors what this project does: **coordinate governance decisions off-chain using cryptographic signatures, then optionally execute on-chain only when consensus is reached**.

It is worth studying because the discovery doc (`02-discovery.md`) explicitly named it as the closest conceptual reference: *"The functional flow resembles a proposal → approval → execution pattern (conceptually similar to Snapshot in terms of UX mental model, though not necessarily in implementation)."*

---

## 2. How Snapshot Works

### 2.1 Proposal Creation

```
Creator → signs EIP-712 typed data → uploads to IPFS → indexed by Snapshot Hub
```

- The creator must hold enough **voting power** at a specific block (the "snapshot block") — the system takes a point-in-time snapshot of balances, preventing last-minute token acquisition to influence votes.
- The proposal is stored as a content-addressed IPFS document. The Hub (a centralized API operated by Snapshot Labs) indexes it by CID.
- A proposal includes: title, body, vote choices, the snapshot block, start time, and end time.

**Voting power strategies** — pluggable modules that define how much influence each address has. Examples: `erc20-balance-of`, `delegation`, `whitelist`, custom on-chain reads. This is Snapshot's equivalent of the signer set + threshold config in `ThresholdConfig`.

### 2.2 Voting

```
Voter → signs EIP-712 message { proposalId, choice, voter, timestamp } → POST to Hub
```

- **Entirely off-chain.** No transaction, no gas.
- The Hub validates: signature is valid, voter had sufficient power at the snapshot block, vote is within the time window.
- Votes are stored in the Hub database (and optionally on IPFS for spaces that enable it).
- The Hub is a centralized server — analogous to `orchestrator-be` in this project.

**EIP-712 typed structured data** — Ethereum's standard for signing human-readable, domain-separated structured messages with a wallet. The concept is directly analogous to the SPS-65 sighash scheme used here:

| Snapshot (EIP-712) | Alpen/Strata (SPS-65) |
|---|---|
| `domain separator` (chain ID + contract address) | `tag = "strata/admin/<type_name>"` |
| typed struct hash | `sighash_payload` (SSZ-serialized action; Borsh pre-migration) |
| `seqno` implicit via timestamp | explicit `seqno_be_bytes` (8 bytes, u64) |
| ECDSA over keccak256 | ECDSA over SHA256 |

Both achieve the same goals: **domain separation** (a signature for one action cannot be replayed against another type) and **replay protection**.

### 2.3 Multisig Integration — SafeSnap / Zodiac Reality Module

This is where Snapshot bridges off-chain votes to on-chain execution via a Gnosis Safe multisig.

```
Vote passes in Snapshot
    ↓
Reality.eth oracle opens (24–72h challenge window)
    ↓
No dispute → Gnosis Safe executes the transaction automatically
    ↓
Dispute → Kleros arbitration resolves it
```

- **Reality.eth** is an on-chain optimistic oracle. It assumes the vote passed correctly and opens a dispute window. Anyone can post a bond to challenge the outcome.
- If unchallenged, `execTransactionFromModule()` is called on the Safe — the vote result becomes an on-chain action.
- The dispute window is a **trust delay**, analogous to the ~2016 block (~2 week) confirmation depth in the ASM's queued update system.

**Snapshot X** — the newer version replaces the optimistic oracle with cryptographic on-chain proof of vote results using StarkNet. No challenge window; the vote result is verifiably proven on-chain directly.

---

## 3. Analogy: Snapshot vs. Alpen/Strata Multisig

### 3.1 Structural Mapping

| Layer | Snapshot | Alpen/Strata Multisig |
|---|---|---|
| **Proposal identity** | IPFS CID (content hash of proposal JSON) | `ActionId = hash(MultisigAction, SeqNo)` |
| **Off-chain coordination** | Snapshot Hub (centralized, IPFS fallback) | `orchestrator-be` (Axum + in-memory repo today; Postgres deferred — see NF-6 — manual fallback) |
| **Voting/signing** | EIP-712 via any EVM wallet | SPS-65 ECDSA via hardware wallet (HWI, `m/86'/0'/73'/0/n`) |
| **Signature store** | Hub database per proposalId | `sigs_by_id: Map<ActionId, Vec<Signature>>` |
| **Quorum tracking** | `for/against/abstain` tallied by voting power | `QuorumStatus { collected, required, is_reached }` |
| **Trust delay before execution** | Reality.eth challenge window (24–72h) | ASM confirmation depth (~2016 blocks, ~2 weeks) |
| **On-chain execution** | Gnosis Safe `execTransactionFromModule()` | Bitcoin tx broadcast → ASM parses from block |
| **On-chain validator** | Gnosis Safe contract (EVM) | ASM Administration Subprotocol (Strata node) |
| **Cancellation** | N/A (no on-chain cancel for most flows) | `MultisigAction::Cancel(CancelAction)` during wait window |
| **Proposal expiry** | Defined by end timestamp | 7 days (off-chain); seqno gap limit ≤ 10 (on-chain) |

### 3.2 Lifecycle Comparison

**Snapshot:**
```
Created → Active (voting window) → Closed
                                      ├── Succeeded (quorum met, threshold met)
                                      └── Defeated / Quorum Not Met
                                              ↓ (if SafeSnap)
                                     Reality.eth challenge window
                                              ↓
                                     Safe executes or dispute resolved
```

**Alpen/Strata:**
```
Pending (7-day off-chain window, signatures collecting)
    ├── Expired (timeout before threshold)
    ├── Canceled (manual)
    └── Approved (threshold reached → broadcast Bitcoin tx)
              ↓ (~2016 blocks wait)
         ├── Canceled (during wait window)
         └── Enacted (applied to ASM state → governance change takes effect)
```

Key difference: in Alpen/Strata the off-chain phase and the on-chain phase are **distinct lifecycle states**. Snapshot conflates them — a vote "succeeding" is purely off-chain, and the on-chain execution is a separate optional step (via SafeSnap). Here, Approved means the tx is already on Bitcoin.

### 3.3 Key Similarities

- Both are **off-chain coordination layers** decoupled from the execution layer. Neither is required for the final action — in Snapshot, someone can always submit a Safe tx directly; in this system, signers can aggregate signatures and broadcast manually without the backend.
- Both use **content-addressed proposal identity** — Snapshot via IPFS CID, this system via deterministic hash of action + seqno. Both guarantee that re-submitting the same proposal produces the same ID (idempotent).
- Both treat the **coordination server as non-authoritative**. The Hub does not enforce Gnosis Safe threshold rules; `orchestrator-be` does not enforce ASM validity rules. Canonical validity lives exclusively on-chain.
- Both support **copy/paste signature workflows** as a fallback — the PRD explicitly requires signers to be able to copy all approval signatures for manual broadcast.
- Both use **time-bounded proposals** to prevent stale governance actions from executing unexpectedly.

### 3.4 Key Differences

| Dimension | Snapshot | Alpen/Strata Multisig |
|---|---|---|
| **Permission model** | Permissionless — any token holder can vote, power is proportional to holdings | Permissioned — fixed signer sets per authority, equal weight per signer |
| **Governance scope** | Signal + optional on-chain execution | Always produces a Bitcoin transaction; execution is mandatory, not optional |
| **Trust model** | Snapshot Hub is a single point of trust (though IPFS mitigates some of this) | `orchestrator-be` is similar, but ASM on Bitcoin is the final arbiter |
| **Chain** | Ethereum / EVM | Bitcoin (OP_RETURN + witness envelope) |
| **Signing scheme** | EIP-712 (keccak256, Ethereum addresses) | SPS-65 (double-SHA256 over `tag_hash \|\| seqno_be \|\| sighash_payload`, secp256k1 recoverable ECDSA — *not* BIP-137 / BIP-322, see [`07-hardware-wallet-library-analysis.md`](./07-hardware-wallet-library-analysis.md)) |
| **Hardware wallet UX** | Any EVM wallet (MetaMask, Ledger, Trezor via web) | HWI-compatible, Taproot, `m/86'/0'/73'/0/n`, on-device message display |
| **Sequence numbers** | None — proposals are unordered | Explicit `SeqNo: u64`, replay protection, gap limit |
| **Strict ordering** | N/A | Deliberately non-enforced — seqno gaps are allowed (unlike Gnosis Safe's strict nonce ordering) |
| **Cancellation** | No on-chain cancellation mechanism | `Cancel` action during ~2 week window before enactment |
| **Role separation** | Single token = single governance space | Five fully isolated authorities with no cross-leakage |

### 3.5 The Safe Nonce Analogy — and Where It Diverges

The PRD (`02-multisig-backend.md`, section 4) explicitly documents the Gnosis Safe comparison:

> "In the Safe Model: Proposal N+1 cannot execute until proposal N is executed or explicitly cancelled. In the Strata/Alpen administrative model: a proposal that does not reach quorum MAY be skipped. A proposal with a higher SeqNo MAY be executed without requiring explicit on-chain rejection of earlier unresolved proposals."

This is a deliberate divergence from Safe's strict nonce ordering. The Strata model allows governance to continue even if some proposals expire or are abandoned — operationally more resilient.

---

## 4. What to Study

### 4.1 For Understanding Snapshot's Off-Chain Coordination Model

- **Snapshot Hub source** — `github.com/snapshot-labs/snapshot`: the centralized API that receives and validates vote submissions. Compare its proposal/vote storage pattern against `orchestrator-be`'s `action_by_id` / `sigs_by_id` maps.
- **Snapshot.js SDK** — `github.com/snapshot-labs/snapshot.js`: shows how proposals and votes are constructed, signed (EIP-712), and submitted from a TypeScript client. Compare against `desktop-app/src/api/proposals.ts` and the auth flow in `useAuth.ts`.

### 4.2 For Understanding EIP-712 vs. SPS-65 Signing

- **EIP-712 spec** — `eips.ethereum.org/EIPS/eip-712`: the Ethereum standard for typed structured data hashing. The domain separator + struct hash pattern maps directly to the tag + sighash_payload pattern in SPS-65. Reading EIP-712 will solidify understanding of *why* the SPS-65 sighash is constructed the way it is.
- **`signing.rs`** in this repo — [`desktop-app/src-tauri/src/infrastructure/signing.rs`](../../desktop-app/src-tauri/src/infrastructure/signing.rs): the production implementation of `compute_sighash`, `sign_sighash`, `verify_threshold` using Alpen crates. Read alongside `docs/specs/poc3-signing-lib.md`.

### 4.3 For Understanding Multisig On-Chain Execution (Gnosis Safe)

- **Gnosis Safe contracts** — `github.com/safe-global/safe-smart-account`: specifically `GnosisSafe.sol`'s `execTransaction` function and the nonce-based replay protection. Compare the nonce model against `ActionId = hash(MultisigAction, SeqNo)` and the `last_seqno` tracking in `MultisigAuthority`.
- **Safe TypeScript SDK** — `github.com/safe-global/safe-core-sdk`: shows how signatures are collected off-chain (via `signTransaction`) and aggregated before the on-chain call. This is the closest analog to the `submitSignature` → `QuorumStatus` flow in `orchestrator-be`.

### 4.4 For Understanding Snapshot X (Trustless On-Chain Vote Verification)

- **Snapshot X monorepo** — `github.com/snapshot-labs/sx-monorepo`: the successor to SafeSnap that eliminates the optimistic oracle by proving vote results cryptographically on-chain via StarkNet. Conceptually relevant if the project ever considers reducing trust in `orchestrator-be` — though the ASM already provides on-chain finality via Bitcoin, making this largely solved differently here.

### 4.5 For Understanding Reality.eth (Optimistic Oracle Pattern)

- **Reality.eth docs** — `reality.eth.link`: the optimistic oracle used by SafeSnap. The challenge window pattern is worth understanding as an alternative to the ASM's fixed confirmation depth model — one is stake-based (economic security), the other is block-based (temporal security).

---

## 5. Summary

Snapshot and the Alpen/Strata multisig system solve the same core problem — **coordinating multi-party governance decisions off-chain before committing them on-chain** — using structurally identical patterns: content-addressed proposals, signature collection on a coordination server, quorum tracking, and a trust delay before execution.

The meaningful differences are in the execution environment (EVM vs. Bitcoin), the permission model (token-weighted vs. fixed signer sets), and the trust delay mechanism (optimistic oracle vs. fixed block depth). Understanding Snapshot's architecture gives a useful mental model for explaining this system to people with an Ethereum background, and studying the Safe SDK provides the most directly transferable code patterns for the signature aggregation layer.
