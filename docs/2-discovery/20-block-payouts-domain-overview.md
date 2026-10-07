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

**Source (frozen client input):** [`docs/0-prd/07-supplementary-false-claim-reports.md`](../0-prd/07-supplementary-false-claim-reports.md)  
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

### User input (Alpen design proposal)

For uniformity, Alpen's author proposes to **always require the Claim transaction txid**. Under the hood the app fetches Contest and Ack transactions from Bitcoin and validates them. This is a proposal ("I think…"), not a settled requirement.

Optional: the user may supply the **deposit index** exactly, or a range; if unknown, the implementation may brute-force over `0..max_deposit_idx`.

### Validation rules (summary)

1. **Contest authenticity:** parse the contest input witness; compare the N/N bridge key against configured `n_of_n_pubkey`.
2. **Operator identification:** the contest's 1st output ("contest proof connector") carries the operator pubkey tweaked with the game index; match against the configured operator list and deposit index.
3. **Ack format:** Ack must spend the **contest payout connector**, not the contest proof connector, and reference the correct contest txid.
4. **Same operator:** the Ack and the Claim to be blocked must belong to the **same operator**.
5. **Spent filter:** ignore claim payout outpoints already spent on-chain (PRD §6.4.1).

### Reference implementation (`strata-bridge` @ `3b69ece`)

Alpen's reference lives in the `tx-graph` crate. Both modules are pure functions with no I/O, tested against real signet transactions. They exist only in the unmerged draft [PR #800](https://github.com/alpenlabs/strata-bridge/pull/800) ("[DO NOT MERGE]"); `main` @ `5d3c8dc` (checked 2026-10-05) does not include them.

