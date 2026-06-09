# Spec: Unblock broadcast UI — decouple submit from confirmation

## Objective

After `submit_package` succeeds (seconds), the broadcast flow currently keeps blocking for
up to `confirm_timeout_ms` (~10 min) while polling for **one** reveal confirmation. During
that window:

- The UI is stuck on "Broadcasting…" with the stepper frozen on Commit/Reveal.
- If no block is mined before the timeout, `broadcast_commit_then_reveal` returns
  `Err(Timeout)` and the error path reports the proposal as **`failed`** — even though the
  reveal is still in the mempool and may confirm later. This is a **false failure** with no
  auto-reconcile.

We want to **split "submitted" from "confirmed"**:

- The submit call returns within seconds once both txs are broadcast (`reveal_broadcasted`).
- Confirmation is awaited in the **background**; the user can leave the screen.
- A confirmation timeout with 0 confirmations **keeps** the proposal at `reveal_broadcasted`
  (mempool-pending) — it is **never** marked `failed` on slow blocks alone.
- `failed` is reserved for **real submission errors** (node rejection, build failure, etc.).
- Late confirmations still reach `reveal_confirmed` via the background task (and an on-open
  reconcile path), promoting the orchestrator record.

## Scope

### Included

- **Backend (Tauri `desktop-app/src-tauri`)**: split the orchestrator-backed broadcast
  (`broadcast_commit_then_reveal`) into two production functions — `submit_commit_then_reveal`
  and `await_reveal_confirmation` — and make the `proposals_broadcast` command return after
  submit while a spawned task awaits confirmation.
- **Frontend (`desktop-app/src`)**: new `awaiting-confirmation` phase; the hook unblocks on
  submit, shows txids, polls for `reveal_confirmed`, and reconciles a `reveal_broadcasted`
  proposal on screen open.
- **Progress UI**: render Commit/Reveal as ✓ and step 3 as "Awaiting block" during
  `awaiting-confirmation`, with copyable txids.

### NOT included

- **Manual broadcast** (`broadcast_manual` / `proposals_broadcast_manual`). That path has no
  orchestrator reporting, so it never produces a false `failed` orchestrator state. Making it
  non-blocking is a separate follow-up and is out of scope here.
- No new `BroadcastStatus` variant — `reveal_broadcasted` already models "submitted, awaiting
  confirmation". `awaiting-confirmation` is a **frontend phase only**.
- No change to fee estimation, commit/reveal construction, claim/coordination, or the
  `resubmit_reveal` / `resolveBroadcastStatus` commands (reused as-is for reconcile).

## Technical Design

### Backend — `desktop-app/src-tauri/src/application/proposals.rs`

Introduce a confirmation outcome type and split the blocking function:

```rust
/// Outcome of awaiting the reveal confirmation.
#[derive(Debug, PartialEq, Eq)]
pub enum ConfirmOutcome {
    /// Reveal reached >= 1 confirmation; orchestrator reported `reveal_confirmed`.
    Confirmed,
    /// Timed out with 0 confirmations; reveal remains in mempool (`reveal_broadcasted`).
    /// NOT a failure — no `failed` report is sent.
    PendingConfirmation,
}
```

**`submit_commit_then_reveal(...) -> Result<(String, String), BroadcastError>`**
Single responsibility: claim, build, broadcast, and report up to `reveal_broadcasted`; return
`(commit_txid, reveal_txid)`. Contains everything currently in
`broadcast_commit_then_reveal` **except** the `wait_for_confirmation` + `reveal_confirmed`
report + pending removal. On any error within this stage (real submission error), it reports
`failed` to the orchestrator (unchanged behavior for genuine errors).

**`await_reveal_confirmation(...) -> Result<ConfirmOutcome, BroadcastError>`**
Single responsibility: poll `get_transaction_confirmations(reveal_txid)`.
- On `>= 1` conf → report `reveal_confirmed`, remove the `PendingReveals` entry, return
  `Confirmed`.
- On timeout with `0` conf → return `PendingConfirmation`. **Does NOT** report `failed`,
  **keeps** the `PendingReveals` entry (so resubmit/reconcile remain possible), and leaves the
  last reported status at `reveal_broadcasted`.
