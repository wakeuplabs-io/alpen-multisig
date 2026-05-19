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
- Ordered execution:
  - Build commit + reveal bundle.
  - Broadcast/confirm commit.
  - Broadcast reveal after commit is confirmed.
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
  - Operator key loaded in the Tauri process only (never the React webview).
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
- Builds commit address and fee estimate using local Bitcoin RPC + operator key.
- Returns commit address and sats to the UI (no network submit).

### Step 2: Claim + broadcast (desktop + orchestrator)

On user confirmation (`Broadcast`):

1. `POST /proposals/:action_id/broadcast/claim` — orchestrator atomically sets `broadcast_status = commit_broadcasted` (or `409` if already claimed).
2. Tauri submits commit tx, waits for confirmation, submits reveal tx (local Bitcoin RPC).
3. After each phase, `PATCH /proposals/:action_id/broadcast` with `broadcast_status` and optional `commit_txid` / `reveal_txid`. After reveal confirmation, report `reveal_confirmed` and leave `proposal_status` as `approved`.
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

- **No** `broadcast_tx` module or operator key in server config.
- Application: `claim_broadcast_coordination`, `report_broadcast_progress`.
- Repository: `claim_broadcast`, `update_broadcast_status`.
- Bitcoin RPC on server: `/ready` health check only.

### Desktop Tauri (`desktop-app/src-tauri`)

- `infrastructure/broadcast_env.rs` — process env for RPC + operator key.
- `application/proposals.rs` — `prepare_broadcast_bundle`, `broadcast_commit_then_reveal` with coordination callbacks.
- IPC: `proposals_prepare_broadcast`, `proposals_broadcast` (no secrets in React).

### Frontend (`desktop-app/src`)

- Route `'/proposals/:actionId/broadcast'`.
- Re-fetch proposal after broadcast (P-062).
- In-flight guard (P-020 partial).

## Manual Fallback

Signers may construct and broadcast commit/reveal without the coordinator when it is down, per PRD §2. When the coordinator is reachable again, progress may be reported via `PATCH` or reconciled manually.
