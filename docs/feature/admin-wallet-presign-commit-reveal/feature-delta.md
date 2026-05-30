## Wave: DESIGN / [REF] Design Decisions (DDD)

| ID | Decision | Verdict | One-line rationale |
|---|---|---|---|
| D1 | Replace `CommitFunding::fund_commit` (build+sign+broadcast → txid) with `build_signed_commit(...) -> Transaction` (build+sign only) | ✅ | Single broadcast owner in the application layer; no double-broadcast; drops the second `bitcoincore_rpc::Client`. |
| D2 | Build + sign **both** commit and reveal before broadcasting either; drop the ephemeral key right after signing | ✅ | The signed reveal is key-independent; dropping the key after signing is what closes the R1.0 crash window. |
| D3 | Build the reveal from the **local** signed commit `Transaction`; delete the `get_raw_transaction(commit_txid)` round-trip | ✅ | `build_reveal_tx` needs only the commit outpoint/output; no confirmation dependency. |
| D4 | Add `BitcoinRpcClient::submit_package`; try it first, fall back to sequential `send_raw(commit)`→`send_raw(reveal)` on unknown-method | ✅ | Atomic on Core 24+; back-compatible with older nodes; both txs pre-signed so sequential is safe. |
| D5 | **No durable persistence** of the signed reveal; keep it in a session-scoped in-memory store keyed by `action_id` | ✅ | `submitpackage` atomicity closes the window; in-memory store + resubmit IPC covers live-session transient failures with no new storage/dependency. |
| D6 | Recovery via `proposals_resubmit_reveal(action_id)` IPC command (no startup scan) | ✅ | Re-broadcasts the in-memory signed reveal; explicit, minimal; no persistence layer to introduce. |
| D7 | Collapse the two regtest `mine_blocks(1)` calls into one | ✅ | Both txs are in the mempool together; one block confirms parent (commit) + child (reveal). |
| D8 | Report `commit_broadcasted`→`reveal_broadcasted`→`reveal_confirmed`; drop the `commit_confirmed` PATCH | ✅ | PATCH enforces no sub-status ordering; both confirm together, so a separate commit-confirm step is redundant. |
| D9 | **No** new orchestrator `BroadcastStatus` variant | ✅ | "Reveal signed/persisted" is a local desktop concern; avoids a protocol/DB migration. |
| D10 | On-chain commit/reveal pair byte-identical to R1.0 for the same inputs | ✅ | R1.0.1 changes operation *ordering* only; SPS-50/51/65 semantics untouched. |

## Wave: DESIGN / [REF] Component Decomposition

| Component | Path | Change type | Single responsibility |
|---|---|---|---|
| `WalletService::build_signed_commit` | `desktop-app/src-tauri/src/application/wallet_service.rs` | MODIFY (rename `fund_commit`) | Build + sign the commit tx (no broadcast). |
| `CommitFunding` trait + `BdkAdminWalletMnemonic` | `desktop-app/src-tauri/src/application/commit_funding.rs` | MODIFY | Produce a signed commit `Transaction` from the Admin Wallet. |
| `BitcoinRpcClient::submit_package` | `desktop-app/src-tauri/src/infrastructure/bitcoin_rpc.rs` | EXTEND | `submitpackage` RPC for atomic commit+reveal. |
| `PendingReveals` store | `desktop-app/src-tauri/src/application/pending_reveals.rs` | CREATE NEW | Hold in-memory signed reveals keyed by `action_id` for resubmit. |
| `broadcast_commit_then_reveal` + `resubmit_reveal` | `desktop-app/src-tauri/src/application/proposals.rs` | MODIFY | Orchestrate pre-sign → broadcast (package/sequential) → confirm; resubmit. |
| `proposals_broadcast` / `proposals_resubmit_reveal` | `desktop-app/src-tauri/src/commands/proposals.rs` | MODIFY + CREATE NEW | IPC wiring; register resubmit command. |
| Managed-state registration / logout teardown | `desktop-app/src-tauri/src/main.rs` (+ `commands/authentication.rs`) | MODIFY | Register and clear `PendingReveals` per session. |

## Wave: DESIGN / [REF] Driving Ports

| Port | Kind | Direction | Owner |
|---|---|---|---|
| `proposals_broadcast` | Tauri IPC command (existing) | TS → Rust | `commands/proposals.rs` |
| `proposals_resubmit_reveal` | Tauri IPC command (**new**) | TS → Rust | `commands/proposals.rs` |

## Wave: DESIGN / [REF] Driven Ports + Adapters

| Driven port | Adapter | Side-effect |
|---|---|---|
| `BitcoinRpcClient::submit_package` | `HttpBitcoinRpcClient` (`submitpackage` JSON-RPC) | Atomic mempool admission of commit+reveal. |
| `BitcoinRpcClient::send_raw_transaction` | `HttpBitcoinRpcClient` (`sendrawtransaction`) | Sequential fallback + resubmit. |
| `BitcoinRpcClient::mine_blocks` / `get_transaction_confirmations` | `HttpBitcoinRpcClient` | Regtest confirm (single mine). |
| `CommitFunding::build_signed_commit` | `BdkAdminWalletMnemonic` → `WalletService` (BDK) | Build + sign commit (no broadcast). |
| `OrchestratorClient::claim_broadcast` / `report_broadcast_progress` | HTTP orchestrator client | Claim + status PATCH (contract unchanged). |

