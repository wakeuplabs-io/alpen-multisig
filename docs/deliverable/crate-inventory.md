### 1.1 Crate Inventory

**Confirmed and in use** (pinned in workspace `Cargo.toml`, `alpenlabs/asm` rev `a8559d3` / `alpenlabs/strata-common` tag `v0.1.0-alpha-rc16`):

| Crate                       | Source                      | Key types / functions                                                                                                                  | Used by                | Replaceable?                                 |
| --------------------------- | --------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ---------------------- | -------------------------------------------- |
| `strata-asm-txs-admin`      | `alpenlabs/asm`             | `MultisigAction`, `UpdateAction`, `CancelAction`, `Sighash::compute_sighash()`, `parser::parse_tx()`, `SignedPayload`                  | desktop-app, e2e-tests | No — canonical SSZ layout and sighash tags   |
| `strata-asm-params`         | `alpenlabs/asm`             | `Role` enum — **2 variants today**: `StrataAdministrator`, `StrataSequencerManager`                                                    | desktop-app, e2e-tests | No — SSZ discriminant must match ASM         |
| `strata-asm-common`         | `alpenlabs/asm`             | `TxInputRef`                                                                                                                           | e2e-tests              | No — required by `parser::parse_tx()`        |
| `strata-asm-txs-test-utils` | `alpenlabs/asm`             | `TEST_MAGIC_BYTES`, reveal-tx construction helpers                                                                                     | e2e-tests              | No — builds exact witness envelope structure |
| `strata-crypto`             | `alpenlabs/strata-common`   | `CompressedPublicKey`, `ThresholdConfig`, `ThresholdConfigUpdate`, `verify_threshold_signatures()`, `SignatureSet`, `IndexedSignature` | desktop-app, e2e-tests | No — types embedded in SSZ serialization     |
| `strata-l1-txfmt`           | `alpenlabs/strata-common`   | `ParseConfig`, `TagData` (SPS-50 parsing)                                                                                              | e2e-tests              | No — protocol header format                  |
| `strata-identifiers`        | `alpenlabs/strata-common`   | `Buf32` (sighash return type)                                                                                                          | transitive             | No — return type of `compute_sighash()`      |
| `ssz`                       | `alpenlabs/ssz-gen` v0.15.0 | `Encode`, `Decode` traits used by our codec                                                                                            | desktop-app            | No — must match the upstream derive output   |

**Required for the final delivery, not yet integrated in the workspace:**

| Crate                                   | Source                    | Needed for                                                                                                     | PRD driver                                                                                             |
| --------------------------------------- | ------------------------- | -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `strata-asm-subprotocols-admin`         | `alpenlabs/alpen`         | Reading canonical signer sets via `AdministrationSubprotoState` / `MultisigAuthority` (backend access control) | Backend PRD §3 — "backend must run the ASM STF to get the canonical set of signers for each authority" |
| `strata-l1-envelope-fmt`                | `alpenlabs/strata-common` | SPS-51 reveal-script envelope construction (`EnvelopeScriptBuilder`, auto-chunks at 520 bytes)                 | UI PRD req 13.2 — create and broadcast approval transactions                                           |
| `strata-btcio` (`writer::builder`)      | `alpenlabs/alpen`         | Commit + reveal transaction construction (`EnvelopeConfig`, `create_envelope_transactions`) at broadcast time  | UI PRD req 13.2.1 — "Send" button, sat/vB fee-rate control                                             |
| `bitcoind-async-client` (or equivalent) | external / `alpen`        | Bitcoin RPC client for wallet signing of the commit tx and raw-tx broadcast                                    | UI PRD req 13.2 — broadcast via the application's Bitcoin RPC                                          |

None of these four are compiled into the workspace today; they are the integration surface remaining for Phase 3.

> **Sources:** [`docs/2-discovery/03-poc1-findings.md`](../2-discovery/03-poc1-findings.md) §5–§6, [`docs/2-discovery/08-alpen-crate-prd-coverage.md`](../2-discovery/08-alpen-crate-prd-coverage.md), [`docs/2-discovery/10-asm-bitcoin-state-model.md`](../2-discovery/10-asm-bitcoin-state-model.md).
