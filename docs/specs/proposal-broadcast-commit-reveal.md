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
- Backend API contract for preparing and executing broadcast.
- Desktop app integration for broadcast screen and CTA.
- Lifecycle/state updates after each broadcast stage.
- Manual fallback artifacts (hex bundle export/copy).
- Error handling and retries for Bitcoin node/network failures.

### Not included

- Re-defining SPS-50/SPS-51/SPS-65 validation rules.
- Recomputing threshold signature validity beyond protocol/parsing checks already provided by Alpen/Strata crates.
- Mempool policy tuning beyond baseline fee-rate input and standard rejection handling.
- A new long-running indexer architecture (this spec only defines required checks/contracts for status refresh).

## Requirements Alignment

- Backend remains coordination-only:
  - Assemble envelope transactions and track lifecycle.
  - Do not re-implement protocol validity rules that belong to protocol crates.
- Signer safety:
  - Broadcast is only enabled after quorum is reached.
  - UI shows high-signal confirmation and deterministic artifacts before sending.
- Manual survivability:
  - Users can copy/export broadcast bundle (`commit`/`reveal` hex + metadata) and broadcast externally if backend or node path is degraded.

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
- `enacted` is set only after reveal tx is confirmed and payload is recognized as enacted by canonical state checks.

### Broadcast Sub-status (new response field)

Add optional `broadcast_status` to proposal/broadcast DTOs:
- `idle` - no broadcast attempt yet.
- `commit_broadcasted` - commit sent to network.
- `commit_confirmed` - commit mined and reveal can be sent.
- `reveal_broadcasted` - reveal sent to network.
- `reveal_confirmed` - reveal mined; awaiting or completing enacted transition.
- `failed` - latest attempt failed (with reason code/message).

This field is operational metadata and does not replace canonical proposal lifecycle state.

## Product Flow

### Entry

- User opens proposal in `approved` state from dashboard and lands on broadcast screen.
- Screen loads proposal payload + signatures and requests a broadcast preview bundle from backend.

### Step 1: Prepare Bundle

Backend prepares deterministic broadcast payload:
- Build signed SPS payload from proposal:
  - `seq_no`
  - `action`
  - collected threshold signatures
- Build reveal transaction tied to a commit-funded input.
- Return:
  - `commit_tx_hex`
  - `commit_txid` (if determinable pre-broadcast; otherwise return after submit)
  - `reveal_tx_hex`
  - `reveal_txid` (deterministic from hex)
  - fee/weight summary
  - integrity hints (payload hash/sighash references)

### Step 2: Broadcast Commit

- On user confirmation (`Broadcast`), backend submits `commit_tx_hex`.
- Backend records `commit_txid` and sets `broadcast_status = commit_broadcasted`.
- Backend waits for confirmation depth policy (minimum `1` block for reveal gating in this slice).
- On confirmation, set `broadcast_status = commit_confirmed`.

### Step 3: Broadcast Reveal

- Backend submits `reveal_tx_hex` only after commit confirmation check passes.
- Set `broadcast_status = reveal_broadcasted`.
- Backend waits for reveal confirmation and updates:
  - `broadcast_status = reveal_confirmed`
  - proposal lifecycle to `enacted` when canonical enacted condition is observed.

### Step 4: Finalize UX

- UI shows success with reveal txid and copy links/artifacts.
- Dashboard eventually shows proposal under executed/enacted grouping.

## API Contract

### 1) Prepare broadcast payload

`POST /proposals/:action_id/broadcast/prepare`

Purpose:
- Validate proposal is `approved`.
- Assemble commit/reveal artifacts without sending to network.

Response:

```json
{
  "actionId": "hex",
  "proposalStatus": "approved",
  "broadcastStatus": "idle",
  "commitTxHex": "hex",
  "commitTxid": "optional-hex",
  "revealTxHex": "hex",
  "revealTxid": "hex",
  "summary": {
    "requiredSignatures": 3,
    "collectedSignatures": 3,
    "feeSats": 1234,
    "vbytes": 456
  }
}
```

### 2) Execute broadcast

`POST /proposals/:action_id/broadcast`

Purpose:
- Run commit -> confirm -> reveal sequence.
- Return progress/result payload (sync for now; async migration possible later).

Response (success):

```json
{
  "actionId": "hex",
  "proposalStatus": "enacted",
  "broadcastStatus": "reveal_confirmed",
  "commitTxid": "hex",
  "revealTxid": "hex"
}
```

Response (failure example):

```json
{
  "actionId": "hex",
  "proposalStatus": "approved",
  "broadcastStatus": "failed",
  "errorCode": "REVEAL_REJECTED",
  "errorMessage": "Reveal transaction rejected by node policy"
}
```

