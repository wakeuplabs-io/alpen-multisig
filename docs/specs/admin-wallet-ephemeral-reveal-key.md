# Spec: Admin Wallet — Ephemeral Reveal Key (R1.0)

> Release 1, step R1.0 of the [Admin Wallet implementation plan](./admin-wallet-implementation-plan.md).
> Supersedes the seed-derived commit/reveal key at `m/86'/0'/73'/2/0` introduced in Phase 3.5 / 3.7b.
> Protocol context: [`proposal-broadcast-commit-reveal.md`](./proposal-broadcast-commit-reveal.md).

## Objective

Replace the SPS-50 commit/reveal **internal (envelope) key** — currently derived from the login-session
seed at `m/86'/0'/73'/2/0` — with a **per-broadcast ephemeral key** generated in-memory by the app.

The envelope key is **not custody-significant**: governance authority lives entirely in the SPS-65
`SignatureSet` carried inside the reveal payload. The envelope key is only a carrier for a taproot
**script-path** spend, which a hardware wallet cannot sign anyway. Decoupling it from the seed:

- Makes reveal signing **login-agnostic** (no dependency on the session mnemonic for the envelope key).
- Shrinks R1.1 to "the session signer signs only the commit-funding tx" (the part that spends real funds).
- Removes a derivation path and a cached secret from the session lifecycle.

Because the ephemeral key is a throwaway, the **reveal change output must be redirected to an Admin Wallet
address** so no funds are stranded on the throwaway key.

## Scope

### Included

- New in-app generator for a per-broadcast ephemeral envelope keypair (cryptographically secure RNG,
  in-memory only, never persisted, never seed-derived).
- `broadcast_commit_then_reveal` and `prepare_broadcast_bundle` generate a fresh ephemeral key per call
  instead of consuming a session-cached keypair.
- Reveal transaction **change output redirected to an Admin Wallet address** (rotating internal/change
  keychain via `reveal_next_address(Internal)`), instead of the envelope key's own key-path address.
- `BroadcastEnv` no longer carries a commit/reveal keypair; gating (`MnemonicSigningDisabled`,
  `WalletSessionRequired`, `ReadOnly`/watch-only) is preserved via `WalletSession` capability checks.
- Removal of the seed-derived key: delete `derive_commit_reveal_keypair`, the `m/86'/0'/73'/2/0` path,
  `SessionState.commit_reveal_keypair`, and `WalletSession::commit_reveal_keypair()`.
- Frontend: relabel the broadcast preview from "Commit TX" to **"Commit TX (preview)"** so the now-indicative
  commit address is not read as the final on-chain address.
- Update [`proposal-broadcast-commit-reveal.md`](./proposal-broadcast-commit-reveal.md) to document the
  ephemeral key, the change-to-Admin-Wallet redirect, and the (temporary) crash window closed by R1.0.1.

### Not included

- **R1.0.1** — building/signing both commit and reveal *before* broadcasting (crash-recovery). R1.0
  retains today's ordering (commit broadcast first, reveal built after commit confirms). The ephemeral
  key therefore lives across the commit→reveal window; loss on crash is bounded to the commit dust + fee.
  Documented as a known, temporary limitation.
- **R1.1** — hardware-wallet (PSBT) signing of the commit-funding tx. In R1.0 the commit funding is still
  software-signed by the mnemonic session (`BdkAdminWalletMnemonic` → `WalletService::fund_commit`);
  watch-only/HW sessions remain `ReadOnly` for broadcast.
- Receive-address rotation as a user feature (R1.3). This spec only rotates the *reveal change* address.
- Any change to SPS-50/51/65 envelope semantics, payload shape, op_return tagging, or signature handling.
- RBF of the commit (Phase 5) — would require re-deriving/persisting the envelope key; out of scope.

## Technical Design

### Current state (to be replaced)

```
login (mnemonic) ──► WalletSession.build_session_from_mnemonic
                        ├─ derive_commit_reveal_keypair(mnemonic, m/86'/0'/73'/2/0) ──► cached keypair
                        └─ load_admin_wallet ──► WalletService

broadcast/prepare ──► load_broadcast_env(session)
                        └─ resolve_commit_reveal_keypair ──► BroadcastEnv.commit_reveal_keypair
                                                                 │
              ┌──────────────────────────────────────────────────┘
              ▼
  derive_commit_address(keypair, payload)        build_reveal_tx(keypair, …)
                                                    └─ change → Address::p2tr(keypair, …)  ◄── strands on key
```

### Target state (R1.0)

```
login (mnemonic) ──► WalletSession  (no envelope key cached)
                        └─ load_admin_wallet ──► WalletService (can_sign = true)

broadcast/prepare ──► load_broadcast_env(session)  [gates only: dev-signing, session present, can_sign]
                        │
                        ├─ generate_ephemeral_envelope_keypair()  ◄── fresh per call, in-memory
                        │     └─ derive_commit_address(ephemeral, payload)
                        │
                        └─ (broadcast only) WalletService.reveal_change_address()  [reveal_next_address(Internal)]
                              └─ build_reveal_tx(ephemeral, …, change_spk)  ◄── change → Admin Wallet
```

