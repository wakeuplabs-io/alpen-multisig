# Feature Delta — Admin Wallet Session-Driven Broadcast Signing (R1.1)

**Wave:** DESIGN
**Spec:** [admin-wallet-session-driven-broadcast-signing.md](../../specs/admin-wallet-session-driven-broadcast-signing.md)

---

## Wave: DESIGN / [REF] Domain-Driven Decisions

| ID | Decision | Verdict | Rationale |
|---|---|---|---|
| DDD-1 | `PsbtSigner` driven port on `WalletService` (hexagonal); build PSBT via BDK for both paths, `signer.sign_psbt`, then finalize + extract_tx | Accepted | One signing seam; downstream `CommitFunding` + `broadcast_commit_then_reveal` unchanged |
| DDD-2 | Two implementors behind the same port: `MnemonicPsbtSigner` (simulated HW) and `HwPsbtSigner` (real device) | Accepted | Identical flow; only the signer differs — mnemonic exercises the unified path without a device |
| DDD-3 | Remove `ALLOW_DEV_MNEMONIC_SIGNING`; replace with `signer.allowed_on(network)` | Accepted | Env flag conflated enabled-vs-network; capability is typed and per-signer |
| DDD-4 | HW device access via `spawn_blocking`, re-open by fingerprint at sign time; device-absent/refusal → typed error before broadcast | Accepted | Trezor client is synchronous; no live connection held; nothing hits the network on failure |
| DDD-5 | Reveal unchanged — signed by ephemeral envelope key, never routed to `PsbtSigner` | Accepted | Taproot script-path over a custom envelope leaf; a HW cannot sign it |
| DDD-6 | Slice R1.1 into (a) port + `MnemonicPsbtSigner` + flag removal, (b) `HwPsbtSigner` | Accepted | (a) is the walking skeleton (regtest, no device) and de-risks (b); both ship under R1.1 |
| DDD-7 | PSBT carries taproot derivation metadata; BDK builds the fully-annotated unsigned PSBT for both paths | Accepted | Watch-only descriptor wallet knows BIP-86 derivation; device can sign the built PSBT |

---

## Wave: DESIGN / [REF] Component Decomposition

| Component | Path | Change |
|---|---|---|
| `PsbtSigner` port + `MnemonicPsbtSigner` | `application/psbt_signer.rs` | New — driven port + software signer |
| `HwPsbtSigner` | `infrastructure/hw_wallet/hw_psbt_signer.rs` | New — on-device taproot key-path signer |
| `WalletService` | `application/wallet_service.rs` | Modified — split `build_psbt`/sign, hold `Option<Arc<dyn PsbtSigner>>`, new `can_sign` |
| `WalletSession` | `application/wallet_session.rs` | Modified — attach signer per login; new HW error variants |
| `TrezorAdapter` / `LedgerAdapter` | `infrastructure/hw_wallet/{trezor,ledger}.rs` | Modified — add `sign_psbt` (taproot key-path) |
| `broadcast_env` | `infrastructure/broadcast_env.rs` | Modified — drop `allow_dev_mnemonic_signing` |
| `AdminWalletError` | `infrastructure/admin_wallet/wallet.rs` | Modified — add `SignerNotAllowedOnNetwork` |
| `CommitFunding` / `BdkAdminWalletMnemonic` | `application/commit_funding.rs` | Unchanged signature (optional rename → `AdminWalletCommitFunding`) |
| `broadcast_commit_then_reveal` | `application/proposals.rs` | Unchanged |
| `proposals_broadcast` | `commands/proposals.rs` | Minimal / none |

---

## Wave: DESIGN / [REF] Driving Ports

| Port | Surface | Notes |
|---|---|---|
| `proposals_broadcast` (Tauri IPC command) | Unchanged | Builds `CommitFunding` from session, calls `broadcast_commit_then_reveal` — no contract change |

---

## Wave: DESIGN / [REF] Driven Ports + Adapters

| Port | Adapter(s) | Notes |
|---|---|---|
| `PsbtSigner` | `MnemonicPsbtSigner` (software/simulated HW) ; `HwPsbtSigner` (Trezor/Ledger on-device) | `allowed_on`: mnemonic = regtest\|testnet ; HW = any |
| `CommitFunding` | `BdkAdminWalletMnemonic` | Unchanged; routes through the session signer transparently |
| `BitcoinRpcClient` | (existing) | Unchanged |

---

## Wave: DESIGN / [REF] Technology Choices

| Choice | Use | Pin |
|---|---|---|
| Rust / Tauri 2 | Desktop shell + backend | Workspace (no new pin) |
| `bdk_wallet` | PSBT build, finalize, software sign | Workspace (no new pin) |
| `trezor_client` / Ledger | On-device taproot key-path PSBT signing | Workspace (no new pin) |
| `tokio::task::spawn_blocking` | Bridge synchronous HW client into async flow | Workspace (no new pin) |

No new dependencies beyond what the workspace already pins.

---

## Wave: DESIGN / [REF] Decisions Table

| DDD | Status | Owner area |
|---|---|---|
| DDD-1 | Locked | application/wallet_service, psbt_signer |
| DDD-2 | Locked | psbt_signer, hw_wallet |
| DDD-3 | Locked | broadcast_env, wallet_service, admin_wallet error |
| DDD-4 | Locked | hw_wallet/hw_psbt_signer, wallet_session |
| DDD-5 | Locked | application/proposals (reveal) |
| DDD-6 | Locked | release slicing |
| DDD-7 | Locked | wallet_service/build_psbt |

---

## Wave: DESIGN / [REF] Reuse Analysis

| Element | Path | Verdict | Justification |
|---|---|---|---|
| `WalletService` | `application/wallet_service.rs` | EXTEND | Split build/sign; hold optional signer; reuses existing sync + BDK build |
| `CommitFunding` / `BdkAdminWalletMnemonic` | `application/commit_funding.rs` | EXTEND + RENAME (no signature change) | Rename → `AdminWalletCommitFunding` **in slice (a)**: the file is already touched and the name actively misleads once `HwPsbtSigner` exists |
| `broadcast_commit_then_reveal` | `application/proposals.rs` | NO CHANGE | Downstream-unchanged is a key invariant of this slice |
| `WalletSession` | `application/wallet_session.rs` | EXTEND | Attach correct signer per login type at init |
| `TrezorAdapter` / `LedgerAdapter` | `infrastructure/hw_wallet/{trezor,ledger}.rs` | EXTEND | Add taproot PSBT signing to the existing adapter surface |
| `broadcast_env` | `infrastructure/broadcast_env.rs` | EXTEND | Remove the env flag function |
| `proposals_broadcast` | `commands/proposals.rs` | MINIMAL / NONE | Session wiring already in place |
| `PsbtSigner` port | `application/psbt_signer.rs` | CREATE NEW | No existing signing seam — justified |
| `MnemonicPsbtSigner` | `application/psbt_signer.rs` | CREATE NEW | New software signer; wraps existing `wallet.sign` |
| `HwPsbtSigner` | `infrastructure/hw_wallet/hw_psbt_signer.rs` | CREATE NEW | New device adapter; wraps existing hw_wallet clients |

---

## Wave: DESIGN / [REF] Open Questions

- Pinned `trezor_client` / Ledger app support for BIP-86 taproot **key-path** PSBT signing (confirm at slice (b)).
- ~~Whether to ship the `BdkAdminWalletMnemonic` → `AdminWalletCommitFunding` rename in R1.1 or defer~~ — **Resolved:** rename in slice (a); the file is already touched and the name actively misleads once `HwPsbtSigner` exists.

---

(end)
