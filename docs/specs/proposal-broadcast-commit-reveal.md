# Spec: Proposal Broadcast via Commit + Reveal

## Objective

Define the production broadcast flow for approved proposals using the ASM commit/reveal envelope model, including transaction building, confirmation gating, reveal broadcast, and operator-facing UX/safety behavior.

This spec ensures quorum-approved proposals can move from offchain coordination to onchain execution with clear status transitions and manual fallback support.

## Scope

### Included

- Broadcast flow for `approved` proposals only.
- Construction of both:
  - `commit` transaction (funding/output anchor for reveal spend).
  - `reveal` transaction (carries SPS payload/witness data).
- Ordered execution (R1.0.1):
  - Build **and sign** the commit + reveal bundle before broadcasting either.
  - Broadcast commit→reveal — atomically via `submitpackage` when supported, otherwise sequentially.
  - Confirm.
- **Orchestrator** coordination API (claim + progress reporting).
- **Desktop** on-chain execution (prepare preview + commit/reveal submit).
- Lifecycle/state updates after each broadcast stage.
- Manual fallback artifacts (hex bundle export/copy).
- Error handling and retries for Bitcoin node/network failures.

### Not included

- Re-defining SPS-50/SPS-51/SPS-65 validation rules.
- Recomputing threshold signature validity beyond protocol/parsing checks already provided by Alpen/Strata crates.
- Mempool policy tuning beyond baseline fee-rate input and standard rejection handling.
- A new long-running indexer architecture (this spec only defines required checks/contracts for status refresh).

## Requirements Alignment

- **Orchestrator remains coordination-only** (PRD backend guidelines §1):
  - Proposals, signatures, quorum/off-chain lifecycle.
  - Broadcast **metadata** (`broadcast_status`, txids, errors) via `claim` + `PATCH`.
  - Does **not** submit Bitcoin transactions or hold the production operator key.
- **Desktop owns execution** (PRD UI + `docs/2-discovery/01-conceptual-overview.md` §6.5):
  - Commit/reveal construction and RPC submit from Tauri (`broadcast_env` process config).
  - **R1.0 (current):** The SPS-50 envelope key is a **per-broadcast ephemeral key** generated in-memory (`generate_ephemeral_envelope_keypair`, OsRng). It is never persisted, never seed-derived, and discarded after the reveal is signed. The reveal change output is redirected to an Admin Wallet internal address (`WalletService::reveal_change_address`) so no funds are stranded on the throwaway key. The commit **funding** tx is still software-signed by the mnemonic session (`BdkAdminWalletMnemonic`); `WalletSession` no longer caches an envelope keypair. See [`admin-wallet-ephemeral-reveal-key.md`](./admin-wallet-ephemeral-reveal-key.md).
  - **R1.0.1 (current — crash window closed):** The commit **and** reveal are both built and signed *before* either is broadcast; the ephemeral key is dropped immediately after the reveal is signed. The pair is then broadcast commit→reveal via `submitpackage` (Bitcoin Core 24+) when available, otherwise sequentially (`sendrawtransaction` commit then reveal). The reveal is built from the **local** signed commit `Transaction`, so the previous `getrawtransaction` round-trip is gone. The signed reveal is held in a session-scoped **in-memory** store keyed by `action_id` and re-broadcastable via the `proposals_resubmit_reveal` IPC command (no key needed). See [`admin-wallet-presign-commit-reveal.md`](./admin-wallet-presign-commit-reveal.md).
  - **Residual limitation (R1.0.1):** No durable cross-process persistence. `submitpackage` atomicity means a crash leaves nothing on-chain (clean retry); the in-memory store + resubmit covers live-session transient failures. A hard process crash on the **sequential-fallback** path (pre-24 node) between the commit and reveal sends is an accepted, documented limitation. Durable persistence (orchestrator-stored reveal) is a possible future hardening, out of scope for R1.0.1.
  - Pre-R1.0 history: Phase 3.5 folded the key into `m/86'/0'/73'/2/0`; Phase 3.7c cached it in `WalletSession`. Both superseded by R1.0.
- **Signer safety:**
  - Broadcast is only enabled after quorum is reached.
  - UI shows high-signal confirmation and deterministic artifacts before sending.
- **Manual survivability:**
  - Users can copy/export broadcast bundle (`commit`/`reveal` hex + metadata) and broadcast externally if coordination or node path is degraded.

## State Model

Canonical states remain:
- `pending`
- `approved`
- `enacted`
- `canceled`
- `expired`

