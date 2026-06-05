# Spec: Proposal Lifecycle — Expiry, Countdown, and Status Completion

## Objective

Close the PRD §3–4 gaps identified in the lifecycle audit (`docs/analysis/proposal_status_lifecycle_audit.md`).
Specifically: expose `created_at` through the full stack, enforce the 7-day expiry rule, surface countdown
UI in all relevant screens, add a quorum-reached broadcast prompt, and add full end-to-end lifecycle test
coverage via the integration harness.

## Scope

### Included

- Expose `created_at` from the DB through domain → DTO → frontend type.
- Derive and expose `expires_at_ms` in the Tauri DTO layer (7-day offset from `created_at`).
- Lazy expiry enforcement in `get_proposal` and `list_proposals` handlers.
- `<PendingExpiryCountdown>` component rendered on dashboard list rows and the proposal detail page.
- Expiry urgency state (red/orange warning when < 24 h remain).
- Expiry countdown on the manual signing screen.
- Verify and, if missing, add the "Send" button for quorum-reached proposals not yet confirmed.
- Post-approve quorum broadcast prompt.
- Harness integration test covering the complete proposal state machine.

### Not included

- Backend scheduled expiry job (deferred to a follow-up once MVP is stable).
- Changes to SPS-50/51/65 semantics.
- New proposal states beyond the current canonical set.

---

## Task 1 — Expose `created_at` through the full stack

### Problem

`created_at` exists in the DB but is not selected, not present on the `Proposal` domain struct, not in the
Tauri DTO, and not in the frontend type. This blocks both countdown UI and expiry enforcement.

### Backend changes

**`orchestrator-be/src/domain/proposal.rs`**

Add to the `Proposal` struct:

```rust
pub created_at: DateTime<Utc>,
```

**`orchestrator-be/src/infrastructure/postgres_repo.rs`**

Add `created_at` to `SELECT_PROPOSAL_COLS` so it is populated when mapping rows.

### Tauri DTO changes

**`desktop-app/src-tauri/src/commands/proposals.rs` — `ProposalDto`**

Add two fields:

```rust
pub created_at_ms: u64,   // Unix epoch ms
pub expires_at_ms: u64,   // created_at_ms + 7 * 24 * 3600 * 1000
```

Compute `expires_at_ms` in the `From<Proposal>` impl; do not add a DB column for it.

### Frontend type changes

**`desktop-app/src/api/proposals.ts` — `Proposal`**

```typescript
createdAtMs: number
expiresAtMs: number
```

### Acceptance criteria

1. `GET /proposals` and `GET /proposals/:id` responses include `created_at` in the JSON body.
2. The Tauri DTO contains `createdAtMs` and `expiresAtMs` for every proposal.
3. Frontend `Proposal` type has both fields and TypeScript compilation passes.

---

## Task 2 — Lazy expiry enforcement on read

### Problem

Pending proposals never transition to `Expired` unless a client manually reports it. A user can open a
proposal that silently passed its deadline and the status still reads `Pending`.

### Design

Implement lazy expiry in two handlers:

- `get_proposal` in `orchestrator-be/src/handlers/`
- `list_proposals` in `orchestrator-be/src/handlers/`

Before returning a pending proposal, check:

```
if proposal.status == Pending && proposal.created_at + 7 days < Utc::now()
    → call application::proposals::expire(proposal_id)
    → reload and return the updated record
```

The `expire` transition should reuse or mirror the existing `application/proposals.rs` transition that moves
a proposal to `Expired`, emitting any relevant domain events.

### Acceptance criteria

1. A `pending` proposal whose `created_at` is > 7 days ago is returned as `expired` from both endpoints.
2. The transition is persisted: a subsequent read also returns `expired`.
3. No background task or cron job is required — this is a read-path side effect only.
4. Proposals in any other status are unaffected.

---

## Task 3 — `<PendingExpiryCountdown>` component

