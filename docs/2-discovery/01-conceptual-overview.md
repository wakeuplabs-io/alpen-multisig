# Alpen & Strata — Conceptual Overview

> **Status: Reference / Phase 1 historical.** On-chain and protocol background from early discovery. Off-chain coordination: Postgres when `DATABASE_URL` is set; in-memory fallback for local dev — see [`architecture/overview.md`](../architecture/overview.md) and NF-6 in [`non-functional-items.md`](../3-stories/non-functional-items.md).

## 1. What is Strata?

Strata is a **bridging protocol for Bitcoin** developed by Alpen Labs. It is designed as a neutral, open-source public good — infrastructure that any Layer 2 can use to connect securely to Bitcoin.

Strata's core function is enabling **trustless asset transfers between Bitcoin and L2 execution environments**. It achieves this through a bridge built on BitVM2 technology with significant optimizations, operating under a **1-of-N trust model**: as long as at least one operator is honest, the bridge remains secure.

Strata is not a rollup itself — it is the infrastructure layer that rollups plug into to gain access to Bitcoin's security.

## 2. What is Alpen?

Alpen is the **first rollup (L2 execution layer) built on Strata**. It is an EVM-compatible validity rollup that:

- Uses **Reth** (Ethereum execution client) for transaction execution
- Uses **SP1** for zero-knowledge proof generation
- Has a centralized **sequencer** producing blocks every ~5 seconds
- Posts batch checkpoints to Bitcoin, finalized after 6 Bitcoin confirmations
- Uses Bitcoin for consensus and data availability

Alpen enables developers to build secure, on-chain BTC applications — DeFi, lending, trading — without relying on custodial solutions.

## 3. What is Alpen Labs?

Alpen Labs is the **company** building both Strata and Alpen. Founded to scale Bitcoin using zero-knowledge technology, it has raised ~$19M in funding from investors including DBA, Ribbit Capital, Castle Island Ventures, and Cyber Fund.

## 4. Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│  Bitcoin (L1)                                       │
│  Consensus · Data Availability · Proof Verification │
├─────────────────────────────────────────────────────┤
│  Strata (Bridging Protocol)                         │
│  Trustless bridge · 1-of-N trust · BitVM2-based     │
│  Governance (ASM) · SPS-50/51/65 specs              │
├─────────────────────────────────────────────────────┤
│  Alpen (EVM Rollup — first L2 on Strata)            │
│  Execution · Sequencing · ZK Proofs                 │
└─────────────────────────────────────────────────────┘
```

Strata and Alpen were originally under a single brand. They were split so that Strata could focus on being neutral bridge infrastructure while Alpen focuses on being a production-grade EVM execution environment.

---

## 5. The Strata Bridge — How It Works

The Strata Bridge is the core component of the Strata protocol. It enables moving BTC between Bitcoin (L1) and Alpen (L2) while maintaining a strict **1:1 backing ratio**: 1 BTC on Alpen is always backed by 1 BTC locked on Bitcoin.

### 5.1 Participants

| Role | Function |
|------|----------|
| **Operators** | Control bridge deposit addresses, front withdrawal funds, manage stake chains, execute claims |
| **Challengers** | Can dispute operator claims during optimistic lock periods |
| **Disprovers** | Provide cryptographic evidence that an operator acted fraudulently |

An operator is **functional** if it follows protocol rules. It is **faulty** if it deviates — whether malicious, malfunctioning, or coerced.

### 5.2 Deposit Flow (Bitcoin → Alpen)

```
Step 1: Alice sends BTC to a bridge deposit address controlled by operators
            ↓
Step 2: The deposit is confirmed on Bitcoin
            ↓
Step 3: Strata mints equivalent wrapped BTC to Alice's account on Alpen
            ↓
