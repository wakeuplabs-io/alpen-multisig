# Block Payouts — Spike Findings

> **Status:** S1a recorded (2026-10-07). Later steps append a section here.
> Discovery material — not SSOT, not committed scope.
> Plan: [`21-block-payouts-spike-plan.md`](./21-block-payouts-spike-plan.md).

## Track A — S1a desk research: can an admin sign `AdminBurn` on Trezor or Ledger?

**Verdict.** A Payout Admin can sign the `AdminBurn` input in software. No stock Trezor, including Safe 7, and no
stock Ledger Bitcoin app, with or without miniscript, can sign that input. The signature those devices know how to
produce for a taproot input is a key-path BIP-340 signature over the internal key, and that internal key is the
operators' N/N aggregate, not an admin key.

This is a source-level no-go for Track A on stock firmware. It does not replace S1: the emulator step still records
the exact device error. Fee inputs from the Admin Wallet stay on the key-path the app already signs.

Checked 2026-10-07. Upstream pins compared byte-for-byte: `strata-bridge` @ `3b69ece` (draft
[PR #800](https://github.com/alpenlabs/strata-bridge/pull/800)) and `main` @ `5d3c8dc`. Identical at both commits:
`crates/primitives/src/scripts/general.rs`, `crates/connectors/src/claim_payout.rs`, `crates/connectors/src/lib.rs`,
`crates/tx-graph/src/transactions/not_presigned.rs`, `crates/tx-graph/src/game_graph.rs`.
`crates/primitives/src/scripts/taproot.rs` differs by a comment line-wrap in a test helper. `build_taptree` and
`create_script_spend_hash` are unchanged.

### What is being spent

A `block_payout` spends one or more **claim payout connectors**. Each connector is the
`ClaimTx::PAYOUT_VOUT` output of a Claim. [`ClaimPayoutConnector`](https://github.com/alpenlabs/strata-bridge/blob/3b69ece5068b76def66dda15ef6f4fa747087b54/crates/connectors/src/claim_payout.rs)
builds the taproot output as follows.

| Piece | Value |
|---|---|
| Internal key | N/N aggregate (`n_of_n_pubkey`). Key-path spend is the normal operator payout, tweaked by the merkle root. |
| Leaf 0, `AdminBurn` | `threshold_multisig_script(admin_pubkeys, admin_threshold)`. This is the leaf a Payout Admin signs. |
| Leaf 1, `UnstakingBurn` | `OP_SIZE 32 OP_EQUALVERIFY OP_SHA256 <unstaking_image> OP_EQUAL`. Never spent by `block_payout`. Its leaf hash is in the control block of every `AdminBurn` spend. |
| Leaf version | `LeafVersion::TapScript` (`0xc0`). |
| Tree shape | Two scripts, so both leaves sit at depth 1 (`build_taptree` in [`taproot.rs`](https://github.com/alpenlabs/strata-bridge/blob/3b69ece5068b76def66dda15ef6f4fa747087b54/crates/primitives/src/scripts/taproot.rs)). Leaf 0 is added first (left). Leaf 1 is the right sibling. |
| Output value | `script_pubkey().minimal_non_dust()` (330 sats on signet). |

Admin pubkeys keep the order from bridge `params.toml` `[keys.admin]`. The script does not sort them. Duplicate
pubkeys, an empty set, a zero threshold, or a threshold above the key count are rejected at construction.

### Leaf 0 — `AdminBurn` script

[`threshold_multisig_script`](https://github.com/alpenlabs/strata-bridge/blob/3b69ece5068b76def66dda15ef6f4fa747087b54/crates/primitives/src/scripts/general.rs#L34)
(`general.rs` lines 34–56 at both pins):

```text
<admin_1> OP_CHECKSIG
<admin_2> OP_CHECKSIGADD
…
<admin_M> OP_CHECKSIGADD
<K> OP_EQUAL
```

Each admin key is a 32-byte x-only pubkey (`push_slice` of `XOnlyPublicKey::serialize`, push opcode `0x20`).
The first key is checked with `OP_CHECKSIG` (`0xac`). Every later key is checked with `OP_CHECKSIGADD` (`0xba`).
`K` is `Builder::push_int(threshold)`: for `K` in `1..=16` that is `OP_1`..`OP_16`. The last opcode is
`OP_EQUAL` (`0x87`).

The unit test `threshold_multisig_script_matches_spec_shape` locks the 2-of-3 shape ending in `OP_2 OP_EQUAL`.
The connector tests `admin_burn_spend_{1_of_1,1_of_3,2_of_3,3_of_3}` spend that leaf with software keys.
`admin_burn_payout` in `game_graph.rs` spends it inside a version-3 transaction (`N_ADMINS = 4`,
`ADMIN_THRESHOLD = 2`).

Worked 2-of-3, illustrative keys `0x11…11`, `0x22…22`, `0x33…33` (32 bytes each). These bytes document the
encoding. They are not a live connector and are not asserted to be curve points.

```text
20 1111…11 ac
20 2222…22 ba
20 3333…33 ba
52 87
```

Script hex (104 bytes):

```text
201111111111111111111111111111111111111111111111111111111111111111ac
202222222222222222222222222222222222222222222222222222222222222222ba
203333333333333333333333333333333333333333333333333333333333333333ba
5287
```

Miniscript `multi_a(2, k1, k2, k3)` is the same script with the last byte changed from `0x87` (`OP_EQUAL`) to
`0x9c` (`OP_NUMEQUAL`):

```text
…ba529c
```

rust-miniscript compiles `MultiA` / `SortedMultiA` as
`<pk> OP_CHECKSIG <pk> OP_CHECKSIGADD … <k> OP_NUMEQUAL`
([`astelem.rs`](https://github.com/rust-bitcoin/rust-miniscript/blob/master/src/miniscript/astelem.rs)).
Ledger's policy compiler emits the same sequence and comments it as such
([`policy.c`](https://github.com/LedgerHQ/app-bitcoin-new/blob/master/src/handler/lib/policy.c) `process_multi_a_sortedmulti_a_node`,
`k` limited to 16). Bitcoin Core's `MatchMultiA` returns no match unless `script.back() == OP_NUMEQUAL`.

`OP_EQUAL` still requires the `OP_CHECKSIGADD` accumulator to equal `K`. The leaf is consensus-valid tapscript.
It is outside the miniscript grammar because that grammar's threshold fragment ends in `OP_NUMEQUAL`.
`sortedmulti_a` would also sort the keys; this leaf keeps `params.toml` order, so the matching fragment, if the
final opcode were changed, would be `multi_a`, not `sortedmulti_a`.

### Leaf 1 — `UnstakingBurn` script

From `ClaimPayoutConnector::leaf_scripts` (`claim_payout.rs` lines 82–99):

```text
OP_SIZE
<32> OP_EQUALVERIFY
OP_SHA256
<unstaking_image> OP_EQUAL
```

`push_int(0x20)` is outside `1..=16`, so 32 is a one-byte script number: `0x01 0x20`. Then `OP_EQUALVERIFY`
(`0x88`), `OP_SHA256` (`0xa8`), a 32-byte push of the image, `OP_EQUAL` (`0x87`).

Worked example, image `0x44…44` (39 bytes):

```text
82 01 20 88 a8 20 4444…44 87
```

```text
82012088a820444444444444444444444444444444444444444444444444444444444444444487
```

This leaf is the miniscript fragment `sha256(h)`: `SIZE <20> EQUALVERIFY SHA256 <h> EQUAL`, byte for byte. It is
valid miniscript, but it is not *sane*: nothing in it requires a signature. `block_payout` does not spend it. Every
`AdminBurn` control block still commits to its leaf hash.

### Tap tree and control block

`build_taptree` puts both scripts at depth 1. The merkle path for leaf 0 is one node: the TapLeaf hash of leaf 1.

BIP341 TapLeaf hash, version `0xc0`, for the worked scripts above:

| Leaf | TapLeaf hash |
|---|---|
| 0 `AdminBurn` | `ad457948b1b515e6955a31591462e5a4583d08064a278a66ec4b0d46071a6531` |
| 1 `UnstakingBurn` | `c627dc1b35681184af62c8a6da9f663419de8a0a58d40ac117b6d81236548841` |

Leaf 0 sorts before leaf 1, so the branch is
`TapBranch(leaf0 ‖ leaf1)` =
`ada37a908b4e50817921f838e7d815c278824f2025f3fd7b436c9e7aac0e7432`.
The control block does not carry that branch hash. It carries the sibling.

Control block for an `AdminBurn` spend of this two-leaf tree (65 bytes):

| Offset | Bytes | Contents |
|---|---|---|
| 0 | 1 | `0xc0` (tapscript) with the output-key parity bit set in the low bit (`0xc0` or `0xc1`) |
| 1 | 32 | Internal x-only key, the raw N/N aggregate |
| 33 | 32 | Sibling TapLeaf hash (leaf 1) |

`finalize_input` (`connectors/src/lib.rs`) builds the witness as: script inputs, then the leaf script bytes, then
`control_block.serialize()`.

### Witness and sighash

`ClaimPayoutConnector::get_taproot_witness` for `AdminBurn` (`claim_payout.rs`):

1. Drop unknown pubkey indices and duplicate indices.
2. Sort the remaining signatures by pubkey index ascending, then keep only the first `K` (the lowest indices).
3. Emit one witness item per admin key in **descending** index order: the 64-byte Schnorr signature if that index
   was kept, otherwise an empty push.
4. The connector then appends the leaf script and the control block.

For a 2-of-3 where indices 0 and 1 signed, the stack (first item is the one executed first) is:

```text
<>                          # index 2, empty
<64-byte sig of index 1>
<64-byte sig of index 0>
<AdminBurn leaf script>
<control block>
```

The test `admin_burn_witness_truncates_surplus_indexed_signatures` locks that order: extra signatures past `K`
are dropped, and a missing key is an empty item, not a gap.

Sighash, for every connector spend (`get_signing_info` in `connectors/src/lib.rs`):

- `TapSighashType::Default` (BIP341 `SIGHASH_DEFAULT`, equivalent to `SIGHASH_ALL`, and the sighash byte is **not**
  appended to the 64-byte signature).
- Script-path: `create_script_spend_hash` hashes with the leaf hash of the spent script
  (`taproot_script_spend_signature_hash`). There is no `OP_CODESEPARATOR` in this leaf, so the extension uses the
  default codeseparator position.
- The signing key is **not** tweaked (`TaprootTweak::Script`). `SigningInfo::sign` calls `keypair.sign_schnorr`.
- The signature commits to every input and output. Fee inputs and the change output are inside the sighash.

`AdminBurnTx` (`not_presigned.rs`) is version 3, locktime 0, one connector input at `ClaimTx::PAYOUT_VOUT`,
sequence `Sequence::MAX`. Further inputs and outputs are pushed by the caller. The `admin_burn_payout` test pushes
one coinbase fee input and one change output, signs threshold admin keys in software, and broadcasts.

### Protocol answer

An admin can sign. The signature is a BIP-340 Schnorr signature over the script-path sighash above, placed in the
witness slot of that admin's pubkey index. Upstream's `admin_burn_payout` test does this with software keys and
expects `bitcoind` to accept the version-3 transaction. Quorum is exactly `K` of those signatures.

That is independent of the hardware question. The Admin Wallet fee input is a separate key-path spend
(BIP-86). Both Trezor and Ledger already sign that class of input. It is not the `AdminBurn` leaf.

### Trezor — every production model, including Safe 7

Trezor does not ship a separate Bitcoin app. Bitcoin signing lives in the firmware. Universal and bitcoin-only
builds of a given version share that code. Current production firmware, from
[Trezor's firmware changelog](https://trezor.io/other/product-updates/trezor-firmware-changelog-all-versions-and-release-dates)
(checked 2026-10-07):

| Model | Latest firmware | Bitcoin signing code |
|---|---|---|
| Safe 7 | core **2.12.5** (16 September 2026) | `trezor-firmware` tag `core/v2.12.5` |
| Safe 5 | core 2.12.5 (same date) | same tag |
| Safe 3 | core 2.12.5 (same date) | same tag |
| Model T | core 2.12.5 (same date) | same tag |
| Model One | legacy **1.14.1** (18 March 2026) | legacy line, not core 2.12.5 |

On `core/v2.12.5`, `Bitcoin.sign_taproot_input` derives `txi.address_n` and BIP-340-signs
`hash341(i, tx, sighash_type)`. `hash341` writes `spend_type 0` and returns. The comment in `sig_hasher.py` is
"no tapscript message extension, no annex". There is no leaf hash argument. `InputScriptType` has a single
taproot value, `SPENDTAPROOT = 5`. `write_witness_p2tr` writes one witness item and comments "Taproot key path
spending without annex". Sources:

- [`sign_tx/bitcoin.py`](https://github.com/trezor/trezor-firmware/blob/core/v2.12.5/core/src/apps/bitcoin/sign_tx/bitcoin.py)
  `sign_taproot_input` (around line 656) and the `SPENDTAPROOT` branch (around line 676).
- [`sign_tx/sig_hasher.py`](https://github.com/trezor/trezor-firmware/blob/core/v2.12.5/core/src/apps/bitcoin/sign_tx/sig_hasher.py)
  `hash341` (around line 129), `spend_type 0` at the end of the message.
- [`scripts.py`](https://github.com/trezor/trezor-firmware/blob/core/v2.12.5/core/src/apps/bitcoin/scripts.py)
  `write_witness_p2tr`.
- [`messages-bitcoin.proto`](https://github.com/trezor/trezor-firmware/blob/core/v2.12.5/common/protob/messages-bitcoin.proto)
  `InputScriptType`.

Safe 7 (internal model T3W1) runs that same core. Nothing in the 2.12.5 changelog adds tapscript or miniscript.
The open draft [PR #6936](https://github.com/trezor/trezor-firmware/pull/6936) (miniscript, updated 2026-09-22,
still a draft) lists "support TS7" only as a Trezor-client THP TODO, and states "Taproot is not supported.
Currently only WSH is supported." It is not in the 2.12.5 firmware. [PR #4159](https://github.com/trezor/trezor-firmware/pull/4159)
(taproot multisig, still open) received a maintainer reply on 2025-12-26: "We do not plan to support this in the
near term." Bitcoin Core HWI still has `TODO: Support script path signing` and only matches
`tap_internal_key` ([`hwilib/devices/trezor.py`](https://github.com/bitcoin-core/HWI/blob/master/hwilib/devices/trezor.py)).

Model One's legacy firmware speaks the same `SPENDTAPROOT` input type. HWI's script-path TODO is not gated on
model. No production Trezor image, Safe 7 included, can place an admin signature into the `AdminBurn` witness.

A Trezor key-path signature on this connector would sign the tweaked N/N output key. The admin does not hold that
key. The operators do.

### Ledger — without miniscript, and with it

The Bitcoin app is one binary across Nano S, Nano S Plus, Nano X, Stax, Flex, and Apex. Bitcoin Test is the same
app for coin type `1'`. Current release: **2.5.1** (9 September 2026),
[`CHANGELOG.md`](https://github.com/LedgerHQ/app-bitcoin-new/blob/master/CHANGELOG.md).

| App generation | What taproot signing exists | Can it sign `AdminBurn`? |
|---|---|---|
| Before 2.1.2 (through 2.1.1, and the 2.0.x line) | Key-path `tr(KP)` only. No tapleaves. | No. There is no script-path signer. |
| 2.1.2 (3 April 2023) | Tap trees of at most 8 leaves. The only tapleaf scripts are `pk`, `multi_a`, and `sortedmulti_a`. | No. `AdminBurn` ends in `OP_EQUAL`, and the sibling leaf is none of those three. |
| 2.2.0 (29 January 2024) through **2.5.1** | Full taproot miniscript, plus `multi_a` / `sortedmulti_a`. 2.4.0 adds `musig()` key expressions. 2.5.1 allows `musig()` inside `multi_a`. | No. Three independent mismatches, below. |

Current policy language, [`doc/wallet.md`](https://github.com/LedgerHQ/app-bitcoin-new/blob/master/doc/wallet.md)
on `master` at the 2.5.1 line:

- A taproot tree leaf must be `multi_a(...)`, `sortedmulti_a(...)`, or a valid taproot miniscript template.
- The key information vector is a list of **xpubs** (optionally with origin). The device decodes each entry into
  `serialized_extended_pubkey_t` (`get_pubkey_from_merkle_tree` in `policy.c`). The 2.1.0 policy doc stated the
  same limit in one sentence: "Key expressions only support xpubs at this time (no hex-encoded pubkeys)."
- Every key expression is derived. A placeholder must be followed by `/**` or `/<M;N>/*`, with unhardened `M` and
  `N` ([`wallet.c`](https://github.com/LedgerHQ/app-bitcoin-new/blob/dab93a1af0e623d107c227c35a3fb51da4f7585c/src/common/wallet.c#L545-L556)
  for V2 policies; V1 policies must end in `/**`). The key the device uses is always a child two levels below an
  xpub, never the xpub's own point.
- `musig()` is allowed as a taproot key expression (internal key or a tapleaf key) from app 2.4.0. Only
  `musig(...)/**` and `musig(...)/<M;N>/*` are supported: at most 5 participant xpubs, aggregated first, and the
  aggregate is then derived. Participants derived before aggregation are not supported
  ([`doc/musig.md`](https://github.com/LedgerHQ/app-bitcoin-new/blob/dab93a1af0e623d107c227c35a3fb51da4f7585c/doc/musig.md)).
- Registration rejects any miniscript leaf that is not sane. Each tapleaf is checked, and a leaf that can be
  satisfied without a signature fails with "Miniscript does not always require a signature"
  ([`policy.c`](https://github.com/LedgerHQ/app-bitcoin-new/blob/dab93a1af0e623d107c227c35a3fb51da4f7585c/src/handler/lib/policy.c#L1841-L1844),
  status `EC_REGISTER_WALLET_POLICY_NOT_SANE` in `register_wallet.c`).

Three blockers, each enough on its own. Changing `OP_EQUAL` to `OP_NUMEQUAL` removes only the first.

1. **The leaf is not `multi_a`.** Ledger emits `<k> OP_NUMEQUAL`. This leaf emits `<K> OP_EQUAL`. A policy parser
   that only accepts the miniscript fragment will not register the script that is actually on chain. (With a
   single admin, `<A> OP_CHECKSIG OP_1 OP_EQUAL` is miniscript `thresh(1,pk(A))`. Blocker 2 still applies.)
2. **The internal key cannot be expressed.** It is the BIP-327 KeyAgg of the raw operator pubkeys active at the
   claim height, with no derivation after aggregation (`key_agg.rs`, `TaprootTweak::Script`). Aggregation only
   needs public keys, so missing seeds is not the obstacle. Derivation is: every Ledger key expression, `musig()`
   included, ends in a derivation step, and no xpub derives to a given point. A plain `@i/**` cannot produce the
   aggregate either. A wallet policy has nowhere to put a raw x-only internal key.
3. **The sibling leaf fails registration.** `UnstakingBurn` is miniscript `sha256(h)`, so the policy language can
   write it. Registration then rejects the policy because that leaf requires no signature. Registering the tree
   means registering every leaf, so the output the admin is spending cannot be registered, even if leaf 0 were
   rewritten.

`AdminBurn` keys are also raw x-only pubkeys from `params.toml`, not xpubs at a `/**` or `/<0;1>` placeholder.
That is a fourth mismatch for the keys inside the leaf. Blockers 1–3 already close the policy.

### What this does not decide

- S1 still runs the emulators and records the error strings. Ledger registers a descriptor template, not a script,
  and the on-chain connector has no descriptor. S1 can only submit the closest policies. The expected outcomes are:
  - **Trezor:** no script-path message exists. `SPENDTAPROOT` signs the key-path sighash with the BIP-86-tweaked key.
  - **Ledger, raw internal key or raw admin keys:** the template does not parse ("Expected /** or /<M;N>/* in key
    expression"). There is no syntax for an underived key.
  - **Ledger, `UnstakingBurn` written as `sha256(h)`:** registration fails with `EC_REGISTER_WALLET_POLICY_NOT_SANE`.
  - **Ledger, a registrable stand-in policy (`multi_a`, derived keys, signed sibling):** the device derives a
    different `scriptPubKey`. `compare_wallet_script_at_path` does not match, so the input is treated as external
    and is not signed.
- S5 still needs a decision with Alpen if the product requires a hardware signature. For Ledger, every one of these
  is an upstream protocol change:
  - the leaf ends in `OP_NUMEQUAL` (`multi_a`);
  - each admin key is an xpub child at the same `/<M;N>/*` index as the rest of the policy;
  - the N/N internal key is `musig(...)/<M;N>/*`, a derived aggregate of at most 5 operator xpubs;
  - `UnstakingBurn` requires a signature or leaves the tree.

  The alternatives are to accept a software signer for this leaf, or to change the PRD. This desk pass does not
  pick one.
- A custom firmware or a custom Ledger Bitcoin app could sign the leaf. That is outside stock devices and outside
  the matrix of devices this app supports today.