### Problem

The PRD (§3) requires users to see how much time remains before a pending proposal expires. Nothing is
currently shown.

### Component spec

**File:** `desktop-app/src/components/pending-expiry-countdown.tsx`

Props:

```typescript
interface PendingExpiryCountdownProps {
  expiresAtMs: number
}
```

Behaviour:

- Display a human-readable countdown: `"Expires in 2 d 14 h"`, `"Expires in 5 h 32 m"`, `"Expires in 43 m"`.
- Update every 60 seconds via `setInterval`.
- When < 24 h remain: switch label colour to orange/amber and prepend `"⚠ Expiring soon — "`.
- When < 1 h remain: switch colour to red.
- When `expiresAtMs <= Date.now()`: display `"Expired"` in red (defensive fallback; server should have already
  transitioned the status).
- Model after the existing `activation-countdown.tsx` for interval management and cleanup.

### Integration points

| Location | File | When to render |
|----------|------|----------------|
| Dashboard list row | `proposals-dashboard.tsx` | `status === 'pending'` |
| Proposal detail page | `proposal-detail.tsx` or `proposal-detail-screen.tsx` | `status === 'pending'` |
| Manual signing screen | `manual-proposal-screen.tsx` | `status === 'pending'` |

### Acceptance criteria

1. Pending proposals show a live countdown on the dashboard and detail page.
2. Countdown turns amber at < 24 h and red at < 1 h.
3. The component unmounts cleanly with no timer leaks.
4. Non-pending proposals do not render the countdown.
5. The manual signing screen shows the same countdown so offline signers see the deadline.

---

## Task 4 — Verify and complete the "Send" button for quorum-reached proposals

### Problem

The PRD (§3.2.3.1) requires that a proposal which has reached quorum but has not yet been confirmed on
Bitcoin always shows a "Send" button. The audit flagged this as unverified.

### Verification step

Open `desktop-app/src/screens/proposal-detail.tsx` (or equivalent) and confirm whether the CTA rendered
when `hasReachedQuorum(proposal) && proposal.status === 'pending'` is a broadcast/send action button
accessible without navigating to a separate screen.

### If the button is missing or hidden

Add a primary `Send` CTA to the proposal detail view when `hasReachedQuorum && status === 'pending'`:

- Label: `"Broadcast transaction"`
- Action: invoke the existing broadcast flow (`proposals_broadcast` or `proposals_prepare_broadcast`).
- The button must be visible above the fold — not inside a secondary menu or tab.

### Acceptance criteria

1. A proposal with `hasReachedQuorum === true` and `status === 'pending'` shows a `"Broadcast transaction"`
   button on the detail page without any additional navigation.
2. A proposal with `hasReachedQuorum === false` does not show the button.
3. Proposals in terminal states (`enacted`, `canceled`, `expired`) do not show the button.

---

## Task 5 — Post-approve quorum broadcast prompt

### Problem

The PRD (§3.2.3, SHOULD) requires that the signer whose approval causes quorum to be reached is immediately
offered the option to broadcast the transaction.

### Design

In the approval flow (wherever `approveProposal` is invoked):

```typescript
const result = await approveProposal(actionId, signature)
if (result.status === 'approved' || hasReachedQuorum(result)) {
  // show prompt
}
```

The prompt can be an inline banner or modal:

- Title: `"Quorum reached"`
- Body: `"This proposal now has enough signatures. Do you want to broadcast the Bitcoin transaction now?"`
- Primary action: `"Broadcast now"` → navigate to/invoke broadcast flow.
- Secondary action: `"Later"` → dismiss and stay on detail page.

### Acceptance criteria

1. When the current signer's approval completes quorum, the prompt is shown before any navigation.
2. Choosing "Broadcast now" initiates the broadcast flow.
3. Choosing "Later" dismisses the prompt; the updated proposal state is visible on the detail page.
4. If quorum was already reached before this signer (i.e. `status === 'approved'` on load), the prompt is
   not shown again on subsequent visits.