Step 4: Alice can now use her BTC on Alpen (transfer, DeFi, etc.)
```

During the deposit setup, operators collectively pre-sign structured transactions that define all possible future spending paths for this deposit (using MuSig2). This is critical — it creates the "emulated covenants" that enforce bridge rules without Bitcoin native covenant support.

### 5.3 Withdrawal Flow (Alpen → Bitcoin)

```
Step 1: Bob burns his wrapped BTC on Alpen, initiating a withdrawal request
            ↓
Step 2: An operator fronts Bob the requested BTC (minus fee) directly on Bitcoin
         Bob has his BTC — he doesn't need to wait for the claim process
            ↓
Step 3: The operator creates a "kick-off transaction" referencing staked collateral
            ↓
Step 4: The operator creates a "claim transaction" with an optimistic payout,
         locked for a challenge period
            ↓
Step 5a (Happy Path): No challenge → operator receives Alice's original deposit
         plus their collateral back. Operator keeps the fee.
            ↓
Step 5b (Disputed): A challenger disputes → challenge/disproof process begins
         (see Section 5.6)
```

Key insight: the operator **fronts their own money** to the withdrawing user immediately, then later claims the original deposit. This makes withdrawals fast for users while keeping the system secure.

### 5.4 Core Technical Components

#### Emulated Covenants (MuSig2)

Bitcoin does not natively support covenants (rules restricting how outputs can be spent). Strata emulates them:

- Operators pre-generate **all possible structured transactions** needed for any bridge scenario
- They collectively sign these using **MuSig2** (Schnorr-based multisignature scheme)
- Only one honest operator is needed to refuse signing non-compliant transactions
- This creates covenant-like restrictions with a 1-of-N trust assumption

#### Connector Outputs

Connector outputs enforce **exclusive transaction paths**. They work by exploiting Bitcoin's double-spend prevention:

- A common transaction generates outputs consumed by competing paths
- Since Bitcoin prevents double-spending, only one path can execute
- Some connectors carry value (collateral), others are zero-value (pure information passing)

Example: The "claim" and "challenge" paths share a connector output — executing one invalidates the other.

#### Winternitz One-Time Signatures

These pass **verifiable state between transactions** within Bitcoin script:

- Constructed using hash functions compatible with Bitcoin script
- Each deposit-withdrawal instance uses **fresh keys used only once**
- Transactions embed identical verification keys hardcoded in script
- Enables state verification across multi-transaction sequences without complex introspection

#### SNARK Proof Verification (Groth16)

The bridge uses Groth16 proofs to verify two conditions:

1. The withdrawal transaction exists on Bitcoin's main chain (not a private fork)
2. The Strata state is valid and contains the user's fund burn

Since full Groth16 verification is too expensive for a single Bitcoin script, the proof is split into **subprograms** that chain together. Only the failing subprogram needs to appear on-chain during a dispute — the full program is never required.

### 5.5 Anchor Blocks and Chain Verification

A critical security requirement: the bridge must verify that withdrawal transactions exist on Bitcoin's **actual main chain**, not a malicious private fork.

**How it works:**

1. At periodic checkpoints, operators agree on a recent Bitcoin block (the **anchor block**)
2. Each operator generates a Schnorr signing key for that checkpoint
3. Keys are aggregated into a **group key** using MuSig2-style aggregation
4. The group signing key and anchor block hash are **embedded into the pre-signed deposit transaction**

**Why this matters:**

When an operator claims a deposit, they must commit to a checkpoint. If they commit to an old checkpoint (trying to use a private fork), a disprover can reveal the group key from a more recent checkpoint, proving the operator is faulty and triggering collateral slashing.

This is a key improvement over BitVM2's "superblock" approach, which had a vulnerability where a faulty disprover could provide a superior superblock not anchored to the expected chain.

### 5.6 Challenge and Dispute Process

```
Operator submits claim with optimistic payout
                    ↓
         ┌──── Lock Period 1 ────┐
         │                       │
    No challenge           Challenger posts
         │                 challenge tx (with fee)
         ↓                       ↓
  Operator receives        Operator must respond with
  deposit + collateral     "assertion transactions"
                           (partial proof of validity)
                                 ↓
                      ┌──── Lock Period 2 ────┐
                      │                       │
                 No disproof             Disprover posts
                      │                  disproving tx with
                      ↓                  cryptographic evidence
                 Operator payout              ↓
                 executes              Operator collateral
                                       is SLASHED:
                                       - portion burned
                                       - portion → disprover reward