### Production functions

| Function | Module | Responsibility |
|---|---|---|
| `generate_ephemeral_envelope_keypair() -> UntweakedKeypair` | `infrastructure/admin_wallet/ephemeral_envelope_key.rs` (new) | Generate one fresh, in-memory SPS-50 envelope keypair from a CSPRNG. Never persisted, never seed-derived. |
| `WalletService::reveal_change_address(&self) -> Result<Address, AdminWalletError>` | `application/wallet_service.rs` | Reveal the next unused internal (change) address so reveal change lands on (and is recoverable by) the Admin Wallet. Locks the wallet; uses `reveal_next_address(Internal)`. |
| `broadcast_tx::build_reveal_tx(…, change_spk: ScriptBuf, fee_sats)` | `infrastructure/broadcast_tx.rs` | Build/sign the reveal; change output goes to the **passed-in** `change_spk`. Drops the `network`-derived self-change. Rename `operator_keypair` → `envelope_keypair`. |
| `broadcast_tx::derive_commit_address(envelope_keypair, payload, network)` | `infrastructure/broadcast_tx.rs` | Unchanged behavior; param renamed `operator_keypair` → `envelope_keypair`. |
| `proposals::broadcast_commit_then_reveal(…)` | `application/proposals.rs` | Drops the `operator_keypair` param; generates the ephemeral key internally; accepts `reveal_change_spk: ScriptBuf`; threads both into derive/build. |
| `proposals::prepare_broadcast_bundle` / `prepare_broadcast_local` | `application/proposals.rs` | Drops the `operator_keypair` param; generates an ephemeral key inline for the **indicative** preview address + fee estimate. |
| `load_broadcast_env(session) -> Result<BroadcastEnv, _>` | `infrastructure/broadcast_env.rs` | Keeps RPC/asm/network/magic parsing and all three gates; **removes** keypair resolution and the `commit_reveal_keypair`/`operator_keypair` fields. |

### IPC commands (`commands/proposals.rs`) — no signature change to the React boundary

