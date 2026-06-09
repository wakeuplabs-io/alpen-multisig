# Spec: Admin Wallet — Pre-sign Commit + Reveal Before Broadcasting (R1.0.1)

> Release 1, step R1.0.1 of the [Admin Wallet implementation plan](./admin-wallet-implementation-plan.md).
> Builds directly on R1.0 ([`admin-wallet-ephemeral-reveal-key.md`](./admin-wallet-ephemeral-reveal-key.md)).
> Protocol context: [`proposal-broadcast-commit-reveal.md`](./proposal-broadcast-commit-reveal.md).

## Objective

Reorder the desktop broadcast flow so the **commit and the reveal are both built and signed before
either is broadcast**, then broadcast them commit→reveal — atomically via `submitpackage` when the node
supports it, otherwise sequentially. This closes the R1.0 crash window.

In R1.0 the per-broadcast **ephemeral envelope key** must survive across the commit→reveal window: the
commit is broadcast first, and the reveal is built/signed only *after* the commit confirms (via a
`getrawtransaction` round-trip). A crash in that window loses the in-memory ephemeral key, leaving the
commit UTXO (dust + fee) permanently unspendable for the reveal and the governance action unrevealed.

R1.0.1 removes that dependency: once **both** transactions are signed, the ephemeral key is no longer
needed and is dropped. The signed reveal is self-contained (it carries its own signature, not the key),
so it can be re-broadcast without the ephemeral key. Building the reveal from the **locally signed commit
`Transaction`** also eliminates the `getrawtransaction` round-trip — there is no need to wait for the
commit to confirm before constructing the reveal.

## Scope

### Included

- **CommitFunding: separate build/sign from broadcast.** Replace `CommitFunding::fund_commit` (which
  builds, signs, *and* broadcasts internally, returning a txid) with `build_signed_commit(...) ->
  Transaction` (build + sign only, **no** broadcast). The application layer becomes the single owner of
  broadcast ordering and broadcasts via the `BitcoinRpcClient` trait. Eliminates the second
  `bitcoincore_rpc::Client` that `fund_commit` used purely to broadcast, and removes any double-broadcast
  risk.
- **Reorder `broadcast_commit_then_reveal`:** claim → build+sign commit → build+sign reveal (from the
  local commit `Transaction`) → drop the ephemeral key → broadcast commit→reveal → confirm. The Step-3
  `btc_rpc.get_raw_transaction(&commit_txid)` round-trip is **deleted**.
- **Atomic broadcast with fallback.** Add `BitcoinRpcClient::submit_package` (Bitcoin Core 24+
  `submitpackage`). The flow tries `submit_package([commit_hex, reveal_hex])` first and **falls back** to
  sequential `send_raw_transaction(commit)` then `send_raw_transaction(reveal)` when the node reports the
  method as unknown.
- **In-memory pending-reveal store + resubmit IPC.** Hold the fully-signed reveal hex in a session-scoped
  in-memory map keyed by `action_id`. Add an IPC command `proposals_resubmit_reveal(action_id)` that
  re-broadcasts the stored reveal. This recovers transient broadcast failures (e.g. an RPC blip on the
  sequential path) **within a live session** without the ephemeral key.
- **Regtest mining choreography:** collapse the two `mine_blocks(1)` calls into a single mine — once both
  txs are in the mempool, one block confirms parent (commit) and child (reveal) together.
- **Orchestrator reporting:** report `commit_broadcasted` (with `commit_txid`) then `reveal_broadcasted`
  (with `reveal_txid`) back-to-back after the broadcast call, then `reveal_confirmed` after confirmation.
  The intermediate `commit_confirmed` PATCH is dropped (the PATCH contract does not enforce sub-status
  ordering; both txs confirm together).
- Update [`proposal-broadcast-commit-reveal.md`](./proposal-broadcast-commit-reveal.md) to document the
  pre-sign ordering, the `submitpackage`/sequential broadcast, and the resubmit recovery path.

### Not included

