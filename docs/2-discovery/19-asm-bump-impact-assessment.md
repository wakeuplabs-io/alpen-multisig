# 19 — ASM pin bump impact assessment (`a8559d3` → `a53b6a8`)

> **Status:** Complete — bump assessed against `alpenlabs/asm` `origin/main` at `a53b6a8` (2026-05-13).
> **Date:** 2026-05-13.
> **Supersedes (in scope of the wire-format claim):** sections of
> [`08-alpen-crate-prd-coverage.md`](./08-alpen-crate-prd-coverage.md),
> [`11-asm-repo-migration.md`](./11-asm-repo-migration.md) §"Wire format compatibility",
> [`10-asm-bitcoin-state-model.md`](./10-asm-bitcoin-state-model.md) §8 (sighash formula),
> [`16-poc5-trezor-findings.md`](./16-poc5-trezor-findings.md) §§2, 4, 5, 11,
> [`external/research-assessment.md`](../external/research-assessment.md) §1–§2,
> and [`ADR-001`](../architecture/adrs/001-alpen-crate-dependencies.md) §"Wire format" / §"Crate inventory".
> Earlier statements that "the bytes signers sign are byte-identical across the Borsh→SSZ migration" remain
> true for the `308211f` → `a8559d3` window but **do not** hold for `a8559d3` → `a53b6a8`. See §A.

## 0. Executive summary

