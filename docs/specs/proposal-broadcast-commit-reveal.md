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
2. Tauri builds **and signs both** the commit and the reveal locally, drops the ephemeral key, and stores the signed reveal (`PendingReveals`).
3. **Pre-register (#516).** Before anything reaches the network, `PATCH /proposals/:action_id/broadcast` reports `commit_broadcasted` — the status the claim already set — **with both `commit_txid` and `reveal_txid`** (the backend keeps txids with `COALESCE`). The report is retried (3 attempts); if it still fails, nothing is broadcast: the commit's coins are released, the pending reveal is dropped, and `failed` is reported (best effort). A bundle the orchestrator cannot track would otherwise be unrecoverable once its later reports failed.
4. Tauri broadcasts commit→reveal (`submitpackage` if available, otherwise sequential `sendrawtransaction`), then reports `reveal_broadcasted` (retried and logged; never `failed` once the bundle is on the network — see the reservation section), and a background task reports `reveal_confirmed` after confirmation, leaving `proposal_status` as `approved`. The intermediate `commit_confirmed` report is not sent (both txs confirm together; the PATCH contract enforces no sub-status ordering).
5. UI re-fetches `GET /proposals/:action_id` and displays **persisted** fields (no hard-coded status strings). Its confirmation poll reads `commitTxid`/`revealTxid` from the row, which are present from step 3 on.
6. On `GET /proposals` or `GET /proposals/:action_id`, the orchestrator reconciles: a row at `commit_broadcasted`, `commit_confirmed` or `reveal_broadcasted` whose `reveal_txid` is mined is promoted to `reveal_confirmed` (so a bundle whose later reports never landed still converges), and `approved` + `reveal_confirmed` rows become `enacted` when ASM post-conditions match (coordination hygiene only).

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
  commit is signed but before the broadcast (`CommitFunding::release`), or when a fee bump is
  aborted before its broadcast (fee lookup error, package-rate shortfall).
- **Record on success.** Once a broadcaster accepts a tx (commit and reveal, Send, fee bump), it is
  inserted into the wallet graph as unconfirmed (`Wallet::apply_unconfirmed_txs`,
  `WalletService::record_broadcast` / `CommitFunding::record_broadcast`) and its reservation is
  dropped: BDK itself treats the inputs as spent, even if no sync ever sees the tx (e.g. the node
  fallback broadcast it while Electrum is down).
- **Settle a failed broadcast from the broadcaster's own answer.** Nothing is looked up afterwards.
  Each broadcaster classifies what happened to **each transaction** it sent
  (`TxBroadcastError::kind`, typed — never matched from message text):

  | Outcome | Node (JSON-RPC) | Electrum |
  |---|---|---|
  | `Accepted` | success, or "already known" | success, "already known", or bitcoind's -27 ("already in block chain") relayed |
  | `Rejected` | the node answered `sendrawtransaction` with a JSON-RPC error object | the server's error carries bitcoind's refusal code, -25 or -26 (see below) |
  | `NotDelivered` | the connection never opened (`reqwest` `is_connect`), or Core's HTTP server refused the request before running any RPC: 401, 403, 404 or 405 without a JSON-RPC error body | `Client::new` failed; or the tx could not even be decoded |
  | `Ambiguous` | anything else: timeout, connection dropped after sending, any other HTTP error without a JSON-RPC error body (5xx), unexpected result; a transport failure of `submitpackage` | any other server error (daemon unreachable or warming up, index not ready, a refusal relayed without its code); any transport error after the request went out; a failed `spawn_blocking` |

  `submitpackage` is asked first. When the node **answers** without taking the whole package — an
  unknown method, a package error, a non-success `package_msg` — part of it may be in the mempool,
  so the node is asked again tx by tx with `sendrawtransaction`, whose answer is definitive per tx
  ("already known" = `Accepted`, so a commit the package already admitted is detected). Only a
  `submitpackage` call the node never answered stays `Ambiguous` (or `NotDelivered`).

  **Electrum servers relay bitcoind's refusal in different shapes**, and only the relayed code is
  read (`relayed_bitcoind_code`), never the text around it: the error's own `code` when it is a
  bitcoind code (`{"code": -26, …}`); Blockstream's electrs (esplora) JSON inside the message
  (`sendrawtransaction RPC error: {"code":-26,"message":…}`); ElectrumX's daemon-error repr
  (`daemon error: DaemonError({'code': -26, …})`). romanz/electrs 0.10 (the regtest stack) relays
  only the daemon's message under its own code 2, and ElectrumX's broadcast refusal names no code:
  those are `Ambiguous`; the node fallback still answers definitively, and a tx left `Ambiguous`
  keeps its coins reserved.

  In the sequential fallback (`sendrawtransaction` commit, then reveal) each tx keeps its own
  outcome: a commit accepted before the reveal fails stays `Accepted`. Across broadcasters, per tx:
  any `Accepted` → `Accepted`; else any `Ambiguous` → `Ambiguous`; else all `NotDelivered` →
  `NotDelivered`; else `Rejected` (`TxOutcome::combine`).

  The **commit's** combined outcome drives the proposal (`submit_commit_then_reveal`,
  `broadcast_manual`):

  | Commit | Coins | Pending reveal | Error (IPC code) | `failed` reported | UI |
  |---|---|---|---|---|---|
  | `Accepted` + reveal `Accepted` | both recorded | kept until confirmed | — | no | success |
  | `Accepted`, reveal not | commit recorded | kept | `RevealNotBroadcast` (`reveal_not_broadcast`) | no | no Retry, no manual panel |
  | `Rejected` | **released at once** | removed (`action_id` / `manual-<sighash>`) | `BroadcastRejected` (`broadcast_rejected`), no hexes | yes | Retry send |
  | `NotDelivered` | reserved | kept | `AllBroadcastersFailed` (`broadcast_unavailable`) **with both hexes** | no | send-manually panel, no Retry |
  | `Ambiguous` | reserved | kept | `BroadcastUncertain` (`broadcast_uncertain`), no hexes | no | "may already be on the network", no Retry |

  The send-manually panel renders only for `broadcast_unavailable`: nothing reached any source,
  the coins stay reserved and the claim stays taken, so broadcasting those exact transactions by
  hand cannot double-spend. It never renders after a rejection, whose coins were released and
  whose claim was reopened. `BroadcastUncertain` carries no hexes on purpose: the tx most likely
  reached a source that did not answer, the reservation already protects its coins, and the row
  converges once the network sees it; handing the operator a manual path for a state nobody can
  read would invite action on a guess.

  A **Send** or **fee bump** follows the same classification for its single tx
  (`WalletService::broadcast_reserved_tx`): `Accepted` → recorded; `Rejected` or `NotDelivered`
  (there is no manual panel for these, and nothing was sent) → released at once and
  `BroadcastFailed`; `Ambiguous` → kept reserved and `BroadcastUncertain` ("may have been sent —
  its coins stay reserved, shown as in flight, until it is settled; do not send again").

  Every screen that sends a bundle — the send and cancel screens and the manual screen — offers a
  fresh send (Retry / Back to signing) only when `offersRetry` holds: never for
  `broadcast_uncertain`, `reveal_not_broadcast`, `bundle_in_flight` or `broadcast_unavailable`.
  The manual path also refuses a second send of the same proposal while its `manual-<sighash>`
  reveal is still stored (`BundleInFlight`, IPC `bundle_in_flight`): that bundle's commit may be
  live, and silently replacing the entry would fund a second commit.
- **When reserved coins come back.** A reservation without a definitive answer (`Ambiguous`,
  `NotDelivered` commit) ends only when a wallet sync sees the tx — after `apply_update`, every
  reservation whose txid the wallet graph now holds is dropped, and BDK itself treats the inputs as
  spent — or when the session ends. There is no time grace and no lookup.
- **Balance.** `get_balance` and `list_utxos` leave reserved outpoints out; `BalanceDto` carries
  them as `reservedSats`, and the wallet panel shows an "N sats in flight" line when it is non-zero.
  The Send estimate and Max use the same set.

Reservations live in memory only and die with the wallet session.

Once the pair broadcast has succeeded, the bundle is on the network and the send succeeds: a failing
`reveal_broadcasted` report is retried (3 attempts) and logged, never reported as `failed` — that
would reopen the claim while the bundle is live — and the txids are returned so the confirmation
watcher starts. The orchestrator already holds both txids from the pre-registration, so its own
reconcile promotes the row once the reveal is mined, even if no later report lands. Errors before
the broadcast, and a commit every answering source rejected, still report `failed`.

**Known limits (BDK 1.2 and in-memory reservations, documented, not fixed):**

- BDK cannot evict a transaction from its graph. A commit — or any broadcast tx recorded in the
  graph on success — that is dropped from the mempool **without** a conflicting spend keeps its
  inputs looking spent until the session wallet is rebuilt.
- A commit funded from unconfirmed change of an earlier commit dies with its parent if the parent
  is dropped or replaced.
- An `Ambiguous` or `NotDelivered` reservation lives in memory only: an app restart (or the end of
  the session) loses it, and the coins become selectable again even if the tx later lands.
- A proposal row left at `commit_broadcasted` by a `NotDelivered`, `Ambiguous` or
  `RevealNotBroadcast` outcome is not degraded here, and a reveal kept in `PendingReveals` has no
  resubmit path in the UI; the desktop's detection of stuck bundles (phase 2 of #516) handles both.
  The row already holds both pre-registered txids, so a bundle that did land — or one broadcast by
  hand from the send-manually panel — is promoted by the orchestrator's reconcile once its reveal
  is mined.
- A source that answers with an explicit rejection is trusted to not hold the tx: the node's
  JSON-RPC error answer, or an Electrum error that carries bitcoind's -25/-26.

### Frontend (`desktop-app/src`)

- Route `'/proposals/:actionId/broadcast'`.
- Re-fetch proposal after broadcast (P-062).
- In-flight guard (P-020 partial).

## Manual Fallback

Signers may construct and broadcast commit/reveal without the coordinator when it is down, per PRD §2. When the coordinator is reachable again, progress may be reported via `PATCH` or reconciled manually.