- **Durable cross-process persistence of the signed reveal.** R1.0.1 deliberately does **not** persist the
  reveal to disk or to the orchestrator. The window is closed by `submitpackage` **atomicity** (nothing
  lands on-chain unless both txs are accepted together), and live-session transient failures are covered by
  the in-memory store + resubmit IPC. A hard process crash on the **sequential-fallback** path (a pre-24
  node) between the commit and reveal sends is an accepted, documented limitation — see *Risks*. (This
  relaxes the original plan §R1.0.1 done-when; the plan is updated accordingly — back-propagation below.)
- **R1.1** — hardware-wallet (PSBT) signing of the commit-funding tx. R1.0.1 keeps the mnemonic
  software-signer (`BdkAdminWalletMnemonic` → `WalletService`); watch-only/HW sessions stay `ReadOnly` for
  broadcast.
- **RBF of the commit (Phase 5).** A commit fee-bump would require re-deriving/persisting the ephemeral key
  to rebuild the reveal; out of scope.
- Any change to SPS-50/51/65 envelope semantics, payload shape, OP_RETURN tagging, or signature handling.
  **Only the operation ordering changes** — the on-chain commit/reveal pair is byte-identical to R1.0.
- Receive-address rotation as a user feature (R1.3).

## Technical Design

### Current state (R1.0 — to be reordered)

```
claim ──► fund_commit(commit_addr, amount, fee)            [WalletService: build+sign+BROADCAST commit]
            └─ returns commit_txid
       ──► report commit_broadcasted
       ──► mine_blocks(1) (regtest) ──► wait commit confirm ──► report commit_confirmed
       ──► get_raw_transaction(commit_txid)                 [ROUND-TRIP — ephemeral key still alive here]
       ──► build_reveal_tx(ephemeral, …, &commit_tx, change_spk, fee)   [build+sign reveal]
       ──► send_raw_transaction(reveal_hex) ──► report reveal_broadcasted
       ──► mine_blocks(1) (regtest) ──► wait reveal confirm ──► report reveal_confirmed
                                  ▲
            crash anywhere above this point loses the ephemeral key ⇒ reveal can never be built
```

### Target state (R1.0.1)

```
claim
  └─ build_signed_payload_bytes + generate_ephemeral_envelope_keypair + derive_commit_address   (as R1.0)
  └─ estimate fee → commit_amount = COMMIT_DUST_SATS + reveal_fee
  ├─ commit_tx  = commit_funding.build_signed_commit(commit_addr, amount, fee)   [build+sign, NO broadcast]
  ├─ reveal_tx  = build_reveal_tx(ephemeral, …, &commit_tx, change_spk, fee)     [build+sign from LOCAL tx]
  ├─ DROP the ephemeral keypair                              ◄── both txs signed; key no longer needed
  ├─ pending_reveals.insert(action_id, reveal_hex)           ◄── in-memory, for resubmit
  ├─ broadcast: submit_package([commit_hex, reveal_hex])  ──or── send_raw(commit); send_raw(reveal)
  ├─ report commit_broadcasted(commit_txid) ; report reveal_broadcasted(reveal_txid)
  ├─ mine_blocks(1) (regtest, once)  ──► wait reveal confirm
  └─ report reveal_confirmed ; pending_reveals.remove(action_id)
```

### Production functions

