# Technical Report: Hardware Wallet Signing for Strata Block Payouts

> **Status:** Draft for external review (2026-10-20). Diagnosis only: no proposed solution and no effort estimate.
> Source-level evidence at pinned commits, checked 2026-10-07.
> Working notes behind this report: [`22-block-payouts-spike-findings.md`](./22-block-payouts-spike-findings.md).

## Executive summary

A `block_payout` spends the `AdminBurn` path of the bridge's claim payout connector. That path is a taproot
**script-path** spend over a threshold of admin keys. The bridge protocol defines its shape; the multisig app does not.

A Payout Administrator **can** sign `AdminBurn` with a software key. The upstream test `admin_burn_payout` does it and
broadcasts the result. **No stock hardware wallet that the app supports can.** The signature these devices produce for
a taproot input is a key-path signature over the output's internal key. On this connector, the internal key is the
operators' N/N aggregate, not an admin key.

Fee inputs from the Admin Wallet are not affected. They are BIP-86 key-path spends, and both device families already
sign them in the app today.

**Findings:**

- **Trezor (Safe 7, Safe 5, Safe 3 and Model T at core 2.12.5; Model One at legacy 1.14.1):** blocked at firmware.
  Taproot signing is key-path only. There is no message, input type, or witness writer for a tapscript spend.
- **Ledger (Bitcoin app through 2.5.1, on every model):** blocked by the wallet policy language. The device signs
  script paths only for outputs it can register as a wallet policy, and this output cannot be registered. Three
  independent reasons, each enough on its own: the leaf ends in `OP_EQUAL` instead of miniscript's `OP_NUMEQUAL`;
  the internal key is a raw aggregate that no derived key expression produces; and the sibling leaf requires no
  signature, which registration rejects.
- **App side:** there is nothing the multisig app can change to make either device sign this input. Every change that
  would unblock Ledger sits in the bridge protocol's script. Trezor has no path on stock firmware.

## 1. Background

The bridge locks each Claim's payout in a taproot output, the **claim payout connector** (`ClaimTx::PAYOUT_VOUT`).
In the normal case, the operators spend it by key-path with their N/N aggregate key. When a claim is false, the
Payout Administrators can block the payout by spending the same output through the `AdminBurn` leaf. A
`block_payout` transaction does that for one or more connectors, plus fee inputs from the Admin Wallet and a change
output.

The PRD requires this signature to come from the hardware signer:

- §3: the user connects a supported hardware signer to sign messages and transactions. Supported signers include
  the devices supported by HWI.
- §3: for the Payout Administrator, the Admin ID (P2TR, `m/86'/0'/73'/0/0`) on the connected hardware signer "MUST be
  used for authenticating with the multisig app backend and signing all Payout Administrator transactions".
- §6: the "Sign" button on a Pending `block_payouts` transaction triggers the signing flow.

So the admin key inside the `AdminBurn` leaf is expected to be a hardware-held key, and the device has to produce the
script-path signature for that leaf.

## 2. Objectives

This report documents:

- How the `AdminBurn` spend executes: script, tap tree, witness and sighash.
- Why stock Trezor firmware cannot sign it.
- Why the stock Ledger Bitcoin app cannot sign it, with or without miniscript.
- What would have to change for a device to sign it.

## 3. Current state

### 3.1 What works

- **Software signing of `AdminBurn`.** Upstream connector tests `admin_burn_spend_{1_of_1,1_of_3,2_of_3,3_of_3}` spend
  the leaf with software keys. `admin_burn_payout` (`game_graph.rs`, 4 admins, threshold 2) builds a version-3
  transaction with one fee input and one change output, signs it with threshold software keys, and broadcasts it.
- **Admin Wallet fee inputs on hardware.** Fee inputs are BIP-86 key-path spends. Trezor and Ledger already sign this
  class of input in the app.
- **What the app signs on hardware today:** taproot key-path spends and raw ECDSA messages (SPS-65). See the
  [hardware wallet matrix](../external/hardware-wallet-matrix.md). None of these is a script-path spend.

### 3.2 How the `AdminBurn` spend executes

