# Spec: Cancel an Approved Proposal

## Objective

Define the full stack for canceling an Approved proposal: signature collection, cancel transaction broadcast via commit/reveal, and operator-facing UX. A cancel removes a queued update from the ASM before its activation height is reached.

This spec covers the orchestrator backend (DB, application layer, API), the Tauri IPC layer, and the React frontend (new route, new domain, changes to existing screens).

## Scope

### Included

- Cancel proposal lifecycle: creation, signature collection, quorum, broadcast.
- DB schema additions: `target_action_id`, `activation_height` on `proposals`.
- Activation height calculation from ASM authority config + Bitcoin confirmation block.
- New orchestrator endpoint: `POST /proposals/:action_id/cancel`.
- New frontend route `/proposals/:actionId/cancel` with dedicated cancel screen.
- Cancel CTA and activation countdown on the existing `ProposalDetailScreen`.
- Cancel proposals visible in `ProposalsDashboardScreen` with visual differentiation.
- Edge case handling: already-enacted target, duplicate cancel, unsupported authority.

### Not included

- Protocol implementation of `MultisigAction::Cancel` in Alpen/Strata crates (assumed already available).
- Changes to the commit/reveal broadcast pipeline (reused as-is for cancel proposals).
- Proposal expiry enforcement (tracked separately).

## Requirements Alignment

- **PRD §5.2**: Users must see all Approved updates and be able to cancel any of them; copy cancellation signatures; create and broadcast a cancellation transaction or copy the raw hex for manual broadcast.
- **PRD §5.2.2**: Cancel/Approved state does NOT apply to Sequencer Manager or Security Council.
- **Orchestrator remains coordination-only**: collects cancel signatures, tracks cancel proposal lifecycle, reports txids — does not sign or submit transactions.
- **Desktop owns execution**: builds and broadcasts the cancel commit/reveal bundle via Tauri.
- **Signer safety**: explicit confirmation step before hardware wallet signing; payload summary visible before signing.
- **Manual survivability**: user can copy all available cancel signatures and broadcast manually.

## Protocol Recap

A cancel is a standard `SignedPayload` carrying `MultisigAction::Cancel(CancelAction { target_id: UpdateId })`:

```
SignedPayload {
    seqno:      u64,              // new seqno > authority.last_seqno
    action:     MultisigAction::Cancel(CancelAction { target_id }),
    signatures: SignatureSet,     // quorum of ECDSA sigs from the target's authority
}
```

- Sighash domain tag: `strata/admin/cancel` (distinct from all update tags).
- Required signers and threshold are derived from the **target update's authority** — not hardcoded.
- On success: `authority.last_seqno` advances; `next_update_id` does NOT increment.
- The activation window (`lock_period`) is a per-authority value in the ASM state — **not a hardcoded constant**. `activation_height = reveal_confirm_block + lock_period`.
- A cancel tx confirmed after `activation_height` is silently ignored by the ASM.
- Block processing order: activations are processed before incoming txs, so a cancel arriving in the same block as its target's activation height is correctly rejected.

## State Model

Cancel proposals are first-class `Proposal` rows in the same table. They follow the identical lifecycle:

```
Pending → (quorum + confirmed) → Approved → Enacted (cancel broadcast confirmed)
```

The **target** proposal transitions:

```
Approved → Canceled    (when cancel reveal tx is confirmed and ASM reconciliation detects it)
```

A cancel proposal's `action_hex` encodes `MultisigAction::Cancel`; it is distinguishable from update proposals via the new `target_action_id` field.

## DB Schema Changes

**New migration** — `YYYYMMDD_add_cancel_and_activation_fields.sql`:

```sql
ALTER TABLE proposals
  ADD COLUMN target_action_id TEXT REFERENCES proposals(action_id),
  ADD COLUMN activation_height BIGINT;

CREATE INDEX proposals_target_action_id_idx ON proposals(target_action_id);
```

- `target_action_id IS NOT NULL` → this row is a cancel proposal; value is the `action_id` of the Approved target.
- `target_action_id IS NULL` → normal update proposal (all existing rows unaffected).
- `activation_height` → computed and stored when the target proposal's `broadcast_status` reaches `RevealConfirmed`.

## Backend Changes (`orchestrator-be`)

### Domain (`src/domain/proposal.rs`)

Add fields to `Proposal`:

```rust
pub target_action_id: Option<ActionId>,
pub activation_height: Option<u64>,
```

Add derived helpers:

```rust
pub fn is_cancel(&self) -> bool {
    self.target_action_id.is_some()
}
```

### Infrastructure (`src/infrastructure/postgres_repo.rs`)

- Map `target_action_id` and `activation_height` in all `SELECT`, `INSERT`, and `UPDATE` queries.
- Add method to `ProposalRepository` trait:

```rust
async fn find_cancel_for_target(&self, target: &ActionId) -> Result<Option<Proposal>>;
```

### Activation Height (`src/application/proposals.rs`)

When `report_broadcast_progress` transitions the **target** proposal to `RevealConfirmed`:

1. Fetch the confirmation block height of `reveal_txid` from the bitcoin node RPC (`gettransaction` or `getblockheader`).
2. Query the live ASM state for the proposal's authority to read `lock_period` (the number of blocks before an approved update activates). This is a per-authority config value in the ASM — not a hardcoded constant.
3. Compute `activation_height = reveal_confirm_block + lock_period`.
4. Persist `activation_height` on the proposal row via a new `update_activation_height` repository method.

```rust
async fn compute_and_store_activation_height(
    proposal: &Proposal,
    repo: &dyn ProposalRepository,
    bitcoin_rpc: &dyn BitcoinRpc,
    asm_rpc: &dyn AsmRpc,
) -> Result<()>
```

### New Application Function: `create_cancel_proposal`

```rust
pub async fn create_cancel_proposal(
    repo: &dyn ProposalRepository,
    target_action_id: ActionId,
    seq_no: u64,
    signer_pubkey: String,
    signature_hex: String,
) -> Result<Proposal>
```

Logic:

1. Load target proposal; verify `status == Approved`.
2. Verify target's authority is `AlpenAdmin` or `StrataAdmin` — return `400 Bad Request` for other authorities.
3. Check for existing cancel proposal for this target via `find_cancel_for_target`. If found, return it (idempotent — no duplicate).
4. Construct `action_hex` using `MultisigAction::Cancel(CancelAction { target_id: target.update_id })` with the provided `seq_no`.
5. Persist new Proposal with `target_action_id`, `status = Pending`, and the first signature.

### Updates to Existing Application Functions

- `report_broadcast_progress`: after setting `reveal_confirmed` on an update proposal, call `compute_and_store_activation_height`.
- `reconcile_enacted_for_authority`: skip cancel proposals (their enactment changes the target's status, not the cancel's own post-condition).
- `approve_action`, `transition_to_approved`, `claim_broadcast_coordination`: no changes required — cancel proposals flow through these unchanged.

### API Endpoints

**New: `POST /proposals/:action_id/cancel`**

- `:action_id` is the **target** proposal's action_id.
- Body: `{ "seqNo": u64, "signerPubkey": string, "signatureHex": string }`
- Creates or returns existing cancel proposal.
- Returns `200 OK` with the cancel `Proposal` JSON (same shape as existing proposal responses).
- Errors: `404` if target not found; `400` if target not Approved or authority not supported; `409` if a cancel proposal already exists at quorum (no new sigs needed).

**Modified: `GET /proposals/:action_id`**

Extend response to include cancel proposal info when the target has one:

```json
{
  "actionId": "...",
  "status": "approved",
  "activationHeight": 850032,
  "cancelProposal": {
    "actionId": "...",
    "status": "pending",
    "signatures": [...],
    "requiredSignatures": 3
  }
}
```

`cancelProposal` is `null` when no cancel has been initiated.

**Existing: `GET /proposals`**

No changes to default behavior. Cancel proposals appear in the list alongside update proposals. Optional `?target_action_id=<id>` filter may be added in a follow-up if needed.

**Route registration (`src/handlers/mod.rs`):**

```rust
.route("/proposals/:action_id/cancel", post(create_cancel_proposal_handler))
```

## Tauri IPC (`desktop-app/src-tauri`)

Add one new IPC command:

```rust
#[tauri::command]
async fn proposals_create_cancel(
    action_id: String,
    seq_no: u64,
    signer_pubkey: String,
    signature_hex: String,
    state: tauri::State<'_, AppState>,
) -> Result<Proposal, String>
```

This calls `POST /proposals/:action_id/cancel` on the orchestrator. The cancel proposal's subsequent broadcast reuses the existing `proposals_prepare_broadcast` and `proposals_broadcast` commands unchanged.

## Frontend Changes (`desktop-app/src`)

### Types (`src/types/proposal.ts`)

