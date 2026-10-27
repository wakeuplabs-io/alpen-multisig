# ADR-008: ASM pin `v0.5.0-rc.1` for the P2WPKH signer identity

**Status:** Accepted
**Date:** 2026-10-27
**Extends:** [ADR-001](./001-alpen-crate-dependencies.md) and supersedes the pin chosen in
[ADR-007](./007-asm-pin-for-security-council.md). It applies the same pin-update procedure.
**Feature:** [`specs/asm-v0.5-signer-address.md`](../../specs/asm-v0.5-signer-address.md) (feedback item 60)

## Context

Feedback item 60 asks the app to follow [asm#277](https://github.com/alpenlabs/asm/pull/277) and
[asm#284](https://github.com/alpenlabs/asm/pull/284). In #277, an admin signer is identified by a
P2WPKH address instead of a compressed public key. In #284, `Harbour` is renamed `Harbor`, including in
the signed message. Neither change exists at our pin, `v0.1-alpha.11` (`b84eb28`), which is 135 commits
behind.

The bump breaks the wire format and the signed message, so it happens once, with an operational reset.
ADR-007 turned down every tag after `v0.1-alpha.11` because those tags need Bitcoin Core 30.2. That cost
has already been paid: Phase 1 moved the toolchain to `nightly-2026-07-01` (#623), and Phase 2 moved the
`bitcoind` binary to 30.2 (#624). Both were measured green at the old pin.

| Candidate | Date | Has #277 / #284 | strata-common | ssz-gen | Bitcoin Core features |
|---|---|---|---|---|---|
| `v0.1-alpha.11` (current) | 2026-06-02 | no | `v0.1.0-alpha-rc23` | `v0.16.0` | `29_0` |
| **`v0.5.0-rc.1`** (`7ada090`) | 2026-10-02 | **yes** (17 and 19 commits after the merges) | `v0.4.0` | `v0.17.0` | `30_2` |
| `main` | after the tag | yes | `v0.4.0` | `v0.17.0` | `30_2` |

`main` is past the tag by #305 (SP1 toolchain in CI) and #307 (`strata_asm_getAdminSeqNo`, an additive RPC).
Neither one is needed here.

## Decision

**Pin to tag `v0.5.0-rc.1` → commit `7ada090580ac12af9cd411254415090725efcec1`**, together with:

| What moves | From | To |
|---|---|---|
| `alpenlabs/strata-common` | `v0.1.0-alpha-rc23` | `v0.4.0` |
| `alpenlabs/ssz-gen` | `v0.16.0` | `v0.17.0` |
| `bitcoin-bosd` | `v0.11.0` | `v0.12.0` |
| `corepc-node` / `bitcoind-async-client` | `0.10.0` / `=0.10.6`, feature `29_0` | `0.12.0` / `0.10.8`, feature `30_2` |
| New crates | — | `strata-asm-admin-threshold-sig`, `strata-asm-admin-types` |

Pin the commit, not the annotated tag object (`c47d09c`). `bitcoin` `0.32.6` and `secp256k1` `0.29.1`
already match the tag.

The findings are listed below, most consequential first.

### 1. The signer identity is on the wire

- `ThresholdConfig` moved out of `strata_crypto::threshold_signature` and into
  `strata-asm-admin-threshold-sig` ([asm#272](https://github.com/alpenlabs/asm/pull/272)).
- Its signers are `P2wpkhAddress`: a 20-byte witness program, SSZ-encoded with no length prefix, read with
  `signers()` instead of `keys()`. The type has no network. Bech32 comes from
  `to_address(network)`, and `try_from_address` parses an address while ignoring its HRP.
- `strata-common` `v0.4.0` still exports the old pubkey-based `ThresholdConfig`. Mixing the two is a type
  error, not a silent mismatch.
- `ThresholdConfigUpdate::try_new` is now fallible.
- `verify_threshold_signatures` recovers the key from each signature, hashes it to P2WPKH and compares it
  with `signers[index]`. It accepts headers 0–3 and 31–42 and rejects 27–30 (BIP-137 uncompressed). The
  broadcast path already emits raw 0–3.

### 2. The signed message changes for every admin action

- `SigningMessage::for_action(action, seqno, network)` renders a `Network: <network>` line after the
  header.
- Signer-set updates list members as `N. Add Member: <bech32>` and `N. Remove Member: <bech32>`.
- Safe Harbor lines say `Harbor`. The descriptor SSZ does not change, but the digest does.
- `create_signature_set` and `create_test_admin_tx` also take the network.

Every admin signature collected at `v0.1-alpha.11` fails verification after the bump.

### 3. Crates and types moved or were renamed

- `strata-asm-proto-bridge-v1` → `strata-asm-proto-bridge`.
- `strata-asm-proto-bridge-v1-types` → `strata-asm-bridge-types`, with bridge state in
  `strata-asm-proto-bridge-state`.
- `AdminTxType`, `UpdateTxType`, `Role` and `ConfirmationDepths` moved from `strata_asm_params` to
  `strata_asm_admin_types`.
- `BridgeV1InitConfig` → `BridgeInitConfig`.
- `SafeHarbour*` → `SafeHarbor*`, and `bridge.safe_harbour()` → `safe_harbor()`.
- `SectionState::try_to_state` now needs `SectionStateExt` in scope.
- `parse_tx` returns `Option`.
- The worker builder was rewritten:
  - `with_genesis(state, predicate)` and `with_registry(ExecutionRegistry)` replace
    `with_asm_spec`/`with_params`.
  - `submit_block` takes a `BlockHash`.
  - `WorkerContext` is split into four traits.
  - Upstream now ships `strata_asm_worker::test_utils::TestAsmWorkerContext`. The e2e harness adopts it
    instead of keeping its own context.

### 4. Two runtime breaks that the compiler does not see

- **`strata_asm_getStatus`:** `cur_state` used to be `{ state: [..], logs }`. It is now
  `Option<AnchorState>`, and `AnchorState` serializes as its raw SSZ bytes. Every reader of
  `/cur_state/state` has to read `/cur_state`: the orchestrator, the desktop, and the reset runbook's
  verification script.
- **Params JSON:** each role changed from `{ "keys": [<33-byte hex>], "threshold": N }` to
  `{ "signers": [<bech32>], "threshold": N }`, and depth and bridge keys say `harbor`. `AsmParams`
  deserialization rejects a signer whose HRP does not match `anchor.network`. This affects
  `scripts/asm-params.example.json`, `staging/asm-params.template.json` and the inline fixtures in
  `e2e-tests/src/fixtures/signer_update_enacted.rs`.

## Consequences

- **The bump is one compile boundary and one revert target.** The `29_0` and `30_2` features are mutually
  exclusive, and the threshold types cannot be mixed, so the pins, the identity change and the e2e
  harness land in one commit.
- **`signer_pubkey` stays the backend's internal identity.** It is still the DB key, the auth subject and
  the source of signature recovery. Every comparison against the ASM goes through the witness program,
  and the Admin ID is rendered on the chain's network. The device-native HRP never enters a comparison.
- **The orchestrator needs the chain's network** to render the message it checks in `signed_by_quorum`.
  Phase 3 added `BITCOIN_NETWORK` for this. A wrong value makes every quorum check fail.
- **The reset is mandatory.**
  - Old signer-update `action_hex` fails to decode, because the signer field shrinks from 33 bytes to 20.
  - Old signatures fail to verify.
  - The regtest genesis carries the old params format.

  Run [`asm-pin-bump-reset.md`](../../operations/asm-pin-bump-reset.md) on local and staging. Nothing is
  migrated.
- **We pin a release candidate.** If `v0.5.0` final changes the signed message or the signer identity, that
  is new scope. Otherwise the re-pin uses the reserve of Phase 7.

## Update procedure

Unchanged from ADR-001 and ADR-007. The pin lives in five places that move together:

- root `Cargo.toml`
- `orchestrator-be/Cargo.toml`
- `desktop-app/src-tauri/Cargo.toml`
- `e2e-tests/Cargo.toml`
- the `asm` submodule gitlink

This bump adds one check: the Bitcoin Core feature in `e2e-tests/Cargo.toml` and the binary on `PATH`
must match. If they do not, the regtest suites skip instead of failing.