- `proposals_prepare_broadcast`: calls the new `prepare_broadcast_local` (no keypair arg). `PrepareBroadcastDto`
  is unchanged; `commitAddress` is now **indicative** (regenerated per call, will not equal the broadcast's).
- `proposals_broadcast`: resolves `reveal_change_spk = wallet_service.reveal_change_address().await?` and passes
  it into `broadcast_commit_then_reveal` (no keypair arg).

### Gating (preserved, re-sourced)

`load_broadcast_env` keeps returning, in order:
1. `MnemonicSigningDisabled` when `ALLOW_DEV_MNEMONIC_SIGNING` is unset/false.
2. `WalletSessionRequired` when `wallet_session.current().is_none()`.
3. `ReadOnly` when `!wallet_session.can_sign()` (watch-only/HW session) — R1.1 lifts this for HW.

Rationale: the envelope key no longer needs the session, but **commit funding still does** (software-signed
in R1.0). Failing early in `load_broadcast_env` preserves the existing high-signal UX message; the
`WalletService::fund_commit` `can_sign()` guard remains the defense-in-depth backstop.

### Frontend

- **In scope:** relabel the preview section header from "Commit TX" to "Commit TX (preview)" in
  `domain/broadcast-proposal/components/broadcast-details-card.tsx`, because the displayed `commitAddress`
  is now per-call and indicative (it will not equal the broadcast's commit address). Label-only change —
  no DTO, props, or behavior changes; the address + amount keep rendering as today.

### Production code vs. test helpers

- **Production**: `generate_ephemeral_envelope_keypair`, `WalletService::reveal_change_address`,
  the modified `build_reveal_tx` / `derive_commit_address` / `broadcast_commit_then_reveal` /
  `prepare_broadcast_bundle`, and the trimmed `load_broadcast_env`.
- **Test helpers** (stay in `#[cfg(test)]`, never exposed as commands): deterministic keypair builders,
  `session_with_mnemonic`, `session_with_xpub`, RPC env setters, `MockBitcoinRpcClient`, `SpyCommitFunding`.
- No test helper is registered as a Tauri command or exported in a production API.

## Test Cases

Tests target production functions only.

### Happy path

1. `generate_ephemeral_envelope_keypair` returns **distinct** keypairs on consecutive calls (freshness /
   non-determinism).
2. `build_reveal_tx` change output `script_pubkey` **equals the passed `change_spk`** and is **not** the
   envelope key's key-path P2TR script.
3. `WalletService::reveal_change_address` returns a valid Admin Wallet internal P2TR address; two calls
   return **different** addresses (rotation via `reveal_next_address`).
4. Reveal tx still has exactly two outputs: `OP_RETURN` (tag, value 0) then change to `change_spk`;
   `change_amount = commit_amount - fee`.
5. Regtest integration / e2e-webdriver: an approved proposal broadcasts; commit and reveal confirm using a
   fresh ephemeral key; the reveal change UTXO is owned by the Admin Wallet (visible after sync).

### Edge cases

6. Watch-only session → `load_broadcast_env` returns `ReadOnly` (broadcast and prepare both gated).
7. No session → `WalletSessionRequired`.
8. `ALLOW_DEV_MNEMONIC_SIGNING` unset/false → `MnemonicSigningDisabled`.
9. Two `prepare`/`broadcast` runs for the same proposal produce **different** commit addresses
   (asserts determinism is intentionally gone; replaces the deleted pinned-address test).

### Expected errors

10. `reveal_change_address` propagates `AdminWalletError` (e.g. `Disabled`) rather than panicking when the
    wallet is unavailable.
11. `broadcast_commit_then_reveal` still reports `failed` to the orchestrator on any inner step error
    (unchanged error-reporting path).

### Authority isolation

12. Envelope key change does not touch SPS-65 signature handling: `build_signed_payload_bytes`,
    `compute_sighash`, and `ordered_keys_for_authority` behavior unchanged (existing tests stay green).

### Offline fallback

13. Manual-fallback note: because the commit address is now per-broadcast and non-deterministic, the
    exported preview address is indicative; documented in the protocol spec (no code assertion).

### Frontend

13b. The broadcast preview header reads "Commit TX (preview)"; the address + amount still render. Verified
     via component/snapshot test or the existing broadcast-proposal test, plus `npm run build` / lint / format.

### Regression / removals

14. Delete `derive_commit_reveal_keypair_pinned_xonly_pubkey` and
    `integration_commit_address_pinned_from_mnemonic_and_fixed_payload` (determinism no longer holds).
15. Delete `wallet_session` tests asserting a cached `commit_reveal_keypair`
    (`init_stores_commit_reveal_keypair_matching_derivation`, `commit_reveal_keypair_none_when_slot_empty`,
    `build_session_from_xpub_returns_none_keypair`).
16. `broadcast_env` tests updated: drop `load_broadcast_env_uses_session_commit_reveal_key`; keep/adjust the
    three gate tests to assert `Ok(())`/gating without a returned keypair.
17. `broadcast_commit_uses_commit_funding_abstraction` updated to the new `broadcast_commit_then_reveal`
    signature (no keypair arg; a `change_spk` arg).
18. Mnemonic-login broadcast is otherwise unchanged; full `cargo test --workspace` and frontend CI green.

## Module structure

Single responsibility per file:

- `infrastructure/admin_wallet/ephemeral_envelope_key.rs` — *Generate one per-broadcast ephemeral SPS-50
  envelope keypair.* (Replaces `commit_reveal_key.rs`, which is **deleted** along with its `mod` line.)
- `infrastructure/broadcast_tx.rs` — *Build the commit address and the signed reveal tx from a given
  envelope keypair and an explicit change script.* (No longer decides the change destination.)
- `application/wallet_service.rs` — *Own the BDK wallet lifecycle and expose read/fund/address operations*;
  gains `reveal_change_address` (address provisioning stays with the wallet owner).
- `application/proposals.rs` — *Orchestrate claim → commit → confirm → reveal*; now also owns ephemeral-key
  generation per broadcast and receives the change script as a value.
- `infrastructure/broadcast_env.rs` — *Load broadcast RPC/asm config and enforce signing gates* (no key).
- `application/wallet_session.rs` — *Own session lifecycle* (no envelope key cached).

**Dependency direction:** `application/proposals.rs` (business logic) depends on infrastructure abstractions
(`broadcast_tx` free functions, `generate_ephemeral_envelope_keypair`) and receives the change script as a
plain `ScriptBuf` value resolved by the command layer from `WalletService`. The command layer
(`commands/proposals.rs`) is the only place that wires `WalletService` → change address → application
function, keeping `proposals.rs` value-driven and unit-testable without a live wallet.

**Reuse:** `generate_ephemeral_envelope_keypair` is consumed by both `prepare_broadcast_bundle` and
`broadcast_commit_then_reveal`; `reveal_change_address` reuses the existing `WalletService` mutex + BDK
keychain API (no new wallet instance).

## Open notes for the implementer

- `reveal_next_address(Internal)` requires `&mut wallet`; lock the `WalletService` mutex inside
  `reveal_change_address` (mirror `build_and_sign_tx`'s locking). In the no-persist wallet the revealed
  index is in-memory and re-derived from chain on sync — acceptable for R1.0 (rotation, not persistence,
  is the goal). Both the BDK commit-funding change and the reveal change draw from the internal keychain;
  index interaction is benign (both are Admin Wallet–owned and tracked on sync).
- Use a cryptographically secure RNG for the ephemeral key (`OsRng`), consistent with `secp256k1` keygen.
- Keep `build_reveal_tx` parameter count within the rust-specialist limit; if it exceeds five after adding
  `change_spk` and dropping `network`, group the taproot inputs into a small struct.