- **Target rev (verified on `origin/main` at write time):** `a53b6a8b25ed5fdd95fa3882f2e73067735bf779` — `chore(deps): bump moho to v0.1-alpha.6, zkaleido to v0.1-beta.2, SP1 to 6.2.0 (#102)` (2026-05-13 10:44:34 UTC, Abishek Bashyal). No tag points at this commit; `v0.1-alpha.5` is still the most recent tag (==`a8559d3`).
- **Commits in the range:** 19 (see §1). Classification: 5 protocol-breaking, 1 protocol-additive, 3 dependency-bumps, 3 RPC additions, 1 admin-restructure (rename), 3 checkpoint/bridge refactors, 3 CI/tooling/dev-dep bumps.
- **Breaking surface inside our workspace:** every consumer file that touches `strata-asm-txs-admin`, `strata-asm-params::Role`, `strata-asm-txs-test-utils`, or `strata-asm-proto-administration` breaks. Concretely: 3 files in `desktop-app/src-tauri`, 3 files in `orchestrator-be`, 2 files in `e2e-tests`, and `Cargo.toml` (root and both members). Migration table in §7.
- **PRD coverage delta:** the bump closes the gap for `Role::AlpenAdministrator`, `AlpenAdminMultisigUpdate`, `EeStfVkUpdate`, and the BIP-137 signing-format gap that blocked POC-5. It does **not** close Safe Harbor, Security Council, soft/hard bridge update, Defcon, or Payout Administrator (no upstream presence at HEAD — see §6.2).
- **Go / no-go recommendation:** **GO**, but split the migration into four phases (full plan in §8). The breaking digest change cannot be cleanly separated from the rev bump (PR #96 deletes the `Sighash` trait, so the workspace will not compile without migrating the digest call sites), so **Phase 1** must bundle the rename sweep, `strata-common` rc16→rc21 bump, and the digest-call-site migration into one PR — ~3-4 engineering days plus a signature-rotation pass. **Phase 2** (Trezor `signMessage` adapter), **Phase 3** (domain widening for `AlpenAdmin`/`SequencerManager`), and **Phase 4** (doc updates) follow as independent PRs.
- **Estimated effort:** ~5-7 PR-units of engineering, ~1-2 of QA, plus one Trezor on-device validation pass and one signature-rotation procedure for any proposal currently `Pending` against the old digest.
- **Open questions for Alpen (full list in §10):** is `a53b6a8` going to be tagged as `v0.1-alpha.6` or similar? Is the BIP-137 `signMessage` switch final (no further sighash work expected before mainnet)? What is the intended `AdministrationInitConfig.confirmation_depths` for production (the type now has 8 fields)?
- **Cargo.toml change required (preview):**

```toml
strata-asm-proto-admin-txs       = { git = "https://github.com/alpenlabs/asm", rev = "a53b6a8" }
strata-asm-params                = { git = "https://github.com/alpenlabs/asm", rev = "a53b6a8" }
strata-asm-common                = { git = "https://github.com/alpenlabs/asm", rev = "a53b6a8" }
strata-asm-proto-txs-test-utils  = { git = "https://github.com/alpenlabs/asm", rev = "a53b6a8" }
strata-asm-proto-admin           = { git = "https://github.com/alpenlabs/asm", rev = "a53b6a8" }
# strata-common moves rc16 → rc21 (the rev a53b6a8 hard-requires rc21):
strata-crypto                    = { git = "https://github.com/alpenlabs/strata-common", tag = "v0.1.0-alpha-rc21" }
strata-l1-txfmt                  = { git = "https://github.com/alpenlabs/strata-common", tag = "v0.1.0-alpha-rc21" }
strata-l1-envelope-fmt           = { git = "https://github.com/alpenlabs/strata-common", tag = "v0.1.0-alpha-rc21" }
```

(The previous workspace key `strata-asm-txs-admin` is gone, replaced by `strata-asm-proto-admin-txs`; the previous untagged `strata-asm-proto-administration` is gone, replaced by `strata-asm-proto-admin`; `strata-asm-txs-test-utils` is gone, replaced by `strata-asm-proto-txs-test-utils`. See §D and §7.)

---

## 1. Commit-by-commit classification (`a8559d3..a53b6a8`)

Verified locally with `git log --oneline a8559d3..origin/main` inside the cloned ASM repo at `~/Documents/wakeup/alpen-multisign/repo/asm`. 19 commits, ordered newest → oldest:

| SHA | PR | Date | Title | Classification | Section |
|---|---|---|---|---|---|
| `a53b6a8` | #102 | 2026-05-13 | chore(deps): bump moho to v0.1-alpha.6, zkaleido to v0.1-beta.2, SP1 to 6.2.0 | dependency-bump | §6.3 |
| `a6ff40c` | #95 | 2026-05-11 | refactor(checkpoint): extract verification into its own crate | internal-refactor (crate split) | §6.5 |
| `4f37c74` | #100 | 2026-05-12 | fix(asm-runner): use bn254-encoded vk hash for sp1 groth16 verifier | behavior-changing (prover only — not in our path) | §6.3 |
| `def601c` | #97 | 2026-05-09 | chore: update SP1 to v6.1.0 (also silently bumps `strata-common` rc19→rc21, moho alpha.2→alpha.5, zkaleido) | dependency-bump (headline) + **transitive breaking** (rc21) | §5, §6.3 |
| `e0461f8` | #96 | 2026-05-11 | refactor(admin): standardize signing message | **protocol-breaking (signature digest)** | §A |
| `04ecef3` | #94 | 2026-05-08 | chore(deps)(deps): bump codecov/codecov-action from 5.5.2 to 6.0.0 | CI-only | (omit) |
| `00e9ad3` | #93 | 2026-05-07 | chore(deps)(deps): bump astral-sh/setup-uv from 7.3.0 to 8.1.0 | CI-only | (omit) |
| `0768b67` | #92 | 2026-05-06 | chore(deps)(deps): bump the all-dependencies group with 3 updates | dev-dep bump | (omit) |
| `344b707` | #90 | 2026-05-01 | feat(rpc): add `getAsmState` RPC | API-additive (new RPC) | §6.4 |
| `ed35f80` | #87 | 2026-04-30 | feat(admin): per-update confirmation depths | **protocol-breaking (init-config layout)** + state shape change | §D |
| `a38db60` | #86 | 2026-04-29 | feat(rpc): add `getExportEntryMMRProof` & `getMohoState` RPC | API-additive | §6.4 |
| `7b271f1` | #89 | 2026-04-29 | chore(deps): bump strata-common to rc19 | dependency-bump (later superseded by rc21 in #97) | §5 |
| `59a49db` | #88 | 2026-04-29 | chore: skip SP1 guest build during unit and doc tests | tooling-only | (omit) |
| `8b12392` | #85 | 2026-04-29 | Chore/checkpoint types cleanup | internal-refactor | §6.5 |
| `aa236e2` | #82 | 2026-04-26 | fix(admin): switch multisig signatures to bitcoin `signMessage` | **protocol-breaking (signature scheme)** | §B |
| `3afc520` | #78 | 2026-04-24 | feat(asm-admin): add the alpen administrator role | **protocol-additive (new role + 2 update types)** | §C |
| `d0e490f` | #84 | 2026-04-24 | feat(rpc): add `getCheckpointTip` RPC for verified checkpoint tip | API-additive | §6.4 |
| `7a6a167` | #80 | 2026-04-22 | refactor: colocate subprotocol crates and normalize naming | **API-changing (crate renames + module paths)** | §D |
| `f702715` | #79 | 2026-04-21 | feat(moho-state): persist per-block MohoState on ASM worker | internal-additive (storage) — surfaces via #86 RPC | §6.4 |

Five commits dominate the impact assessment: **`e0461f8` (#96)**, **`aa236e2` (#82)**, **`3afc520` (#78)**, **`ed35f80` (#87)**, **`7a6a167` (#80)**. They are covered in §§A-D below. The cross-cutting findings (dep alignment, RPCs, checkpoint extract, SSZ wire format, SP1) are covered in §§5-6.

---

## A. Signing-message standardization — PR #96 (`e0461f8`)

> **TL;DR:** The `Sighash` trait, `compute_sighash`, and per-tx-type `sighash_tag_hash` constants are **deleted**. They are replaced by a `SigningMessage` struct that renders a human-readable string and hashes it with `bitcoin::sign_message::signed_msg_hash` (BIP-137 / Bitcoin `signMessage`). The bytes our signers sign and our verifiers verify **change completely**. Legacy signatures produced against `a8559d3` are **not** valid against `a53b6a8`.

### A.1 What was removed

`crates/txs/admin/src/actions/sighash.rs` at `a8559d3` defined the `Sighash` trait
([`crates/txs/admin/src/actions/sighash.rs:7-46`](https://github.com/alpenlabs/asm/blob/a8559d3/crates/txs/admin/src/actions/sighash.rs#L7-L46)):

```rust
pub trait Sighash {
    fn tx_type(&self) -> AdminTxType;
    fn sighash_payload(&self) -> Vec<u8>;
    fn sighash_tag_hash(&self) -> &'static [u8; 32] {
        self.tx_type().sighash_tag_hash()
    }
    fn compute_sighash(&self, seqno: u64) -> Buf32 {
        let tag_hash: &[u8] = self.sighash_tag_hash();
        let seqno_bytes = seqno.to_be_bytes();
        let payload = self.sighash_payload();
        hash::sha256_iter([tag_hash, &seqno_bytes, &payload])
    }
}
```

The eight precomputed `SHA256("strata/admin/<name>")` tag constants in
[`crates/txs/admin/src/constants.rs:57-91`](https://github.com/alpenlabs/asm/blob/a8559d3/crates/txs/admin/src/constants.rs#L57-L91) (one per `AdminTxType`) are gone. So is `sighash_payload()` on every action — there is no longer any `Vec<u8>` payload built by the admin crate; the hash input is text.

### A.2 What was added

`crates/subprotocols/admin/txs/src/signing_message.rs` on HEAD
([`crates/subprotocols/admin/txs/src/signing_message.rs:1-44`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/subprotocols/admin/txs/src/signing_message.rs#L1-L44)):

```rust
pub const ADMIN_SUBPROTOCOL_VERSION: u8 = 1;

pub struct SigningMessage(String);

impl SigningMessage {
    pub fn for_action(action: &MultisigAction, seqno: u64) -> Self {
        let mut lines = vec![
            format!("Strata ASM Administration v{ADMIN_SUBPROTOCOL_VERSION}"),
            format!("Action: {}", action.tx_type()),
            format!("Authorized By: {}", action.required_role()),
            format!("Sequence: {seqno}"),
            "Action Details:".to_string(),
        ];
        let mut details = IndentedDetails::new(&mut lines);
        action.render_details(&mut details);
        Self(lines.join("\n"))
    }
    pub fn as_str(&self) -> &str { &self.0 }
    pub fn compute_sighash(&self) -> Buf32 {
        Buf32::from(signed_msg_hash(&self.0).to_byte_array())
    }
}
```

`signed_msg_hash` is `bitcoin::sign_message::signed_msg_hash` from `rust-bitcoin 0.32` — it implements the standard BIP-137 / Bitcoin `signMessage` digest:

```
SHA256( SHA256( "\x18Bitcoin Signed Message:\n" || compact_size(len(msg)) || msg ) )
```

where `msg` is the rendered string above. Action-first-then-role ordering, indented details, version banner all matter: every line is committed to the digest.

A concrete sample of what hardware wallets display and sign (from upstream's own
[`crates/subprotocols/admin/txs/src/actions/updates/ee_stf_vk.rs:46-58`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/subprotocols/admin/txs/src/actions/updates/ee_stf_vk.rs#L46-L58) test):

```
Strata ASM Administration v1
Action: EE STF VK Update
Authorized By: Alpen Administrator
Sequence: 11
Action Details:
  Predicate Type: Sp1Groth16
  Predicate Hex: cafe
```

A multisig update sample (from
[`crates/subprotocols/admin/txs/src/actions/updates/alpen_admin_multisig.rs:55-67`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/subprotocols/admin/txs/src/actions/updates/alpen_admin_multisig.rs#L55-L67)):

```
Strata ASM Administration v1
Action: Alpen Administrator Multisig Update
Authorized By: Alpen Administrator
Sequence: 12
Action Details:
  New Threshold: 2
  Members to Add: 1
  1. Add Member: 020202020202020202020202020202020202020202020202020202020202020202
  Members to Remove: 0
```

The indentation is **exactly two spaces**, hard-coded in `IndentedDetails::push` ([`crates/subprotocols/admin/txs/src/actions/sighash.rs:14-23`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/subprotocols/admin/txs/src/actions/sighash.rs#L14-L23)). Hex strings are lower-case, leading `02` not `0x02`. The line separator is `\n` (single byte, LF only — confirmed by the `lines.join("\n")` call). These are all canonical bytes that the verifier reads.

### A.3 Byte-level before/after — same logical action

To make the breakage explicit, consider the smallest action we currently sign in this workspace
(`Action::MultisigUpdate { role: StrataAdmin, add_keys: [<33-byte pk>], remove_keys: [], new_threshold: 2 }`, `seqno = 1`).

**Before (`a8559d3`):** the message hashed by `compute_sighash(1)` is
([`crates/txs/admin/src/actions/sighash.rs:33-42`](https://github.com/alpenlabs/asm/blob/a8559d3/crates/txs/admin/src/actions/sighash.rs#L33-L42)):

```
SHA256(SHA256("strata/admin/strata_admin_multisig_update"))   = 020eaac5…
  ‖ seqno_be(1)                                               = 00 00 00 00 00 00 00 01
  ‖ len_be(add)=1                                             = 00 00 00 01
  ‖ add[0]                                                    = 02c604…ee5  (33 bytes)
  ‖ len_be(rem)=0                                             = 00 00 00 00
  ‖ threshold                                                 = 02
———————————————————————————————————————————————————————————
SHA256-hashed once more → 32-byte digest fed to raw secp256k1 ECDSA
```

The 8-byte big-endian `seqno`, the `u32` BE length prefixes, the byte-identical 33-byte compressed pubkeys, and the single-byte threshold are all defined hand-coded
in [`crates/txs/admin/src/actions/updates/multisig.rs:46-65`](https://github.com/alpenlabs/asm/blob/a8559d3/crates/txs/admin/src/actions/updates/multisig.rs#L46-L65).

**After (`a53b6a8`):** the message hashed is the literal UTF-8 string:

```
Strata ASM Administration v1\n\
Action: Strata Administrator Multisig Update\n\
Authorized By: Strata Administrator\n\
Sequence: 1\n\
Action Details:\n  \
New Threshold: 2\n  \
Members to Add: 1\n  \
1. Add Member: 02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5\n  \
Members to Remove: 0
```

— wrapped in the BSM prefix and double-SHA256'd. The result is **fully unrelated** to the
old digest: different domain string, different prefix, different hash construction, different bytes.

**Verdict — backward compatibility:** none. Any signature produced against `a8559d3` is silently rejected by `a53b6a8`. Any signature collected on `a53b6a8` is silently rejected by `a8559d3`. There is no shared subset.

### A.4 Migration entry points — old → new (mandatory call sequence)

| Where we are today (`a8559d3`) | Where we go on `a53b6a8` |
|---|---|
| `action.compute_sighash(seqno)` (`Sighash` trait method) | `SigningMessage::for_action(&action, seqno).compute_sighash()` |
| `action.sighash_payload()` | **GONE** — no equivalent. The rendered string is the payload. |
| `<TxType>::sighash_tag_hash()` (8 hex constants) | **GONE** — no domain separation byte string; domain separation is in the `"Strata ASM Administration v1"` prefix line + version byte. |
| `Sighash` trait | Replaced by `RenderSigningMessage` (crate-private, only used to render details) and `SigningMessage` (public). Trait is not implemented by consumers — we just call the helper. |
| `MultisigUpdate::new(config, role)` (action carries `role` field) | `StrataAdminMultisigUpdate::new(config)` / `StrataSeqManagerMultisigUpdate::new(config)` / `AlpenAdminMultisigUpdate::new(config)` — one type per role; role is implicit in variant. See §C and §D.4. |
| `MultisigAction::Update(UpdateAction::Multisig(MultisigUpdate))` | `MultisigAction::Update(UpdateAction::StrataAdminMultisig(StrataAdminMultisigUpdate))` (the 4-variant `UpdateAction` is now 8-variant — see §C). |
| `test_utils::create_signature_set(privkeys, indices, sighash: Buf32)` | `test_utils::create_signature_set(privkeys, indices, action: &MultisigAction, seqno: u64)` — now takes the action and computes the digest internally. The `signed_msg_hash`-built digest is **never** exposed externally; tests pass the action through.<br/>Old signature: [`crates/txs/admin/src/test_utils/mod.rs:42-56`](https://github.com/alpenlabs/asm/blob/a8559d3/crates/txs/admin/src/test_utils/mod.rs#L42-L56). New signature: [`crates/subprotocols/admin/txs/src/test_utils/mod.rs:50-66`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/subprotocols/admin/txs/src/test_utils/mod.rs#L50-L66). |
| `MultisigAuthority::verify_threshold_signatures(...)` (internal to authority) | `MultisigAuthority::verify_action_signature(&SignedPayload, max_seqno_gap)` — now calls `SigningMessage::for_action(...).compute_sighash()` internally and returns a `SeqNoToken` proof token before the state mutation. See [`crates/subprotocols/admin/subprotocol/src/authority.rs:64-94`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/subprotocols/admin/subprotocol/src/authority.rs#L64-L94). |

### A.5 Impact on our workspace

| File | Symbol | Current | Required change | Tests affected |
|---|---|---|---|---|
| `desktop-app/src-tauri/src/infrastructure/signing.rs` | `compute_sighash(seqno, action_hex)` (lines [54-65](../../desktop-app/src-tauri/src/infrastructure/signing.rs#L54-L65)) | `action.compute_sighash(seqno)` | `SigningMessage::for_action(&action, seqno).compute_sighash()`. Consider also exposing `signing_message_text(seqno, action_hex) -> Result<String>` to feed Trezor's `sign_message` API directly (see §B) | `test_compute_sighash_returns_valid_32_byte_hash`, `test_compute_sighash_deterministic`, `test_compute_sighash_different_seqno`, `test_sign_sighash_success`, `test_verify_threshold_full_flow_2_of_3` |
| `desktop-app/src-tauri/src/infrastructure/signing.rs` | `use strata_asm_txs_admin::actions::{MultisigAction, Sighash};` (line [10](../../desktop-app/src-tauri/src/infrastructure/signing.rs#L10)) | `Sighash` import removed | `use strata_asm_proto_admin_txs::actions::MultisigAction; use strata_asm_proto_admin_txs::signing_message::SigningMessage;` | — |
| `desktop-app/src-tauri/src/infrastructure/action_codec.rs` | `use strata_asm_txs_admin::actions::updates::multisig::MultisigUpdate as StrataMultisigUpdate;` (line [11](../../desktop-app/src-tauri/src/infrastructure/action_codec.rs#L11)) | type and module path both gone | `use strata_asm_proto_admin_txs::actions::updates::StrataAdminMultisigUpdate;` etc. The codec must select the role-specific variant (`StrataAdminMultisig` for `Authority::StrataAdmin`, `StrataSeqManagerMultisig` for `SequencerManager`, `AlpenAdminMultisig` for `AlpenAdmin`). | `test_roundtrip_hex`, `test_roundtrip_bytes`, `test_encode_matches_direct_strata_ssz` |
| `desktop-app/src-tauri/src/infrastructure/action_codec.rs` | `to_strata_multisig_update` builds `StrataMultisigUpdate::new(config, role)` (lines [74-92](../../desktop-app/src-tauri/src/infrastructure/action_codec.rs#L74-L92)) | role no longer carried inside `MultisigUpdate` | Build the right wrapper struct per role; remove `to_strata_role` parameter from the new-type constructor. | same |
| `desktop-app/src-tauri/src/infrastructure/action_codec.rs` | `from_strata_action` arm `UpdateAction::Multisig(update)` (lines [108-124](../../desktop-app/src-tauri/src/infrastructure/action_codec.rs#L108-L124)) | variant `Multisig` deleted | 8 arms: `StrataAdminMultisig`, `StrataSeqManagerMultisig`, `AlpenAdminMultisig`, `OperatorSet`, `Sequencer`, `OlStfVk`, `AsmStfVk`, `EeStfVk` (the latter five remain `UnsupportedVariant` until our domain model grows — see §6.2). | same |
| `orchestrator-be/src/application/proposals.rs` | `compute_sighash_for_proposal` (lines [422-431](../../orchestrator-be/src/application/proposals.rs#L422-L431)) | `action.compute_sighash(seq_no)` | `SigningMessage::for_action(&action, proposal.seq_no).compute_sighash().0` | none (broadcast smoke tests against regtest are required) |
| `orchestrator-be/src/infrastructure/broadcast_tx.rs` | the 64-byte recoverable-signature branch in `build_signed_payload_bytes` (lines [60-107](../../orchestrator-be/src/infrastructure/broadcast_tx.rs#L60-L107)) | tries all four recovery IDs against the old digest | The sighash is now BSM. Signatures from a Trezor in `signMessage` mode already arrive 65-byte with BIP-137 header (27..42). The new `strata-crypto` verifier in rc21 strips that header transparently (see §5). We can keep the 64-byte fallback only for the software-mnemonic path (which still emits raw 64 bytes via `sign_with_mnemonic_path`). | none (covered by `e2e_admin_commit_reveal_broadcast_and_verify` once it is migrated) |
| `e2e-tests/tests/e2e_admin_subprotocol.rs` | `use ...Sighash; action.compute_sighash(seqno)` (lines [25, 78](../../e2e-tests/tests/e2e_admin_subprotocol.rs#L25)) | same | same migration as `signing.rs` | the test itself |
| `e2e-tests/tests/e2e_admin_commit_reveal.rs` | `create_signature_set(&privkeys, &signer_indices, sighash)` (line [68](../../e2e-tests/tests/e2e_admin_commit_reveal.rs#L68)) | signature changed | `create_signature_set(&privkeys, &signer_indices, &action, seqno)` | the test itself |
| `e2e-tests/tests/e2e_propose_sign.rs` | `signing::compute_sighash(...)` (lines [142, 228](../../e2e-tests/tests/e2e_propose_sign.rs#L142)) | unchanged at the wrapper level — but the Tauri command now returns the BSM digest | none in the test file itself; the test asserts that `verify_threshold` accepts the produced sig. That still holds. | regression on the BSM path; add an explicit assertion that the digest matches `signed_msg_hash(rendered)` |

### A.6 The POC-4 byte-equivalence test (`test_encode_matches_direct_strata_ssz`)

This test ([`action_codec.rs:195-209`](../../desktop-app/src-tauri/src/infrastructure/action_codec.rs#L195-L209)) is **independent of the signing-message change** — it only asserts SSZ-encoding parity for `MultisigAction`, not signature parity. It survives the bump **conditionally**:

- Yes: if our codec is updated to build the new `MultisigAction::Update(UpdateAction::StrataAdminMultisig(StrataAdminMultisigUpdate::new(config_update)))` instead of `Multisig(MultisigUpdate::new(config_update, Role::StrataAdministrator))`. The SSZ union tag byte for our action moves from `1` (Multisig=index-0 of `UpdateAction` wrapped in `Update`=index-1 of `MultisigAction`) to a new value driven by the new variant ordering (`StrataAdminMultisig` = index-0 of new `UpdateAction`). Our test fixture and the upstream byte stream will both reflect the new union tag, so equality holds.
- No: if we ship the rev bump but keep the codec on the old import paths, the workspace won't compile (the old types don't exist) — the test fails at compile time, not at the byte-compare assertion. Either way it is a hard gate.

The SSZ wire format for the inner `ThresholdConfigUpdate` (`Vec<CompressedPublicKey>`, `Vec<CompressedPublicKey>`, `NonZero<u8>`) is unchanged — `strata-crypto`'s `ThresholdConfigUpdate` definition in rc21 retains the same SSZ derive (we verified the field set is identical in [`strata-common/crates/crypto/src/threshold_signature/indexed/mod.rs`](https://github.com/alpenlabs/strata-common/blob/v0.1.0-alpha-rc21/crates/crypto/src/threshold_signature/indexed/mod.rs) when fetching the tag locally). See §6.6 for the full SSZ inventory.

### A.7 Signer-safety implications

Two are large and must be considered:

1. **Replay risk via ambiguous rendering.** The new digest binds the text exactly, including the version line `Strata ASM Administration v1`. The version byte is `ADMIN_SUBPROTOCOL_VERSION = 1u8` upstream ([`signing_message.rs:7-9`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/subprotocols/admin/txs/src/signing_message.rs#L7-L9)). Any later bump of that constant invalidates all previously-collected signatures by construction — this is a feature (Alpen's stated rationale: "bumping it on any breaking change to the subprotocol after deployment ensures admin signatures cannot be reinterpreted under new subprotocol semantics"). For us this means a `SigningMessage::ADMIN_SUBPROTOCOL_VERSION` constant **must be surfaced** to the orchestrator and pinned in proposal records, so we can refuse to broadcast signatures collected under a different version.
2. **What the user sees ≠ what is signed used to be a documentation gap (POC-5 §4).** It is now closed for free: the rendered string IS the payload, and hardware wallets that display it show the same bytes that get signed. The two-space indent is on the wire; OL/EE STF VK hashes longer than 32 bytes are displayed as their hash, not their full bytes — see [`crates/subprotocols/admin/txs/src/actions/updates/render.rs:30-44`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/subprotocols/admin/txs/src/actions/updates/render.rs#L30-L44):

   ```rust
   if condition.len() <= 32 {
       details.push(format!("Predicate Hex: {}", hex::encode(condition)));
   } else {
       details.push(format!("Predicate Hash: {:x}", hash::raw(condition)));
   }
   ```

   The 32-byte cutoff is a stable, documented contract — but is asymmetric across hardware screens; a Trezor T can display ~30 lines, a Ledger Nano X far fewer. The desktop app must show the exact rendered string in the proposal-review UI so the on-device view is verifiable line-by-line.

---

## B. Switch to Bitcoin `signMessage` — PR #82 (`aa236e2`)

> **TL;DR:** Combined with §A, the digest is no longer raw ECDSA over a tagged SHA256. The signer signs the BIP-137 BSM digest of the rendered message. The verifier accepts BIP-137 header bytes 27-42 natively (in `strata-common` rc21). Hardware wallets that previously could not produce SPS-65-valid signatures can now sign admin actions through their stock `signMessage` flow.

This PR predates #96 chronologically (Apr-26 vs May-11). At the time of merge it kept the *concept* of a sighash trait but changed the digest computation. PR #96 then deleted that trait. The combined effect is the one described in §A — i.e. by HEAD the only entry point is `SigningMessage::for_action(...).compute_sighash()` calling `signed_msg_hash`.

### B.1 What the verifier accepts (signature wire format)

The threshold verifier on HEAD reads 65-byte `IndexedSignature` records (`header || r || s`) and runs ECDSA recovery + key match. The header byte is normalized from BIP-137's 27-42 range *or* raw 0-3, in [`strata-common/crates/crypto/src/threshold_signature/indexed/verification/ecdsa.rs:23-33`](https://github.com/alpenlabs/strata-common/blob/v0.1.0-alpha-rc21/crates/crypto/src/threshold_signature/indexed/verification/ecdsa.rs#L23-L33) (fetched locally):

```rust
fn normalize_recovery_id(header: u8) -> Result<i32, ThresholdSignatureError> {
    let recid = match header {
        0..=3 => header,        // Raw format
        27..=30 => header - 27, // Uncompressed P2PKH
        31..=34 => header - 31, // Compressed P2PKH
        35..=38 => header - 35, // SegWit P2SH
        39..=42 => header - 39, // Native SegWit
        _ => return Err(ThresholdSignatureError::InvalidSignatureFormat),
    };
    Ok(recid as i32)
}
```

The verifier then recovers the pubkey from `RecoverableSignature::from_compact(&compact, normalized_recid)` and **compares to the configured key in the threshold**. This is a *recovery-first* model: it doesn't matter what flag byte the hardware wallet used, only that the recovered key matches.

Upstream test helper that mimics a HW signature is BIP-137 compressed-P2PKH (header `31+raw_recid`):

```rust
// crates/subprotocols/admin/txs/src/test_utils/mod.rs:31-35
pub fn sign_ecdsa_bip137(message_hash: &[u8; 32], secret_key: &SecretKey) -> [u8; 65] {
    let message = Message::from_digest(*message_hash);
    let signature = SECP256K1.sign_ecdsa_recoverable(&message, secret_key);
    MessageSignature::new(signature, true).serialize()
}
```

`MessageSignature::serialize()` in `rust-bitcoin 0.32` is BIP-137 layout: `header_byte || r || s`, 65 bytes total.

### B.2 Mapping to our three signer paths

| Path | Output today | Works on HEAD? | Required change |
|---|---|---|---|
| `desktop-app/src-tauri/src/infrastructure/signing.rs::sign_sighash` (raw demo key) ([lines 67-84](../../desktop-app/src-tauri/src/infrastructure/signing.rs#L67-L84)) | 64-byte compact `r‖s` over raw 32-byte digest | **No** — wrong digest construction. | Sign the BSM digest emitted by `compute_sighash` (which is now `signed_msg_hash(rendered)`). Output stays 64-byte compact; the orchestrator broadcaster already tries all four `recid` bytes ([`broadcast_tx.rs:60-107`](../../orchestrator-be/src/infrastructure/broadcast_tx.rs#L60-L107)). |
| `sign_with_mnemonic_path` ([lines 135-152](../../desktop-app/src-tauri/src/infrastructure/signing.rs#L135-L152)) | 64-byte compact over raw 32-byte digest | **No** — same reason. | Same as above; nothing else changes in this path because it already operates on `sighash_hex`. The regression test `test_mnemonic_signature_verifies_against_raw_sighash` ([line 360](../../desktop-app/src-tauri/src/infrastructure/signing.rs#L360)) keeps asserting "verifies against the digest fed in", which is still true — the digest changes upstream of the call. |
| Trezor adapter (POC-5, `desktop-app/src-tauri/src/infrastructure/hw_wallet/trezor.rs`) | Today: PSBT binding signing a synthetic SegWit sighash, then commit `admin_digest` in OP_RETURN as evidence only. See [`16-poc5-trezor-findings.md`](./16-poc5-trezor-findings.md) §4.2, §11 row B. | **Yes**, now natively via `SignMessage` — and **only via `SignMessage`**. The PSBT binding becomes obsolete: the protocol-level digest is exactly what `SignMessage` produces, byte-for-byte. | Replace `sign_admin_sps65_binding` with a direct `MessageSignReq { msg = rendered_signing_message_string, path = m/86'/0'/73'/0/n }` call. Result is a BIP-137-formatted 65-byte signature accepted by the verifier as-is. Update the Tauri command to take the rendered string (not the digest) so the Trezor screen shows what the user is signing. **Massive UX win** — see §B.4. |
| Ledger (HWI / hwi-rs) | Not implemented today ([`07-hardware-wallet-library-analysis.md`](./07-hardware-wallet-library-analysis.md)) | **Yes**, same story as Trezor: Ledger's Bitcoin app `SIGN_MESSAGE` (INS 0x10) outputs a BIP-137-formatted recoverable signature over `signed_msg_hash(msg)`. The flag byte will be 27-34 or 39-42; the verifier normalizes either. | Add a `LedgerAdapter` that exposes the same `sign_message(path, rendered_message)` shape. |
| Software signer (Tauri demo) | already raw ECDSA over the digest | still works once digest changes |  no path-specific change required |

### B.3 Closing the POC-5 "BIP-137 vs SPS-65" gap

The gap previously documented in [`external/research-assessment.md`](../external/research-assessment.md) §2 (BIP-137 vs SPS-65) — "the Alpen ASM expects bare ECDSA over the raw SPS-65 sighash. Both Trezor and Ledger apply the BIP-137 prefix before hashing — these are incompatible. Recommendation: Add BIP-137 support to the crate asm." — is **closed by this PR**.

Concretely: the recommendation has been *exceeded*. Alpen did not bolt BIP-137 onto the side; they replaced the digest construction entirely with `signed_msg_hash`, which is the BIP-137 prefix. From our POC-5 implementation options table ([`16-poc5-trezor-findings.md`](./16-poc5-trezor-findings.md) §11), option **A (software admin signing)** and option **B (Trezor PSBT binding)** become equivalent and option **C (BIP-137 message sign)** becomes the recommended primary path. Options D (custom Ledger app) and E (protocol change) are no longer needed.

### B.4 Signer-safety implications

- **User sees what is signed.** The hardware wallet displays the rendered SigningMessage string verbatim. Replay risk shrinks because the version line and sequence number are inside the message a user reads on-device.
- **Cross-protocol replay risk.** The BIP-137 prefix is **not** application-specific — any application that signs a message starting with `Strata ASM Administration v1` produces the same digest. The Bitcoin signMessage standard does not have per-application domain separation other than the prefix and the message body. Mitigation: the `"Strata ASM Administration v1"` first line is the domain tag. Adversary models that include "user can be tricked into signing a different application's message with the same first line" should be reviewed. (Practical risk is low: no other application asks users to sign a multi-line string starting with that literal phrase, but it is no longer cryptographically impossible.)
- **Header-byte fungibility.** Because the verifier normalizes header bytes 0-3 / 27-42, a software signer (raw header `0..3`) and a HW signer (header `31..34` or `39..42`) produce interchangeable sigs as far as ASM is concerned. The orchestrator's `build_signed_payload_bytes` should preserve the original header byte rather than re-deriving (today it does `recid_byte || r || s` for the 64-byte mnemonic path; for the 65-byte HW path it should keep the header **as received**).

---

## C. New roles and update types — PR #78 (`3afc520`) plus follow-ups

> **TL;DR:** `Role` gains a third variant, `AlpenAdministrator`. `UpdateTxType` (the renamed, refactored `AdminTxType`) gains two new discriminants: `AlpenAdminMultisigUpdate = 12` and `EeStfVkUpdate = 32`. The four existing role-and-update slots are kept stable. The `UpdateAction` enum is *renamed in shape* (8 variants instead of 4) but the byte discriminants for the additions are appended at the end. No existing discriminant changes value; however, the inner SSZ union tag layout of `UpdateAction` itself changes because new variants are interleaved. See §C.3.

### C.1 Roles — old vs new

`crates/params/src/subprotocols/admin.rs` at `a8559d3` defined `Role` with **2** variants ([file:36-57](https://github.com/alpenlabs/asm/blob/a8559d3/crates/params/src/subprotocols/admin.rs#L36-L57)). On HEAD, [`crates/params/src/subprotocols/admin/roles.rs:14-37`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/params/src/subprotocols/admin/roles.rs#L14-L37) has **3**, with the new one appended at the end:

```rust
#[repr(u8)]
#[ssz(enum_behaviour = "tag")]
pub enum Role {
    StrataAdministrator,      // SSZ tag 0 — unchanged
    StrataSequencerManager,   // SSZ tag 1 — unchanged
    AlpenAdministrator,       // SSZ tag 2 — NEW
}
```

Because the SSZ behavior is `tag` (1-byte tag, no payload) and variant ordering is preserved, the SSZ encoding of `Role::StrataAdministrator` and `Role::StrataSequencerManager` is byte-identical pre/post bump. The only impact is that any code matching `Role` exhaustively without a wildcard arm (none in our repo) breaks at compile time.

The rendered names also matter — they end up inside the signed message body:

```rust
impl Role {
    pub fn name(&self) -> &'static str {
        match self {
            Role::StrataAdministrator      => "Strata Administrator",
            Role::StrataSequencerManager   => "Strata Sequencer Manager",
            Role::AlpenAdministrator       => "Alpen Administrator",
        }
    }
}
```

### C.2 Update tx types — old vs new

The old `AdminTxType` (in `crates/txs/admin/src/constants.rs` at `a8559d3`) was a single enum with 7 variants. It was split into:

- `crates/params/src/subprotocols/admin/admin_tx.rs::AdminTxType` — high-level `Cancel | Update(UpdateTxType)` (replaces the flat enum).
- `crates/params/src/subprotocols/admin/updates.rs::UpdateTxType` — the granular update kinds with byte discriminants.

The byte discriminants on `UpdateTxType` ([file:10-21](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/params/src/subprotocols/admin/updates.rs#L10-L21)) are:

| `UpdateTxType` | u8 | Old `AdminTxType` u8 | Authorized role |
|---|---|---|---|
| `StrataAdminMultisigUpdate` | 10 | 10 — unchanged | StrataAdministrator |
| `StrataSeqManagerMultisigUpdate` | 11 | 11 — unchanged | StrataSequencerManager |
| `AlpenAdminMultisigUpdate` | **12 (NEW)** | not present | AlpenAdministrator |
| `OperatorUpdate` | 20 | 20 — unchanged | StrataAdministrator |
| `SequencerUpdate` | 21 | 21 — unchanged | StrataSequencerManager |
| `OlStfVkUpdate` | 30 | 30 — unchanged | StrataAdministrator |
| `AsmStfVkUpdate` | 31 | 31 — unchanged | StrataAdministrator |
| `EeStfVkUpdate` | **32 (NEW)** | not present | AlpenAdministrator |
| `AdminTxType::Cancel` | 0 | 0 — unchanged | (derived from target) |

The `authorized_role()` association is in [`updates.rs:30-43`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/params/src/subprotocols/admin/updates.rs#L30-L43). No previously-issued (`Pending`/`Approved` but not yet `Enacted`) proposal would suffer a discriminant collision — values 12 and 32 were unused before. The only way an in-flight proposal could become unparseable is the signing-message change (§A); the SPS-50 tag value stays the same.

### C.3 `UpdateAction` shape change — 4 → 8 variants (the breaking part)

At `a8559d3` ([`crates/txs/admin/src/actions/updates/mod.rs:1-50`](https://github.com/alpenlabs/asm/blob/a8559d3/crates/txs/admin/src/actions/updates/mod.rs#L1-L50)):

```rust
#[ssz(enum_behaviour = "union")]
pub enum UpdateAction {
    Multisig(MultisigUpdate),          // union tag 0
    OperatorSet(OperatorSetUpdate),    // union tag 1
    Sequencer(SequencerUpdate),        // union tag 2
    VerifyingKey(PredicateUpdate),     // union tag 3
}
```

At `a53b6a8` ([`crates/subprotocols/admin/txs/src/actions/updates/mod.rs:24-40`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/subprotocols/admin/txs/src/actions/updates/mod.rs#L24-L40)):

```rust
#[ssz(enum_behaviour = "union")]
pub enum UpdateAction {
    StrataAdminMultisig(StrataAdminMultisigUpdate),                 // union tag 0
    StrataSeqManagerMultisig(StrataSeqManagerMultisigUpdate),       // union tag 1
    AlpenAdminMultisig(AlpenAdminMultisigUpdate),                   // union tag 2
    OperatorSet(OperatorSetUpdate),                                 // union tag 3 ← was 1
    Sequencer(SequencerUpdate),                                     // union tag 4 ← was 2
    OlStfVk(OlStfVkUpdate),                                         // union tag 5
    AsmStfVk(AsmStfVkUpdate),                                       // union tag 6
    EeStfVk(EeStfVkUpdate),                                         // union tag 7
}
```

This is a **wire-format break** for the inner `UpdateAction` union tag of any non-`Multisig` variant. Two consequences:

1. **In-flight proposals.** Any proposal in our backend whose `action_hex` was built against `a8559d3` encodes `UpdateAction::Multisig(MultisigUpdate { config, role })`. On `a53b6a8` the same union tag byte (0) now decodes as `StrataAdminMultisig(StrataAdminMultisigUpdate { config })` — same union tag, **different inner struct shape** (no `role` field). For our current sole-use case (Strata Admin multisig update), this is *probabilistically* lucky: the inner SSZ bytes of `MultisigUpdate { config, role: StrataAdministrator }` and `StrataAdminMultisigUpdate { config }` are almost equal — they differ only by the trailing `role` byte (a 1-byte SSZ tag). So the old bytes will decode as `StrataAdminMultisigUpdate` plus 1 byte of *unexpected* trailing bytes, which SSZ rejects strictly. Empirical check: SSZ derive for `ssz_derive::Decode` rejects extra bytes; the decode will fail.
2. **`MultisigUpdate.role` is gone from the wire.** Today our codec serializes `MultisigUpdate::new(config_update, Role::StrataAdministrator)` ([`action_codec.rs:88-92`](../../desktop-app/src-tauri/src/infrastructure/action_codec.rs#L88-L92)). After the bump, the wire format **drops** the role byte because the role is implicit in the variant. Two SSZ encodings of "Strata Admin multisig update with empty add/remove and threshold 2" between old and new differ by exactly one trailing byte. We must re-encode every queued or in-flight proposal.

### C.4 `CancelAction` carries the full target update — breaking

At `a8559d3` ([`crates/txs/admin/src/actions/cancel.rs:5-20`](https://github.com/alpenlabs/asm/blob/a8559d3/crates/txs/admin/src/actions/cancel.rs#L5-L20)):

```rust
pub struct CancelAction { target_id: UpdateId }
```

At `a53b6a8` ([`crates/subprotocols/admin/txs/src/actions/cancel.rs:8-32`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/subprotocols/admin/txs/src/actions/cancel.rs#L8-L32)):

```rust
pub struct CancelAction {
    target_id: UpdateId,
    update: UpdateAction,
}
```

The handler ([`crates/subprotocols/admin/subprotocol/src/handler.rs:97-105`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/subprotocols/admin/subprotocol/src/handler.rs#L97-L105)) explicitly checks that the embedded `update` equals the queued action and emits `CancelUpdateMismatch` otherwise:

```rust
let queued = state.find_queued(cancel.target_id()).ok_or(UnknownAction(*cancel.target_id()))?;
if queued.action() != cancel.update() {
    return Err(CancelUpdateMismatch { target_id: *cancel.target_id() });
}
state.remove_queued(cancel.target_id());
```

This **supersedes** the cancel design proposed in [`17-cancel-action.md`](./17-cancel-action.md), which assumed `CancelAction { target_id: UpdateId }` only. The "Cancel as a signed payload" diagram in that document (and the migration plan) need to be re-rendered with the new field. Our domain `Action` enum (`desktop-app/src-tauri/src/domain/action.rs`) does not yet have a `Cancel` variant, so we don't break anything we have today — but when we add cancel, the design must embed the full target update.

### C.5 PRD coverage matrix — re-validated against `a53b6a8`

This is the updated mirror of [`08-alpen-crate-prd-coverage.md`](./08-alpen-crate-prd-coverage.md) §3 / §2. Every entry below has been validated against `crates/params/src/subprotocols/admin/roles.rs` and the `updates/` folder at HEAD.

#### C.5.1 PRD roles vs `Role` enum on HEAD

| PRD Role | `Role` variant on HEAD | Coverage now | Coverage was on `a8559d3` | Delta |
|---|---|---|---|---|
| Alpen Administrator | `AlpenAdministrator` | **100% (NEW)** — has its own `MultisigAuthority` slot in `AdministrationInitConfig.alpen_administrator` ([`config.rs:30-33`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/params/src/subprotocols/admin/config.rs#L30-L33)) | 0% | **GAP CLOSED** |
| Strata Administrator | `StrataAdministrator` | Same authority surface as before — 4 upstream `UpdateTxType` variants are authorized by this role on HEAD (`StrataAdminMultisigUpdate`, `OperatorUpdate`, `OlStfVkUpdate`, `AsmStfVkUpdate`), see [`updates.rs:30-43`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/params/src/subprotocols/admin/updates.rs#L30-L43). The "43% — 3 of 7" figure in [`08-alpen-crate-prd-coverage.md`](./08-alpen-crate-prd-coverage.md) §3 is a coverage-against-**PRD** metric (3 PRD update types out of 7 PRD-requested for this role); that figure is unaffected by this bump (no new Strata-Admin-authored PRD type was added upstream). | 43% PRD coverage | unchanged |
| Strata Sequencer Manager | `StrataSequencerManager` | 100% — same 2 update types (`StrataSeqManagerMultisigUpdate`, `SequencerUpdate`) | 100% | unchanged |
| Security Council | not present | 0% | 0% | **OPEN** |
| Payout Administrator | not present | 0% | 0% | **OPEN** — still a Bitcoin-native UTXO spend, not an admin variant |

#### C.5.2 PRD update types vs upstream variants on HEAD

| PRD Update Type | Authority | `UpdateTxType` (HEAD) | `AdminTxType` u8 | Status on HEAD | Status on `a8559d3` |
|---|---|---|---|---|---|
| Strata Administrator Signer update | Strata Admin | `StrataAdminMultisigUpdate` | 10 | Available | Available |
| Strata verification key update | Strata Admin | `OlStfVkUpdate` | 30 | Available | Available |
| Operator update | Strata Admin | `OperatorUpdate` | 20 | Available | Available |
| ASM STF VK update | Strata Admin | `AsmStfVkUpdate` | 31 | Available | Available |
| Sequencer Manager Signer update | Seq Manager | `StrataSeqManagerMultisigUpdate` | 11 | Available | Available |
| Sequencer update | Seq Manager | `SequencerUpdate` | 21 | Available | Available |
| Cancel action | Admin / Seq Mgr / Alpen Admin | `AdminTxType::Cancel` | 0 | Available (with `CancelAction { target_id, update }` payload) | Available (with `CancelAction { target_id }` only) — **shape changed** |
| **Alpen verification key update (EE STF VK)** | Alpen Admin | `EeStfVkUpdate` | **32 (NEW)** | **Available** | Blocked |
| **Alpen Administrator Signer update** | Alpen Admin | `AlpenAdminMultisigUpdate` | **12 (NEW)** | **Available** | Blocked |
| Safe Harbor address update | Strata Admin | — | — | **Open** — zero upstream references | Open |
| Security Council Signer update | Security Council | — | — | **Open** | Open |
| "Soft" bridge update | Strata Admin | — | — | **Open** — zero upstream references | Open |
| "Hard" bridge update | Strata Admin | — | — | **Open** | Open |
| Defcon 1 transaction | Security Council | — | — | **Open** | Open |
| Defcon 3 transaction | Security Council | — | — | **Open** | Open |
| Payout Administrator | n/a | — (Bitcoin-native UTXO spend, not admin subprotocol) | — | **Open by design** | Open by design |

#### C.5.3 Net gap-closure delta from the bump

Two PRD items move from **Blocked** to **Available**:

- **Alpen Administrator Signer update** (`AlpenAdminMultisigUpdate`, role `AlpenAdministrator`).
- **Alpen verification key update** (`EeStfVkUpdate`, role `AlpenAdministrator`).

Eight items remain **Open** with no upstream presence: Safe Harbor, Security Council Signer, soft/hard bridge update, Defcon 1, Defcon 3, Payout Administrator, and the "Alpen Administrator" *as distinct from* Strata Administrator (the role is now distinct on-chain, but the PRD lists separate update kinds — see Open Questions §10 #4).

---

## D. Per-update confirmation depths (#87) and admin module restructure (#80)

### D.1 Per-update confirmation depths — `ed35f80`

Old `AdministrationInitConfig` at `a8559d3` ([`crates/params/src/subprotocols/admin.rs:18-40`](https://github.com/alpenlabs/asm/blob/a8559d3/crates/params/src/subprotocols/admin.rs#L18-L40)):

```rust
pub struct AdministrationInitConfig {
    pub strata_administrator: ThresholdConfig,
    pub strata_sequencer_manager: ThresholdConfig,
    pub confirmation_depth: u16,          // ← single, applies to ALL update types
    #[ssz(with = "non_zero_u8")]
    pub max_seqno_gap: NonZero<u8>,
}
```

New on HEAD ([`crates/params/src/subprotocols/admin/config.rs:19-39`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/params/src/subprotocols/admin/config.rs#L19-L39)):

```rust
pub struct AdministrationInitConfig {
    pub strata_administrator: ThresholdConfig,
    pub strata_sequencer_manager: ThresholdConfig,
    pub alpen_administrator: ThresholdConfig,             // ← NEW
    pub confirmation_depths: ConfirmationDepths,          // ← STRUCT, not scalar
    #[ssz(with = "non_zero_u8")]
    pub max_seqno_gap: NonZero<u8>,
}
```

with `ConfirmationDepths` ([`confirmation_depth.rs:21-30`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/params/src/subprotocols/admin/confirmation_depth.rs#L21-L30)) having 8 named `u16` fields, one per `UpdateTxType`:

```rust
pub struct ConfirmationDepths {
    pub strata_admin_multisig_update: u16,
    pub strata_seq_manager_multisig_update: u16,
    pub alpen_admin_multisig_update: u16,
    pub operator_update: u16,
    pub sequencer_update: u16,
    pub ol_stf_vk_update: u16,
    pub asm_stf_vk_update: u16,
    pub ee_stf_vk_update: u16,
}
```

A field value of `0` is the sentinel for "apply immediately" — the admin handler checks `state.confirmation_depth(...) -> Option<u16>` and dispatches to `handle_update` directly when `None` is returned ([`handler.rs:78-92`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/subprotocols/admin/subprotocol/src/handler.rs#L78-L92)). For us this means we can't assume any specific delay; each proposal must surface the configured CD before broadcast (because if `delay == 0` the proposal cannot be cancelled even within the 2016-block window).

### D.1.1 Domain-model impact

Our `Proposal` ([`orchestrator-be/src/domain/proposal.rs`](../../orchestrator-be/src/domain/proposal.rs)) and our `MultisigUpdate` ([`desktop-app/src-tauri/src/domain/action.rs`](../../desktop-app/src-tauri/src/domain/action.rs)) do not currently carry a confirmation-depth field. We *should* surface it for two reasons:

1. **Status views.** Once a proposal is `Approved` on-chain, the on-chain `activation_height = block_of_reveal + confirmation_depth(tx_type)`. The desktop's "Activates at block N" pane needs that delta.
2. **Cancel UX.** A signer about to authorize a cancel needs to know whether the target is still cancellable; if the target's CD is `0`, it activates immediately and the cancel window is zero. This is a hard requirement of the Alpen Admin Subprotocol §2.3 of [`10-asm-bitcoin-state-model.md`](./10-asm-bitcoin-state-model.md) and is now per-variant rather than global.

The orchestrator can read `ConfirmationDepths` from `AdministrationSubprotoState`. The state shape is ([`state.rs:14-39`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/subprotocols/admin/subprotocol/src/state.rs#L14-L39)):

```rust
pub struct AdministrationSubprotoState {
    authorities: Vec<MultisigAuthority>,
    queued: Vec<QueuedUpdate>,
    next_update_id: UpdateId,
    confirmation_depths: ConfirmationDepths,           // ← new field
    #[ssz(with = "non_zero_u8")]
    max_seqno_gap: NonZero<u8>,
}
```

This adds three SSZ fields versus `a8559d3` (the new `confirmation_depths` struct, plus the `alpen_administrator` slot inside `MultisigAuthority`-vec because the authorities vec is now length-3 instead of length-2 — verified in [`config.rs::get_all_authorities`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/params/src/subprotocols/admin/config.rs#L62-L68)). **Anything decoding `AnchorState` SSZ today will break** if the admin subprotocol section is parsed (see §6.7).

### D.2 Admin module restructure — `7a6a167`

This commit physically relocated and renamed crates. Net impact for our `Cargo.toml`:

| Old path / name | New path / name | Where used in this workspace |
|---|---|---|
| `crates/txs/admin` package `strata-asm-txs-admin` | `crates/subprotocols/admin/txs` package **`strata-asm-proto-admin-txs`** | root `Cargo.toml:10`; both consumers; e2e-tests `:10` |
| (was a re-export crate without a published name; we used it via untagged `rev = "a8559d3"` as `strata-asm-proto-administration`) | `crates/subprotocols/admin/subprotocol` package **`strata-asm-proto-admin`** | `orchestrator-be/Cargo.toml:31` (currently `strata-asm-proto-administration = { git = "https://github.com/alpenlabs/asm", rev = "a8559d3" }`) |
| `crates/asm/txs/test-utils` package `strata-asm-txs-test-utils` | `crates/subprotocols/txs-test-utils` package **`strata-asm-proto-txs-test-utils`** | root `Cargo.toml:13`; `e2e-tests/Cargo.toml:17` |
| `crates/asm/common` package `strata-asm-common` | `crates/common` package **`strata-asm-common`** (name unchanged) | root `Cargo.toml:12` — unchanged name |
| `crates/params` package `strata-asm-params` | `crates/params` package **`strata-asm-params`** (name unchanged, module layout changed) | root `Cargo.toml:11` — unchanged name, but `use strata_asm_params::Role` continues to work; `strata_asm_params::AdminTxType` is **new** (was in `strata-asm-txs-admin::constants`), and `strata_asm_params::ConfirmationDepths`, `UpdateTxType`, `AdministrationInitConfig` are also new module exports |

The bridge-v1 / checkpoint / debug-v1 crates were renamed in the same commit (`strata-bridge-types → strata-asm-proto-bridge-v1-types`, `strata-asm-checkpoint-msgs → strata-asm-proto-checkpoint-msgs`, etc.) — we don't depend on any of those today, so it's informational only.

The admin subprotocol crate (`strata-asm-proto-admin`) requires `edition = "2024"` ([`Cargo.toml:4`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/subprotocols/admin/subprotocol/Cargo.toml#L4)). Our workspace members use `edition = "2021"`. This is **fine** — Cargo allows mixed editions within a workspace as long as the toolchain supports the highest one. `edition = "2024"` requires Rust 1.85+ stable; our pinned nightly is well past that. We do **not** need to bump our own edition.

### D.3 Module-path migration table

| Today | After bump |
|---|---|
| `strata_asm_txs_admin::actions::{MultisigAction, UpdateAction, Sighash}` | `strata_asm_proto_admin_txs::actions::{MultisigAction, UpdateAction}` (no `Sighash` — see §A) |
| `strata_asm_txs_admin::actions::updates::multisig::MultisigUpdate` | `strata_asm_proto_admin_txs::actions::updates::StrataAdminMultisigUpdate` (and 7 sibling types) |
| `strata_asm_txs_admin::test_utils::create_test_admin_tx` | `strata_asm_proto_admin_txs::test_utils::create_test_admin_tx` (same name, new path) |
| `strata_asm_txs_test_utils::TEST_MAGIC_BYTES` | `strata_asm_proto_txs_test_utils::TEST_MAGIC_BYTES` |
| `strata_asm_proto_administration::{AdministrationSubprotoState, AdministrationSubprotocol}` | `strata_asm_proto_admin::{AdministrationSubprotoState, AdministrationSubprotocol}` — verified via [`crates/subprotocols/admin/subprotocol/src/lib.rs`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/subprotocols/admin/subprotocol/src/lib.rs) (lib.rs unchanged in content, just relocated) |
| `strata_asm_txs_admin::parser::{parse_tx, SignedPayload}` | `strata_asm_proto_admin_txs::parser::{parse_tx, SignedPayload}` — SignedPayload struct unchanged: `{ seqno: u64, action: MultisigAction, signatures: SignatureSet }` ([`parser.rs:13-24`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/subprotocols/admin/txs/src/parser.rs#L13-L24)) |

### D.4 `SignedPayload` SSZ wire format — unchanged at the top level

```rust
// At both a8559d3 and a53b6a8:
pub struct SignedPayload {
    pub seqno: u64,                       // 8 bytes little-endian per SSZ scalar rules
    pub action: MultisigAction,           // SSZ union (1-byte selector + variant payload)
    pub signatures: SignatureSet,         // strata-crypto type, unchanged across rc16..rc21
}
```

Three top-level fields, identical names, identical SSZ derive. **The inner `MultisigAction` *content* changes** (§A, §C.3), but the outer struct layout doesn't. This means the broadcast/parsing code in [`broadcast_tx.rs:11-13, 131-132`](../../orchestrator-be/src/infrastructure/broadcast_tx.rs#L11-L13) only needs path-level changes, not structural ones.

---

## 5. Workspace-level dependency alignment

### 5.1 `strata-common` chain: rc16 → rc21

PR #89 (`7b271f1`) bumped from rc16 to rc19. PR #97 (`def601c`) later bumped from rc19 to rc21 (the headline of #97 is "update SP1 to v6.1.0", but the diff also covers `strata-common` and `moho` — verified with `git show def601c -- Cargo.toml`). The chain at HEAD is:

```toml
strata-btc-types        = { tag = "v0.1.0-alpha-rc21" }
strata-codec            = { tag = "v0.1.0-alpha-rc21" }
strata-codec-utils      = { tag = "v0.1.0-alpha-rc21" }
strata-crypto           = { tag = "v0.1.0-alpha-rc21" }
strata-identifiers      = { tag = "v0.1.0-alpha-rc21" }
strata-l1-envelope-fmt  = { tag = "v0.1.0-alpha-rc21" }
strata-l1-txfmt         = { tag = "v0.1.0-alpha-rc21" }
strata-merkle           = { tag = "v0.1.0-alpha-rc21" }
strata-msg-fmt          = { tag = "v0.1.0-alpha-rc21" }
strata-predicate        = { tag = "v0.1.0-alpha-rc21" }
strata-service          = { tag = "v0.1.0-alpha-rc21" }
strata-ssz-tests        = { tag = "v0.1.0-alpha-rc21" }
strata-tasks            = { tag = "v0.1.0-alpha-rc21" }
```

Three of these are direct deps of ours: `strata-crypto`, `strata-l1-txfmt`, `strata-l1-envelope-fmt`. We must bump all three.

Material API/behaviour deltas in rc21 vs rc16 that we touch:

- `strata_crypto::threshold_signature::indexed::verification::ecdsa::normalize_recovery_id` — new function, accepts BIP-137 header byte ranges 27-42 in addition to raw 0-3. See §B.1 for the exact byte ranges. This is **purely additive**: a 64-byte compact signature with recid stored as the leading byte still works.
- `strata_crypto::threshold_signature::ThresholdConfigUpdate` — SSZ derive layout unchanged (verified locally by fetching the rc21 tag and reading `crates/crypto/src/threshold_signature/indexed/mod.rs`). Method `add_members()`, `remove_members()`, `new_threshold()` are the same names we use in our codec.
- `strata_l1_txfmt` API surface — `ParseConfig::new(magic_bytes)`, `ParseConfig::encode_script_buf`, `MagicBytes`, `TagData::new` — all preserved. No rename. Verified the rc21 sources are byte-compatible with our consumer code in [`broadcast_tx.rs:201-206`](../../orchestrator-be/src/infrastructure/broadcast_tx.rs#L201-L206).
- `strata_l1_envelope_fmt::builder::EnvelopeScriptBuilder` — preserved. Verified in rc21 sources.

We do **not** see any `strata_crypto` or `strata_l1_*` API change that breaks `orchestrator-be/src/infrastructure/broadcast_tx.rs` or `desktop-app/src-tauri/src/infrastructure/signing.rs` at the type-signature level.

### 5.2 `ssz` / `tree_hash` / `ssz_types` — unchanged

`ssz = { tag = "v0.15.0" }` is the same pin upstream uses ([`Cargo.toml:115-121`](https://github.com/alpenlabs/asm/blob/a53b6a8/Cargo.toml#L115-L121)). No bump needed. The nightly-Rust footprint described in [`15-nightly-dependency-finding.md`](./15-nightly-dependency-finding.md) is unchanged. `generic_const_exprs` remains the nightly feature in `BitVectorRef`.

### 5.3 `moho` / `zkaleido` / `sp1-sdk` — bumped, but not in our path

PR #102 (`a53b6a8`) bumps `moho` to v0.1-alpha.6, `zkaleido` to v0.1-beta.2, `sp1-sdk` to 6.2.0. None of `moho-*`, `zkaleido-*`, `sp1-sdk`, `sp1-verifier` is in our `Cargo.lock`'s transitive closure — we don't depend on `bin/asm-runner`, `bin/prover-perf`, or `guest-builder/*`. Verified by `grep -E "moho|zkaleido|sp1-" Cargo.lock` returning no results before the bump. The transitive load is therefore unchanged.

PR #100 (`4f37c74`) fixes the SP1 Groth16 verifier in `asm-runner`. We don't run `asm-runner` (we use Alpen's RPC) — informational only.

### 5.4 Toolchain — nightly pin remains valid

The nightly pin in `rust-toolchain.toml` is `nightly-2026-01-01` per [`15-nightly-dependency-finding.md`](./15-nightly-dependency-finding.md) §8. Both `a8559d3` and `a53b6a8` compile under nightly, no nightly date bump is required. The new `edition = "2024"` crates upstream do not affect our edition.

### 5.5 Nightly attack surface unchanged

No new `#![feature(...)]` is introduced anywhere in the admin / common / params / proto-admin-txs crates between `a8559d3` and `a53b6a8`. The blast-radius assessment in [`15-nightly-dependency-finding.md`](./15-nightly-dependency-finding.md) §7 still applies.

---

## 6. Other cross-cutting findings

### 6.1 RPC additions — could we use them?

| New RPC | Method | What it returns | Closes which Open Question in [`08-alpen-crate-prd-coverage.md`](./08-alpen-crate-prd-coverage.md) §5? |
|---|---|---|---|
| `strata_asm_getCheckpointTip` (#84) | `(block_hash) -> Option<CheckpointTip>` | Latest verified checkpoint tip | None directly — but useful to surface in the broadcaster's "where are we" view |
| `strata_asm_getAsmState` (#90) | `(block_hash) -> Option<AsmState>` | Full SSZ-encoded `AsmState` for the block | **Could replace** our current `strata_asm_getStatus` + `cur_state.state` decode in [`asm_role_membership.rs:181-206`](../../orchestrator-be/src/infrastructure/asm_role_membership.rs#L181-L206) with a typed call. We'd need to add a client dependency on `strata-asm-rpc` (the new crate at HEAD), or hand-decode `Option<AsmState>` from JSON. Not strictly needed for this bump; keep current `getStatus` path. |
| `strata_asm_getMohoState` (#86) | `(block_hash) -> Option<Vec<u8>>` | SSZ-encoded `MohoState` | Not in our scope (no Moho consumer in orchestrator-be) |
| `strata_asm_getExportEntryMMRProof` (#86) | `(block_hash, container_id, leaf) -> Option<Vec<u8>>` | MMR inclusion proof | Not in our scope |

Recommendation: **do not** adopt the new RPCs in this bump. They are additive and we have no UI surface for them yet. Keep our existing `strata_asm_getStatus` path. Track a follow-up issue for `getAsmState` adoption once we want stronger typed RPCs.

### 6.2 PRD gaps still open after bump

Reconfirmed against `crates/params/src/subprotocols/admin/roles.rs` and `crates/subprotocols/admin/txs/src/actions/updates/*.rs` on HEAD:

| PRD item | Why still open |
|---|---|
| Safe Harbor address update | Zero references to "safe harbor" in `crates/` on HEAD (`rg -i "safe.harbor"` returns nothing). |
| Security Council Signer update | No `Role::SecurityCouncil`. No `SecurityCouncilMultisigUpdate`. |
| "Soft" bridge update | No upstream definition of "soft" vs "hard" bridge update. Term remains PRD-only. |
| "Hard" bridge update | Same. |
| Defcon 1 transaction | No `Defcon1` mechanism — neither in admin subprotocol nor in bridge-v1 / checkpoint. |
| Defcon 3 transaction | Same. |
| Payout Administrator | `block_payout` is by design **not** an admin subprotocol action. Remains a Bitcoin-native UTXO spend (see [`08-alpen-crate-prd-coverage.md`](./08-alpen-crate-prd-coverage.md) §2 "Payout Administrator — fundamentally different"). |

### 6.3 Bridge / batch-withdraw / checkpoint refactors

- **PR #95 (`a6ff40c`) extract checkpoint verification.** Moves `crates/subprotocols/checkpoint/src/{state,verification}.rs` into a new sibling crate `strata-checkpoint-verification`. Public types `Checkpoint*Msg` etc. remain in `strata-asm-proto-checkpoint-msgs`. We don't depend on either today. The `crates/subprotocols/checkpoint/types/ssz/{claim,payload}.ssz` SSZ fixtures are updated — purely internal.
- **PR #85 (`8b12392`) checkpoint types cleanup.** Removes `log_payloads.rs`, simplifies `claim.rs`, `payload.rs`. Not in our path.
- **PR #79 (`f702715`) persist MohoState.** Adds `crates/proof/db/src/sled/moho_state.rs`. Surfaces via `getMohoState` RPC. Not in our path.
- **Bridge withdrawal output (`cd48728` "fold selected_operator into WithdrawOutput", inside PR #95).** Changes `crates/subprotocols/bridge-v1/types/src/withdrawal.rs` to fold the selected operator into the withdrawal output struct. Not in our path — we don't import `strata-asm-proto-bridge-v1-types`.

### 6.4 SP1 toolchain — not in our build

The asm-runner now depends on `sp1-sdk 6.2.0` and `sp1-verifier 6.2.0` (PR #102). Our backend doesn't link against `bin/asm-runner` and we never call the prover crates. **No impact on our `Cargo.lock`.**

### 6.5 New RPCs we should NOT adopt yet

See §6.1. Two `strata-asm-rpc` types (`AssignmentsApi`, `AsmProofApi`) are now split traits ([`crates/rpc/src/traits.rs:12-40, 42-60`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/rpc/src/traits.rs)). If we want a typed client, we'd add a workspace dep `strata-asm-rpc = { workspace = true, default-features = false, features = ["client"] }`. **Don't.** Postpone — out of scope for the bump.

### 6.6 SSZ wire-format compatibility — full inventory

Following the methodology of [`11-asm-repo-migration.md`](./11-asm-repo-migration.md) §"Wire format compatibility":

| Type we (de)serialize | SSZ derive change | Field add/remove/reorder | Wire compatible across the bump? |
|---|---|---|---|
| `MultisigAction` | `Encode/Decode + ssz(enum_behaviour="union")` — unchanged derive | 2 variants (Cancel, Update) — unchanged | **Yes** at the outer level; inner `UpdateAction` and `CancelAction` change (see below) |
| `UpdateAction` | union, derived | 4 → 8 variants; first 3 are added in front of `OperatorSet`, shifting union tags for `OperatorSet` (1→3), `Sequencer` (2→4); `VerifyingKey(PredicateUpdate)` (3) is replaced by three separate variants (`OlStfVk`=5, `AsmStfVk`=6, `EeStfVk`=7) carrying just the `PredicateKey` instead of a `(ProofType, PredicateKey)` pair. **Union tag for "Strata Admin multisig update" goes from 0 (was `Multisig`) to 0 (now `StrataAdminMultisig`) — coincidentally unchanged value, but conceptually different.** | **No** in general. For the specific action we send today (Strata Admin multisig update), the union tag byte stays 0; but the inner struct loses the trailing `role: Role` byte. |
| `MultisigUpdate { config: ThresholdConfigUpdate, role: Role }` | (gone) | replaced by 3 separate types each `(ThresholdConfigUpdate,)` | **No** — `role` byte dropped from wire. Our action_codec produces a 1-byte-longer encoding today than the new equivalent. |
| `CancelAction { target_id: UpdateId }` | unchanged derive | added field `update: UpdateAction` | **No** — fully different layout |
| `SignedPayload { seqno, action, signatures }` | unchanged | unchanged | **Yes** at this level (but inner `action` and `signatures` are unchanged shape-wise; signatures still `SignatureSet`) |
| `SignatureSet { signatures: Vec<IndexedSignature> }` | unchanged across rc16..rc21 | unchanged | **Yes** |
| `IndexedSignature { index: u8, signature: [u8; 65] }` | unchanged | unchanged. Header byte interpretation widens (raw vs BIP-137) but the wire bytes are identical. | **Yes** |
| `ThresholdConfigUpdate { add_members, remove_members, new_threshold }` | unchanged | unchanged | **Yes** |
| `AnchorState` (when decoding admin subprotocol state from JSON-RPC) | unchanged outer derive | inner `AdministrationSubprotoState` adds `confirmation_depths`, plus the `authorities` vec is length-3 instead of length-2. | **No** — the encoded state from a freshly genesis'd ASM running `a53b6a8` is longer than `a8559d3`'s. Our `decode_anchor_state_from_status` would still call into `AnchorState::from_ssz_bytes` (which is generic), and the **admin subprotocol section** is now structurally larger. Our `decode_admin_state` decodes it via `AdministrationSubprotocol::try_to_state` ([`asm_role_membership.rs:209-212`](../../orchestrator-be/src/infrastructure/asm_role_membership.rs#L209-L212)) which uses the new `AdministrationSubprotoState`. This works once we re-pin and rebuild. The mock paths in the same file (`mock_*` helpers) only return `Authority::StrataAdmin`/`SequencerManager` and stay valid. |

### 6.7 Will `test_encode_matches_direct_strata_ssz` still pass?

Yes — but only after the codec is updated. The test calls `MultisigAction::Update(UpdateAction::Multisig(StrataMultisigUpdate::new(config_update, Role::StrataAdministrator)))`. On `a53b6a8`, those types and constructors *don't exist*. The test fails at compile time. After the codec is updated to `MultisigAction::Update(UpdateAction::StrataAdminMultisig(StrataAdminMultisigUpdate::new(config_update)))`, both the test fixture and our codec produce the same byte stream, so the equality assertion still holds. **Verdict:** test remains an effective wire-format guard.

---

## 7. Migration table — file-by-file impact

Risk level legend: C = critical (silent on-chain rejection or wrong signed data), H = high (compile-time break + behavioural change), M = medium (compile-time break only), L = low (renames / unaffected at runtime).

| File | Symbol / line range | Breaking? | Required change | Test that exercises it | Risk |
|---|---|---|---|---|---|
| `Cargo.toml` (root) | `[workspace.dependencies]` lines 10-13, 16-18 | **Y** | Rename `strata-asm-txs-admin` → `strata-asm-proto-admin-txs`; rename `strata-asm-txs-test-utils` → `strata-asm-proto-txs-test-utils`; bump rev `a8559d3 → a53b6a8`; bump `strata-common` tag `v0.1.0-alpha-rc16 → v0.1.0-alpha-rc21` for `strata-crypto`, `strata-l1-txfmt`, `strata-l1-envelope-fmt` | `cargo build --workspace` | M |
| `orchestrator-be/Cargo.toml` | line 31 | **Y** | Rename `strata-asm-proto-administration` → `strata-asm-proto-admin`; rev `a8559d3 → a53b6a8` | `cargo build -p orchestrator-be` | M |
| `orchestrator-be/Cargo.toml` | lines 29-34 | **Y** | Workspace dep `strata-asm-txs-admin` is gone; switch to `strata-asm-proto-admin-txs = { workspace = true }` | same | M |
| `e2e-tests/Cargo.toml` | lines 8-9, 13-16, 20-26 | **Y** | Same rev bump; also bump the directly-listed git-pinned deps (`strata-asm-manifest-types`, `strata-asm-spec`, `strata-asm-worker`, `strata-btc-verification`, `strata-test-utils-arb`, `strata-test-utils-btcio` from `rev = "a8559d3"` → `rev = "a53b6a8"`); `strata-btc-types`, `strata-identifiers`, `strata-l1-envelope-fmt`, `strata-merkle`, `strata-tasks` from `tag = "v0.1.0-alpha-rc16"` → `tag = "v0.1.0-alpha-rc21"`. Also rename `strata-asm-txs-admin` → `strata-asm-proto-admin-txs` and `strata-asm-txs-test-utils` → `strata-asm-proto-txs-test-utils`. | same | M |
| `desktop-app/src-tauri/src/infrastructure/signing.rs` | line 10 (`use ... Sighash`); lines 55-65 (`compute_sighash`) | **Y** | Replace import `Sighash` with `signing_message::SigningMessage`; replace `action.compute_sighash(seqno)` with `SigningMessage::for_action(&action, seqno).compute_sighash()`. Module path `strata_asm_txs_admin → strata_asm_proto_admin_txs`. | `test_compute_sighash_*`, `test_sign_sighash_*`, `test_verify_threshold_*`, `test_mnemonic_signature_verifies_against_raw_sighash` | C |
| `desktop-app/src-tauri/src/infrastructure/action_codec.rs` | lines 9-14 imports; 67-72 (`to_strata_action`); 74-92 (`to_strata_multisig_update`); 99-103 (`to_strata_role`); 107-124 (`from_strata_action`); 126-147 (`from_strata_multisig_update`); 153-158 (`from_strata_role`) | **Y** | Switch import paths; emit `MultisigAction::Update(UpdateAction::StrataAdminMultisig(StrataAdminMultisigUpdate::new(config)))` per authority; the role is no longer a field, only a variant selector. The reverse path matches on `UpdateAction::StrataAdminMultisig | StrataSeqManagerMultisig | AlpenAdminMultisig` to derive `Authority`. The other 5 variants stay `UnsupportedVariant` until the domain model grows. | `test_roundtrip_*`, `test_encode_matches_direct_strata_ssz` | H |
| `desktop-app/src-tauri/src/domain/action.rs` | `Action` enum (single variant `MultisigUpdate`) | N (today) | Optional: extend `Action` with `Cancel(CancelTarget)` and `MultisigUpdate.role` widened beyond `StrataAdmin` once we want to expose Alpen Admin / Sequencer Manager UI. Not strictly required to land the bump. | `test_action_builds` | L |
| `desktop-app/src-tauri/src/domain/authority.rs` | `Authority` enum (single variant `StrataAdmin`) | N (today) | Optional: add `SequencerManager`, `AlpenAdmin` variants when the UI grows. Today the backend has 5 authorities ([`orchestrator-be/src/domain/authority.rs`](../../orchestrator-be/src/domain/authority.rs)) and the desktop has 1. Bump alone doesn't force this. | `test_*` in same file | L |
| `orchestrator-be/src/application/proposals.rs` | line 370 (`MultisigAction::from_ssz_bytes`); lines 425-430 (`compute_sighash_for_proposal`) | **Y** | Replace `use strata_asm_txs_admin::actions::{MultisigAction, Sighash}` with `use strata_asm_proto_admin_txs::{actions::MultisigAction, signing_message::SigningMessage}`. Replace `action.compute_sighash(proposal.seq_no)` with `SigningMessage::for_action(&action, proposal.seq_no).compute_sighash()`. | (integration) `test_create_*`, `test_approve_*`, `test_get_*` (these don't compute real sighashes, but regression manual broadcast test required) | C |
| `orchestrator-be/src/infrastructure/broadcast_tx.rs` | line 12-15 (`use strata_asm_txs_admin::*`); 27-133 (`build_signed_payload_bytes`); 181 (param `&MultisigAction`) | **Y** | Rename `strata_asm_txs_admin → strata_asm_proto_admin_txs`; `parser::SignedPayload`, `CompressedPublicKey`, `IndexedSignature`, `SignatureSet` paths unchanged at the top-level. Behaviour of `build_signed_payload_bytes` is mostly unchanged: it already accepts either 64-byte compact (raw) or 65-byte recid-leading. Drop the all-four-recid recovery for the 64-byte branch *if* we want, since `strata-crypto` rc21 will recover the right key from the header anyway — but keep it for the software-mnemonic path which produces no header. | (none — covered by `e2e_admin_commit_reveal` once migrated; verify on regtest) | H |
| `orchestrator-be/src/infrastructure/asm_role_membership.rs` | line 7 (`use strata_asm_proto_administration::*`); 5-7 imports | **Y** | Rename `strata_asm_proto_administration → strata_asm_proto_admin`. Decoder downstream (`decode_admin_state`, `authority_keys_hex`, `last_seqno_for_authority`, `threshold_for_authority`) works unchanged. Add `Authority::AlpenAdmin → Role::AlpenAdministrator` mapping in `authority_to_role`. | `authority_mapping_is_fail_closed_for_unmapped_authorities` (today asserts `AlpenAdmin` is **Err** — invert that assertion) | H |
| `orchestrator-be/src/domain/proposal.rs` | `ActionId` derivation | N | No change. `compute_action_id(seq_no, action_hex)` is opaque to the digest. | `test_create_*` | L |
| `e2e-tests/tests/e2e_admin_subprotocol.rs` | lines 24-29 imports; 68-78 (`compute_sighash`); 95-100 (`create_test_admin_tx`) | **Y** | Same imports rename; switch to `MultisigAction::Update(UpdateAction::StrataAdminMultisig(StrataAdminMultisigUpdate::new(config_update)))`; the `Role::StrataAdministrator` parameter passed to `MultisigUpdate::new` is gone; the `sighash` for `verify_threshold_signatures` becomes `SigningMessage::for_action(&action, seqno).compute_sighash().0`. | the test itself | H |
| `e2e-tests/tests/e2e_admin_commit_reveal.rs` | lines 19-24 imports; 59-69 build action + sign | **Y** | Same migration; `create_signature_set(&privkeys, &signer_indices, sighash)` → `create_signature_set(&privkeys, &signer_indices, &action, seqno)` (the helper takes the action and computes the digest internally now — verified at [`crates/subprotocols/admin/txs/src/test_utils/mod.rs:50-66`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/subprotocols/admin/txs/src/test_utils/mod.rs#L50-L66)). | the test itself | H |
| `e2e-tests/tests/e2e_propose_sign.rs` | lines 142, 228 | **Y (indirectly)** | The HTTP-level test code is unaffected. The Tauri `compute_sighash` wrapper is internally migrated (see `signing.rs`). The verify call still works because the signature is over the digest, whatever the digest is. | the test itself | M |
| `e2e-tests/src/test_harness.rs` | line 47 (`AdministrationInitConfig`) | **Y** | The `AdministrationInitConfig` constructor now needs `alpen_administrator: ThresholdConfig` and `confirmation_depths: ConfirmationDepths` (no `confirmation_depth: u16`). The arbitrary-generated fallback at line 549 (`let mut asm_params: AsmParams = ArbitraryGenerator::new().generate();`) keeps working because upstream added an `Arbitrary` impl ([`config.rs:73-94`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/params/src/subprotocols/admin/config.rs#L73-L94)). | `e2e_harness_hello_world` | M |
| `desktop-app/src-tauri/src/infrastructure/hw_wallet/trezor.rs` | `sign_admin_sps65_binding`, `connect`, `resolve` | (not yet on the protocol path) | Add a new `sign_admin_sps_signmessage` entry point that calls Trezor's `MessageSignReq { msg, address_n: m/86'/0'/73'/0/n }`. The message is the rendered `SigningMessage` string. The returned 65-byte BIP-137 signature is accepted directly by the verifier. Deprecate `sign_admin_sps65_binding` once parity tests pass. See §B.2. | new e2e test (Trezor emulator) | H |
| `docs/architecture/adrs/001-alpen-crate-dependencies.md` | "Wire format" section, "Crate inventory" section, "Risks" #5 | **Y** | Update §"Wire format" to note that the upstream digest is now BSM-prefixed and that signatures are no longer cross-rev portable. Update §"Crate inventory" to the new names. Add a revision-history line for 2026-05-13 documenting the bump. | (doc) | L |
| `docs/2-discovery/08-alpen-crate-prd-coverage.md` | "Re-validated on 2026-04-17" line; §2 tables; §3 table | **Y** | Re-validation line moves to 2026-05-13. §2 "Implemented in Alpen crates" grows by 2 entries (AlpenAdminMultisigUpdate, EeStfVkUpdate). §2 "Not implemented" shrinks by 2. §3 Role coverage marks Alpen Administrator as 100%. | (doc) | L |
| `docs/2-discovery/11-asm-repo-migration.md` | §"Wire format compatibility", §"PRD coverage — no change" | **Y** | Add a clear "Superseded for the next bump" note. The migration `308211f → a8559d3` was format-stable; `a8559d3 → a53b6a8` is not. | (doc) | L |
| `docs/2-discovery/10-asm-bitcoin-state-model.md` | §8.1 sighash formula `SHA256(SHA256("strata/admin/<type_name>") ‖ seqno_be ‖ payload)` | **Y** | Replace the formula with the BSM construction. Note that hardware wallets now sign the rendered string directly. | (doc) | L |
| `docs/2-discovery/16-poc5-trezor-findings.md` | §§2, 4, 5, 11 (BIP-137 vs SPS-65 incompatibility) | **Y** | Add a "2026-05-13" revision note: PR #82 closes the gap; option C in §11 is now the primary recommended path. | (doc) | L |
| `docs/2-discovery/17-cancel-action.md` | "Cancel as a signed payload" section, "Files to change" table | **Y** | Replace `CancelAction { target_id: UpdateId }` with `CancelAction { target_id: UpdateId, update: UpdateAction }`. Note that on HEAD the handler enforces `queued.action() == cancel.update()` and returns `CancelUpdateMismatch` otherwise. | (doc) | L |
| [`external/research-assessment.md`](../external/research-assessment.md) | §1 (Borsh→SSZ portability claim), §2 (BIP-137 vs SPS-65 gap), sighash formula | **Y** | Add a 2026-05-13 amendment: signatures are no longer byte-portable across the upstream bump; BIP-137 / signMessage is now the canonical digest; §3.5 formula replaced. | (doc) | L |

---

## 8. Phased migration plan

### Phase 1 — Rev bump + rename sweep + digest call-site migration (1 PR)

Scope: bring the workspace onto `a53b6a8`. This is **not** mechanical-only: PR #96 deletes the `Sighash` trait, so the workspace cannot compile against the new pin without migrating every `compute_sighash` call site simultaneously. We bundle these together because they form a single atomic compile boundary and a single atomic revert target.

Steps:

- Bump `strata-common` tag rc16 → rc21 in root and `e2e-tests` `Cargo.toml`. Verify `strata_crypto::threshold_signature::ThresholdConfig`, `ThresholdConfigUpdate`, `CompressedPublicKey`, `SignatureSet`, `IndexedSignature`, `verify_threshold_signatures` still compile at the call sites in `signing.rs`, `action_codec.rs`, `broadcast_tx.rs`.
- Rename workspace deps: `strata-asm-txs-admin → strata-asm-proto-admin-txs`, `strata-asm-txs-test-utils → strata-asm-proto-txs-test-utils`, and in `orchestrator-be` `strata-asm-proto-administration → strata-asm-proto-admin`. Update all `use` paths (mechanical, ~12 files).
- Bump rev `a8559d3 → a53b6a8` for the 4 ASM workspace deps + the 6 direct `e2e-tests` ASM git deps.
- Replace `action.compute_sighash(seqno)` with `SigningMessage::for_action(&action, seqno).compute_sighash()` at every call site (5 sites: `signing.rs`, `proposals.rs`, `e2e_admin_subprotocol.rs`, `e2e_admin_commit_reveal.rs`, plus the test helper inside `signing.rs`'s `tests` module). **This is the unavoidable semantic change.** Signatures are now BSM-prefixed.
- Rewrite `action_codec.rs` to use the role-specific `StrataAdminMultisigUpdate` (Phase 1 only needs the `StrataAdmin` variant; the other two role-specific variants can come later in Phase 3).
- Update `AdministrationInitConfig` constructor at `e2e-tests/src/test_harness.rs` if any non-arbitrary test path explicitly builds the struct. The default `Arbitrary` fallback works without code change.
- Add `Authority::AlpenAdmin → Role::AlpenAdministrator` mapping (and the test inversion) in `orchestrator-be/src/infrastructure/asm_role_membership.rs::authority_to_role` — required for the workspace to remain semantically consistent with HEAD.
- Confirm `cargo build --workspace` and `cargo test --workspace` succeed.
- Update `test_encode_matches_direct_strata_ssz` fixture (call site only, no logic change). Run the test — it must pass.

**Signature-rotation footnote.** Any in-flight proposal whose `signatures` field was collected against the old digest is now invalid and must be re-collected. The `Proposal.action_hex` SSZ bytes are also one byte shorter than they were (the trailing `role` byte on `MultisigUpdate` is gone). **The orchestrator must invalidate or re-encode every `Pending`/`Approved` proposal currently in the repository.** A migration script that re-encodes `action_hex` and clears `signatures` (forcing signers to re-sign) is the cleanest path. Today there are no production proposals (we are pre-mainnet), so the practical cost is bounded to test fixtures.

### Phase 2 — Hardware wallet alignment (1 PR)

Scope: take advantage of the new digest to ship a real Trezor signing path.

- Add `desktop-app/src-tauri/src/infrastructure/hw_wallet/trezor.rs::sign_admin_signmessage(rendered_message, path) -> [u8; 65]`. Internally call Trezor's `SignMessage` over the rendered string. Return the 65-byte BIP-137 sig as-is.
- Expose a new Tauri command `sign_admin_with_trezor` that takes `(seq_no, action_hex)`, renders the signing message via `SigningMessage::for_action(&decode(action_hex)?, seq_no).as_str()`, and forwards to the device. Display the same rendered string in the UI so the user can verify on-device.
- Deprecate `sign_admin_sps65_binding` (PSBT binding) — keep the function and call site for one release for rollback but mark `#[deprecated]`.
- Add an e2e test: emulator-backed, signs a multisig update, verifies via `verify_threshold_signatures` (recid normalization in `strata-crypto` rc21 handles the BIP-137 header).
- Update `desktop-app/src/wallet/trezor-poc-adapter.ts` to call the new Tauri command.

### Phase 3 — Domain model widening (1-2 PRs, optional, can defer)

Scope: expose AlpenAdmin and the new update types to the application layer.

- Add `Authority::AlpenAdmin` and `Authority::SequencerManager` to `desktop-app/src-tauri/src/domain/authority.rs`. Wire serde strings (`"alpen_admin"`, `"sequencer_manager"`).
- Extend `Action` enum in `domain/action.rs` to include `EeStfVkUpdate`, `AlpenAdminMultisigUpdate`, `AsmStfVkUpdate`, `OperatorUpdate`, `SequencerUpdate` — each as their own domain type mirroring the upstream payload. Update `action_codec.rs` to encode/decode all 8 variants.
- Update the proposal-creation UI to support choosing the action type and the role-dependent field set.
- The orchestrator's `Authority` enum already has all 5 PRD authorities — no backend change.

### Phase 4 — Doc updates (1 PR, in parallel with Phase 1)

Touch the doc files listed in the "doc" rows of §7's migration table. None of these block code merges; bundle them into one cleanup PR so the discovery folder stays internally consistent.

### Whether to do the bump as one PR or several

**Several.** Specifically: Phase 1 must land alone, because it carries the breaking digest change and forces signature rotation — putting it behind a single git revert is essential. Phase 2 is high-value but should be a separate PR with HW-emulator-backed CI. Phase 3 is incremental and can land per-authority. Phase 4 can be one doc cleanup PR landing in parallel.

---

## 9. Regression tests to add or update

### 9.1 Unit tests

- `SigningMessage` rendering: byte-exact assertion against the upstream rendered strings for every `UpdateAction` variant we care about. Mirror the upstream test corpus in [`crates/subprotocols/admin/txs/src/actions/updates/render.rs`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/subprotocols/admin/txs/src/actions/updates/render.rs) and [`crates/subprotocols/admin/txs/src/actions/updates/<each>.rs`](https://github.com/alpenlabs/asm/blob/a53b6a8/crates/subprotocols/admin/txs/src/actions/updates) — even a one-character drift in the rendered string invalidates signatures, so we want our own renderer-equivalence check.
- `compute_sighash` regression: assert the desktop-side `compute_sighash` returns exactly `bitcoin::sign_message::signed_msg_hash(rendered).to_byte_array()` for a fixture action. This guards against the upstream changing the digest construction silently.
- `action_codec` roundtrip plus a *cross-version negative test*: assert that decoding bytes produced by `MultisigUpdate { config, role }` (the OLD format) into the NEW `UpdateAction` fails — this prevents accidentally accepting stale in-flight proposals.

### 9.2 Integration tests

- Update `e2e_admin_subprotocol` and `e2e_admin_commit_reveal` per §7.
- Add a Trezor emulator end-to-end test exercising the new `sign_admin_with_trezor` path. Use the existing emulator harness from POC-5 ([`16-poc5-trezor-findings.md`](./16-poc5-trezor-findings.md) §8). Verify the returned 65-byte BIP-137 sig passes `verify_threshold_signatures` against the threshold config.
- Add a regtest end-to-end test of the full proposal → sign → broadcast → ASM-state-decode flow against an `asm-runner` running at `a53b6a8`. Confirm the proposal becomes `Enacted` and that `getStatus` returns an `AdministrationSubprotoState` whose `authorities.last_seqno` advanced.

### 9.3 RPC integration test (if we adopt `getAsmState`)

Not in scope for this bump (see §6.1). When we adopt it, add a typed-RPC roundtrip test asserting that `getAsmState(block_hash).admin_state().authority(Role::AlpenAdministrator).is_some()`.

---

## 10. Open questions for Alpen

1. **Tag for `a53b6a8`.** Will this commit be tagged as `v0.1-alpha.6` (or `v0.2.0-alpha.1`, etc.) before our migration lands? If yes we should pin by tag per ADR-001 §"Pinning strategy". If no, we keep rev pinning and call it out in our updated ADR-001.
2. **Finality of `signMessage` switch.** Is `signed_msg_hash` the long-term canonical digest? Or is BIP-322 / SLIP-0019 still under consideration? PR #82's title is "fix", not "feat" — that suggests the team treats this as a stable correction rather than a transient experiment. We'd like confirmation.
3. **`ADMIN_SUBPROTOCOL_VERSION = 1`.** Will the version line in the rendered message be bumped during alpha? If so, on what cadence? Each bump invalidates all collected signatures — a hard signal we'd want to receive in advance.
4. **"Alpen Administrator" mapping.** The PRD distinguishes "Alpen Admin" and "Strata Admin" as separate signing groups with separate update authorities, including separate signer-set updates ("Alpen Administrator Signer update"). Upstream now has `Role::AlpenAdministrator` + `AlpenAdminMultisigUpdate` + `EeStfVkUpdate` (authority for EE STF VK). Does the PRD's "Alpen Administrator Signer update" map exactly to `AlpenAdminMultisigUpdate`? Does the PRD's "Alpen verification key update" map exactly to `EeStfVkUpdate`? Pending confirmation.
5. **Future Roles.** Timeline for `Role::SecurityCouncil`, Safe Harbor, soft/hard bridge update types, and Defcon mechanism? These are PRD requirements with zero upstream presence.
6. **`AdministrationInitConfig.confirmation_depths` production values.** Eight `u16` fields, one per `UpdateTxType`. What is the intended mainnet config — same depth for all (the historical 2016 blocks), or differentiated per update kind (e.g., faster for `OperatorUpdate`, slower for `OlStfVkUpdate`)? Affects our cancel-window UX.
7. **Signature rotation on bump.** Are there any production proposals already collecting signatures against the old digest? If yes, the rotation cost is non-trivial. We assume **no** — this assessment was triggered by a pre-production migration. Confirm.
8. **In-flight wire format guarantees.** Will Alpen freeze `UpdateAction` variant ordering at v1, or should we expect more breaking variant reshuffles before mainnet? This determines whether we can pin by `tag` after this bump or must stay on `rev`.

---

## 11. What we deliberately do NOT adopt in this bump

| Upstream item | Reason for deferral |
|---|---|
| `strata-asm-rpc` typed client (#90, #84, #86) | Our `getStatus`-based decoder works. Adopting typed client adds a workspace dep, a feature flag for `client`, and a transitive `jsonrpsee` pin. Punt until we have a UI surface needing those RPCs. |
| `strata-checkpoint-verification` crate (#95) | Not in our path. We do not verify checkpoints; that is the ASM runner's job. |
| `strata-asm-proto-bridge-v1*` crates (renamed in #80) | Not used today. The bridge-v1 module restructure is informational. |
| `strata-asm-proto-debug-v1` | Not in our scope. |
| `Moho`/`zkaleido`/`SP1` toolchain bumps (#102, #100, #97) | Not in our Cargo.lock. |
| New `getAsmState` / `getMohoState` / `getExportEntryMMRProof` adoption (#90, #86) | Postpone — see §6.1. No PRD blocker. |
| `getCheckpointTip` adoption (#84) | Postpone. No PRD requirement to surface checkpoint tip today. |
| `bitcoind-async-client` upgrades (transitive in upstream) | Pinned independently in `e2e-tests/Cargo.toml:34`; not affected. |

---

## 12. Summary verdict

The bump is mandatory if we want any HW wallet path to be on-chain-valid for admin actions: PR #82 + #96 close the BIP-137 / SPS-65 gap that has blocked POC-5 since day one and was the single largest hardware-wallet risk in this project ([`12-upstream-readiness-findings.md`](./12-upstream-readiness-findings.md) finding #4). It also closes two PRD-requirement gaps (Alpen Administrator role; EE STF VK update).

The cost is a hard re-collect of every in-flight signature (none in production today, by assumption), a strict crate-rename sweep, and a `strata-common` tag bump. None of these is unbounded; the migration table in §7 enumerates every touch site. The biggest residual risk is the *un*audited Trezor signMessage path on the Strata domain string — we should not consider Phase 2 done before an on-device validation run with a physical Model T (not just the emulator).

**Recommendation: Approve. Phase 1 (rename + rc21 + BSM digest) as one PR; Phase 2 (Trezor signMessage) as a follow-up; Phase 3 (domain widening) as multiple incremental PRs; Phase 4 (docs) in parallel with Phase 1.**

---

## Appendix — Citations index

All claims above are sourced from one of:

- `~/Documents/wakeup/alpen-multisign/repo/asm` at `a8559d3` (tag `v0.1-alpha.5`) — file paths under `crates/...` cited inline.
- `~/Documents/wakeup/alpen-multisign/repo/asm` at `a53b6a8` (`origin/main` at time of write) — file paths under `crates/...` cited inline, GitHub-blob URLs given for cross-reference.
- `~/Documents/wakeup/alpen-multisign/repo/strata-common` at `v0.1.0-alpha-rc21` — for `normalize_recovery_id` and `ThresholdConfig` shape.
- This workspace at HEAD — file paths under `desktop-app/`, `orchestrator-be/`, `e2e-tests/`, `docs/` cited with workspace-relative paths.

Verification commands used (re-runnable):

```bash
cd ~/Documents/wakeup/alpen-multisign/repo/asm
git fetch origin
git log a8559d3..origin/main --oneline                    # commit list
git log -1 origin/main --format='%H %ci %an %s'           # HEAD metadata
git diff --stat a8559d3..origin/main                       # change shape
git show e0461f8 aa236e2 3afc520 ed35f80 7a6a167 --stat   # PR-specific diffs
git show origin/main:crates/params/src/subprotocols/admin/roles.rs
git show origin/main:crates/params/src/subprotocols/admin/updates.rs
git show origin/main:crates/params/src/subprotocols/admin/confirmation_depth.rs
git show origin/main:crates/subprotocols/admin/txs/src/signing_message.rs
git show origin/main:crates/subprotocols/admin/txs/src/actions/mod.rs
git show a8559d3:crates/txs/admin/src/actions/sighash.rs
git show a8559d3:crates/txs/admin/src/constants.rs
git show a8559d3:crates/params/src/subprotocols/admin.rs
git log a8559d3..origin/main -p -G"alpha-rc21" -- Cargo.toml
```