Broadcast state semantics for `approved` proposals:
- `approved` means quorum reached and broadcast-eligible.
- During broadcast processing, proposal remains `approved` but exposes broadcast sub-status in API response (see below).
- `enacted` is set only when ASM canonical state satisfies the proposal action post-conditions (signer set / threshold / `last_seqno`), typically after the confirmation-delay queue executes — not when the reveal tx reaches Bitcoin confirmation.

### Broadcast Sub-status (response field)

Optional `broadcast_status` on proposal/broadcast DTOs:
- `idle` - no broadcast attempt yet.
- `commit_broadcasted` - commit sent to network.
- `commit_confirmed` - commit mined and reveal can be sent.
- `reveal_broadcasted` - reveal sent to network.
- `reveal_confirmed` - reveal mined; proposal stays `approved` until ASM enactment is detected.
- `failed` - latest attempt failed (with reason code/message).

This field is operational metadata and does not replace canonical proposal lifecycle state.

## Product Flow

### Entry

- User opens proposal in `approved` state from dashboard and lands on broadcast screen.
- Screen loads proposal payload + signatures; Tauri prepares fee/commit preview **locally**.

### Step 1: Prepare (desktop)

Tauri `proposals_prepare_broadcast`:
- Validates proposal is `approved` (via orchestrator `GET`).
- Generates a fresh ephemeral envelope keypair in-memory; builds **indicative** commit address and fee estimate using local Bitcoin RPC.
- Returns commit address and sats to the UI (no network submit). The displayed address is **indicative** — a new ephemeral key is generated at broadcast time, so the preview address will differ from the final on-chain commit address. The UI labels this section "Commit TX (preview)" accordingly.

### Step 2: Claim + broadcast (desktop + orchestrator)

On user confirmation (`Broadcast`):

1. `POST /proposals/:action_id/broadcast/claim` — orchestrator atomically sets `broadcast_status = commit_broadcasted` (or `409` if already claimed).
2. Tauri builds **and signs both** the commit and the reveal locally, drops the ephemeral key, stores the signed reveal in memory, then broadcasts commit→reveal (`submitpackage` if available, otherwise sequential `sendrawtransaction`) and waits for confirmation (local Bitcoin RPC).
3. `PATCH /proposals/:action_id/broadcast` reports `commit_broadcasted` (commit_txid) then `reveal_broadcasted` (reveal_txid) after the broadcast, then `reveal_confirmed` after confirmation, leaving `proposal_status` as `approved`. The intermediate `commit_confirmed` report is no longer sent (both txs confirm together; the PATCH contract enforces no sub-status ordering).
4. UI re-fetches `GET /proposals/:action_id` and displays **persisted** fields (no hard-coded status strings).
5. On `GET /proposals` or `GET /proposals/:action_id`, the orchestrator reconciles `approved` + `reveal_confirmed` rows to `enacted` when ASM post-conditions match (coordination hygiene only).

### Step 3: Finalize UX

- UI shows success with reveal txid and copy links/artifacts.
- Dashboard shows proposal under enacted grouping when coordinator state matches on-chain outcome.

## API Contract (orchestrator `/api/v1`)

### 1) Claim broadcast

`POST /proposals/:action_id/broadcast/claim`

- Validates session authority matches proposal.
- Validates `approved` + threshold snapshot (P-035).
- Atomically transitions `broadcast_status`: `idle` → `commit_broadcasted`.
- Returns updated `Proposal` JSON.

### 2) Report progress

`PATCH /proposals/:action_id/broadcast`

Request body:

```json
{
  "broadcastStatus": "commit_confirmed",
  "proposalStatus": null,
  "commitTxid": "hex",
  "revealTxid": null,
  "broadcastError": null
}
```

- Desktop reports each phase after local Bitcoin steps.
- Returns updated `Proposal` JSON.

### 3) Read proposal status

`GET /proposals` and `GET /proposals/:action_id` include `broadcastStatus`, `commitTxid`, `revealTxid`, `broadcastError` when present.

## Technical Design

### Orchestrator (`orchestrator-be`)

- **No** `broadcast_tx` module or signing key in server config.
- Application: `claim_broadcast_coordination`, `report_broadcast_progress`.
- Repository: `claim_broadcast`, `update_broadcast_status`.
- Bitcoin RPC on server: `/ready` health check only.

### Desktop Tauri (`desktop-app/src-tauri`)