- A genuine RPC error while polling propagates as `Err(BroadcastError::BitcoinRpc(_))` and is
  **not** reported as `failed` (the tx was already broadcast; the caller logs and lets the
  on-open reconcile path recover).

**`broadcast_commit_then_reveal(...)`** is retained as a thin sequential wrapper
(`submit_commit_then_reveal` then `await_reveal_confirmation`) so existing unit tests and any
synchronous callers keep working, but it **no longer reports `failed` on confirmation
timeout**.

### Backend — `desktop-app/src-tauri/src/commands/proposals.rs`

`proposals_broadcast` changes:

1. Build `client` (wrapped in `Arc`), `btc_rpc` (already `Arc`), funding, change spk.
2. Call `submit_commit_then_reveal(...)` → `(commit_txid, reveal_txid)`.
3. **Spawn** a background task (`tauri::async_runtime::spawn`) that owns clones of the `Arc`
   client, `Arc` btc_rpc, and the `PendingReveals` `Arc`, and runs `await_reveal_confirmation`.
   The task logs via `tracing`; it never panics the command.
4. Return `BroadcastResultDto` immediately with `broadcast_status: "reveal_broadcasted"` and
   the txids — the command resolves in seconds.

The spawned future must be `Send`; `HttpOrchestratorClient`/`HttpBitcoinRpcClient` (reqwest)
and `PendingReveals` (`Arc<Mutex<…>>`) satisfy this. `Arc::clone` the three before `move`.

### Frontend — `desktop-app/src/domain/broadcast-proposal/model/broadcast-proposal.ts`

Add the phase:

```ts
export type BroadcastPhase =
  | 'idle' | 'preparing' | 'confirming' | 'awaiting-device'
  | 'broadcasting' | 'awaiting-confirmation' | 'done' | 'error'
```

### Frontend — `desktop-app/src/domain/broadcast-proposal/hooks/use-broadcast-proposal.ts`

- After `broadcastProposal(...)` resolves OK, set the proposal/result from the refreshed row
  and set phase to **`awaiting-confirmation`** (not `done`) when
  `broadcastStatus === 'reveal_broadcasted'`; if it is already `reveal_confirmed`/`enacted`,
  go straight to `done`.
- Start a **confirmation poll** (interval, e.g. 8s) that calls `getProposalByActionId`; when
  `broadcastStatus` becomes `reveal_confirmed` (or status `enacted`), update state and set
  `done`. The poll is cleaned up on unmount and when leaving `awaiting-confirmation`.
- **Reconcile on open**: in the initial effect, when the loaded proposal is already
  `reveal_broadcasted`, set phase `awaiting-confirmation` (instead of the current `done`) and
  start the same poll, so a user returning later still converges to `reveal_confirmed`.
- The user may navigate away at any time during `awaiting-confirmation`; the backend task
  (and the on-open reconcile next visit) still promotes the orchestrator record.

### Frontend — `broadcast-phase-progress.tsx`

- Treat `awaiting-confirmation` as: Commit (0) ✓, Reveal (1) ✓, step 3 = **active** with label
  "Awaiting block" (detail: reveal is in the mempool, confirming on Bitcoin).
- Show the Commit/Reveal TXIDs during `awaiting-confirmation` (currently only shown on
  `done`), so the user can track/leave.
- Header copy for `awaiting-confirmation`: "Submitted — awaiting confirmation…".

### Frontend — `broadcast-proposal-screen.tsx`

- Include `awaiting-confirmation` in `showProgress`.
- Allow "Back to proposals" during `awaiting-confirmation` (the existing nav already permits
  leaving; just ensure the awaiting state renders the progress card with txids and a hint that
  it is safe to leave).

### Flow (orchestrator-backed)

