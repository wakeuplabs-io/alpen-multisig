# Proposal Status Lifecycle — Implementation Audit

> **Type:** Implementation audit (not a product spec). **Supersedes:** nothing — use [`proposal-lifecycle-expiry-and-status-completion.md`](../../specs/proposal-lifecycle-expiry-and-status-completion.md) for the delivery spec. **Backlog:** [US-EXP](../deferred-backlog.md) (expiry). **Story context:** [`story-map.md`](../../3-stories/story-map.md).

**Date:** 2026-06-04  
**Branch:** `feat/manual-proposal-flow`  
**Scope:** PRD §3–4 — Pending, Expired, and Past proposal states

---

## 1. PRD Requirements (Reference)

| # | Requirement | Priority |
|---|-------------|----------|
| 3 | See all **Pending** updates (proposed, not yet quorum, not confirmed on bitcoin) | MUST |
| 3 | Show **time remaining** before a Pending update expires | MUST |
| 3 | Show **signature count** (received / required) for each Pending update | MUST |
| 3.1 | Pending updates are offchain and visible only to multisig signers | MUST |
| 3.2 | Produce an approval signature for any Pending update | MUST |
| 3.2.1 | Copy all available approval signatures to clipboard | MUST |
| 3.2.2 | Create approval transaction, paste quorum of signatures, broadcast to bitcoin (via RPC or raw tx copy) — UI similar to "send" screen | MUST |
| 3.2.3 | User whose signature reaches quorum SHOULD be offered to broadcast the bitcoin transaction immediately | SHOULD |
| 3.2.3.1 | Pending updates that reached quorum but not yet confirmed MUST have a "Send" button | MUST |
| 3.3 | Pending updates expire after **7 days** if not approved | MUST |
| 3.3.1 | **Expired** updates are offchain and visible only to multisig signers | MUST |
| 4 | See all **Past** updates (enacted, canceled, or expired) | MUST |

---

## 2. State Model

### 2.1 Backend — `ProposalStatus` enum

**File:** `orchestrator-be/src/domain/proposal.rs:62–73`

```rust
pub enum ProposalStatus {
    Pending,   // offchain, collecting signatures — comment says "Expires after 7 days"
    Approved,  // threshold reached, ready to broadcast
    Enacted,   // ASM applied the change
    Canceled,  // canceled during approved window
    Expired,   // expired before reaching threshold
}
```

All five states are defined. The "Expires after 7 days" note on `Pending` is a **comment only** — no code enforces it.

### 2.2 Frontend — `ProposalStatus` type

**File:** `desktop-app/src/api/proposals.ts:6`

```typescript
export type ProposalStatus = 'pending' | 'approved' | 'enacted' | 'canceled' | 'expired'
```

Matches the backend enum 1:1.

---

## 3. What Is Implemented

### 3.1 State display

| Surface | File | Status |
|---------|------|--------|
| Status badge (5 color-coded states) | `proposal-detail.tsx:54–61` | ✅ Complete |
| Pending / Past tabs on dashboard | `proposals-dashboard.tsx:34–150` | ✅ Complete |
| Quorum progress bar (sigs received / required) | `proposals-dashboard.tsx:110–113` | ✅ Complete |
| Terminal state guard (blocks actions on enacted/canceled/expired) | `proposal-detail.tsx:115–118` | ✅ Complete |
| `expired` bucket in "Past" tab | `proposals-dashboard-screen.tsx:79–91` | ✅ Complete |

### 3.2 Signature flows

| Flow | File | Status |
|------|------|--------|
| Export signatures as JSON bundle (copy to clipboard) | `proposal-detail-screen.tsx:145–149` | ✅ Complete |
| Import signatures from JSON bundle (paste) | `proposal-detail-screen.tsx:37–71` | ✅ Complete |
| Add approval signature via Tauri command | `invoke.rs` — `proposals_approve` | ✅ Complete |
| Manual signature aggregation (offline flow) | `manual-proposal-screen.tsx` | ✅ Complete |