- `infrastructure/broadcast_env.rs` — process env for RPC/asm config + signing gates (no keypair since R1.0).
- `infrastructure/admin_wallet/ephemeral_envelope_key.rs` — `generate_ephemeral_envelope_keypair()` via OsRng (R1.0).
- `application/wallet_service.rs` — `reveal_change_address()` returns next unused internal address (R1.0).
- `application/proposals.rs` — `prepare_broadcast_bundle`, `broadcast_commit_then_reveal`; generates ephemeral key internally; accepts `reveal_change_spk: ScriptBuf` (R1.0).
- IPC: `proposals_prepare_broadcast`, `proposals_broadcast` (no secrets in React). `proposals_broadcast` resolves `reveal_change_spk` from `wallet_service.reveal_change_address()`.

### In-flight UTXO reservation (desktop, #516)

A signed commit (or send) is invisible to BDK until a wallet sync sees it, and the wallet lock is
released while a hardware signer signs (up to 180 s). Without a guard, a second build in that
window — or seconds later — selects the same UTXO, the two commits are mutually exclusive, and the
loser's reveal is unrecoverable because its envelope key is already evicted.

`WalletService` therefore keeps an in-memory set of reserved outpoints, keyed by the txid that
spends them:

- **Reserve.** Commit funding (`build_and_sign_tx`), Send BTC (`build_send_psbt`) and fee bumps
  (the CPFP child and the RBF replacement in `bump_fee`) skip the reserved outpoints, call
  `finish()`, and reserve the new PSBT's inputs **before** releasing the wallet lock. Commit, Send
  and RBF pass the set to `TxBuilder::unspendable`; the CPFP child leaves reserved coins out of
  its spare funding. An RBF replacement's reservation also covers the inputs of the tx it
  replaces, which it spends by design. The Send estimate honours the same set, so Max and the
  insufficient-funds boundary match the real send.
- **Release immediately** when building or signing fails, when any setup step fails after the
  commit is signed but before `broadcast_via` (`CommitFunding::release`), or when a fee bump is
  aborted before its broadcast (fee lookup error, package-rate shortfall).
- **Mark attempted** right before `broadcast_via` (`CommitFunding::mark_broadcast_attempted`), and
  before the broadcast of a Send or a fee bump. From then on a failed broadcast does **not** release: a partial
  broadcast (commit accepted, reveal rejected) can leave the commit live.
- **Record on success.** Once a broadcaster accepts a tx (commit and reveal, Send, fee bump), it is
  inserted into the wallet graph as unconfirmed (`Wallet::apply_unconfirmed_txs`,
  `WalletService::record_broadcast` / `CommitFunding::record_broadcast`) and its reservation is
  dropped: BDK itself treats the inputs as spent, even if no sync ever sees the tx (e.g. the node
  fallback broadcast it while Electrum is down). This complements the attempt mark, which only
  governs a broadcast that failed or was partial.
- **Drop on sync.** After `apply_update`, every reservation whose txid the wallet graph now holds is
  dropped — BDK itself treats those inputs as spent.
- **Grace** (failed or partial broadcasts only). A reservation no sync has seen expires 10 minutes after its last touch (build or
  broadcast attempt): well above the 180 s signing timeout plus a few 30 s background syncs, short
  enough not to strand coins after a broadcast that never landed.

Reservations live in memory only and die with the wallet session.

Once `broadcast_via` has succeeded, the bundle is on the network and the send succeeds: a failing
progress report is retried (3 attempts) and logged, never reported as `failed` — that would reopen
the claim while the bundle is live — and the txids are returned so the confirmation watcher starts;
its `reveal_confirmed` report heals the orchestrator row. Errors before or at the broadcast still
report `failed`.

**Known limits (BDK 1.2, documented, not fixed):**

- BDK cannot evict a transaction from its graph. A commit — or any broadcast tx recorded in the
  graph on success — that is dropped from the mempool **without** a conflicting spend keeps its
  inputs looking spent until the session wallet is rebuilt.
- A commit funded from unconfirmed change of an earlier commit dies with its parent if the parent
  is dropped or replaced.

### Frontend (`desktop-app/src`)

- Route `'/proposals/:actionId/broadcast'`.
- Re-fetch proposal after broadcast (P-062).
- In-flight guard (P-020 partial).

## Manual Fallback

Signers may construct and broadcast commit/reveal without the coordinator when it is down, per PRD §2. When the coordinator is reachable again, progress may be reported via `PATCH` or reconciled manually.