```
submit_commit_then_reveal: claim → build → insert pending → submit_package
   → report commit_broadcasted → report reveal_broadcasted → return (txids)   [seconds]
proposals_broadcast: returns reveal_broadcasted + txids; spawns:
   await_reveal_confirmation: poll reveal conf
      ├─ >=1 conf → report reveal_confirmed → remove pending → Confirmed
      └─ timeout 0 conf → PendingConfirmation (stays reveal_broadcasted, pending kept)
UI: broadcast() → awaiting-confirmation (txids shown) → poll → reveal_confirmed → done
    on-open with reveal_broadcasted → awaiting-confirmation → poll → done
```

### Production code vs. test helpers

- **Production functions**: `submit_commit_then_reveal`, `await_reveal_confirmation`,
  `ConfirmOutcome` (Rust); `proposals_broadcast` command; the hook + components (TS).
- **Test helpers**: existing `demo_action_hex`, `generate_test_keypair`, `SpyCommitFunding`,
  `MockBtcRpc`, `MockOrchestratorClient*` stay in `#[cfg(test)]`. A new `MockBtcRpc`
  configuration for **0 confirmations** and a report-recording orchestrator mock are
  test-only. None are registered as Tauri commands.

## Test Cases

Targeting production functions only.

### Backend (Rust unit tests in `application/proposals.rs`)

- **Happy submit**: `submit_commit_then_reveal` with `submit_package` Ok → returns
  `(commit_txid, reveal_txid)`; orchestrator's last reported status is `reveal_broadcasted`;
  `reveal_confirmed` is **never** reported; pending entry **present** afterward.
- **Submit sequential fallback**: unknown-method `submit_package` → two `send_raw_transaction`
  calls; still returns txids at `reveal_broadcasted`.
- **Submit real error → failed**: `submit_package` hard error → `Err(BitcoinRpc)` and a
  `failed` report is sent (genuine submission error path preserved).
- **Confirm happy**: `await_reveal_confirmation` with 1 conf → `Confirmed`; `reveal_confirmed`
  reported; pending entry removed.
- **Confirm timeout (no false failure)**: `await_reveal_confirmation` with 0 conf and a tiny
  timeout → `PendingConfirmation`; **no `failed` report**; pending entry **retained**; no
  `reveal_confirmed` report.
- **Wrapper no longer fails on timeout**: `broadcast_commit_then_reveal` with 0 conf →
  returns Ok-with-txids semantics (or `PendingConfirmation`) and does **not** report `failed`.

### Frontend (vitest, model/hook)

- `BroadcastPhase` union includes `awaiting-confirmation` (type-level / model test).
- Hook: broadcast success with `reveal_broadcasted` → phase `awaiting-confirmation`, txids
  exposed via `result`.
- Hook: poll observes `reveal_confirmed` → phase `done`.
- Hook on-open reconcile: loaded proposal `reveal_broadcasted` → phase
  `awaiting-confirmation` (not `done`).

### Authority isolation / Offline fallback

- Authority handling is unchanged (orchestrator-bound to session). Offline: if the desktop app
  closes before confirmation, the on-open reconcile (`getProposalByActionId` +, when needed,
  `resolveBroadcastStatus` → `reportBroadcastProgress`) converges the record — no new offline
  surface introduced.

## Module structure

- `application/proposals.rs` — **single responsibility per fn**:
  - `submit_commit_then_reveal`: "broadcast commit+reveal and report up to reveal_broadcasted".
  - `await_reveal_confirmation`: "await one reveal confirmation and report/clean up or stay pending".
  - `ConfirmOutcome`: confirmation result enum (lives with the awaiting fn).
  - `broadcast_commit_then_reveal`: thin sequential wrapper composing the two.
- `commands/proposals.rs` — `proposals_broadcast`: "submit synchronously, spawn confirmation,
  return reveal_broadcasted". Background task owns `Arc` clones; no `tauri::State` crosses the
  spawn boundary.
- Frontend keeps domain-by-feature layout: phase in `model/`, side effects in `hooks/`,
  presentation in `components/`. The poll lives in the hook; the component stays presentational.

Dependency direction is preserved: business logic depends on the `OrchestratorClient` /
`BitcoinRpcClient` traits (abstractions), not concrete HTTP clients; the command layer injects
concretes and owns the spawn.
