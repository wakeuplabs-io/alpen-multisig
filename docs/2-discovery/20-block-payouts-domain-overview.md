# Block Payouts — Domain Overview

## What this section covers

The **Payout Administrator** section manages `block_payout` transactions: Bitcoin transactions that block fraudulent refund claims from bridge operators.

### Context: how the bridge works

When a user wants to withdraw BTC from Alpen to Bitcoin L1, a bridge **operator** advances the funds from their own pocket. Then, the operator creates a "claim transaction" to recover that money from the bridge's locked funds — optimistically: if no one challenges it during the challenge period, the operator gets paid.

A fraudulent operator could try to collect without having actually advanced the funds. If a **challenger** detects this, they generate a **false claim report** with cryptographic proof that the claim is invalid.

### Role of the Payout Administrator

The Payout Administrator uses those reports to create a `block_payout` transaction that **spends the claimed UTXOs before the fraudulent operator can**, blocking the undue reimbursement.

**Full flow:**

1. A challenger detects a fraudulent claim and generates a **false claim report**
2. A Payout Admin signer creates a `block_payout` tx using the report's outpoints as inputs
3. The other signers **sign** it until quorum is reached
4. Once quorum is reached, the tx is **broadcast to Bitcoin** — the fraudulent operator loses their claim

---

## Current state of the code

What exists today is a **100% frontend mock** — no real backend or Tauri IPC calls.

```
domain/block-payouts/
├── components/                  ← Full UI (dashboard, modals, cards)
├── hooks/use-block-payouts.ts   ← React state, mock actions
└── model/
    ├── block-payouts.types.ts   ← Defined types
    └── block-payouts.mock.ts    ← Hardcoded data
```

All state lives in React, initialized with fake data. No action (sign, create tx, rebroadcast) calls any real service. The goal is to validate the visual flow before connecting it to the real backend.

### Example of a false claim report

The user pastes (or uploads) one or more reports in JSON format. Each report represents an off-chain detected fraudulent withdrawal attempt:

```json
{"claimId":"claim-test-001","outpoint":"aabb1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab:2","amount":500000,"proof":"a1b2c3d4e5f6a1b2c3d4e5f6"}
```

| Field | Description |
|---|---|
| `claimId` | Unique identifier of the operator's claim being disputed |
| `outpoint` | UTXO to spend (`txid:vout`) — the bridge output the operator fraudulently wants to collect |
| `amount` | Amount in satoshis of that output |
| `proof` | Cryptographic proof that the operator's claim is false (real validation pending) |

The modal parses these reports, filters already-spent outpoints, and builds the input list for the `block_payout` transaction. In the mock, any JSON with a non-empty `proof` is considered valid.

---

## Is ASM integration needed?

**Not directly.** The Payout Administrator is different from the other roles:

| Aspect | Strata / Alpen Admin | Payout Admin |
|---|---|---|
| Signer set | Defined in **ASM state** (Strata chain) | Defined in the **Bridge multisig script** (Bitcoin L1) |
| Authentication | Nonce signature; backend verifies against ASM | **Does not use ASM** — uses BIP-86 derivation `m/86'/0'/73'/0/0` |
| Transactions | `MultisigAction` with OP_RETURN envelope (SSZ) | `block_payout` tx — pure Bitcoin, no ASM envelope |

---

## What comes next (out of scope for the mock)

When the real backend is integrated, the connection points are:

1. **Tauri IPC** — derive the P2TR address from the hardware wallet (BIP-86) to authenticate the signer
2. **Orchestrator backend** — persist pending txs, collect signatures between signers, broadcast
3. **Real signature validation** — Schnorr/Taproot in Rust (currently any string ≥ 64 chars passes)
4. **False claim proof validation** — real cryptographic validation (currently: any JSON with a non-empty `proof` field passes)

References:
- PRD source: [docs/0-prd/03-prd-update.md](../0-prd/03-prd-update.md) §6
- Mock UI spec: [docs/specs/block-payouts-ui-mock.md](../specs/block-payouts-ui-mock.md)
- ASM vs Bitcoin L1 difference: [docs/2-discovery/10-asm-bitcoin-state-model.md](./10-asm-bitcoin-state-model.md)