### 3.3 Broadcast flows

| Flow | File | Status |
|------|------|--------|
| Prepare broadcast (commit + reveal txs) | `proposals_prepare_broadcast` | ✅ Complete |
| Broadcast via app's bitcoin RPC | `proposals_broadcast` | ✅ Complete |
| Copy raw tx to clipboard for external broadcast | `proposals_prepare_broadcast_manual` / `proposals_broadcast_manual` | ✅ Complete |
| Report broadcast progress (commit, reveal, confirmation) | `proposals_report_broadcast` | ✅ Complete |
| Resubmit reveal on failure | `proposals_resubmit_reveal` | ✅ Complete |
| Activation countdown (cancel proposals only) | `activation-countdown.tsx` | ✅ Complete |

### 3.4 Backend lifecycle transitions

| Transition | Where | Status |
|------------|-------|--------|
| `create` → `Pending` | `application/proposals.rs:32` | ✅ |
| `Pending` → `Approved` (quorum reached) | `application/proposals.rs:115` | ✅ |
| `Approved` → `Enacted` (ASM enactment detected) | `application/proposals.rs:344` | ✅ |
| `Approved` → `Canceled` (cancel proposal enacted) | `application/proposals.rs` | ✅ |
| Any → `Expired` (client-reported) | `application/proposals.rs:532` | ⚠️ Partial (client-driven only) |

---

## 4. What Is Missing

### 4.1 Critical — Time remaining / expiry display (PRD §3)

**Problem:** The PRD says users MUST see how much time is left before a Pending update expires. No countdown or expiry date is shown anywhere in the UI.

**Root cause — missing `created_at` across all layers:**

| Layer | File | Gap |
|-------|------|-----|
| Domain struct | `domain/proposal.rs:90–110` | `created_at` field absent from `Proposal` struct |
| DB query | `postgres_repo.rs:~60` — `SELECT_PROPOSAL_COLS` | `created_at` column exists in DB but is not selected |
| Tauri DTO | `commands/proposals.rs:86–105` — `ProposalDto` | No `created_at` / `expires_at` field |
| Frontend type | `api/proposals.ts:27–48` — `Proposal` | No `createdAt` or `expiresAtMs` field |
| UI | `proposals-dashboard.tsx`, `proposal-detail.tsx` | No countdown component for pending proposals |

**What to add:**
1. Add `created_at: DateTime<Utc>` to `Proposal` struct and include it in `SELECT_PROPOSAL_COLS`.
2. Add `expires_at_ms: u64` to `ProposalDto` (computed as `created_at + 7 days` in the Tauri layer).
3. Add `expiresAtMs: number` to the frontend `Proposal` type.
4. Build a `<PendingExpiryCountdown>` component (similar to `activation-countdown.tsx`) and render it on the dashboard list rows and proposal detail page for `status === 'pending'`.

---

### 4.2 Critical — Automatic expiry enforcement (PRD §3.3)

**Problem:** The 7-day TTL is documented in a comment but never enforced. Pending proposals stay `Pending` forever unless a client explicitly calls `reportBroadcastProgress` with `proposal_status: "expired"` — which never happens automatically.

**Options:**

| Option | Complexity | Notes |
|--------|-----------|-------|
| **A — Backend scheduled job** | Medium | A periodic task (e.g., every hour) queries `WHERE status = 'pending' AND created_at < NOW() - INTERVAL '7 days'` and batch-transitions to `Expired`. Most reliable. |
| **B — DB-level view / computed column** | Low | A Postgres view that returns effective status. Keeps stored status clean, no write logic needed. |
| **C — Frontend reconciliation on load** | Low | When the frontend loads a pending proposal, if `expiresAtMs < Date.now()` it calls `reportBroadcastProgress` to mark it expired. Simple but only fires on user interaction. |
| **D — Handler-level lazy check** | Low-Medium | `get_proposal` and `list_proposals` handlers check age and auto-expire inline. No background task needed, but adds latency to reads. |