---

## Task 6 — End-to-end lifecycle harness tests

### Objective

Provide integration tests that exercise the complete proposal state machine — from creation through all
terminal states — using the existing e2e/integration test harness.

### Test file location

`e2e-tests/src/` (or `orchestrator-be/tests/` if backend-only integration tests fit there better; match the
existing pattern in `docs/specs/e2e-tests-workspace-integration.md`).

### Scenarios to cover

Each scenario drives the system through a realistic flow and asserts the final state and any side-effects.

#### Scenario A — Happy path: Pending → Approved → Enacted

1. Create a proposal. Assert `status = pending`, `created_at` is set, `expires_at` is 7 days in the future.
2. Submit signatures until quorum is reached. Assert `status = approved`.
3. Simulate ASM enactment event. Assert `status = enacted`.
4. Fetch via list and detail endpoints; confirm terminal state.

#### Scenario B — Pending → Expired (lazy enforcement)

1. Create a proposal.
2. Manipulate the DB row's `created_at` to be 8 days in the past (or mock the clock).
3. Call `GET /proposals/:id`. Assert the response returns `status = expired`.
4. Verify the transition is persisted (second read also returns `expired`).

#### Scenario C — Pending → Approved → Canceled

1. Create a proposal and reach quorum (`status = approved`).
2. Simulate a cancel proposal being enacted. Assert original proposal `status = canceled`.

#### Scenario D — Signature rejection / quorum not reached before expiry

1. Create a proposal.
2. Submit fewer signatures than required.
3. Expire the proposal (via clock manipulation or direct DB update to `created_at - 8 days`).
4. Call `GET /proposals`. Assert `status = expired`; signature count reflects partial collection.

#### Scenario E — Duplicate signature guard

1. Create a proposal.
2. Submit a signature for signer X.
3. Attempt to submit a second signature for the same signer X.
4. Assert the backend rejects the duplicate (error response) and the signature count is unchanged.

#### Scenario F — Dashboard state grouping contract

1. Create multiple proposals covering all statuses: `pending`, `approved`, `enacted`, `canceled`, `expired`.
2. Call the list endpoint. Assert each proposal appears exactly once with the correct status.
3. Assert `created_at` and `expires_at_ms` are present on all proposals.

### Harness requirements

- Tests MUST run against a real Postgres instance (no mocks) — consistent with the existing integration
  test policy.
- Clock manipulation for expiry tests: prefer adjusting `created_at` directly via a test helper rather than
  introducing a global time-mock that could affect other tests.
- Each scenario should be fully isolated: create its own proposals and clean up after itself (or use a
  separate test DB schema/transaction rollback).

### Acceptance criteria

1. All six scenarios pass in CI against a real Postgres instance.
2. No `created_at` / `expires_at` assertion relies on approximate timing; use DB-level manipulation.
3. Tests are grouped under a `proposal_lifecycle` module and can be run independently with
   `cargo test -p <test-crate> proposal_lifecycle`.

---

## Summary

| Task | PRD req | Risk | Effort |
|------|---------|------|--------|
| 1 — Expose `created_at` full stack | §3 time remaining | High | Small |
| 2 — Lazy expiry on read | §3.3 7-day TTL | High | Small |
| 3 — `<PendingExpiryCountdown>` component | §3 time remaining | High | Medium |
| 4 — "Send" button for quorum-reached | §3.2.3.1 | Medium | Small (verify first) |
| 5 — Post-approve quorum prompt | §3.2.3 (SHOULD) | Low | Small |
| 6 — Lifecycle harness tests | — | High | Medium |

**Suggested order:** 1 → 2 → 3 → 4 → 5 → 6. Tasks 1–2 are blockers for 3; all implementation tasks should
be complete before 6 is written so the tests cover the final behaviour.