| Function | What it does |
|----------|--------------|
| [`verify_contest(params, tx, candidate)`](https://github.com/alpenlabs/strata-bridge/blob/3b69ece5068b76def66dda15ef6f4fa747087b54/crates/tx-graph/src/verify_contest.rs#L210-L241) | Authenticates a Contest (N/N leaf in the input witness) and returns `GameId { operator_idx, deposit_idx }`. Without a candidate it searches every operator × deposit index up to `max_deposit_idx`. |
| [`verify_ack(source, params, claim_txid, candidate)`](https://github.com/alpenlabs/strata-bridge/blob/3b69ece5068b76def66dda15ef6f4fa747087b54/crates/tx-graph/src/verify_ack.rs#L190-L217) | Walks Claim → Contest → spender of the contest payout connector, and reports whether that spender is a watchtower Ack (as opposed to the bridge proof timeout or the contested payout). |
| [`VerifyParams`](https://github.com/alpenlabs/strata-bridge/blob/3b69ece5068b76def66dda15ef6f4fa747087b54/crates/tx-graph/src/verify_contest.rs#L24-L38) | The bridge config struct: network, N/N key, proof timelock, operator keys, `max_deposit_idx`. |

Signet test claims from the supplementary doc, both used as fixtures in the reference tests:

- [`f599a165…71e9`](https://mempool.space/signet/tx/f599a16569f0ad0fdafb4fa0dabcb4be4776447afe327a14deff63f0a41671e9) — operator 3, deposit 1; its game ended in a proof timeout, **not** an Ack.
- [`eaf25a3e…7716`](https://mempool.space/signet/tx/eaf25a3ef1a0ada03f8864dd65adb8367333ac050aedd2e4a2f835cc2b607716) — operator 2, deposit 0; its game ends in a real watchtower Ack.

Constraints the reference imposes on our implementation:

- **Outspend lookups.** `verify_ack` needs a source that answers "which transaction spends this outpoint". A bare `bitcoind` cannot, because spent outputs leave the UTXO set. Our stack runs electrs 0.10.7 (Electrum protocol only, no direct outspend call). The spender can be derived from `blockchain.scripthash.get_history` on the output's script, which the app already uses in `desktop-app/src-tauri/src/infrastructure/tx_lookups.rs`.
- **Confirmed transactions only.** Authentication relies on consensus having validated the N/N signature. Applied to a mempool or off-chain transaction, the check proves nothing.

**Open question for Alpen.** `verify_ack` answers whether the game of *the given* Claim reached an Ack. Blocking *other* claims of an operator already marked faulty (design decision 1) requires identifying the operator of that other Claim, which needs its Contest. If that Claim has no Contest yet, the reference returns `AckError::NoContest` and there is no authenticated binding to an operator. The expected app behaviour in that case is undefined.

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

**Upstream shape on `main`.** The bridge `params.toml` ([`params.rs` @ `5d3c8dc`](https://github.com/alpenlabs/strata-bridge/blob/5d3c8dc4bef3246b88b2aa0cdfbe857891422e03/crates/common/src/params.rs)) already models this:

- `[[keys.operators]]` is an operator set schedule: index, covenant key, P2P key, payout descriptor, `activation_height`, optional `deactivation_height`. The N/N for a claim is the aggregate of the operators active at its height (`CovenantId`).
- `[keys.admin]` holds the admin multisig (`pubkeys` in script order, `threshold`) used by the `AdminBurn` leaf.
- Params are consensus-critical and fixed at genesis, so changing the admin set is a coordinated params change across operators.
- The unstaking image is not in params: it is per operator stake (`KeyData.unstaking_image`, exchanged at setup).

### What the app derives after validation

From a validated Claim, the app derives the **claim payout connector outpoint(s)** to include as `block_payout` inputs (see [`04-relevant-block-payouts-transactions.md`](../0-prd/04-relevant-block-payouts-transactions.md)).

---

## The block payout transaction (`strata-bridge` @ `3b69ece`)

Upstream calls it the **Admin Burn** transaction. The connector, leaf, `AdminBurnTx` and sighash below are unchanged on `main` @ `5d3c8dc`. What the code fixes:

**Claim payout connector** ([`claim_payout.rs`](https://github.com/alpenlabs/strata-bridge/blob/3b69ece5068b76def66dda15ef6f4fa747087b54/crates/connectors/src/claim_payout.rs)), output `ClaimTx::PAYOUT_VOUT` of the Claim:

- Internal key: N/N (key path = normal payout).
- Leaf 0, `AdminBurn`: `<admin_1> OP_CHECKSIG <admin_2> OP_CHECKSIGADD … <admin_M> OP_CHECKSIGADD <K> OP_EQUAL`. Keys keep the configured order (not sorted). The final opcode is `OP_EQUAL`, so the leaf is **not** miniscript `multi_a` (which ends in `OP_NUMEQUAL`).
- Leaf 1, `UnstakingBurn`: `OP_SIZE 32 OP_EQUALVERIFY OP_SHA256 <unstaking_image> OP_EQUAL`. We never spend it, but its leaf hash is needed in the control block of every `AdminBurn` spend.
- Value: minimal non-dust for its script (330 sats on signet).

**Witness for `AdminBurn`:** one entry per admin key in **descending** key index — the signature if that key signed, an empty push otherwise — then the leaf script and the control block. Exactly `K` signatures are kept.

**Sighash:** `TapSighashType::Default` for every connector spend. Each signer commits to the whole transaction, fee input and change included.

**Transaction builder:** [`AdminBurnTx`](https://github.com/alpenlabs/strata-bridge/blob/3b69ece5068b76def66dda15ef6f4fa747087b54/crates/tx-graph/src/transactions/not_presigned.rs) spends **one** connector in input 0 and leaves other inputs and outputs malleable. It uses **version 3 (TRUC)**, whose size limit is 10,000 vB. PRD §6.4.2 wants several connectors per transaction up to the standard size limit, so the app needs its own builder (or an extension) and an explicit version 2 / version 3 decision.

**Full shape required by the PRD:** N connector inputs (script-path, `AdminBurn`) + fee input(s) from the creator's Admin Wallet (key-path, BIP-86) + one change output to the creator's Admin Wallet.

### Hardware signing constraint

S1a (2026-10-07) checked current firmware against the disassembled leaf. A Payout Admin can sign `AdminBurn` in
software (upstream's `admin_burn_payout` test). No stock device this app supports can sign that input:

- **Trezor**, including Safe 7 at core 2.12.5 (16 September 2026), Safe 5, Safe 3, and Model T on the same tag, and
  Model One on legacy 1.14.1: taproot signing is key-path only. The witness writer emits a single Schnorr signature
  and no tapscript extension.
- **Ledger** Bitcoin app through 2.5.1 (9 September 2026): apps before tapleaves have no script-path signer. Apps
  with miniscript (`multi_a` / taproot miniscript, from 2.1.2 and 2.2.0 onward) still cannot register this output.
  The leaf ends in `OP_EQUAL` rather than `OP_NUMEQUAL`. The N/N internal key is a raw aggregate, and every
  Ledger key expression, `musig()` included, is derived. The `UnstakingBurn` sibling is miniscript `sha256(h)`, but
  registration rejects it because it requires no signature.

Fee inputs are unaffected: key-path signing from the Admin Wallet already works on both. Script, witness, and the
firmware citations: [`22-block-payouts-spike-findings.md`](./22-block-payouts-spike-findings.md). Spike plan:
[`21-block-payouts-spike-plan.md`](./21-block-payouts-spike-plan.md).

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
| Signature | ECDSA message signature (BIP-137) | Schnorr (BIP-340) over a taproot script-path sighash |

**Open question:** which key the bridge puts in the `AdminBurn` leaf for each admin — the BIP-86 internal key at `m/86'/0'/73'/0/0` (untweaked) or the output key of the Admin ID address (tweaked). Devices sign script-path spends with the untweaked key only, and signer membership checks must compare against the same form.

---

## What comes next (out of scope for the mock)

When the real backend is integrated, the connection points are:

1. **Tauri IPC** — derive the P2TR address from the hardware wallet (BIP-86) to authenticate the signer
2. **Orchestrator backend** — persist pending txs, collect signatures between signers, broadcast coordination
3. **Real signature validation** — Schnorr over the script-path sighash of the `AdminBurn` leaf, not string-length checks. The existing `verify_threshold` is ECDSA and does not cover this.
4. **False claim validation** — on-chain Claim/Contest/Ack parsing per supplementary doc, not mock JSON
5. **Bridge config** — versioned operator/N/N parameters for report validation and connector rebuild

References:
- PRD §6: [`docs/0-prd/06-prd-hardware-signer-and-block-payouts-update.md`](../0-prd/06-prd-hardware-signer-and-block-payouts-update.md) (latest; §6 unchanged from `05`)
- False claim reports: [`docs/0-prd/07-supplementary-false-claim-reports.md`](../0-prd/07-supplementary-false-claim-reports.md)
- Block payout tx shape: [`docs/0-prd/04-relevant-block-payouts-transactions.md`](../0-prd/04-relevant-block-payouts-transactions.md)
- Spike plan: [`docs/2-discovery/21-block-payouts-spike-plan.md`](./21-block-payouts-spike-plan.md)
- Spike findings (S1a hardware desk research): [`docs/2-discovery/22-block-payouts-spike-findings.md`](./22-block-payouts-spike-findings.md)
- Mock UI spec (obsolete input format): [`docs/specs/block-payouts-ui-mock.md`](../specs/block-payouts-ui-mock.md)
- ASM vs Bitcoin L1 difference: [`docs/2-discovery/10-asm-bitcoin-state-model.md`](./10-asm-bitcoin-state-model.md)