| Function | Module | Responsibility |
|---|---|---|
| `WalletService::build_signed_commit(&self, commit_address, amount_sats, fee_rate) -> Result<Transaction, AdminWalletError>` | `application/wallet_service.rs` | Steps 0–3 of today's `fund_commit` (ReadOnly guard, `check_enabled`, sync, lock+`build_and_sign_tx`) returning the signed commit `Transaction`. **No** RPC broadcast — the `bitcoincore_rpc::Client` send is removed. Replaces `fund_commit`. |
| `CommitFunding::build_signed_commit(&self, …) -> Result<Transaction, CommitFundingError>` | `application/commit_funding.rs` | Trait method renamed/retyped from `fund_commit`; `BdkAdminWalletMnemonic` delegates to `WalletService::build_signed_commit`. |
| `BitcoinRpcClient::submit_package(&self, tx_hexes: &[String]) -> Result<(), String>` | `infrastructure/bitcoin_rpc.rs` | Call `submitpackage` (Core 24+) with `[[commit_hex, reveal_hex]]`; map a non-`success` package result to `Err`. An unknown-method error (`-32601` / "Method not found") is surfaced verbatim so the caller can detect it and fall back. |
| `proposals::broadcast_commit_then_reveal(…)` | `application/proposals.rs` | Reordered: build+sign commit and reveal before broadcasting; drop the ephemeral key after signing; store the signed reveal; broadcast via package-or-sequential; single regtest mine; reordered reporting. Takes the `PendingReveals` store. |
| `proposals::resubmit_reveal(pending, btc_rpc, client, action_id) -> Result<String, BroadcastError>` | `application/proposals.rs` | Re-broadcast the stored signed reveal hex for `action_id` via `send_raw_transaction`; re-report `reveal_broadcasted`. Typed error when no pending reveal exists for `action_id`. |

`build_reveal_tx`, `derive_commit_address`, `generate_ephemeral_envelope_keypair`, and
`WalletService::reveal_change_address` are **unchanged** from R1.0 — only their *call site / ordering*
changes. `broadcast_commit_then_reveal` keeps its R1.0 parameter list (it already receives
`reveal_change_spk: ScriptBuf`) plus a `&PendingReveals` argument.

### Broadcast strategy (capability detection)

```rust
match btc_rpc.submit_package(&[commit_hex.clone(), reveal_hex.clone()]).await {
    Ok(()) => { /* both accepted atomically */ }
    Err(e) if is_unknown_method(&e) => {
        btc_rpc.send_raw_transaction(&commit_hex).await?;   // parent first
        btc_rpc.send_raw_transaction(&reveal_hex).await?;   // child references parent in mempool
    }
    Err(e) => return Err(BroadcastError::BitcoinRpc(e)),
}
```

`is_unknown_method(e)` matches `-32601` or "Method not found" in the RPC error string. Both txs are
already fully signed, so sequential submission is safe; the child (reveal) sits in the mempool referencing
the parent (commit). On regtest a single `mine_blocks(1)` then confirms both (a block may contain parent
then child).

### In-memory pending-reveal store

`PendingReveals` is session-scoped managed state — `Arc<Mutex<HashMap<String, PendingReveal>>>`, key =
`action_id`, value = `{ reveal_tx_hex, reveal_txid, commit_txid }`. Lifecycle:

- **Insert** the signed reveal **before** broadcasting the commit.
- **Remove** on `reveal_confirmed` (terminal success).
- **Retain** on broadcast failure so `proposals_resubmit_reveal` can retry within the session.
- **Cleared** on `auth_logout` alongside the `WalletService` (mirrors the R1.0/3.7 session teardown).

It holds **no key material** — only a fully-signed transaction — so it carries no custody risk and never
crosses the IPC boundary in raw form.

### IPC commands (`commands/proposals.rs`)

- `proposals_broadcast` — unchanged React signature. Resolves `reveal_change_spk` from
  `WalletService::reveal_change_address()` (R1.0) and now also passes the `PendingReveals` managed state
  into `broadcast_commit_then_reveal`. The commit funder is constructed as today (`BdkAdminWalletMnemonic`)
  but its method is `build_signed_commit`.
- `proposals_resubmit_reveal(action_id: String) -> Result<ResubmitDto, String>` — **new**. Re-broadcasts
  the in-memory signed reveal for `action_id`; returns the reveal txid. Returns a typed error
  (`no pending reveal for action`) when the store has no entry (e.g. after a process restart) so the UI can
  tell the user to re-run the broadcast.

### Orchestrator (`orchestrator-be`)