```

The challenge fee prevents frivolous disputes against honest operators. The two-phase lock period gives operators time to respond while ensuring faulty operators are caught.

### 5.7 Stake Chains (Capital Efficiency)

In vanilla BitVM2, operators must lock collateral **per deposit** — extremely capital inefficient.

Strata introduces **stake chains**:

- Pre-signed transactions lock operator stake **without tying it to a single deposit**
- Each "link" in the chain allows the stake to be used for a different deposit
- Operators can reuse the same collateral across multiple sequential claims
- For parallel claims, operators open multiple separate stake chains

**Safety guarantees:**
- Stake cannot be used for two claims simultaneously
- If an operator tries to reuse stake improperly, a "burn transaction" slashes the stake and breaks the chain
- Burn transactions come in sets to mitigate censorship attacks

### 5.8 Differences from BitVM2

| Aspect | BitVM2 | Strata Bridge |
|--------|--------|---------------|
| **Chain verification** | Superblock selection (statistical) | Anchor blocks (deterministic, checkpoint-based) |
| **Block height** | Separate "start time transaction" | Committed directly in claim tx |
| **Assertions** | Single assertion transaction | Multi-stage assertions across multiple txs |
| **Collateral** | Per-deposit staking | Reusable stake chains |
| **Disproving** | Two separate mechanisms | Unified approach via anchor blocks |
| **Private fork risk** | Vulnerable to unanchored superblocks | Eliminated by anchor block design |

### 5.9 Complete Transaction Lifecycle Example

```
1. DEPOSIT:  Alice sends 1 BTC to bridge address
2. MINT:     Strata mints 1 wrapped BTC to Alice on Alpen
3. TRANSFER: Alice sends wrapped BTC to Bob on Alpen
4. BURN:     Bob burns wrapped BTC on Alpen (withdrawal request)
5. FRONT:    Operator sends Bob 0.99 BTC on Bitcoin (keeps 0.01 fee)
6. KICKOFF:  Operator creates kickoff tx referencing collateral
7. CLAIM:    Operator claims Alice's original 1 BTC deposit
8. PAYOUT:   Optimistic payout locked for challenge window
9. RESOLVE:  Either unchallenged payout or challenge/disproof sequence
```

---

## 6. Governance — The Multisig System

Both Strata and Alpen require administrative governance for critical protocol operations. This governance is implemented through **on-chain multisigs** where a threshold of authorized signers must approve any change.

### 6.1 Why Multisigs?

The bridge and protocol manage real BTC. A single administrator would be:
- A single point of failure
- A security risk (key compromise = total control)
- A centralization risk (one entity controls the protocol)

Multisigs distribute control: e.g., 3-of-5 signers must approve, so no individual can act unilaterally.

### 6.2 The Five Multisig Authorities

| Authority | Protocol | What It Governs |
|-----------|----------|-----------------|
| **Alpen Administrator** | Alpen | Verification keys, administrator signer set |
| **Strata Administrator** | Strata | Safe harbor address, verification keys, signer set, Security Council signers, operators, bridge updates (soft/hard) |
| **Sequencer Manager** | Strata | Sequencer signer set, sequencer key rotation |
| **Security Council** | Strata | Emergency actions: Defcon 1 (full pause), Defcon 3 (alert) |
| **Payout Administrator** | Strata | `block_payout` transactions — payments to bridge operators |

Each authority has its own signer set, threshold, and sequence number — completely isolated from the others.

### 6.3 The ASM (Administration State Machine)

The ASM is the **on-chain state machine** that is the single source of truth for all governance state. Defined in SPS-65 and implemented in the Alpen crate (`crates/asm/subprotocols/admin`), it:

- Tracks the canonical signer set for each authority
- Validates threshold signatures (ECDSA with indexed signers)
- Enforces sequence numbers (replay protection with bounded gaps)
- Manages the update queue (pending → enacted after ~2016 blocks)
- Processes cancellations of queued updates
- Executes immediate updates (sequencer changes) vs delayed updates (everything else)

**Key property:** The ASM is the *only* authoritative validator. The off-chain backend and desktop app perform basic checks but never override ASM rules.

### 6.4 Update Lifecycle

```
Signer proposes update → PENDING (offchain, 7-day expiry)
                              ↓
            Signers approve (threshold signatures collected)
                              ↓
         Transaction broadcast to Bitcoin → APPROVED (onchain)
                              ↓
                    ~2016 block waiting period
                    (can be CANCELED during this window)
                              ↓
                         ENACTED (applied to protocol state)