**Recommendation:** Option D (lazy expiry on read) for MVP — zero infrastructure overhead; combine with Option A later for correctness guarantees when the backend is unattended.

---

### 4.3 Important — "Send" button for quorum-reached proposals (PRD §3.2.3.1)

**Problem:** The PRD requires that proposals which have reached quorum but are not yet confirmed on bitcoin show a "Send" button allowing the user to broadcast the approval transaction.

**Current state:** When `hasReachedQuorum(proposal) === true` and `status === 'pending'`, the dashboard groups the proposal under `quorumReached` and the detail view changes to a broadcast flow, but it is not clear from the audit whether the "Send" button is rendered without navigating to the manual broadcast screen. Needs verification.

**File to check:** `proposal-detail.tsx` — confirm the CTA rendered when `hasReachedQuorum && status === 'pending'`.

---

### 4.4 Minor — Quorum-reached notification / prompt (PRD §3.2.3)

**Problem:** The SHOULD requirement says the user whose signature causes quorum to be reached should be offered the option of broadcasting immediately (or declining).

**Current state:** No "You reached quorum — broadcast now?" prompt after `approveProposal` returns. The detail view refreshes and the user lands back on the updated state, but no proactive offer is made.

**Suggestion:** After `approveProposal`, if the returned proposal has `status === 'approved'` or `hasReachedQuorum === true`, show a modal/inline prompt offering to broadcast.

---

### 4.5 Minor — Expiry warning near deadline

**Suggestion:** When a pending proposal has less than 24 hours remaining, the expiry countdown should turn red/orange and display a warning label ("Expiring soon"). This is not in the PRD but is high-value UX.

---

### 4.6 Minor — Manual flow has no expiry context

**File:** `manual-proposal-screen.tsx`

When a user collects signatures offline for a pending proposal, they have no indication that the proposal will expire in X days. If they take longer than the deadline, they waste effort. Display the expiry date/countdown during offline signing.

---

## 5. Summary Matrix

| PRD Req | Implemented | Gap | Risk |
|---------|-------------|-----|------|
| See all Pending updates | ✅ Dashboard "Pending" tab | — | Low |
| Time remaining before expiry | ❌ No countdown anywhere | `created_at` not exposed | **High** |
| Signature count (received / required) | ✅ Progress bar on dashboard | — | Low |
| Offchain visibility only | ✅ Auth-gated backend | — | Low |
| Produce approval signature | ✅ Approve command + UI | — | Low |
| Copy signatures to clipboard | ✅ Export JSON bundle | — | Low |
| Import sigs + broadcast (RPC or raw tx copy) | ✅ Manual proposal screen | — | Low |
| "Send" button when quorum reached, not confirmed | ⚠️ Probably present — verify | See §4.3 | Medium |
| Prompt user who reached quorum to broadcast | ❌ No post-approve prompt | See §4.4 | Low (SHOULD) |
| 7-day expiry enforced | ❌ No enforcement at any layer | See §4.2 | **High** |
| Expired updates visible in Past tab | ✅ Expired bucket in Past tab | — | Low |
| Past updates (enacted / canceled / expired) | ✅ Past tab groups all three | — | Low |

---

## 6. Suggested Implementation Order

1. **Expose `created_at`** through the full stack (domain → DB query → DTO → frontend type). This unblocks both the countdown UI and lazy expiry enforcement.
2. **Add `<PendingExpiryCountdown>`** component to dashboard list rows and proposal detail page.
3. **Implement lazy expiry on read** in `get_proposal` / `list_proposals` handlers.
4. **Verify "Send" button** on quorum-reached pending proposals (§4.3).
5. **Add post-approve quorum prompt** (§4.4) — low effort, high UX value.
6. **(Later) Backend scheduled expiry job** for correctness when no user is logged in.
