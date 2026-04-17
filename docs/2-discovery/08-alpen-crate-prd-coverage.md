# Finding — Alpen Crate Coverage vs PRD Requirements

> **Status:** In progress — investigation ongoing
> **Re-validated:** 2026-04-17 against `alpenlabs/asm` rev `a8559d3` (== tag `v0.1-alpha.5`). None of the coverage gaps listed below are closed by the ASM repo migration; `Role` enum, `AdminTxType` discriminants, and `sighash_payload` bytes are identical to the pre-migration version. See [`11-asm-repo-migration.md`](./11-asm-repo-migration.md) for migration details.

## Overview

This document maps every update type and authority role required by the PRD against the types currently available in the Alpen/Strata crates (pinned at `alpenlabs/asm` rev `a8559d3`, `alpenlabs/strata-common` tag `v0.1.0-alpha-rc16`). The goal is to identify what can be built today, what is blocked on upstream Alpen crate additions, and what requires a fundamentally different approach.

### Sources

- **PRD** — [`docs/0-prd/01-multisig-ui.md`](../0-prd/01-multisig-ui.md) (requirements 11–20)
- **Backend PRD** — [`docs/0-prd/02-multisig-backend.md`](../0-prd/02-multisig-backend.md)
- **Alpen crate source** — `https://github.com/alpenlabs/asm` rev `a8559d3` (`crates/params/src/subprotocols/admin.rs`, `crates/txs/admin/src/`)
- **ADR-001** — [`docs/architecture/adrs/001-alpen-crate-dependencies.md`](../architecture/adrs/001-alpen-crate-dependencies.md)

---

## 1. Current Alpen Crate Surface

### `Role` enum (2 variants)

Defined in `crates/params/src/subprotocols/admin.rs` (upstream path in `alpenlabs/asm`):

- `StrataAdministrator`
- `StrataSequencerManager`

### `AdminTxType` enum (7 variants)

Defined in `crates/txs/admin/src/constants.rs` (upstream path in `alpenlabs/asm`):

| Variant | u8 | Sighash tag |
|---|---|---|
| `Cancel` | 0 | `strata/admin/cancel` |
| `StrataAdminMultisigUpdate` | 10 | `strata/admin/strata_admin_multisig_update` |
| `StrataSeqManagerMultisigUpdate` | 11 | `strata/admin/strata_seq_manager_multisig_update` |
| `OperatorUpdate` | 20 | `strata/admin/operator_update` |
| `SequencerUpdate` | 21 | `strata/admin/sequencer_update` |
| `OlStfVkUpdate` | 30 | `strata/admin/ol_stf_vk_update` |
| `AsmStfVkUpdate` | 31 | `strata/admin/asm_stf_vk_update` |

### `UpdateAction` enum (4 variants)

Defined in `crates/asm/txs/admin/src/actions/updates/mod.rs`:

- `Multisig(MultisigUpdate)` — parameterized by `Role`
- `OperatorSet(OperatorSetUpdate)`
- `Sequencer(SequencerUpdate)`
- `VerifyingKey(PredicateUpdate)` — parameterized by `ProofType` (Asm / OLStf)

### `MultisigAction` enum (2 variants)

- `Cancel(CancelAction)`
- `Update(UpdateAction)`

---

## 2. PRD Update Type Mapping

### Implemented in Alpen crates (5 update types + cancel)

| PRD Update Type | Authority | `UpdateAction` variant | `AdminTxType` | Sighash tag |
|---|---|---|---|---|
| Strata Administrator Signer update | Strata Admin | `Multisig(MultisigUpdate)` | `StrataAdminMultisigUpdate` (10) | `strata/admin/strata_admin_multisig_update` |
| Strata verification key update | Strata Admin | `VerifyingKey(PredicateUpdate)` | `OlStfVkUpdate` (30) | `strata/admin/ol_stf_vk_update` |
| Operator update | Strata Admin | `OperatorSet(OperatorSetUpdate)` | `OperatorUpdate` (20) | `strata/admin/operator_update` |
| Seq Manager Signer update | Seq Manager | `Multisig(MultisigUpdate)` | `StrataSeqManagerMultisigUpdate` (11) | `strata/admin/strata_seq_manager_multisig_update` |
| Sequencer update | Seq Manager | `Sequencer(SequencerUpdate)` | `SequencerUpdate` (21) | `strata/admin/sequencer_update` |
| Cancel action | Admin / Seq Mgr | `MultisigAction::Cancel` | `Cancel` (0) | `strata/admin/cancel` |

These can be built, tested, and integrated today.

### Not implemented in Alpen crates (8 update types — blocked on upstream)