**No change.** The broadcast status enum
(`idle|commit_broadcasted|commit_confirmed|reveal_broadcasted|reveal_confirmed|failed`) and the
claim/PATCH contracts are untouched. "Reveal signed/persisted" is a purely **local** desktop concern and
is intentionally **not** modeled as an orchestrator status. The PATCH endpoint does not enforce sub-status
ordering, so dropping the `commit_confirmed` report is contract-compatible.

### Production code vs. test helpers

- **Production:** `WalletService::build_signed_commit`, `CommitFunding::build_signed_commit`,
  `BitcoinRpcClient::submit_package`, the reordered `broadcast_commit_then_reveal`, `resubmit_reveal`, the
  `PendingReveals` store, and the `proposals_resubmit_reveal` IPC command.
- **Test helpers** (stay in `#[cfg(test)]`, never exposed as commands): `SpyCommitFunding` (now returning a
  `Transaction`), `MockBtcRpc` (now implementing `submit_package`), deterministic keypair/tx builders,
  session/RPC env setters.
- No test helper is registered as a Tauri command or exported in a production API.

## Test Cases

Tests target production functions only.

### Happy path

1. `WalletService::build_signed_commit` returns a **signed** commit `Transaction` and makes **no** RPC
   broadcast call (assert via a spying RPC that `send_raw_transaction` is never invoked by it).
2. `broadcast_commit_then_reveal` builds the reveal from the **local** signed commit — `get_raw_transaction`
   is **never** called (spy asserts zero invocations), proving the round-trip is gone.
3. When `submit_package` succeeds, the flow uses it and does **not** call `send_raw_transaction` for the
   commit/reveal (package path).
4. When `submit_package` returns an unknown-method error, the flow **falls back** to two
   `send_raw_transaction` calls in order commit→reveal (sequential path).
5. The signed reveal is inserted into `PendingReveals` **before** the commit is broadcast, and **removed**
   after `reveal_confirmed`.
6. Orchestrator reporting order is `commit_broadcasted` (commit_txid) → `reveal_broadcasted` (reveal_txid)
   → `reveal_confirmed`; no `commit_confirmed` PATCH is sent.
7. Regtest integration / e2e-webdriver: an approved proposal broadcasts; commit and reveal confirm in a
   single mined block; the reveal change UTXO is owned by the Admin Wallet (unchanged from R1.0).

### Recovery

8. `resubmit_reveal` re-broadcasts the stored reveal hex for a known `action_id` and returns the reveal
   txid (simulates a transient `send_raw_transaction` failure on the sequential path, then a successful
   retry within the same session).
9. `resubmit_reveal` returns a typed "no pending reveal" error for an `action_id` absent from the store
   (simulates a fresh process / empty in-memory map).

### Edge cases

10. Watch-only / no-session / dev-signing-disabled gates still fire (R1.0 behavior preserved) **before**
    any build or broadcast — `load_broadcast_env` gating and the `build_signed_commit` `ReadOnly`/
    `check_enabled` guards both hold.
11. A failure during build/sign of the commit or reveal (before broadcast) reports `failed` to the
    orchestrator and broadcasts **nothing** on-chain.

### Authority isolation

12. The reorder does not touch SPS-65 signature handling: `build_signed_payload_bytes`, `compute_sighash`,
    and `ordered_keys_for_authority` behavior unchanged (existing tests stay green). The on-chain
    commit/reveal pair is byte-identical to R1.0 for the same inputs.

### Offline fallback

13. Manual-fallback bundle (commit/reveal hex export) is still derivable; documented in the protocol spec.

### Regression / updates

14. `broadcast_commit_uses_commit_funding_abstraction` updated: `SpyCommitFunding` returns a `Transaction`
    whose output script matches the derived commit address (so `build_reveal_tx` finds the commit vout);
    `MockBtcRpc` gains `submit_package`; the assertion that `get_raw_transaction` is unused is added.
15. `WalletService` `fund_commit` tests become `build_signed_commit` tests (assert a signed `Transaction`
    is returned, no broadcast).
16. `commit_funding.rs` trait/impl compiles against the renamed method; the pointer-equality construction
    test is unchanged.