## Wave: DESIGN / [REF] Technology Choices

- **Language/runtime:** Rust (Tauri 2 backend) — unchanged; `bdk_wallet` 1.x, `bitcoin` crate
  (workspace-pinned), `tokio`, `async-trait`.
- **Atomic broadcast:** Bitcoin Core `submitpackage` (Core 24+) with sequential `sendrawtransaction`
  fallback. No new crate — reuses the existing generic JSON-RPC `call` helper.
- **Persistence:** none added — in-memory `std::collections::HashMap` behind a `Mutex` in Tauri managed
  state. No SQLite, no `tauri-plugin-store`, no orchestrator schema change.

## Wave: DESIGN / [REF] Decisions Table

| Decision | Locked |
|---|---|
| DDD-1 Replace `fund_commit` → `build_signed_commit` | ✅ |
| DDD-2 Pre-sign both; drop ephemeral key after signing | ✅ |
| DDD-3 Build reveal from local commit tx; delete round-trip | ✅ |
| DDD-4 `submit_package` + sequential fallback | ✅ |
| DDD-5 No durable persistence; in-memory store | ✅ |
| DDD-6 Resubmit via IPC command | ✅ |
| DDD-7 Single regtest mine | ✅ |
| DDD-8 Drop `commit_confirmed` PATCH | ✅ |
| DDD-9 No new orchestrator status | ✅ |

## Wave: DESIGN / [REF] Reuse Analysis

| Existing Component | File | Overlap | Decision | Justification |
|---|---|---|---|---|
| `WalletService::fund_commit` | `application/wallet_service.rs` | Build+sign commit | EXTEND (split) | Reuse build/sign; remove only the internal broadcast — ~5 LOC delta vs a new builder. |
| `CommitFunding` trait | `application/commit_funding.rs` | Funding seam | EXTEND | Rename/retype one method; single implementor unchanged structurally. |
| `HttpBitcoinRpcClient::call` | `infrastructure/bitcoin_rpc.rs` | JSON-RPC plumbing | EXTEND | `submit_package` is one more `call("submitpackage", …)` — no new client. |
| `build_reveal_tx` / `derive_commit_address` | `infrastructure/broadcast_tx.rs` | Reveal/commit build | EXTEND (reuse as-is) | R1.0 already accepts `&commit_tx` + `change_spk`; only the call site moves earlier. |
| `broadcast_commit_then_reveal` | `application/proposals.rs` | Broadcast orchestration | EXTEND | Reorder steps within the existing function; no new orchestrator. |
| In-memory signed-reveal store | `application/pending_reveals.rs` | Recoverable artifact holding | CREATE NEW | No existing store holds a per-action signed tx; a 1-responsibility `HashMap` wrapper is the minimal fit (no persistence layer exists to extend). |

## Wave: DESIGN / [REF] Open Questions

- **Package feerate floor:** confirm on regtest that `COMMIT_DUST_SATS + reveal_fee` clears the node's
  minimum package feerate under `submitpackage` (CPFP). If not, lift the commit amount — resolve in
  DELIVER, do not touch the reveal fee math. (Deferred to implementation/verification.)
- **`submitpackage` result schema:** pin the exact success-detection (`package_msg == "success"` + no
  per-tx error) and the unknown-method marker (`-32601` / "Method not found") against the regtest
  Core version in CI — verify in DELIVER.
- **Logout/teardown timing:** confirm clearing `PendingReveals` on `auth_logout` does not race an
  in-flight resubmit — DISTILL acceptance test should cover the concurrent case.

## Wave: DESIGN / [REF] Reconciliation with Prior Waves

Back-propagated change to `docs/specs/admin-wallet-implementation-plan.md` §R1.0.1 done-when:

- **Original (plan §R1.0.1):** *"killing the app between the commit and reveal broadcast leaves a
  recoverable signed reveal (resubmittable without the ephemeral key)."* This wording mandates durable
  cross-process persistence.
- **New assumption (DDD-5/DDD-6):** the crash window is closed by `submitpackage` **atomicity** (nothing
  lands on-chain unless both txs are accepted together). Live-session transient failures are recovered via
  an in-memory signed-reveal store + `proposals_resubmit_reveal`. A **hard process crash on the
  sequential-fallback path** (pre-24 node) between the two sends is an accepted, documented limitation.
- **Rationale:** durable persistence (local file or orchestrator field) was rejected by the design owner
  as disproportionate given atomicity already closes the window on modern nodes; it would add storage and
  couple recovery to orchestrator availability for a residual edge case on legacy nodes.
- **Propagation:** plan §R1.0.1 done-when updated; protocol spec `proposal-broadcast-commit-reveal.md`
  updated to describe pre-sign ordering and the resubmit path. No upstream user-story/acceptance-criteria
  change (no DISCUSS artifacts exist for this lean step).
