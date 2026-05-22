# Spec: Proposal Signing Flow and Dashboard Status Alignment

## Objective

Define and implement the product flow for signing a proposal from the dashboard, aligned with the current proposal lifecycle states and the target UI references.

This spec covers:
- A production route for signing a specific proposal.
- Dashboard behavior and grouping by proposal state.
- CTA behavior per state, including mandatory Sign CTA for pending proposals.
- UX and safety expectations for signer confirmation with hardware wallet.

## Scope

### Included

- Add a production sign route scoped by proposal identity:
  - `'/proposals/:actionId/sign'`
- Wire pending proposal cards to navigate to sign flow.
- Align dashboard state grouping and section labels with product references.
- Keep status mapping aligned with canonical backend/PRD states:
  - `pending`, `approved`, `enacted`, `canceled`, `expired`
- Define CTA rules by state (sign, broadcast, or read-only).
- Replace sign view mock wiring with proposal-driven data loading.
- Add loading/error/conflict handling for sign flow.

### Not included

- New backend protocol validation rules (backend remains coordination-only).
- Changes to SPS-50/51/65 semantics.
- New proposal states beyond current canonical set.
- Full broadcast execution flow details (kept as existing behavior, referenced only for CTA visibility).

## State Model and UI Semantics

Canonical proposal states:
- `pending`: offchain proposal collecting signatures.
- `approved`: quorum reached and proposal confirmed as approved; broadcast-ready context in dashboard.
- `enacted`: successfully enacted.
- `canceled`: canceled proposal.
- `expired`: proposal expired before quorum.

Dashboard sections:
- **Quorum reached**
  - Contains: `approved`
- **Pending**
  - Contains: `pending`
- **Executed & Canceled**
  - Contains: `enacted`, `canceled`
- **Expired / Skipped** (keep current naming to avoid regression in existing UI copy)
  - Contains: `expired`

Notes:
- If product later prefers 3 sections only (as visual simplification), `expired` can be moved under `Executed & Canceled` while preserving state badge and semantics.
- This spec keeps existing 4-section implementation to minimize behavior regressions.

## Dashboard Behavior

Each proposal card must display:
- Title, seq no, authority, proposal type label.
- Status badge with color and text.
- Signature progress strip and signed count.
- Contextual footer and CTA based on state.

CTA rules:
- `pending`:
  - Primary CTA: `Sign`
  - Action: navigate to `'/proposals/:actionId/sign'`
- `approved`:
  - Primary CTA: `Broadcast`
  - Action: existing broadcast flow
- `enacted`, `canceled`, `expired`:
  - No primary action CTA
  - Card remains review-only

Pending section requirement:
- All pending cards MUST render a visible Sign CTA above the fold (without extra expansion or hidden action menu).

## Sign Proposal Screen

Route:
- `'/proposals/:actionId/sign'`

Input source:
- `actionId` from route params.
- Proposal payload loaded from orchestrator by `actionId`.

Required UI blocks:
- Back navigation to proposals dashboard.
- Screen title and context copy:
  - "Review the payload, then confirm on your Trezor. Nothing is sent until you sign."
- Proposal summary card:
  - Proposal identifier (seq-based label), authority, proposal type, proposal title.
- Payload review area:
  - For key update style payloads: `Before` and `After` values.
  - For non-diff payloads: fallback structured details block.
- Sighash display:
  - `SPS-65 Sighash (32 bytes)` label
  - Copy button
- Hardware wallet safety callout:
  - explicit instruction to verify value on device before confirming.
- Primary action:
  - `Sign with Trezor`

Screen states:
- **Loading**: proposal/sighash data loading.
- **Ready**: proposal loaded and sign button enabled.
- **Signing**: button disabled, waiting indicator.
- **Error**: recoverable error surface (read, sign, copy).
- **Success**: signature collected confirmation block.

## Data and Behavior Requirements

- Sign flow must work with real proposal data (not static mocks).
- Proposal payload returned by backend MUST include `requiredSignatures` as a per-proposal snapshot of the
  authority threshold at creation time.
- Dashboard signature counter MUST use `collected_signatures / requiredSignatures` from proposal data and MUST NOT
  derive required signatures using UI heuristics.
- If proposal is no longer `pending` at time of signing:
  - Block signing action.
  - Show high-signal conflict message:
    - "This proposal is no longer pending and cannot be signed."
  - Provide navigation back to dashboard.
- If proposal is missing/not found:
  - Show not-found error state and back action.
- If session expires:
  - Trigger existing re-auth/session refresh flow.
- If wallet is disconnected:
  - Disable sign CTA and show recovery guidance.

## Technical Design

Frontend routing:
- Add route in `desktop-app/src/App.tsx`:
  - `'/proposals/:actionId/sign'` -> sign screen component.
- Keep `'/dev/sign'` only for internal dev preview if needed.

Dashboard integration:
- Extend `ProposalsDashboard` card API to receive callbacks:
  - `onSignProposal(actionId: string)`
  - `onBroadcastProposal(actionId: string)` (optional if broadcast already handled elsewhere)
- In `ProposalsDashboardScreen`, wire:
  - pending card CTA -> navigate(`/proposals/${actionId}/sign`)

Sign screen integration:
- Convert current POC screen behavior to route-param driven behavior.
- Use orchestrator API to fetch proposal by `actionId`.
- Build derived view model for:
  - title
  - type label
  - before/after or fallback payload preview
  - sighash value for wallet signing
- Preserve current hardware wallet adapter invocation and error surfacing patterns.

Backend + data contract integration:
- Add `required_signatures` column to `proposals` table.
- Persist `required_signatures` when creating the proposal by reading the authority threshold from ASM state.
- Expose `required_signatures` in orchestrator proposal responses.
- Propagate field through Tauri DTOs and frontend `Proposal` type as `requiredSignatures`.

## Acceptance Criteria

1. A proposal with `pending` status always shows a visible `Sign` CTA in dashboard cards.
2. Clicking `Sign` opens `'/proposals/:actionId/sign'`.
3. Sign screen renders real proposal data tied to route `actionId`.
4. Signing flow invokes hardware wallet signing with displayed sighash.
5. On successful sign, user sees signature-collected confirmation.
6. Non-pending proposals cannot be signed from the sign route.
7. Not-found, expired session, and disconnected wallet paths show actionable error/recovery UX.
8. Dashboard state grouping remains consistent with canonical statuses.

## Test Plan

### Unit / Component

- Dashboard card CTA rendering by state:
  - pending -> Sign
  - approved -> Broadcast
  - enacted/canceled/expired -> no primary CTA
- Proposal group mapping logic by status.
- Sign screen state machine:
  - loading -> ready -> signing -> success
  - loading/signing/read errors
  - non-pending conflict guard

### Integration / Route

- Navigate from pending card to `'/proposals/:actionId/sign'`.
- Load proposal by `actionId` and render fields.
- Simulate sign success and verify confirmation block appears.
- Simulate status change to non-pending before submit and verify signing blocked.

### Manual QA

- Validate visual parity against target sign view and proposals list references.
- Validate copy-to-clipboard feedback for sighash.
- Validate Trezor prompt text and signing wait state.
- Validate session timeout behavior and recovery.

## Risks and Mitigations

- Risk: proposal payload type heterogeneity may not always fit before/after diff layout.
  - Mitigation: include fallback generic payload details view.
- Risk: stale dashboard data opens sign screen for proposal that already changed state.
  - Mitigation: revalidate proposal status on sign screen load and before signing.
- Risk: confusion between dev sign route and production sign route.
  - Mitigation: document dev route as internal-only and ensure product nav uses production route.
