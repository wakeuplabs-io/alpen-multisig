# Evolution: Admin Wallet — Pre-sign Commit + Reveal Before Broadcasting (R1.0.1)

**Date:** 2026-05-30
**PR:** #198
**Branch:** feat/admin-wallet-presign-commit-reveal
**Spec:** docs/specs/admin-wallet-presign-commit-reveal.md
**Builds on:** R1.0 (docs/archive/evolution/2026-05-30-admin-wallet-ephemeral-reveal-key.md)

## Summary

R1.0.1 reorders the desktop broadcast flow so the commit **and** the reveal are both built and
signed **before either is broadcast**, closing the R1.0 crash window. `CommitFunding::fund_commit`
(build+sign+broadcast → txid) was split into `build_signed_commit(...) -> Transaction` (build+sign
only); the application layer became the single owner of broadcast ordering. The reveal is now built
from the **local** signed commit `Transaction`, eliminating the `get_raw_transaction` round-trip and
the dependency on commit confirmation. The ephemeral envelope key is dropped immediately after both
transactions are signed — once signed, the reveal is self-contained and key-independent.

Broadcasting tries `submitpackage` (Bitcoin Core 24+) for atomic commit+reveal admission and falls
back to sequential `sendrawtransaction` (commit then reveal) on older nodes. A session-scoped
in-memory `PendingReveals` store holds the signed reveal hex keyed by `action_id`; the new
`proposals_resubmit_reveal` IPC command re-broadcasts it to recover live-session transient failures
with no ephemeral key. The orchestrator reporting collapsed to
`commit_broadcasted → reveal_broadcasted → reveal_confirmed` (the `commit_confirmed` PATCH was
dropped), and the two regtest `mine_blocks(1)` calls collapsed to one.

## Business Context

Close the R1.0 crash window: in R1.0 the per-broadcast ephemeral key had to survive the
commit→reveal window (commit broadcast first, reveal built only after the commit confirmed). A crash
in that window lost the in-memory key, leaving the commit UTXO (dust + fee) permanently unspendable
and the governance action unrevealed. R1.0.1 removes that dependency entirely via pre-signing +
`submitpackage` atomicity.

## Steps Completed

| Step ID | Name | Status |
|---------|------|--------|
| 01-01 | Add `submit_package` to `BitcoinRpcClient` trait + `HttpBitcoinRpcClient` | PASS |
| 01-02 | Create `PendingReveals` in-memory store module | PASS |
| 02-01 | Rename `CommitFunding::fund_commit` → `build_signed_commit` returning `Transaction` | PASS |
| 02-02 | Rename `WalletService::fund_commit` → `build_signed_commit`, remove broadcast | PASS |
| 03-01 | Add `resubmit_reveal` + `BroadcastError::NoPendingReveal` | PASS |
| 03-02 | Rewrite `broadcast_commit_then_reveal` with pre-sign + `submit_package` | PASS |
| 04-01 | Add `proposals_resubmit_reveal` IPC command, register in invoke handlers | PASS |
| 04-02 | Register `PendingReveals` managed state; clear on `auth_logout` | PASS |

All 8 steps executed and committed on 2026-05-30 (DES integrity: 8/8 complete traces).
Post-DELIVER UI alignment commits (copy + stepper) followed in the same PR — see Files Changed.

## Key Decisions

- **Split build/sign from broadcast** (D1): `build_signed_commit` returns the signed `Transaction`;
  the application layer owns broadcast ordering. Eliminates the second `bitcoincore_rpc::Client`
  that `fund_commit` used purely to broadcast, removing any double-broadcast risk.
- **Pre-sign both, drop key after** (D2): the ephemeral keypair is dropped right after
  `build_reveal_tx` returns — before the first broadcast. This is what actually closes the window.
- **Build reveal from local commit tx** (D3): `build_reveal_tx` needs only the commit
  outpoint/output, so the `get_raw_transaction(commit_txid)` round-trip and the
  wait-for-commit-confirmation step were deleted.
- **`submit_package` + sequential fallback** (D4): atomic on Core 24+; back-compatible via
  `is_unknown_method` detection (`-32601` / "Method not found") falling back to sequential sends.
- **No durable persistence** (D5/D6): the crash window is closed by `submitpackage` atomicity;
  live-session transient failures are covered by the in-memory store + `proposals_resubmit_reveal`.
  A hard process crash on the sequential-fallback path (pre-24 node) between the two sends is an
  accepted, documented limitation.
- **No orchestrator changes** (D9): "reveal signed/persisted" is a purely local desktop concern;
  the broadcast status enum and PATCH contracts are untouched. Dropping `commit_confirmed` is
  contract-compatible (the PATCH enforces no sub-status ordering).
- **Byte-identical on-chain pair** (D10): only operation ordering changed; SPS-50/51/65 envelope
  semantics, payload shape, and signature handling are unchanged.