| PRD Update Type | Authority | Status | Notes |
|---|---|---|---|
| Alpen verification key update | Alpen Admin | **Blocked** | No `Role::AlpenAdministrator` — zero references in entire Alpen codebase |
| Alpen Administrator Signer update | Alpen Admin | **Blocked** | Same — role does not exist |
| Safe Harbor address update | Strata Admin | **Blocked** | Zero references to "safe harbor" in Alpen codebase |
| Security Council Signer update | Strata Admin | **Blocked** | No `Role::SecurityCouncil` |
| "Soft" bridge update | Strata Admin | **Blocked** | Zero references — term only appears in PRD, semantics unclear |
| "Hard" bridge update | Strata Admin | **Blocked** | Zero references — same |
| Defcon 1 transaction | Security Council | **Blocked** | Zero references to "defcon" in Alpen codebase |
| Defcon 3 transaction | Security Council | **Blocked** | Same |

These types require Alpen to add new `Role` variants, `AdminTxType` values, sighash tags, and `UpdateAction` variants to their crates before the multisig app can implement them.

### Payout Administrator — fundamentally different

`block_payout` is **not part of the admin subprotocol**. It is a Bitcoin-native UTXO spend from the bridge multisig script, not an SPS-50/SPS-65 tagged admin transaction. It requires:

- Direct PSBT construction using the `bitcoin` crate
- Knowledge of the bridge script spending conditions
- A Bitcoin RPC client for broadcast
- Custom application logic, not Alpen admin crates

The bridge-v1 subprotocol (`crates/asm/subprotocols/bridge-v1/`) handles deposits, withdrawals, and operator assignments, but does not contain `block_payout` spending logic.

---

## 3. Role Coverage

| PRD Role | Exists in `Role` enum? | Coverage |
|---|---|---|
| Alpen Administrator | **No** | 0% — role not defined anywhere |
| Strata Administrator | **Yes** | 43% — 3 of 7 update types available |
| Strata Sequencer Manager | **Yes** | 100% — all 2 update types available |
| Security Council | **No** | 0% — role not defined anywhere |
| Payout Administrator | **No** | 0% — separate subprotocol entirely |

---

## 4. Crate Dependency Analysis

### What each crate provides for this project

| Crate | Key types/functions used | Used by | Replaceable? |
|---|---|---|---|
| `strata-asm-txs-admin` | `MultisigAction`, `UpdateAction`, `CancelAction`, `Sighash`, `compute_sighash()`, `parser::parse_tx()`, `SignedPayload` | desktop-app, e2e-tests | No — canonical Borsh layout and sighash tags |
| `strata-crypto` | `CompressedPublicKey`, `ThresholdConfig`, `ThresholdConfigUpdate`, `verify_threshold_signatures()`, `SignatureSet` | desktop-app, e2e-tests | No — types embedded in Borsh serialization |
| `strata-asm-params` | `Role` enum | desktop-app (tests), e2e-tests | No — Borsh discriminant must match ASM |
| `strata-primitives` | `Buf32` (sighash return type) | e2e-tests (transitively) | No — return type of `compute_sighash()` |
| `strata-asm-common` | `TxInputRef` | e2e-tests | No — required by `parser::parse_tx()` |
| `strata-l1-txfmt` | `ParseConfig`, `TagData` (SPS-50 parsing) | e2e-tests | No — protocol header format |
| `strata-asm-txs-test-utils` | `TEST_MAGIC_BYTES`, tx construction helpers | e2e-tests | No — builds exact witness envelope structure |
| `strata-test-utils` | General test utils | e2e-tests (declared, not directly imported) | Could be removed if unused |

### Why none are replaceable

All crates define the **canonical SSZ serialization layout** that the ASM on-chain parser expects. A single byte difference in enum discriminant, field ordering, or hash tag produces a different sighash, and the ASM rejects the transaction. These crates are the protocol definition, not utility libraries. (Until upstream PR `alpenlabs/asm#8` on 2026-03-25 the format was Borsh; the discriminants and `sighash_payload` bytes carried across unchanged.)

### Crates that should be added

| Crate | Source | Needed for |
|---|---|---|
| `strata-asm-proto-administration` | `alpenlabs/asm` | Reading canonical signer sets from ASM state (`AdministrationSubprotoState`, `MultisigAuthority`). Required by the backend for access control per PRD §3. (Renamed upstream from `strata-asm-subprotocols-admin` when the repo was split.) |
| `strata-l1-envelope-fmt` | `alpenlabs/strata-common` | SPS-51 envelope construction for production Bitcoin transactions (currently only a transitive dependency). |

---

## 5. Open Questions

- **"Soft" vs "Hard" bridge update** — What do these terms mean? They appear only in the PRD but have zero references in the Alpen codebase. Need clarification from Alpen.
- **Safe Harbor address** — No references in codebase. Is this a new concept not yet implemented, or is it represented differently?
- **Defcon 1/3** — Are these standard admin transactions or a completely separate mechanism? They may not even be `UpdateAction` variants.
- **Alpen Administrator** — The SPS-65 spec mentions "there should be another entity called Alpen Administrator". Is this planned for a specific release?
- **`block_payout` bridge script** — Where are the spending conditions defined? What crates (if any) provide the script templates?
- **ASM state RPC** — What RPC endpoint provides the current `AdministrationSubprotoState`? Is there a client crate, or does the app need to implement its own RPC client?
- **Timeline for upstream gaps** — When does Alpen plan to add the missing roles and update types?
