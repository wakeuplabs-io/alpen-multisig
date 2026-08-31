# Block Payouts — Domain Overview

## What this section covers

The **Payout Administrator** section manages `block_payout` transactions: Bitcoin transactions that block fraudulent refund claims from bridge operators.

### Context: how the bridge works

When a user wants to withdraw BTC from Alpen to Bitcoin L1, a bridge **operator** advances the funds from their own pocket. Then, the operator creates a "claim transaction" to recover that money from the bridge's locked funds — optimistically: if no one challenges it during the challenge period, the operator gets paid.

A fraudulent operator could try to collect without having actually advanced the funds. If a **watchtower** detects this, it posts an **Ack** transaction in the challenge-response graph — marking the operator as faulty. The Payout Administrator then uses that evidence to block the operator's claim.

### Role of the Payout Administrator

The Payout Administrator uses validated false-claim evidence to create a `block_payout` transaction that **spends the claimed UTXOs before the fraudulent operator can**, blocking the undue reimbursement.

**Full flow:**

1. A watchtower posts an **Ack** for a faulty operator's claim graph (or the admin supplies a Claim txid and the app discovers it on-chain).
2. A Payout Admin signer creates a `block_payout` tx spending the claim payout connector(s) derived from those claims.
3. The other signers **sign** it until quorum is reached.
4. Once quorum is reached, the tx is **broadcast to Bitcoin** — the fraudulent operator loses their claim.

---

## False claim reports (PRD §6.4.1)

Alpen's supplementary document defines what a **false claim report** actually is. It is **not** an off-chain JSON blob with a `proof` field — it is **on-chain validation** of the Claim → Contest → Ack transaction graph.

**Source (frozen client input):** [`docs/0-prd/06-supplementary-false-claim-reports.md`](../0-prd/06-supplementary-false-claim-reports.md)  
**Notion:** [Strata multisig app supplementary info](https://app.notion.com/p/Strata-multisig-app-supplementary-info-3c8901ba000f80839664e0189abc9c4c)

### Transaction graph

```
Claim  ──►  Contest  ──►  Ack
         (contest spends claim output)
                    (ack spends contest output)
```

| Term | Meaning |
|------|---------|
| **False claim** | A claim by a **faulty operator** |
| **Faulty operator** | An operator who posted a Claim tx and later had a watchtower post an **Ack** tx in the same graph |
| **False claim report** | Evidence linking the **Claim tx to block** with a prior **Ack** for the same operator |

### User input (Alpen design decision)

For uniformity, the application should **always require the Claim transaction txid**. Under the hood it fetches Contest and Ack transactions from Bitcoin and validates them.

Optional: the user may supply the **deposit index** exactly, or a range; if unknown, the implementation may brute-force over `0..max_deposit_idx`.

### Validation rules (summary)

1. **Contest authenticity:** parse the contest input witness; compare the N/N bridge key against configured `n_of_n_pubkey`.
2. **Operator identification:** the contest's 1st output ("contest proof connector") carries the operator pubkey tweaked with the game index; match against the configured operator list and deposit index.
3. **Ack format:** Ack must spend the **contest payout connector**, not the contest proof connector, and reference the correct contest txid.
4. **Same operator:** the Ack and the Claim to be blocked must belong to the **same operator**.
5. **Spent filter:** ignore claim payout outpoints already spent on-chain (PRD §6.4.1).

The reference implementation lives in `strata-bridge` (`claim_contest.rs`, related modules). Signet test claims are referenced in the supplementary doc.

### Bridge config (required)

The application needs bridge parameters to validate reports and rebuild connectors. Example shape from Alpen (signet):

| Field | Purpose |
|-------|---------|
| `network` | e.g. signet, regtest, mainnet |
| `n_of_n_pubkey` | N/N bridge key for contest authentication |
| `proof_timelock` (Δproof) | e.g. 24 blocks |
| `game_index` | `deposit_idx + 1` |
| `max_deposit_idx` | upper bound for deposit-index search |
| `operator pubkeys` | ordered x-only list |

**Dynamic operator set:** operators may be added or removed; the bridge key changes with each update. Claims may correspond to **old or new** operator lists — config cannot be blindly overwritten; historical versions must be retained.

### What the app derives after validation

From a validated Claim, the app derives the **claim payout connector outpoint(s)** to include as `block_payout` inputs (see [`04-relevant-block-payouts-transactions.md`](../0-prd/04-relevant-block-payouts-transactions.md)).

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

All state lives in React, initialized with fake data. No action (sign, create tx, rebroadcast) calls any real service.

**The mock does not implement the false claim report contract.** Step 1 of the create modal accepts fabricated JSON with a `proof` field; that format is obsolete and must not be used as a specification for real implementation.

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
2. **Orchestrator backend** — persist pending txs, collect signatures between signers, broadcast coordination
3. **Real signature validation** — Schnorr/Taproot script-path (`AdminBurn` leaf), not string-length checks
4. **False claim validation** — on-chain Claim/Contest/Ack parsing per supplementary doc, not mock JSON
5. **Bridge config** — versioned operator/N/N parameters for report validation and connector rebuild

References:
- PRD §6: [`docs/0-prd/05-prd-payout-admin-block-payouts-update.md`](../0-prd/05-prd-payout-admin-block-payouts-update.md)
- False claim reports: [`docs/0-prd/06-supplementary-false-claim-reports.md`](../0-prd/06-supplementary-false-claim-reports.md)
- Block payout tx shape: [`docs/0-prd/04-relevant-block-payouts-transactions.md`](../0-prd/04-relevant-block-payouts-transactions.md)
- Implementation estimate: [`docs/proposals/block-payouts-estimate.md`](../proposals/block-payouts-estimate.md)
- Mock UI spec (obsolete input format): [`docs/specs/block-payouts-ui-mock.md`](../specs/block-payouts-ui-mock.md)
- ASM vs Bitcoin L1 difference: [`docs/2-discovery/10-asm-bitcoin-state-model.md`](./10-asm-bitcoin-state-model.md)