## Files Changed

**Production files:**
- `desktop-app/src-tauri/src/infrastructure/bitcoin_rpc.rs` — added `submit_package` to the
  `BitcoinRpcClient` trait and `HttpBitcoinRpcClient` impl (`submitpackage` JSON-RPC)
- `desktop-app/src-tauri/src/application/pending_reveals.rs` — **new** module: `PendingReveal`
  struct, `PendingReveals = Arc<Mutex<HashMap<String, PendingReveal>>>`, `new()`
- `desktop-app/src-tauri/src/application/mod.rs` — `pub mod pending_reveals`
- `desktop-app/src-tauri/src/application/commit_funding.rs` — `CommitFunding::build_signed_commit`
  returning `Transaction`; `BdkAdminWalletMnemonic` delegates to `WalletService::build_signed_commit`
- `desktop-app/src-tauri/src/application/wallet_service.rs` — `build_signed_commit` (build+sign,
  broadcast removed); `Auth` import scoped local to `do_sync`
- `desktop-app/src-tauri/src/application/proposals.rs` — `BroadcastError::NoPendingReveal`,
  `resubmit_reveal`, rewritten `broadcast_commit_then_reveal` (pre-sign, `PendingReveals`,
  `submit_package` + fallback, single mine, reordered reporting)
- `desktop-app/src-tauri/src/commands/proposals.rs` — `proposals_broadcast` wired with
  `PendingReveals` state; new `proposals_resubmit_reveal` command + `ResubmitRevealInput`
- `desktop-app/src-tauri/src/commands/authentication.rs` — `auth_logout` clears `PendingReveals`
- `desktop-app/src-tauri/src/commands/invoke.rs` — registered `proposals_resubmit_reveal`
- `desktop-app/src-tauri/src/main.rs` — registered `PendingReveals` as Tauri managed state

**Frontend (UI alignment with the pre-sign flow):**
- `desktop-app/src/domain/broadcast-proposal/components/broadcast-phase-progress.tsx` — reframed
  Commit/Reveal step copy to describe roles (not sequence); Commit + Reveal steps now light up
  together during broadcast (one package), Enactment follows
- `desktop-app/src/domain/broadcast-proposal/components/broadcast-details-card.tsx` — Reveal TX
  preview copy updated: signed locally and broadcast in the same package as the commit
- `desktop-app/src/domain/broadcast-proposal/model/broadcast-proposal.ts` — removed dead
  `broadcastStatusToPhase` / `broadcastStatusLabel` whose copy described the old R1.0 sequential flow

**Test files:**
- Tests added/updated in: `bitcoin_rpc.rs`, `pending_reveals.rs`, `commit_funding.rs`,
  `wallet_service.rs`, `proposals.rs` (5 broadcast-flow tests), `commands/proposals.rs`,
  `commands/authentication.rs`

## Known Limitations (R1.0.1)

- **No durable cross-process persistence of the signed reveal.** A hard process crash on the
  **sequential-fallback** path (pre-24 node) between the commit and reveal sends is an accepted,
  documented limitation. On `submitpackage`-capable nodes a crash leaves nothing on-chain (clean
  retry). Durable orchestrator-stored persistence is a possible future hardening.
- **R1.1 not included**: commit funding is still software-signed by the mnemonic session
  (`BdkAdminWalletMnemonic` → `WalletService::build_signed_commit`). Watch-only and hardware-wallet
  sessions remain `ReadOnly` for broadcast.
- **RBF of the commit (Phase 5) out of scope**: a commit fee-bump would require re-deriving the
  ephemeral key to rebuild the reveal.
- **Stepper Enactment state (cosmetic)**: in the terminal `reveal_confirmed` state the Enactment
  step is shown as done even though enactment is still pending until `proposalStatus === 'enacted'`.
  Pre-existing; not addressed in R1.0.1.

## Lessons Learned

- The four-phase decomposition (infrastructure primitives → CommitFunding/WalletService refactor →
  application logic → IPC/state wiring) sequenced dependencies cleanly: 01-01/01-02 were
  independent, 02-02 depended on 02-01 (delegation), and 03-02 fanned in from all four prior steps.
- Pre-signing both transactions made the `get_raw_transaction` round-trip and the
  commit-confirmation wait obsolete in one move — the round-trip existed only because the reveal
  was historically built after the commit landed.
- UI copy carried hidden coupling to the old sequential model in three places (stepper detail,
  stepper step activation, preview card) plus two dead helper functions. Reviewing the rendered
  screens after the backend rewrite surfaced these; backend-only diffs would have missed them.
- Two commits during DELIVER touched files outside their declared step scope (`commands/proposals.rs`,
  `main.rs` during 03-02); the orchestrator verified `cargo build`/`test`/`clippy` green before
  proceeding. For state-threading rewrites, allowing the wiring to land with the logic change avoids
  an intermediate non-compiling state.
