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
| DDD-8 | Structured broadcast error contract: `proposals_broadcast` returns `{ code, message }` (backward-compatible JSON string) instead of a flat string; each backend variant maps to a stable `code` classified BEFORE/AFTER the broadcast boundary | Accepted | UI must branch on error kind; resubmit-reveal must be offered ONLY when a live `PendingReveal` exists (post-boundary). Mirrors the existing `serialize_wallet_error` `{ type, message }` precedent |
| DDD-9 | Frontend device-UX + error surfacing: coarse `awaiting-device` state + `BroadcastDevicePrompt` for the HW pre-sign window (skipped for mnemonic); `deriveBroadcastError` consumes `code`; resubmit gated on `recovery === 'resubmit-reveal'` (latent-bug fix); button stays `disabled={!canSign}` (no new gating) | Accepted | The end-to-end HW path needs UI; the mnemonic path must stay visually unchanged (slice (a)); fixes the reachable-today resubmit bug |

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
| `proposals_broadcast` + `map_broadcast_error` | `commands/proposals.rs` | Modified — structured `{ code, message }` error (DDD-8); new `broadcast_error_code` helper; happy-path contract unchanged |
| `admin_wallet_can_sign` (or new `admin_wallet_sign_status`) | `commands/admin_wallet.rs` | Modified (slice b) — return `{ canSign, signerKind, reason? }` DTO; bare-bool still accepted for graceful degradation |
| `deriveBroadcastError` (new) / `BroadcastError` view-model | `domain/broadcast-proposal/model/broadcast-proposal.ts` | EXTEND — add `deriveBroadcastError` (parses `code`, maps to copy + recovery; mirrors `parse-admin-wallet-error.ts`); add `awaiting-device` phase |
| `useBroadcastProposal` (controller hook) | `domain/broadcast-proposal/hooks/use-broadcast-proposal.ts` | EXTEND — recovery-gated `canResubmit`; `awaiting-device` state |
| `BroadcastDetailsCard` | `domain/broadcast-proposal/components/broadcast-details-card.tsx` | EXTEND — recovery-gated resubmit, device prompt mount, kind-specific copy; `disabled={!canSign}` unchanged |
| `BroadcastDevicePrompt` | `domain/broadcast-proposal/components/broadcast-device-prompt.tsx` | CREATE NEW — "Confirm on your device" affordance (HW path only) |
| `useAdminWalletCapability` (real canSign source) | `domain/admin-wallet/hooks/use-admin-wallet-capability.ts` | EXTEND — surface `signerKind` + `canSignReason` |
| `BroadcastProposalScreen` (wiring) | `screens/broadcast-proposal-screen.tsx` | EXTEND — pass `signerKind`/`canSignReason` into controller + card (route composition only) |
| `api/proposals.ts` + `api/ipc-schemas.ts` | `desktop-app/src/api` | EXTEND — Zod-parse structured error + optional wallet-status DTO fields |

---

## Wave: DESIGN / [REF] Driving Ports

The desktop surface (button → controller → api) is a driving port alongside the unchanged IPC command.
Dependency direction: **UI → controller → api(invoke) → Tauri command → application.**