Sources: `strata-bridge` at `3b69ece` (draft [PR #800](https://github.com/alpenlabs/strata-bridge/pull/800)) and at
`main` `5d3c8dc`. The files below are identical at both commits.

**The output being spent ([`claim_payout.rs`](https://github.com/alpenlabs/strata-bridge/blob/5d3c8dc4bef3246b88b2aa0cdfbe857891422e03/crates/connectors/src/claim_payout.rs)):**

| Piece | Value |
|---|---|
| **Internal key** | N/N aggregate of the operator keys (`n_of_n_pubkey`). Its key-path spend is the normal operator payout. |
| **Leaf 0, `AdminBurn`** | `threshold_multisig_script(admin_pubkeys, admin_threshold)`. The leaf a Payout Admin signs. |
| **Leaf 1, `UnstakingBurn`** | `OP_SIZE 32 OP_EQUALVERIFY OP_SHA256 <unstaking_image> OP_EQUAL`. Never spent by `block_payout`. |
| **Leaf version** | `TapScript` (`0xc0`) |
| **Tree shape** | Two leaves, both at depth 1. Leaf 0 on the left, leaf 1 its sibling. |
| **Output value** | Minimal non-dust (330 sats on signet) |

Admin keys keep the order of the bridge `params.toml` `[keys.admin]` list. The script does not sort them.

**Leaf 0 — `AdminBurn` ([`general.rs`](https://github.com/alpenlabs/strata-bridge/blob/5d3c8dc4bef3246b88b2aa0cdfbe857891422e03/crates/primitives/src/scripts/general.rs#L34), lines 34–56):**

```text
<admin_1> OP_CHECKSIG
<admin_2> OP_CHECKSIGADD
...
<admin_M> OP_CHECKSIGADD
<K> OP_EQUAL
```

Each admin key is a raw 32-byte x-only public key. Worked 2-of-3 with illustrative keys (encoding only, not curve
points), 104 bytes:

```text
20 11...11 ac      <admin_1> OP_CHECKSIG
20 22...22 ba      <admin_2> OP_CHECKSIGADD
20 33...33 ba      <admin_3> OP_CHECKSIGADD
52 87              OP_2 OP_EQUAL
```

The miniscript fragment `multi_a(2, k1, k2, k3)` compiles to the same bytes except the last one: `0x9c`
(`OP_NUMEQUAL`) instead of `0x87` (`OP_EQUAL`). Both are consensus-valid tapscript. Only the `OP_NUMEQUAL` form is
miniscript.

**Leaf 1 — `UnstakingBurn`:** `82 01 20 88 a8 20 <32-byte image> 87`. This is exactly the miniscript fragment
`sha256(h)`. It can be satisfied by revealing a preimage, with no signature.

**Control block** for an `AdminBurn` spend (65 bytes):

| Offset | Bytes | Contents |
|---|---|---|
| 0 | 1 | `0xc0` with the output-key parity in the low bit |
| 1 | 32 | Internal x-only key: the raw N/N aggregate |
| 33 | 32 | TapLeaf hash of leaf 1 (the sibling) |

**Witness (`get_taproot_witness`):** one item per admin key, in descending index order. Each item is the 64-byte
Schnorr signature if that admin signed, or an empty push otherwise. Only the `K` lowest-index signatures are kept.
For a 2-of-3 where admins 0 and 1 signed:

```text
<>                          # admin 2, empty
<64-byte sig of admin 1>
<64-byte sig of admin 0>
<AdminBurn leaf script>
<control block>
```

**Sighash (`get_signing_info`, [`connectors/src/lib.rs`](https://github.com/alpenlabs/strata-bridge/blob/5d3c8dc4bef3246b88b2aa0cdfbe857891422e03/crates/connectors/src/lib.rs)):**

- BIP-341 `SIGHASH_DEFAULT`. No sighash byte appended to the signature.
- **Script-path message:** the hash includes the leaf hash of `AdminBurn` (BIP-342 extension).
- The admin key signs **untweaked**: a plain BIP-340 signature with the admin's key.
- The signature commits to every input and output, including fee inputs and change.

The transaction (`AdminBurnTx`, `not_presigned.rs`) is version 3, locktime 0, with the connector input at sequence
`MAX`.

## 4. Root cause analysis

### 4.1 Protocol: the admin signs a script path, the device signs a key path

A hardware wallet that "supports taproot" usually means BIP-86 key-path: it tweaks its own key and signs the
key-path sighash. On the claim payout connector, the key path belongs to the operators: the output key is the N/N
aggregate tweaked by the tap tree. An admin device signing key-path would sign for a key it does not hold, with the
wrong message.

What the admin has to produce instead is a BIP-340 signature with its **untweaked** key, over the **script-path**
sighash that commits to the `AdminBurn` leaf hash. A device can only do that if its firmware has a script-path signer
and it can recognise this specific output as one it is allowed to sign.

### 4.2 Trezor: blocked at firmware

Trezor has no separate Bitcoin app. Signing lives in the firmware, and universal and bitcoin-only builds share it.
Current production firmware ([changelog](https://trezor.io/other/product-updates/trezor-firmware-changelog-all-versions-and-release-dates),
checked 2026-10-07):

| Model | Latest firmware | Taproot signing |
|---|---|---|
| Safe 7 | core 2.12.5 (16 September 2026) | Key-path only |
| Safe 5 | core 2.12.5 | Key-path only |
| Safe 3 | core 2.12.5 | Key-path only |
| Model T | core 2.12.5 | Key-path only |
| Model One | legacy 1.14.1 (18 March 2026) | Key-path only |

Evidence in `trezor-firmware` at tag `core/v2.12.5`:

- `InputScriptType` has one taproot value, `SPENDTAPROOT = 5`
  ([`messages-bitcoin.proto`](https://github.com/trezor/trezor-firmware/blob/core/v2.12.5/common/protob/messages-bitcoin.proto)).
  There is no way to describe a script-path input.
- `sign_taproot_input` signs `hash341(i, tx, sighash_type)` with the key at `address_n`
  ([`bitcoin.py`](https://github.com/trezor/trezor-firmware/blob/core/v2.12.5/core/src/apps/bitcoin/sign_tx/bitcoin.py)).
  `hash341` takes no leaf hash and writes `spend_type 0`, commented "no tapscript message extension, no annex"
  ([`sig_hasher.py`](https://github.com/trezor/trezor-firmware/blob/core/v2.12.5/core/src/apps/bitcoin/sign_tx/sig_hasher.py)).
- `write_witness_p2tr` writes a single signature, commented "Taproot key path spending without annex"
  ([`scripts.py`](https://github.com/trezor/trezor-firmware/blob/core/v2.12.5/core/src/apps/bitcoin/scripts.py)).

**Roadmap signals:**

- Draft [PR #6936](https://github.com/trezor/trezor-firmware/pull/6936) (miniscript, updated 2026-09-22) states
  "Taproot is not supported. Currently only WSH is supported." It is not in any released firmware.
- [PR #4159](https://github.com/trezor/trezor-firmware/pull/4159) (taproot multisig, open): maintainer reply on
  2025-12-26, "We do not plan to support this in the near term."
- Bitcoin Core HWI's Trezor driver carries `TODO: Support script path signing`
  ([`trezor.py`](https://github.com/bitcoin-core/HWI/blob/master/hwilib/devices/trezor.py)).

No production Trezor, Safe 7 included, can place an admin signature in the `AdminBurn` witness.

### 4.3 Ledger: the output cannot be registered as a wallet policy

The Bitcoin app is the same binary on Nano S, Nano S Plus, Nano X, Stax, Flex and Apex. Current release is 2.5.1
(9 September 2026, [`CHANGELOG.md`](https://github.com/LedgerHQ/app-bitcoin-new/blob/master/CHANGELOG.md)).

Ledger does sign taproot script paths, but only for outputs that match a **registered wallet policy** (BIP-388): a
descriptor template plus a list of keys. The device derives the expected `scriptPubKey` from the policy and signs an
input only if it matches.

| App version | Taproot script-path support | Can it sign AdminBurn? |
|---|---|---|
| Up to 2.1.1 (incl. 2.0.x) | None. Key-path `tr(KP)` only. | No |
| 2.1.2 (3 April 2023) | Up to 8 leaves; leaves limited to `pk`, `multi_a`, `sortedmulti_a` | No |
| 2.2.0 (29 January 2024) to 2.5.1 | Full taproot miniscript; `musig()` from 2.4.0 | No: three blockers below |

**Policy language rules ([`doc/wallet.md`](https://github.com/LedgerHQ/app-bitcoin-new/blob/master/doc/wallet.md)):**

- A tap leaf must be `multi_a`, `sortedmulti_a`, or a valid taproot miniscript.
- Keys are **xpubs**, never raw public keys, and every key is **derived** (`/**` or `/<M;N>/*`).
- `musig()` keys (2.4.0+) are aggregated from at most 5 xpubs and then derived
  ([`doc/musig.md`](https://github.com/LedgerHQ/app-bitcoin-new/blob/dab93a1af0e623d107c227c35a3fb51da4f7585c/doc/musig.md)).
- Every leaf must be sane. A leaf that can be satisfied without a signature fails registration with
  "Miniscript does not always require a signature"
  ([`policy.c`](https://github.com/LedgerHQ/app-bitcoin-new/blob/dab93a1af0e623d107c227c35a3fb51da4f7585c/src/handler/lib/policy.c#L1841-L1844),
  `EC_REGISTER_WALLET_POLICY_NOT_SANE`).

Three blockers, each sufficient on its own:

| # | Blocker | Connector | What Ledger requires |
|---|---|---|---|
| 1 | **Leaf shape** | `<K> OP_EQUAL` | `multi_a` ends in `<k> OP_NUMEQUAL` |
| 2 | **Internal key** | Raw BIP-327 aggregate of operator keys, not derived after aggregation | A derived key expression; `musig(...)` is derived after aggregation |
| 3 | **Sibling leaf** | `UnstakingBurn` = `sha256(h)`, needs no signature | Every leaf must require a signature |

A fourth mismatch sits inside leaf 0: the admin keys are raw x-only keys from `params.toml`, not xpub children at a
derivation step. Blockers 1–3 already close the policy.

Changing only `OP_EQUAL` to `OP_NUMEQUAL` removes blocker 1 and leaves 2 and 3 in place.

### 4.4 Per-claim tap trees

Leaf 1 embeds the unstaking image, and the internal key is the operator aggregate active at the claim. Both are
per-claim parameters of the connector. A wallet policy is registered once per wallet on the device. If the tree
differs per claim, each claim would be a different policy. This is a structural concern for any policy-based device,
on top of blockers 1–3. It has not been measured yet.

### 4.5 Why this cannot be fixed in the app

The app does not choose the script, the internal key, or the tree. They are fixed by the bridge protocol, and the
on-chain output must match them byte for byte. The app can only build a PSBT for that output and hand it to the
device:

- On Trezor, there is no firmware message to receive it.
- On Ledger, there is no policy that describes it, so the device treats the input as external and does not sign.

## 5. What would have to change

Diagnostic only. This lists the conditions under which a stock device could sign, not a recommendation.

**Ledger** could register and sign the output if **all** of these hold in the bridge protocol:

1. The `AdminBurn` leaf ends in `OP_NUMEQUAL` (miniscript `multi_a`).
2. Each admin key is an xpub child at the same `/<M;N>/*` index as the rest of the policy.
3. The N/N internal key is a derived aggregate, `musig(...)/<M;N>/*`, of at most 5 operator xpubs.
4. `UnstakingBurn` requires a signature, or leaves the tree.

Section 4.4 would still need an answer.

**Trezor:** no change in the bridge protocol is enough. Stock firmware has no script-path signer.

**Custom firmware or a custom Ledger app** could sign the leaf. That is outside stock devices and outside the
supported device matrix.

## 6. References

- `strata-bridge` `main` @ `5d3c8dc`:
  [`claim_payout.rs`](https://github.com/alpenlabs/strata-bridge/blob/5d3c8dc4bef3246b88b2aa0cdfbe857891422e03/crates/connectors/src/claim_payout.rs),
  [`general.rs`](https://github.com/alpenlabs/strata-bridge/blob/5d3c8dc4bef3246b88b2aa0cdfbe857891422e03/crates/primitives/src/scripts/general.rs),
  [`connectors/src/lib.rs`](https://github.com/alpenlabs/strata-bridge/blob/5d3c8dc4bef3246b88b2aa0cdfbe857891422e03/crates/connectors/src/lib.rs),
  [`not_presigned.rs`](https://github.com/alpenlabs/strata-bridge/blob/5d3c8dc4bef3246b88b2aa0cdfbe857891422e03/crates/tx-graph/src/transactions/not_presigned.rs),
  [`game_graph.rs`](https://github.com/alpenlabs/strata-bridge/blob/5d3c8dc4bef3246b88b2aa0cdfbe857891422e03/crates/tx-graph/src/game_graph.rs).
  Same files at `3b69ece` (draft [PR #800](https://github.com/alpenlabs/strata-bridge/pull/800)).
- Trezor firmware `core/v2.12.5`:
  [`bitcoin.py`](https://github.com/trezor/trezor-firmware/blob/core/v2.12.5/core/src/apps/bitcoin/sign_tx/bitcoin.py),
  [`sig_hasher.py`](https://github.com/trezor/trezor-firmware/blob/core/v2.12.5/core/src/apps/bitcoin/sign_tx/sig_hasher.py),
  [`scripts.py`](https://github.com/trezor/trezor-firmware/blob/core/v2.12.5/core/src/apps/bitcoin/scripts.py),
  [`messages-bitcoin.proto`](https://github.com/trezor/trezor-firmware/blob/core/v2.12.5/common/protob/messages-bitcoin.proto).
- Ledger `app-bitcoin-new`: [`doc/wallet.md`](https://github.com/LedgerHQ/app-bitcoin-new/blob/master/doc/wallet.md),
  [`doc/musig.md`](https://github.com/LedgerHQ/app-bitcoin-new/blob/dab93a1af0e623d107c227c35a3fb51da4f7585c/doc/musig.md),
  [`policy.c`](https://github.com/LedgerHQ/app-bitcoin-new/blob/dab93a1af0e623d107c227c35a3fb51da4f7585c/src/handler/lib/policy.c).
- [BIP-341](https://github.com/bitcoin/bips/blob/master/bip-0341.mediawiki),
  [BIP-342](https://github.com/bitcoin/bips/blob/master/bip-0342.mediawiki),
  [BIP-388](https://github.com/bitcoin/bips/blob/master/bip-0388.mediawiki).
- PRD §3 and §6: [`06-prd-hardware-signer-and-block-payouts-update.md`](../0-prd/06-prd-hardware-signer-and-block-payouts-update.md).
- Working notes with the full disassembly and leaf hashes: [`22-block-payouts-spike-findings.md`](./22-block-payouts-spike-findings.md).