```

Exceptions:
- **Sequencer updates** execute immediately (no queue/delay)
- **Sequencer Manager and Security Council** updates don't have Approved/Canceled states
- Sequence numbers can be skipped (unlike Ethereum Safe's strict ordering)

### 6.5 The Multisig Desktop Application

This is the application being built — a **Tauri-based desktop app** (Rust backend + React frontend) that enables authorized signers to:

1. Connect a **hardware wallet** (HWI-compatible, Taproot, derivation path `m/86'/0'/73'/0/n`)
2. Authenticate via **ephemeral session keys** (sign a structured message binding session to authority)
3. View and create **proposals** for protocol updates
4. **Sign** proposals with their hardware wallet
5. Track **quorum progress** (how many signatures collected vs required)
6. **Broadcast** approved transactions to Bitcoin
7. **Cancel** queued updates before they are enacted
8. Manage **block_payout** transactions for the Payout Administrator role

The app is supported by an **off-chain coordination backend** (Axum; in-memory repository today, Postgres deferred — see [`non-functional-items.md` NF-6](../3-stories/non-functional-items.md)) that aggregates signatures and tracks proposal state — but it is not a single point of failure. Signers can always construct and broadcast transactions manually if the backend is unavailable.

---

## 7. Protocol Specifications

The system is defined by three core specifications:

| Spec | Purpose |
|------|---------|
| **SPS-50** | L1 transaction header format — how protocol transactions are tagged with `OP_RETURN` outputs (magic bytes + subprotocol ID + tx type + aux data) |
| **SPS-51** | Generic envelope format — how large payloads are chunked (520-byte limit) and embedded in Bitcoin transactions using `OP_FALSE OP_IF ... OP_ENDIF` blocks |
| **SPS-65** | Administration subprotocol — the full governance state machine: authorities, actions, signatures, sequence numbers, queuing, cancellation, and execution |

---

## 8. Summary

Strata is the bridge infrastructure. Alpen is the rollup that uses it. Together they enable trustless, programmable Bitcoin applications with governance managed through on-chain multisigs. The multisig desktop application is the tool that makes this governance practical and secure for human signers using hardware wallets.

```
Alpen Labs (company)
  ├── builds Strata (bridge protocol — neutral public good)
  │     ├── 1-of-N trust model via BitVM2 + optimizations
  │     ├── Governed by: Strata Admin, Sequencer Manager,
  │     │                Security Council, Payout Admin
  │     └── Connects Bitcoin ↔ any L2
  │
  └── builds Alpen (EVM rollup — first L2 on Strata)
        ├── EVM-compatible execution (Reth + SP1)
        ├── Governed by: Alpen Admin
        └── Connects to Bitcoin via Strata Bridge
```