17. Mnemonic-login broadcast is otherwise unchanged; full `cargo test --workspace` and frontend CI green.

## Module structure

Single responsibility per file:

- `application/wallet_service.rs` — *Own the BDK wallet lifecycle and expose read/build/address operations*;
  `fund_commit` becomes `build_signed_commit` (build + sign only; broadcast moves out).
- `application/commit_funding.rs` — *Abstraction seam that produces a signed commit `Transaction` from the
  Admin Wallet* (no broadcast). Single implementor `BdkAdminWalletMnemonic`.
- `infrastructure/bitcoin_rpc.rs` — *Bitcoin Core–compatible RPC client*; gains `submit_package`
  (`submitpackage`) alongside the existing `send_raw_transaction` used for the fallback and resubmit.
- `application/pending_reveals.rs` (new) — *Hold in-memory, session-scoped signed reveals keyed by
  `action_id` for resubmit.* No key material; no persistence.
- `application/proposals.rs` — *Orchestrate claim → build+sign commit+reveal → drop key → broadcast
  (package|sequential) → confirm*; owns the ephemeral-key lifecycle (dropped after signing) and writes the
  signed reveal into the `PendingReveals` store; gains `resubmit_reveal`.
- `commands/proposals.rs` — *IPC wiring*: injects the funder, the `PendingReveals` state, and the change
  script; registers `proposals_resubmit_reveal`.
- `main.rs` — registers `PendingReveals` managed state; `auth_logout` clears it.

**Dependency direction:** `application/proposals.rs` (business logic) depends on infrastructure abstractions
(`CommitFunding`, `BitcoinRpcClient`, `broadcast_tx` free functions) and on the value-typed `PendingReveals`
store. The command layer (`commands/proposals.rs`) is the only place that wires `WalletService` → change
address + funder + managed state into the application function, keeping `proposals.rs` value-driven and
unit-testable without a live wallet or node.

**Reuse:** `build_reveal_tx` / `derive_commit_address` / `generate_ephemeral_envelope_keypair` /
`reveal_change_address` are reused **unchanged** from R1.0; `send_raw_transaction` is reused for both the
sequential fallback and `resubmit_reveal`.

## Open notes for the implementer

- **Transaction type alignment:** `WalletService::build_signed_commit` returns
  `bdk_wallet::bitcoin::Transaction`; `broadcast_tx::build_reveal_tx` takes `&bitcoin::Transaction`. These
  are the same workspace-pinned `bitcoin` crate, so pass the value directly. If a version skew is ever
  introduced, convert via consensus encode/decode (do not silently `transmute`).
- **`submitpackage` result parsing:** the result is an object (`package_msg` + per-`wtxid` `tx-results`),
  **not** a bare txid string. Treat `package_msg == "success"` (and no per-tx error) as `Ok(())`; surface
  any other shape as `Err` with the node message. Keep the unknown-method string detectable for fallback.
- **Package fee policy:** the commit is funded at `COMMIT_DUST_SATS + reveal_fee` (`fee_constants.rs`:
  `COMMIT_DUST_SATS = 1500`, `REVEAL_TX_VBYTES = 350`). Under `submitpackage`, the node evaluates package
  feerate (CPFP). Verify on regtest that the combined package clears the node's minimum package feerate;
  if not, lift the commit amount slightly — do not change the reveal fee math (protocol-adjacent).
- **Claim ordering:** keep `claim` first (cheap lock that 409s on a concurrent broadcaster) before
  build+sign, so wasted signing work is avoided when another client already claimed.
- **Ephemeral key drop:** explicitly `drop(envelope_keypair)` (or let it fall out of scope) immediately
  after `build_reveal_tx` returns, so the key is gone before the first broadcast — this is what actually
  closes the R1.0 window.
- **Resubmit scope:** `proposals_resubmit_reveal` recovers a live-session transient failure only; after a
  process restart the in-memory store is empty and the command returns the typed "no pending reveal"
  error. With `submitpackage` atomicity a restart implies nothing landed on-chain, so re-running the full
  broadcast is safe.