| Port | Surface | Notes |
|---|---|---|
| `proposals_broadcast` (Tauri IPC command) | Happy-path unchanged; **error shape modified** | Builds `CommitFunding` from session, calls `broadcast_commit_then_reveal`; now rejects with structured `{ code, message }` (DDD-8) |
| `BroadcastDetailsCard` + `useBroadcastProposal` (frontend driving surface) | EXTEND | The human-facing driving port: confirm/resubmit controls, device prompt, kind-specific errors; gates on `canSign` (unchanged) and `recovery` (new) |

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
| DDD-8 | Locked | commands/proposals (map_broadcast_error), broadcast-proposal model |
| DDD-9 | Locked | broadcast-proposal hook/components, admin-wallet capability hook, wallet-status DTO |

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
| `proposals_broadcast` / `map_broadcast_error` | `commands/proposals.rs` | EXTEND | Structured `{ code, message }` error (DDD-8); session wiring already in place; new pure `broadcast_error_code` helper |
| `admin_wallet_can_sign` | `commands/admin_wallet.rs` | EXTEND (slice b) | Return `{ canSign, signerKind, reason? }`; bare-bool still accepted |
| `PsbtSigner` port | `application/psbt_signer.rs` | CREATE NEW | No existing signing seam — justified |
| `MnemonicPsbtSigner` | `application/psbt_signer.rs` | CREATE NEW | New software signer; wraps existing `wallet.sign` |
| `HwPsbtSigner` | `infrastructure/hw_wallet/hw_psbt_signer.rs` | CREATE NEW | New device adapter; wraps existing hw_wallet clients |
| `deriveBroadcastError` + `BroadcastError`/phase model | `desktop-app/src/domain/broadcast-proposal/model/broadcast-proposal.ts` | EXTEND | Parse `code`, map to copy + recovery; add `awaiting-device`; seam already commented "structured error codes in R1.1" |
| `useBroadcastProposal` controller | `desktop-app/src/domain/broadcast-proposal/hooks/use-broadcast-proposal.ts` | EXTEND | Recovery-gated `canResubmit` (latent-bug fix); `awaiting-device` state |
| `BroadcastDetailsCard` | `desktop-app/src/domain/broadcast-proposal/components/broadcast-details-card.tsx` | EXTEND | Recovery-gated resubmit; device-prompt mount; kind-specific copy; `disabled={!canSign}` unchanged |
| `BroadcastDevicePrompt` | `desktop-app/src/domain/broadcast-proposal/components/broadcast-device-prompt.tsx` | CREATE NEW | No existing device-interaction affordance — justified |
| `BroadcastPhaseProgress` | `desktop-app/src/domain/broadcast-proposal/components/broadcast-phase-progress.tsx` | EXTEND (minor) | Rank `awaiting-device` as commit-active |
| `useAdminWalletCapability` (real canSign source) | `desktop-app/src/domain/admin-wallet/hooks/use-admin-wallet-capability.ts` | EXTEND | Surface `signerKind` + `canSignReason` (this is the actual `canSign` source the screen consumes) |
| `BroadcastProposalScreen` | `desktop-app/src/screens/broadcast-proposal-screen.tsx` | EXTEND (wiring) | Pass `signerKind`/`canSignReason` into controller + card; route composition only |
| `api/proposals.ts` + `api/ipc-schemas.ts` | `desktop-app/src/api` | EXTEND | Zod-parse structured error + optional wallet-status DTO fields; happy-path schemas unchanged |
| `useCancelBroadcast` | `desktop-app/src/domain/cancel-proposal/hooks/use-cancel-broadcast.ts` | VERIFY / small change | Spreads `useBroadcastProposal`; once `error` becomes `BroadcastError \| null`, consumers reading `error` as a string must read `error?.message` — audit the 1–2 call sites |

---

## Wave: DESIGN / [REF] Open Questions

- Pinned `trezor_client` / Ledger app support for BIP-86 taproot **key-path** PSBT signing (confirm at slice (b)).
- ~~Whether to ship the `BdkAdminWalletMnemonic` → `AdminWalletCommitFunding` rename in R1.1 or defer~~ — **Resolved:** rename in slice (a); the file is already touched and the name actively misleads once `HwPsbtSigner` exists.
- **Device progress: coarse pending state vs Rust→JS event channel.** **Recommended: coarse `awaiting-device`** derived from `inFlight` + `signerKind` is enough for R1.1 (the single IPC call blocks through device signing; no channel needed). A fine-grained event stream is a Phase-7 nicety.
- **`canSign` reason in the status DTO.** **Recommended: carry `{ canSign, signerKind, reason? }`** from the backend (it already knows the capability rule) rather than inferring in the controller — avoids duplicating DDD-3 in TS. Additive + Zod-parsed; bare-bool still accepted (graceful degradation). Confirm command name (`admin_wallet_can_sign` overload vs new `admin_wallet_sign_status`) at slice (b).
- **Frontend layout reconciliation (resolved).** The repo uses a `domain/`-oriented layout, not an FSD `features/` layout, and there is no confirmation-modal/button/error-alert split — the controller is `hooks/use-broadcast-proposal.ts`, the confirm button + disabled tooltip live in `components/broadcast-details-card.tsx`, the error banner in `components/broadcast-phase-progress.tsx`, and `canSign` is sourced by `useAdminWalletCapability` (`domain/admin-wallet/hooks/use-admin-wallet-capability.ts`), not by a broadcast-local hook. **There is no "Resubmit reveal" control on this card today** — `canResubmit` is a forward-looking gating contract (recovery-driven) introduced so the latent bug is structurally impossible before any resubmit affordance is wired. The design targets these real files; "modal/panel" means an in-card affordance unless a new component is named.

---

(end)