```typescript
type ProposalKind = 'update' | 'cancel'

type Proposal = {
  // …existing fields…
  kind: ProposalKind              // derived from presence of targetActionId
  targetActionId: string | null
  activationHeight: number | null
  cancelProposal: CancelProposalSummary | null
}

type CancelProposalSummary = {
  actionId: string
  status: ProposalStatus
  signatures: ProposalSignature[]
  requiredSignatures: number
}
```

### New Domain: `src/domain/cancel-proposal/`

```
cancel-proposal/
  components/
    cancel-details-card.tsx       # Target summary + cancel payload review before signing
    cancel-sig-collection.tsx     # Sig progress bar, copy-sigs button, paste-and-broadcast
    activation-countdown.tsx      # Shows activation_height and estimated remaining time
  hooks/
    use-cancel-proposal.ts        # Full cancel lifecycle orchestration
  services/
    cancel-proposal-api.ts        # createCancelProposal() typed adapter
  model/
    cancel-proposal-view-model.ts # Maps Proposal → view shape; computes time remaining
```

**`use-cancel-proposal` state machine:**

```
idle
  → loading         (fetch target + existing cancel proposal)
  → confirming      (show cancel-details-card for user review)
  → signing         (hardware wallet signs the cancel sighash)
  → submitting      (POST /proposals/:id/cancel)
  → collecting      (cancel proposal Pending — show sig count, copy-sigs CTA)
  → broadcasting    (quorum reached — delegate to useBroadcastProposal)
  → done | error
```

If a cancel proposal already exists on load, skip to `collecting` or `broadcasting` based on its status.

### New Screen + Route

**File:** `src/screens/cancel-proposal-screen.tsx`

**Route:** `/proposals/:actionId/cancel` — registered in `App.tsx`, wrapped in the existing auth guard.

**Layout:**

```
← Back to proposal

Cancel proposal                            [Authority badge] [Session] [Disconnect]
──────────────────────────────────────────────────────────────────────────────────
  ┌─ Target proposal card ──────────────────────────────────────────────────┐
  │  Action ID: abc…def · Seq: 42            Authority: Alpen Administrator │
  │  Status: Approved                        ⏱ ~13 days 4 hours remaining  │
  └──────────────────────────────────────────────────────────────────────────┘

  [loading skeleton]
  ─ OR ─
  ┌─ Cancel details card ────────────────────────────────────────────────────┐
  │  Cancel seq no: 43                                                        │
  │  Target update ID: <hex>                                                  │
  │  Cancel payload (reviewable hex, copy button)                            │
  │  Signatures: 1 / 3              [Copy all cancel signatures]             │
  │  [Sign with <signer>]        ← signer named per connected vendor         │
  │  ─────────────────────────────────────────────────────────────────────── │
  │  [Broadcast cancel tx]       ← enabled only when quorum reached          │
  └──────────────────────────────────────────────────────────────────────────┘
```

**Guards:**

| Condition | Behavior |
|---|---|
| `proposal.status !== 'approved'` | Redirect to `/proposals/:actionId` |
| Authority is not `alpen_admin` or `strata_admin` | Redirect to `/proposals/:actionId` |
| `activationHeight` already passed (target is now enacted) | Show `AlertBanner`: "This proposal has already been enacted. Cancellation is no longer possible." No actions rendered. |
| Cancel proposal already exists at quorum | Skip directly to broadcast state |

### `ProposalDetailScreen` Updates

**File:** `src/screens/proposal-detail-screen.tsx`

**Add Cancel CTA** — visible when ALL of:
- `proposal.status === 'approved'`
- `proposal.authority` is `alpen_admin` or `strata_admin`
- `activationHeight` not yet passed (or unknown, to avoid false negatives)

```
[Cancel this proposal]  →  navigates to /proposals/:actionId/cancel
```

**Add `ActivationCountdown`** — shown below broadcast status when `activationHeight` is set:

```
⏱ Activation in block 850,032 · ~13 days 4 hours
```

**Add in-progress cancel banner** — shown when `cancelProposal` is non-null:

```
⚠  Cancellation in progress — 1 / 3 cancel signatures collected.
   [View cancel →]    ← links to /proposals/:actionId/cancel
```

### `ProposalsDashboardScreen` Updates

**File:** `src/screens/proposals-dashboard-screen.tsx`

Cancel proposals (`kind === 'cancel'`) appear in existing status-based sections with visual differentiation:

- Status badge label: `"Cancel · Pending"` / `"Cancel · Quorum reached"` / `"Cancel · Past"`.
- Subtitle: `"Cancels proposal <targetActionId shortened>"`.
- Clicking a cancel proposal navigates to `/proposals/:targetActionId/cancel` (not to a generic detail page).

## Edge Cases

| Scenario | Behavior |
|---|---|
| `activation_height` passed when cancel screen loads | `AlertBanner` "Enacted — cancellation no longer possible." No actions shown. |
| Cancel proposal already exists for this target | Return existing cancel proposal from `POST /cancel` (idempotent). Frontend shows existing state. |
| User is not a signer on the authority | Sign CTA is disabled. User can still copy existing cancel signatures for manual aggregation. |
| Target authority is Sequencer Manager or Security Council | Backend returns `400`. Frontend does not show Cancel CTA for these authorities. |
| Cancel reaches quorum before target `activation_height` | "Broadcast cancel tx" CTA appears; reuses existing broadcast pipeline. |
| Cancel tx and target activation in the same block | Protocol processes activations before incoming txs — cancel is rejected. Backend reconcile on next poll detects `Enacted` on target; UI updates accordingly. |
| Two signers create cancel proposals concurrently | Second `POST /cancel` returns the existing cancel proposal (idempotency guard in `create_cancel_proposal`). |

## Critical Files

| File | Change |
|---|---|
| `orchestrator-be/migrations/YYYYMMDD_add_cancel_and_activation_fields.sql` | New — `target_action_id`, `activation_height` |
| `orchestrator-be/src/domain/proposal.rs` | Add `target_action_id`, `activation_height`, `is_cancel()` |
| `orchestrator-be/src/application/traits.rs` | Add `find_cancel_for_target`, `update_activation_height` to `ProposalRepository` |
| `orchestrator-be/src/infrastructure/postgres_repo.rs` | Map new columns; implement new repo methods |
| `orchestrator-be/src/application/proposals.rs` | `create_cancel_proposal`, `compute_and_store_activation_height` |
| `orchestrator-be/src/handlers/proposals.rs` | New `create_cancel_proposal_handler`; extend `get_proposal` response |
| `orchestrator-be/src/handlers/mod.rs` | Register `POST /proposals/:action_id/cancel` |
| `desktop-app/src-tauri/src/commands/proposals.rs` | New `proposals_create_cancel` IPC command |
| `desktop-app/src/types/proposal.ts` | Add `kind`, `targetActionId`, `activationHeight`, `cancelProposal` |
| `desktop-app/src/domain/cancel-proposal/` | New domain (all files) |
| `desktop-app/src/screens/cancel-proposal-screen.tsx` | New screen |
| `desktop-app/src/screens/proposal-detail-screen.tsx` | Cancel CTA, activation countdown, in-progress banner |
| `desktop-app/src/screens/proposals-dashboard-screen.tsx` | Visual differentiation for cancel proposals |
| `desktop-app/src/App.tsx` | Register `/proposals/:actionId/cancel` route |

## Verification

**Backend:**

```bash
cargo test -p orchestrator-be
```

Unit tests:
- `create_cancel_proposal` happy path.
- `create_cancel_proposal` returns existing cancel proposal (idempotency).
- `create_cancel_proposal` returns `400` when target is not `Approved`.
- `create_cancel_proposal` returns `400` for unsupported authority (SequencerManager, SecurityCouncil).
- `activation_height` is persisted correctly after `RevealConfirmed` using mock ASM `lock_period`.

Integration test:
- Full flow: create update proposal → approve → broadcast → cancel proposal created → cancel reaches quorum → cancel broadcast → target transitions to `Canceled`.

**Frontend:**

```bash
cd desktop-app && npm run build
```

Manual E2E flow:
1. Connect wallet → authenticate → navigate to an Approved proposal (AlpenAdmin or StrataAdmin).
2. Cancel CTA visible; activation countdown shown.
3. Navigate to `/proposals/:actionId/cancel` — cancel details card renders with correct target summary.
4. Sign with the connected signer — cancel sig collected; sig count updates.
5. Copy all cancel signatures to clipboard.
6. When quorum reached — "Broadcast cancel tx" CTA appears; broadcast reuses existing pipeline.
7. After broadcast — target proposal shows `Canceled` status in dashboard.
8. Enacted target — Cancel CTA hidden; cancel screen shows "no longer possible" banner.
9. Cancel proposals appear in dashboard with "Cancel ·" badge prefix; clicking navigates to cancel screen.
