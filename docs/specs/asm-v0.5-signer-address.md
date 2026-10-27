# ASM v0.5 — Signer Address and Harbor: Master Plan

**Source:** feedback item 60 (feedback document, added after rev. 4). Not filed as a GitHub issue — each
phase PR references this spec.

**Upstream changes followed:** [asm#277](https://github.com/alpenlabs/asm/pull/277) (admin signers are
identified by a P2WPKH address) and [asm#284](https://github.com/alpenlabs/asm/pull/284) (`Harbour` →
`Harbor`, including the signed message).

**Pin target:** `alpenlabs/asm` tag [`v0.5.0-rc.1`](https://github.com/alpenlabs/asm/releases/tag/v0.5.0-rc.1)
→ commit `7ada090580ac12af9cd411254415090725efcec1`. Today: `b84eb28` = `v0.1-alpha.11`
([ADR-007](../architecture/adrs/007-asm-pin-for-security-council.md)).

**Status:** In progress — Phases 0–3 done (#622–#625), Phase 4 in review.

A phase marked ✅ means the engineering step shipped, not that every acceptance criterion in §1.3 is met.
§1.3 stays the measure.

## 1. Purpose and scope

### 1.1 What the feedback asks for

> Update the app to be compatible with the protocol changes implemented in asm#277 and asm#284.
> - Replace instances of compressed public keys with the corresponding P2WPKH bitcoin address, in both
>   displays as well as input fields.
> - Where the compressed public key and Admin ID (P2WPKH bitcoin address) are shown on the same page,
>   remove the compressed public key and leave the Admin ID.

### 1.2 Why this is a protocol bump and not a display swap

On `v0.5.0-rc.1` the identifier of an admin signer **is** the P2WPKH witness program:

- `ThresholdConfig` moved out of `strata-crypto` into the new crate `strata-asm-admin-threshold-sig`
  ([asm#272](https://github.com/alpenlabs/asm/pull/272)). Its signers are `P2wpkhAddress`, 20 bytes,
  SSZ-encoded with no length prefix. They are no longer 33-byte `CompressedPublicKey` values. `bc1q…` and
  `bcrt1q…` over the same program are the same signer. Bech32 is produced only for display, using the
  chain's network.
- `SigningMessage::for_action(action, seqno, network)` now renders a `Network:` line on **every** action.
- A signer-set update renders its members as `Member: <bech32>` instead of pubkey hex.
- #284 renames the Safe Harbor lines `Harbour` → `Harbor`. The descriptor SSZ does not change, but the
  message digest does.

Consequences:

- Every admin signature collected at `v0.1-alpha.11` fails verification after the bump.
- A signer-update `action_hex` from the old pin no longer decodes: the signer field shrinks from 33 bytes to
  20, so decoding fails closed.
- The reset runbook ([`asm-pin-bump-reset.md`](../operations/asm-pin-bump-reset.md)) is part of the bump.
  There is no migration.

### 1.3 Acceptance criteria

1. Wherever the app shows an admin signer (approvals, "Sent by", signer-set change table, create-proposal
   preview, sign view, cancel summary, import bundle, current signers in the signer-update form), it shows
   the Admin ID (P2WPKH address on the chain's network), not the compressed key.
2. The connect screen no longer prints the compressed key next to the Admin ID.
3. The signer-update input (strata admin and security council) accepts a P2WPKH address on the active
   network. A compressed pubkey pasted there is a validation error.
4. Signer identity comparisons (membership, already-signed, "You", originator) match on the witness
   program, so a `bc1` Admin ID from a Trezor and a `bcrt1`/`tb1` rendering of the same key are one signer.
5. Every admin action is signed over the `v0.5.0-rc.1` message: it has a `Network:` line, signer-set
   members are bech32, and Safe Harbor lines say `Harbor`. It is verified by the in-process ASM e2e suites
   and read on a Ledger.
6. Sign-in with an Admin ID resolves membership against the ASM state at the new pin.
7. The app's own protocol strings say `harbor` (`safe_harbor_address_update`).

### 1.4 In scope

- Pin bump to `v0.5.0-rc.1` and everything it drags in: toolchain, `strata-common`, `ssz-gen`,
  `bitcoin-bosd`, the new threshold crate, **Bitcoin Core 30.2** (§3.2).
- Signer identity on the wire, in membership, in enactment checks and in broadcast.
- The item 60 screens and inputs (§1.3).
- The Harbor rename: upstream types, params JSON and our own protocol strings.
- One Ledger pass and the Trezor emulator on the new device text.
- The state reset on local and staging.

### 1.5 Not in scope

- Items 36, 37 and 38 (pubkey beside the Admin ID, certificate into "add signer", Convert page). #277
  removes the reason for all three, and they stay paused.
- Operator-set, sequencer and VK inputs. These are x-only keys or predicate bytes, not admin signers.
- Removing `signer_pubkey` from storage. It stays the backend's internal identifier: the DB primary key
  `(action_id, signer_pubkey)`, auth, and signature recovery. The address is derived from it.
- Migrating old `action_hex` or signatures. Reset, do not translate.
- Following `main` past the tag: `#305` (CI) and `#307` (`strata_asm_getAdminSeqNo`, additive). A
  `v0.5.0` final goes in the reserve of Phase 7.
- New WebDriver coverage. Phase 6 only updates the existing specs that pin the old format.
- The release itself, which follows the normal release process once Phase 7 closes.

## 2. Phase board

| Phase | Name | Visible change | Closes | Touches | Status |
|---|---|---|---|---|---|
| 0 | Master plan (this document) | none | — | `docs/` | Done (#622) |
| 1 | Toolchain `nightly-2026-07-01` at the current pin | none | prerequisite | `rust-toolchain.toml`, workflows, our tree's lint fallout | Done (#623) |
| 2 | Bitcoin Core 30.2 binary | none | prerequisite | CI, `staging/Dockerfile.bitcoin`, dev notes | Done (#624) |
| 3 | Seams: network in the orchestrator, network parameter on the signing path, identity by witness program | none | prerequisite for AC 4, 5 | `orchestrator-be`, `src-tauri` | Done (#625) |
| 4 | Admin ID on screen (derived from the pubkey, before the bump) | display only | AC 1, 2 | `src-tauri` (IPC), `desktop-app` | In review |
| 5 | **The bump to `v0.5.0-rc.1`** | protocol | AC 3, 4, 5, 6 | everything; ADR-008 | Planned |
| 6 | Post-bump cleanup | minor | AC 7; the rest of AC 1 | `desktop-app`, `src-tauri`, WebDriver specs | Planned |
| 7 | Device QA, staging reset, docs, reserve | none | AC 5 (device) | `docs/`, staging | Planned |

Rules:

- **One PR per phase against `develop`**, branched from a freshly pulled `develop`. Phases run in order,
  not in parallel.
- Every PR is green on the full local CI checklist (§5) before it is pushed, and leaves the app working on
  the local stack.
- Every commit compiles and passes tests. If a shared contract blocks that, use expand/contract, not a
  bigger commit.
- **Release gate:** no release ships between Phase 4 and Phase 7. `develop` may carry the address display
  (Phase 4) on the old pin, which is correct on its own. The bump is not released until Phase 7 closes.

## 3. Architecture

### 3.1 What already exists and is reused

| Piece | Where | Why it matters |
|---|---|---|
| Network from env | `desktop-app/src-tauri/src/infrastructure/network_env.rs:19-44` (`network_from_env`, `resolve_network`) | The chain's network for bech32 display and for `SigningMessage::for_action` |
| P2WPKH rendering | `src-tauri/src/infrastructure/admin_wallet/wallet.rs:97` (`admin_id_path`), `:119` (`device_hrp_network`), `:131` (`render_address_with_hrp`) | Admin ID derivation already exists for the wallet card |
| Admin ID presentation (TS) | `desktop-app/src/lib/admin-id.ts` (`truncateAdminId`, `adminIdText`, P2WPKH regex `bc\|tb\|bcrt`) | Format and truncate only. The frontend has no pubkey → address helper, so derivation stays in Rust |
| Reset runbook | [`asm-pin-bump-reset.md`](../operations/asm-pin-bump-reset.md) | DB truncate, runner rebuild, `local-stack.sh --clean`, genesis check |
| Pin procedure | [ADR-001](../architecture/adrs/001-alpen-crate-dependencies.md), [ADR-007](../architecture/adrs/007-asm-pin-for-security-council.md) | The five places the pin lives. Explicit "unsupported" arms instead of `_ =>` |
| Precedent | [`2-discovery/19-asm-bump-impact-assessment.md`](../2-discovery/19-asm-bump-impact-assessment.md) §8 | The breaking bump lands alone, as one compile boundary and one revert target |

### 3.2 What the pin drags in

| What moves | From | To |
|---|---|---|
| `alpenlabs/asm` | rev `b84eb28` (`v0.1-alpha.11`) | tag `v0.5.0-rc.1` (`7ada090`) |
| `alpenlabs/strata-common` | `v0.1.0-alpha-rc23` | `v0.4.0` |
| `alpenlabs/ssz-gen` | `v0.16.0` | `v0.17.0` |
| `bitcoin-bosd` | `v0.11.0` | `v0.12.0` |
| Rust toolchain | `nightly-2026-01-01` | `nightly-2026-07-01` |
| New crate | — | `strata-asm-admin-threshold-sig` |
| **Bitcoin Core** | `29.0` (CI), `28.1` (`staging/Dockerfile.bitcoin`) | **`30.2`** |
| `corepc-node` / `bitcoind-async-client` | `0.10.0` / `=0.10.6`, feature `29_0` | `0.12.0` / `0.10.8`, feature `30_2` |

The Bitcoin Core move is the finding the original estimate did not carry. ADR-007 rejected every tag after
`v0.1-alpha.11` for exactly this reason: the `29_0` and `30_2` features are mutually exclusive, so the whole
graph switches at once. It also reaches every developer's local `bitcoind`. If that version is wrong, the
e2e suites **skip silently** instead of failing.

`bitcoin` `0.32.6` and `secp256k1` `0.29.1` already match the tag.

**Where the pin lives:**

- root `Cargo.toml:12-36`
- `orchestrator-be/Cargo.toml:41`
- `desktop-app/src-tauri/Cargo.toml:36-37`
- `e2e-tests/Cargo.toml`: 8 asm pins, 6 rc23 pins, and the Core features at `:39-40`
- the `asm` submodule gitlink

**Where the toolchain lives:**

- `rust-toolchain.toml`
- `.github/workflows/ci.yml:72`
- `e2e.yml:34`
- `release.yml:52,96,160,268`

### 3.3 The breaking points

| Change | Class | Sites |
|---|---|---|
| `ThresholdConfig` signers: 33-byte key → 20-byte `P2wpkhAddress` | **wire / SSZ** | orchestrator `asm_role_membership.rs:250-277` (`authority_keys_hex`), `asm_enactment.rs:192-199,337-430`, `application/proposals.rs:882-908` (`signed_by_quorum`); desktop `asm_status_rpc.rs:28-48,278-292` (`fetch_multisig_config`), `asm_enactment.rs:59-180`, `signing.rs:200-211` (`verify_threshold`), `broadcast_tx.rs:46-78` (signature index = signer position), `action_codec.rs:218-238,313-340`, `domain/action.rs:17` (`CompressedPubKey`), `commands/action_builder.rs:128-148` |
| `for_action(.., network)` | **signed message** | direct: orchestrator `proposals.rs:886`; desktop `broadcast_tx.rs:128`, `signing.rs:61,142`; e2e `defcon_probe:77`, `admin_commit_reveal:66`, `admin_subprotocol:78`. Wrappers: `signing::compute_sighash`, `broadcast_tx::compute_sighash`, `render_signing_message` (about 25 callers) |
| `create_signature_set(.., network)` | compile | e2e `council_rotation:460`, `admin_commit_reveal:68`, `safe_harbor_address:360`, `cancel_proposal:108,155` |
| `SafeHarbour*` → `SafeHarbor*`, `safe_harbor()` | compile; descriptor SSZ unchanged | 54 Rust lines in 15 files (both `action_codec.rs` and `asm_enactment.rs`, `asm_role_membership.rs`, `asm_status_rpc.rs`, e2e) |
| Params JSON `"keys"` arrays and `safe_harbour_*` keys | **runtime**; the compiler does not see it | `scripts/asm-params.example.json`, `staging/asm-params.template.json`, `e2e-tests/src/fixtures/signer_update_enacted.rs` |
| Worker / manifest / `AuxData` drift over 135 commits | compile, unknown until `cargo check` | `e2e-tests/src/test_harness.rs`, `worker_context.rs` |

### 3.4 Tripwires

**Fail to compile if missed (safe):** everything that imports `strata_crypto::threshold_signature`,
`SafeHarbour*`, or calls `for_action` / `create_signature_set`. Exhaustive matches on `MultisigAction` /
`UpdateAction` keep explicit arms, with no `_ =>`.

**Do not fail to compile, so they must be carried by hand:**

- `orchestrator-be` has **no network setting**. `signed_by_quorum` computes the message server-side, so a
  wrong default network makes every quorum check fail.
- `desktop-app/src-tauri/src/infrastructure/signing.rs:104,124-125` hardcodes `Network::Bitcoin` /
  `KnownHrp::Mainnet` for key derivation and listed addresses. Derivation stays as is. The message network
  must come from `network_env`, not from this path.
- Device HRP: Trezor always yields `bc1…`, Ledger yields `tb1…` off mainnet. Never compare Admin IDs as
  strings. Compare witness programs.
- Our own `'safe_harbour_address_update'` tag in about 20 TS files, `commands/action_builder.rs:34` (serde)
  and `commands/proposals.rs:210,1420`. It is stored as `action_type`.
- `desktop-app/src-tauri/tests/g10_ledger_challenge_screens.rs:179`: a fixture of the rendered message.
- The dev membership mock in `orchestrator-be/src/infrastructure/asm_role_membership.rs:304-337` has
  hardcoded pubkeys.
- WebDriver specs `proposal-add-signer.e2e.js` and `signing-no-sighash.e2e.js` paste compressed keys into
  `e2e-new-signer-pubkey-input`.
- 9 of the 12 suites in `e2e-tests/tests/` boot an in-process ASM over a real `bitcoind`, and they skip when
  the binary is missing. Absence is not a pass.

## 4. Phased plan

### Phase 0 — Master plan

This document, plus its row in [`docs/README.md`](../README.md). Docs only.

### Phase 1 — Toolchain `nightly-2026-07-01` at the current pin

Six months of nightly is mostly clippy and rustfmt on our own tree, and that fallout is independent of the
protocol. Landing it first keeps it out of the bump diff.

| # | Contents |
|---|---|
| 1 | `rust-toolchain.toml` and the six workflow references, plus the fmt/clippy fixes they force, in one commit, because neither half is green alone |

**Spike first.** Check whether `v0.1-alpha.11` and its transitive crates build on the new nightly. If they
do not, record why here and fold this phase into Phase 5. Do not pin a third nightly.

**Not touched:** `asm/rust-toolchain.toml`. It belongs to the submodule and moves with it in Phase 5.

**Result:** go. `v0.1-alpha.11` and every transitive crate build on `nightly-2026-07-01` (rustc 1.98.0-nightly
`f46ec5218`). The fallout was two new clippy lints, five sites, all in
`desktop-app/src-tauri/src/infrastructure/broadcast_tx.rs`: `useless_borrows_in_formatting` (×3) and
`byte_char_slices` (×2, tests). rustfmt reported no changes.

### Phase 2 — Bitcoin Core 30.2 binary

| # | Contents |
|---|---|
| 1 | `.github/workflows/ci.yml` `BITCOIN_CORE_VERSION` → `30.2`; `staging/Dockerfile.bitcoin` `28.1` → `30.2`; the local `bitcoind` note in `e2e-tests/README.md` (where the install note lives) |

**Spike first.** Check whether the harness at `corepc-node 0.10` / `29_0` runs green against a `bitcoind`
30.2 binary, and whether the `v0.1-alpha.11` runner in the Docker stack does. If either fails, record the
error here and fold the binary move into Phase 5, where the features switch to `30_2` together.

**Verification:** the 9 regtest suites ran (not skipped), and `bitcoind --version` printed 30.2 in that run.

**Result:** go. The binary moves now. The crate features stay at `29_0` until Phase 5. Everything below ran
against `bitcoind` v30.2.0, from the release tarball with its `SHA256SUMS` checked:

- **In process:** `cargo test --workspace` passed all 23 test binaries, and all 12 `e2e-tests` suites ran
  rather than skipping. `cargo test -p orchestrator-be -- --ignored postgres_ regtest_` passed 8/8, three
  runs in a row.
- **Docker stack** (`./scripts/local-stack.sh --clean` with the 30.2 image): the `v0.1-alpha.11` runner
  followed the chain (`strata_asm_getStatus` 104 → 106), electrs indexed to the same height, and the
  orchestrator came up healthy.
- **WebDriver:** `wallet-smoke`, `proposal-add-signer`, `proposal-co-sign-mnemonic` and
  `proposal-broadcast-quorum` all passed. The last one confirms the commit and reveal on-chain.

`regtest-dev-api` reports unhealthy because its healthcheck calls `curl`, which the image does not ship. It
did the same on 28.1 and serves `/mine` normally. That is out of scope here.

### Phase 3 — Seams, with no behavior change

Prepare the signature changes so the bump only flips them.

| # | Contents |
|---|---|
| 1 | `orchestrator-be/src/config.rs`: `BITCOIN_NETWORK` (default `regtest`, same parsing as `network_env.rs`), plus `staging/orchestrator.env` and the config docs |
| 2 | `network: Network` threaded into `signing::compute_sighash`, `render_signing_message`, `broadcast_tx::compute_sighash` and `signed_by_quorum`, and their callers. It is unused at this pin, and each function documents that |
| 3 | A signer identity helper (hash160 of the compressed key = witness program) used for the membership and already-signed comparisons in Rust, so that Phase 5 only changes what is on the other side of the comparison |

**Tests:** existing tests stay green unchanged. Unit tests cover the helper (same key → same program;
`bc1`/`bcrt1` renderings of one program are equal) and the config parse.

**Result:** done as planned. Three commits, each green.

1. **Orchestrator network.** `Config.bitcoin_network` comes from `BITCOIN_NETWORK`. The default is
   `regtest`, and an unknown token fails startup. `AppState.network` reaches `signed_by_quorum` through
   `report_broadcast_progress`, `reconcile_enacted_for_{authority,action}`, `reconcile_one`,
   `confirm_reveal_if_mined`, `first_mined_pair` and `reveal_carries_proposal`. `staging/orchestrator.env`
   and `staging/docker-compose.yml` set it.
2. **Desktop signing path.** The three wrappers take the network. The IPC commands read it from
   `network_env::network_from_env`, and the broadcast application layer passes the `network` it already
   received. At this pin the leaf parameter is named `_network`.
3. **`SignerId`.** A new `domain/signer_id.rs` in each crate, byte-identical in both. It drives orchestrator
   role membership, desktop sign-in membership (`application/authentication.rs`) and the broadcast signature
   index (`broadcast_tx.rs`).

`asm_enactment.rs` compares one action against state, so both sides are always the same kind. It stays as
it is and moves in Phase 5.

Every existing test passed unchanged. On the local stack (no `--clean`), the WebDriver chain passed:
`proposal-add-signer` → `proposal-co-sign-mnemonic` → `proposal-broadcast-quorum`.

### Phase 4 — Admin ID on screen, before the bump

The address is derived from the pubkey, which does not depend on the pin, so this phase is
forward-compatible. After the bump, the ASM returns addresses directly. Comparisons and the input stay on
the pubkey until Phase 5.

| # | Contents |
|---|---|
| 1 | Rust derives the P2WPKH address of a signer pubkey with the chain's network (`bitcoin::Address::p2wpkh` + `network_env`) and exposes it over IPC. Either add an `adminId` alongside the existing signer fields or add a batch command; the phase spec decides. The IPC schemas in `desktop-app/src/api/ipc-schemas.ts` are updated |
| 2 | Displays: `components/approvals-list.tsx:58,76`, "Sent by" (`proposal-detail/model/broadcast-pairs.ts:39`), `signer-set-change/components/signer-set-change-table.tsx` and `model/build-signer-set-change.ts`, `create-proposal-preview.tsx`, `sign-proposal-view.tsx:75-93`, `cancel-proposal/components/cancel-target-summary.tsx:54`, `import-bundle-modal.tsx:270`, current signers in `signer-update-form-fields.tsx:123`, the error text in `sign-proposal/wallet-binding.ts:5` |
| 3 | Drop the compressed-pubkey debug control in `connect-wallet/components/connect-phase.tsx:196-203` (AC 2) |

**Tests:** update the vitest files that pin the displayed hex (`approvals-list`, `signer-set-change-table`,
`build-signer-set-change`, `broadcast-pairs`, `cancel-details-card`). Keep the guards in
`admin-wallet/architecture.test.ts` (Rule 9) and `admin-id-presentation.test.ts`. Run a manual walk of one
signer update and one cancel on the local stack.

**Result:** done. Derivation stays in Rust, so there is no second implementation in TS:

- `SignerId::admin_id(network)` and the IPC command `admin_ids_for_pubkeys` run on the process network,
  pinned by the BIP-173 vector for the generator point.
- In the UI, `useAdminIds` turns every row a screen renders in one commit into a single IPC call. Answers
  are cached for the process.
- `<SignerAdminId>` prints the Admin ID, `…` while it resolves, or `Unknown`. It never prints the public
  key.

Surfaces:

- Approvals, in the proposal detail and the cancel card.
- The signer-set change table, in the preview, the detail and the cancel summary.
- "Sent by": `describeBroadcastPairs` now returns a typed sender instead of a truncated key.
- The sign view's signers to add and remove.
- The import preview.
- The signer-update form's current signers.
- The wallet-binding error, which no longer prints keys.
- The connect screen's debug row (AC 2).

WebDriver `proposal-add-signer` now reads the added signer by Admin ID. The chain `proposal-add-signer` →
`proposal-co-sign-mnemonic` → `proposal-broadcast-quorum` passes, as does `signing-no-sighash`.

Carried to later phases:

- The session's own chip still shows the device-native HRP (`bc1q…` for a mnemonic or Trezor session),
  while listed signers use the chain's (`bcrt1q…`). Both are the same program. Phase 6 settles the HRP.
- The desktop `signer_id.rs` now has `admin_id`, so it is no longer byte-identical to the orchestrator
  copy, which has no use for it.

### Phase 5 — The bump to `v0.5.0-rc.1`

One compile boundary and one revert target. ADR-008 records the pin decision with compile evidence and the
Core 30.2 move, as ADR-007 did.

| # | Contents |
|---|---|
| 1 | ADR-008 (docs) |
| 2 | Pins in the five places and the submodule; `strata-common` `v0.4.0`, `ssz-gen` `v0.17.0`, `bitcoin-bosd` `v0.12.0`, `strata-asm-admin-threshold-sig`; Core features `30_2` (and the binary, if Phase 2 folded); upstream Harbor renames; `for_action` takes the real network; threshold types → `P2wpkhAddress` in membership (both `asm_role_membership.rs`), enactment `add/remove_members`, `fetch_multisig_config`, `broadcast_tx` signature index, `signed_by_quorum`, `verify_threshold`; params JSON `"keys"` → addresses and `safe_harbor_*` keys; e2e harness and fixtures. This is the commit that cannot be split |
| 3 | Signer-update input: `addKeys`/`removeKeys` become addresses over IPC (`api/action-builder.ts`, `ipc-schemas.ts:158-159`, `commands/action_builder.rs:128-148`, `domain/action.rs`); validators `create-proposal/model/validators/{types,signer-update}.ts` accept P2WPKH on the active network and reject a pubkey; label and placeholder in `signer-update-form-fields.tsx:189-199` (AC 3) |
| 4 | Frontend comparisons that still match pubkey hex against ASM-provided signers move to the address the IPC now returns (`derive-proposal-actions.ts`, `approvals-list.tsx`, `signer-update.ts` `removesCurrentMembers`, `build-signer-set-change.ts`) |
| 5 | `g10_ledger_challenge_screens.rs` fixture and any other rendered-message fixtures |

If commit 2 needs commits 3–5 to stay green, they merge into it. The rule is that every commit is green,
not a fixed count.

**Tests:** the 9 regtest e2e suites run green (confirm they ran). Unit tests: the codec round-trips an
address, membership resolves a session pubkey against an address set, a pubkey is rejected by the
validator, and the rendered message contains `Network:`, `Member: bcrt1q…` and `Harbor`.

**State:** run the reset runbook on the local stack before the manual walk.

### Phase 6 — Post-bump cleanup

| # | Contents |
|---|---|
| 1 | Our tag `safe_harbour_address_update` → `safe_harbor_address_update` across TS, `commands/action_builder.rs:34` and `commands/proposals.rs`, with `serde(alias)` accepting the old value (AC 7) |
| 2 | Delete `buildAsmConfigSnippet` (`connect-wallet/utils/hw-wallet-connect-utils.ts:8-19`), which has no callers and emits the old `"keys"` format |
| 3 | Display uses the chain's HRP everywhere an Admin ID of another signer is shown. The device-native HRP is kept only for the user's own device verification |
| 4 | Update `e2e-webdriver/test/specs/proposal-add-signer.e2e.js` and `signing-no-sighash.e2e.js` to paste addresses |

### Phase 7 — Device QA, staging reset, docs, reserve

- **Ledger:** sign a signer update, a Safe Harbor update, and one Defcon or VK action. Read `Network:`,
  `Member: bcrt1q…` and `Harbor` on the device. **Trezor emulator:** the same three. Evidence goes outside
  the repo, as with earlier QA.
- Reset runbook on staging.
- Docs that cite `v0.1-alpha.11`:
  - `3-stories/story-map.md`
  - `3-stories/non-functional-items.md`
  - the security council specs
  - the ADR-001 tables
  - the runbook's "first use" note
- Close-out (§7).
- **Reserve:** walk findings, and a re-pin to `v0.5.0` final if it is tagged before release. If the final
  tag changes the signed message or the on-the-wire identity, that is new scope, not reserve.

## 5. Verification (every phase)

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd desktop-app && npm run format:check && npm run lint && npm run build && npm run test:unit
```

- Phases 2 and 5: the 9 regtest suites in `e2e-tests/tests/` **ran**, with the right `bitcoind` on `PATH`.
- Phase 5: `cargo check --release -p desktop-app -p orchestrator-be`. Reset runbook steps 1–4, ending with
  `strata_asm_getStatus` showing the new genesis.
- Phases 4–7, manual walk on the local stack:
  1. Create a signer update by pasting an Admin ID, sign it, and see `Network:` and the address on the
     device.
  2. Sign a Safe Harbor update and check the text says `Harbor`.
  3. Sign a Defcon or VK action and check it carries `Network:`.
  4. Sign in with an Admin ID and check that membership resolves.
  5. Paste a compressed pubkey into "add signer" and check it is rejected.
- After `cargo test`, rebuild with `tauri build` before any WebDriver run. `cargo test` overwrites
  `target/debug/desktop-app`.

## 6. Known debt this plan does not take

- `signer_pubkey` remains the backend's stored identity. Moving storage to the address is 1–2 days more and
  is a separate decision.
- `signing.rs` derives keys and lists addresses on mainnet regardless of the session network. This plan
  does not use that path for the message network, and does not change it.
- `strata_asm_getAdminSeqNo` (`#307`, on `main` after the tag) could replace the anchor-state decode for
  seqno. It is not adopted here.

## 7. Close-out

These headers do not update themselves. Update them in the PR of the last phase:

- This document's **Status** and the §2 board, with PR numbers.
- [ADR-001](../architecture/adrs/001-alpen-crate-dependencies.md) pin tables.
- The [`docs/README.md`](../README.md) row stays `Current` until a release ships the bump.
