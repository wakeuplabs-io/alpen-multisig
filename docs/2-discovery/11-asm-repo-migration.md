# 11 — ASM repo migration (`alpenlabs/alpen` → `alpenlabs/asm`)

**Date:** 2026-04-17
**Related issue:** [wakeuplabs-io/alpen-multisig#36](https://github.com/wakeuplabs-io/alpen-multisig/issues/36)
**Related spec:** [`docs/specs/migrate-asm-deps-to-alpenlabs-asm.md`](../specs/migrate-asm-deps-to-alpenlabs-asm.md)
**Related ADR:** [`001-alpen-crate-dependencies.md`](../architecture/adrs/001-alpen-crate-dependencies.md)

## Context

Until this migration the workspace consumed every Strata/ASM crate from the monorepo `alpenlabs/alpen` at rev `308211f` (2026-04-07). Upstream has since split out dedicated repositories and, as part of that split, changed the wire format of admin transactions. This document records what changed, why we followed, and how the migration was done.

## Upstream timeline

| Date | Repo / ref | Event |
|---|---|---|
| 2026-03-17 | `alpenlabs/asm` @ `26d081d` | Repo initialized by copying ASM crates from `alpenlabs/alpen` (still Borsh-based at this point). |
| 2026-03-25 | `alpenlabs/asm` @ `fd57abb` | PR `#8` `feat!(ssz): finish ASM serialization migration`. Borsh derives removed, SSZ derives added across `MultisigAction`, `CancelAction`, `UpdateAction`, and every nested admin type. |
| 2026-04-07 | `alpenlabs/alpen` @ `308211f` | Our pre-migration pin. Still the Borsh version. |
| 2026-04-08, -09, -15 | `alpenlabs/asm` tags `v0.1.0-rc1`, `rc2`, `rc3` | Release candidates. All post-SSZ. |
| 2026-04-17 | `alpenlabs/asm` tags `v0.1-alpha.4`, `v0.1-alpha.5` | Alpha tags. `v0.1-alpha.5` == rev `a8559d3`. |
| 2026-04-17 | `alpenlabs/alpen/main` @ `afb3683` | Alpen deletes `crates/asm/txs/admin/` locally and consumes `strata-asm-*` from `alpenlabs/asm` tag `v0.1-alpha.5`. |

## Why we migrated

Staying on `alpenlabs/alpen` rev `308211f` was a dead end:

- Upstream deleted the ASM crates from that monorepo location. Future bumps of `alpenlabs/alpen` no longer contain `strata-asm-*` at all.
- All tagged versions of `alpenlabs/asm` are post-SSZ. There is no "intermediate" Borsh-pinned asm rev with a tag or release backing it.
- All future upstream fixes, new subprotocols, and new admin types land in `alpenlabs/asm`, not in the old monorepo location.

## What changed in our workspace

### Dependency pins

| Crate | Before (`alpenlabs/alpen` @ `308211f`) | After |
|---|---|---|
| `strata-asm-txs-admin` | monorepo | `alpenlabs/asm` rev `a8559d3` |
| `strata-asm-params` | monorepo | `alpenlabs/asm` rev `a8559d3` |
| `strata-asm-common` | monorepo | `alpenlabs/asm` rev `a8559d3` |
| `strata-asm-txs-test-utils` | monorepo | `alpenlabs/asm` rev `a8559d3` |
| `strata-crypto` | monorepo | `alpenlabs/strata-common` tag `v0.1.0-alpha-rc16` |
| `strata-l1-txfmt` | `alpenlabs/strata-common` `v0.1.0-alpha-rc11` | `alpenlabs/strata-common` tag `v0.1.0-alpha-rc16` (bump) |
| `strata-primitives` | monorepo | removed — not imported anywhere |
| `strata-test-utils` | monorepo | removed — not imported anywhere |
| `ssz` | — | added: `alpenlabs/ssz-gen` tag `v0.15.0` |
| `borsh` | `[workspace.dependencies]` | removed (no direct consumer remaining; still available transitively) |

### Code changes

Six Borsh call sites were swapped to SSZ, all internal to the app:

- `desktop-app/src-tauri/src/infrastructure/action_codec.rs`
  - `use` block: `ssz::{Decode, Encode}` replaces implicit `borsh` traits.
  - `encode(action)` now returns `strata.as_ssz_bytes()`.
  - `decode(bytes)` now calls `MultisigAction::from_ssz_bytes(bytes)` (`ssz::DecodeError` formatted via `{e:?}` since it's not `Display`).
  - `test_encode_matches_direct_strata_borsh` renamed to `test_encode_matches_direct_strata_ssz`.
  - `CodecError` messages reference "ssz" instead of "borsh".
- `desktop-app/src-tauri/src/infrastructure/signing.rs`
  - `use ssz::Decode` replaces `use borsh::BorshDeserialize`.
  - `compute_sighash` uses `MultisigAction::from_ssz_bytes(&action_bytes)` instead of `try_from_slice`.
  - Test helper `demo_action_hex` uses `.as_ssz_bytes()`.
- Doc comments in `domain/action.rs`, `application/proposals.rs`, and `e2e-tests/tests/e2e_admin_subprotocol.rs` updated from "borsh" to "SSZ".

No public API of any Tauri command changed. No domain type (`Action`, `MultisigUpdate`, `Authority`, `CompressedPubKey`) changed. `CodecError` variant names are unchanged.

## Wire format compatibility (the important part)

Despite the Borsh→SSZ rewrite upstream, **the bytes signers sign and the bytes verifiers verify are unchanged**.

Evidence, from reading both versions side-by-side:

- `sighash_payload()` on `CancelAction` is hand-coded as `self.target_id.to_be_bytes().to_vec()`. Identical in both versions.
- `sighash_payload()` on `MultisigUpdate` is hand-coded as `len(add) ‖ add[0] ‖ … ‖ len(rem) ‖ rem[0] ‖ … ‖ threshold`, with `u32` BE length prefixes, 33-byte compressed pubkeys, and the threshold as a single byte. Identical in both versions.
- `compute_sighash(seqno)` on the `Sighash` trait is `SHA256(SHA256(tag) ‖ seqno_be ‖ sighash_payload)`. Identical in both versions.

So signatures produced against the old crate remain valid against the new crate, and vice versa. The format flip is confined to:

- How `MultisigAction` is serialized for transport between our codec, our Tauri commands, and the orchestrator's hex field.
- How upstream itself packs `SignedPayload` (action + signatures) into the SPS-51 witness envelope — but that packing is done upstream inside `create_test_admin_tx` and `parser::parse_tx`, which we do not reimplement.

## PRD coverage — no change

The migration does **not** close any of the PRD gaps documented in [`08-alpen-crate-prd-coverage.md`](./08-alpen-crate-prd-coverage.md):

- The `Role` enum upstream still has only `StrataAdministrator` and `StrataSequencerManager`. `AlpenAdministrator` and `SecurityCouncil` remain absent.
- The 8 unsupported update types listed in the coverage document are still not representable.
- `AdminTxType` discriminants are byte-identical pre and post migration.

The coverage status is fully unchanged. See `08-alpen-crate-prd-coverage.md` for the ongoing list.

## Future bumps

1. Check upstream tags:
   ```bash
   gh api repos/alpenlabs/asm/tags --jq '.[] | "\(.name) \(.commit.sha[0:7])"'
   gh api repos/alpenlabs/strata-common/tags --jq '.[] | "\(.name) \(.commit.sha[0:7])"'
   ```
2. Confirm alignment: `alpenlabs/alpen/main` Cargo.toml always shows which versions are considered compatible upstream at any given time.
3. Update `rev` / `tag` fields in root `Cargo.toml`.
4. Run `cargo build --workspace`, `cargo test --workspace`, and in particular verify `test_encode_matches_direct_strata_ssz` passes — any divergence signals an upstream wire-format change.
5. If a bump introduces new upstream crates that would be useful to us (`strata-asm-proto-administration`, `strata-l1-envelope-fmt`, etc.), open a separate issue rather than expanding the bump PR.

## Decisions deliberately out of scope

- **Integration of new crates.** `strata-asm-proto-administration`, `strata-l1-envelope-fmt`, `strata-identifiers` (beyond what's pulled transitively), and `bitcoind-async-client` are available in the new upstream and could simplify future work. They are not adopted here.
- **Tag pinning for `alpenlabs/asm`.** The coexistence of two tag schemes (`v0.1-alpha.N` and `v0.1.0-rcN`) on the same commit history is confusing. We kept `rev` pinning for clarity until upstream settles on one scheme.
- **Closing PRD coverage gaps.** Requires upstream changes; still tracked in `08-alpen-crate-prd-coverage.md`.