### 3) Read proposal status

`GET /proposals` and `GET /proposals/:action_id` should include optional `broadcastStatus`, plus txids when present.

## Technical Design

### Backend (`orchestrator-be`)

- Add broadcast application service with explicit phases:
  - `prepare_broadcast_bundle(action_id)`
  - `broadcast_commit_then_reveal(action_id)`
- Persist operational fields in `proposals` table (or dedicated broadcast table):
  - `broadcast_status`
  - `commit_txid`
  - `reveal_txid`
  - `last_broadcast_error`
  - timestamps for commit/reveal broadcast/confirmation
- Integrate with Bitcoin client adapter:
  - submit raw tx
  - query tx confirmation depth
  - map node errors into typed app errors
- Enforce idempotency:
  - if commit already broadcasted/confirmed, do not resend blindly.
  - if reveal already broadcasted, return current state.
- Enforce authority/session checks on broadcast endpoints same as proposal read/write endpoints.

### Desktop Tauri layer (`desktop-app/src-tauri`)

- Add orchestrator client methods:
  - `prepareProposalBroadcast(actionId)`
  - `broadcastProposal(actionId)`
- Map backend DTOs into frontend-safe types with explicit `broadcastStatus`.
- Keep high-signal error messages but sanitize low-level node internals where required.

### Frontend (`desktop-app/src`)

- Add/complete broadcast route:
  - `'/proposals/:actionId/broadcast'`
- Broadcast screen requirements:
  - Proposal summary card and quorum indicator.
  - Approval bundle block (copyable).
  - Raw tx section showing commit + reveal hex (copyable independently).
  - Primary CTA: `Broadcast`.
  - In-progress state text reflecting current phase:
    - "Broadcasting commit..."
    - "Waiting commit confirmation..."
    - "Broadcasting reveal..."
    - "Waiting reveal confirmation..."
  - Error panel with retry when safe.
- Dashboard CTA behavior:
  - `approved` card CTA routes to broadcast screen.
  - pending/non-approved proposals cannot start broadcast.

## Error Handling

Failure classes:
- Proposal state invalid (`pending`, `expired`, `canceled`, `enacted`) -> reject broadcast with conflict semantics.
- Commit rejected by node -> set `failed`, keep proposal `approved`.
- Commit not confirmed within timeout -> set `failed` with timeout code.
- Reveal rejected after confirmed commit -> set `failed`, include txids and operator guidance.
- Auth/session failure -> uniform unauthorized response behavior.

Recovery expectations:
- Retry path should resume from last safe phase.
- Manual fallback always available by exposing/copying raw hex artifacts.

## Acceptance Criteria

1. Only `approved` proposals can invoke prepare/broadcast endpoints.
2. Prepare endpoint returns deterministic commit/reveal artifacts without network submission.
3. Broadcast endpoint enforces commit confirmation before reveal broadcast.
4. Proposal exposes `broadcastStatus` and txids after attempts.
5. Successful commit+reveal path transitions proposal to `enacted`.
6. Failed broadcast attempts keep proposal in `approved` and provide actionable error metadata.
7. Desktop broadcast UI surfaces phase progress, tx artifacts, copy actions, and retry/manual fallback options.

## Test Plan

### Backend Unit

- Validate state guard: non-approved proposals rejected.
- Validate idempotent phase handling:
  - commit already broadcasted
  - commit confirmed, reveal pending
  - reveal already broadcasted
- Validate error mapping from Bitcoin adapter to API error codes.

### Backend Integration

- Happy path with regtest:
  - prepare -> broadcast commit -> mine block -> broadcast reveal -> mine block -> enacted.
- Commit rejected path.
- Reveal rejected path after commit confirmation.
- Timeout while waiting confirmation.
- Auth/authority isolation for broadcast endpoints.

### Desktop/Frontend

- Approved proposal opens broadcast screen and loads prepare bundle.
- Broadcast button triggers phase state transitions and disables duplicate submits.
- Errors render retry/manual fallback guidance.
- Copy actions for approval bundle, commit hex, reveal hex, and txids.

### E2E

- Reuse/extend commit-reveal broadcast test to verify full flow from proposal approval to enacted status reflected in dashboard.

## Rollout Notes

1. Land backend schema + DTO changes first (`broadcast_status` and txid fields).
2. Implement prepare endpoint, then broadcast endpoint with phase persistence.
3. Wire desktop Tauri + frontend broadcast screen.
4. Add integration/e2e coverage before enabling by default in product navigation.

