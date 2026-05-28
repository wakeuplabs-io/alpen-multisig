# Spec: Block Payouts UI Mock — Payout Administrator

## Objective

Implement a frontend-only UI mock (hardcoded data, no real backend calls) covering the full `block_payouts` management flow for the **Payout Administrator multisig**. The goal is to validate UX and component design before wiring real Tauri IPC and orchestrator endpoints.

PRD source: section 2–4 of the Payout Administrator multisig requirements.

## Scope

### Included

- New route `/block-payouts` gated to `payout-administrator` authority
- `BlockPayoutsScreen` entry point
- `block-payouts` domain with components, hooks, types, and mock data
- Pending transactions list with all metadata and actions
- Past transactions list with rebroadcast and copy actions
- Create block payout modal (multi-step flow)
- Sign modal with quorum-completion behavior
- Import/paste signatures modal with error handling
- Conflicting-input detection and informational UI
- Mock data covering all states defined in this spec

### NOT included

- Real Tauri IPC calls
- Real orchestrator or backend integration
- Cryptographic validation of signatures or false claim proofs
- Real Bitcoin broadcast
- Automatic expiration in real-time (expiry is static/visual only)

## Design Decisions

### Domain-driven structure

Follows the existing pattern established in `domain/proposals-dashboard/`, `domain/create-proposal/`, etc. Screens are thin route entry points; all logic lives in the domain folder.

### Mock data as the source of truth

All state is initialized from `block-payouts.mock.ts` and managed in `use-block-payouts.ts` via React state. Actions (sign, import, create, rebroadcast) mutate local state only.

### Authority gating

The route must only render for users authenticated as `payout-administrator`. Use the existing `RequireAuth` wrapper with an authority check, consistent with how other protected routes are handled.

## File Organization

```
desktop-app/src/
├── screens/
│   └── block-payouts-screen.tsx              ← route entry point (thin)
└── domain/
    └── block-payouts/
        ├── components/
        │   ├── block-payouts-dashboard.tsx         ← tabs + "Block payouts" button
        │   ├── pending-transactions-list.tsx        ← Pending tab content
        │   ├── pending-transaction-card.tsx         ← single Pending tx card
        │   ├── past-transactions-list.tsx           ← Past tab content
        │   ├── past-transaction-row.tsx             ← single Past tx row
        │   ├── create-block-payout-modal.tsx        ← 4-step creation flow
        │   ├── sign-block-payout-modal.tsx          ← sign confirmation flow
        │   ├── paste-signatures-modal.tsx           ← import signatures flow
        │   └── conflicting-input-icon.tsx           ← ⓘ icon + tooltip
        ├── hooks/
        │   └── use-block-payouts.ts                ← state + mock actions
        └── model/
            ├── block-payouts.types.ts
            └── block-payouts.mock.ts
```

**Route addition in `App.tsx`:**

```tsx
<Route path="/block-payouts" element={
  <RequireAuth authority="payout-administrator">
    <BlockPayoutsScreen />
  </RequireAuth>
} />
```

## Types

```typescript
// block-payouts.types.ts

export type BlockPayoutInput = {
  outpoint: string        // 'txid:vout'
  amount: number          // satoshis
  claimId: string
  isConflicting: boolean  // true if this outpoint appears in another Pending tx
}

export type PendingBlockPayoutTx = {
  id: string
  inputs: BlockPayoutInput[]
  signaturesReceived: number
  signaturesRequired: number
  signedByCurrentUser: boolean
  expiresAt: Date         // 4 days from first signature
  createdAt: Date
  rawTx: string           // hex-encoded raw transaction
  signatures: string[]    // hex-encoded signatures collected so far
}

export type PastBlockPayoutTx = {
  id: string
  status: 'unconfirmed' | 'confirmed'
  broadcastAt: Date
  blockTimestamp?: Date   // only present when status === 'confirmed'
  rawTx: string
}

export type CreateBlockPayoutDraft = {
  inputs: BlockPayoutInput[]
  feeRateSatPerVb: number
}
```

## Mock Data

`block-payouts.mock.ts` must cover every UI state defined in this spec:

| Fixture | Purpose |
|---------|---------|
| `PENDING_SHARED_INPUT_A` | Pending tx sharing 1 input with `PENDING_SHARED_INPUT_B` — triggers conflict banner and ⓘ icon |
| `PENDING_SHARED_INPUT_B` | Pending tx sharing 1 input with `PENDING_SHARED_INPUT_A` — triggers conflict banner and ⓘ icon |
| `PENDING_ALREADY_SIGNED` | Pending tx with `signedByCurrentUser: true` — shows checkmark + "Signed" |
| `PENDING_EXPIRING_SOON` | Pending tx with `expiresAt` < 4 hours from now — tests urgency visual |
| `PAST_UNCONFIRMED` | Past tx with `status: 'unconfirmed'` — shows Rebroadcast + Copy buttons |
| `PAST_CONFIRMED` | Past tx with `status: 'confirmed'` and `blockTimestamp` set — shows timestamp only |

## Component Behavior

### `block-payouts-dashboard.tsx`

- Two tabs: **Pending** and **Past**
- **"Block payouts"** button in the top-right corner, opens `create-block-payout-modal`
- Passes pending list to `pending-transactions-list` and past list to `past-transactions-list`

### `pending-transactions-list.tsx`

- If two or more Pending txs share any input, renders a banner above the list:
  > "Some pending transactions share inputs. See the ⓘ icons for details."
- Renders one `pending-transaction-card` per tx

### `pending-transaction-card.tsx`

Displays per transaction:

| Field | Detail |
|-------|--------|
| Transaction ID | Truncated; copy-to-clipboard icon |
| Time remaining | Countdown to `expiresAt`; highlight in red when < 24 hours |
| Signatures | `N / M signatures` with a progress bar |
| Inputs | List of `outpoint` strings; if `isConflicting` → renders `conflicting-input-icon` |
| Signature status | If `signedByCurrentUser`: checkmark + **"Signed"** label. Otherwise: **"Sign"** button |

Actions (rendered as icon buttons or a dropdown):
- **Sign** → opens `sign-block-payout-modal` (hidden when `signedByCurrentUser`)
- **Paste signatures** → opens `paste-signatures-modal`
- **Export** → triggers download of `rawTx` as a `.txt` file
- **Copy signatures** → copies `signatures[]` joined by newline to clipboard

### `conflicting-input-icon.tsx`

- Renders an ⓘ icon next to the outpoint string
- On hover/focus, shows tooltip: *"This input is included in multiple Pending transactions."*

### `past-transactions-list.tsx`

Renders a table with columns: **TX ID | Status | Block timestamp | Actions**

### `past-transaction-row.tsx`

- **Confirmed**: shows block timestamp, no action buttons
- **Unconfirmed**:
  - **Rebroadcast** button → mock action, shows success toast: *"Transaction rebroadcast successfully."*
  - **Copy to clipboard** button → copies `rawTx` to clipboard

### `sign-block-payout-modal.tsx`

Steps:
1. Summary view: TX ID, inputs count, current signatures / required
2. Explicit confirmation copy: *"You are about to sign this block_payouts transaction. This action cannot be undone."*
3. **Sign** button → mock-signs (increments `signaturesReceived`, sets `signedByCurrentUser: true`)
4. If `signaturesReceived === signaturesRequired` after signing:
   - Show toast: *"Quorum reached — transaction broadcast to Bitcoin network."*
   - Move tx from Pending to Past with `status: 'unconfirmed'`

### `paste-signatures-modal.tsx`

- Textarea for pasting one or more signatures (one per line)
- Mock validation: strings shorter than 64 characters are treated as invalid
- On submit:
  - If 1 invalid signature:
    ```
    Warning: Invalid signature. Please provide a valid signature.
    <invalid signature>
    ```
  - If multiple invalid signatures:
    ```
    Warning: Invalid signatures. Please provide valid signatures.
    <list of invalid signatures, one per line>
    ```
  - Error message includes a **"Copy error"** button
  - Valid signatures are added to `signatures[]` and `signaturesReceived` is incremented

### `create-block-payout-modal.tsx`

Multi-step modal:

**Step 1 — Load false claim reports**
- Textarea for pasting raw report JSON, or file input for upload
- Mock validation: any report where `proof` field is absent or empty is marked invalid with inline error
- Derives `BlockPayoutInput[]` from valid reports
- Already-spent outpoints are filtered out (mock: mark any outpoint ending in `:0` as spent)

**Step 2 — Review inputs**
- Displays derived input list; each row has a ✕ button to remove it
- Shows total input count
- If input count exceeds 50 (mock limit standing in for Bitcoin Core standardness), shows critical error banner:
  > "Your transaction exceeds the size limit, please remove one or more inputs to reduce its size."
- **Confirm** button is disabled while the critical error is shown

**Step 3 — Set fee rate**
- Numeric input: sat/vB, step 0.1, min 0.1, max 10,000
- Informational note: *"Fee paid from Admin Wallet · Change returned to first unused change address."*

**Step 4 — Confirm**
- Summary: input count, fee rate
- **Confirm** button:
  - Creates a new `PendingBlockPayoutTx` with `signaturesReceived: 1`, `signedByCurrentUser: true`
  - Adds it to the Pending list
  - Closes the modal

## Mock Actions Summary

All actions operate on React state initialized from `block-payouts.mock.ts`. No Tauri calls are made.

| Action | State mutation |
|--------|---------------|
| Sign | `signaturesReceived++`, `signedByCurrentUser = true`; if quorum → move to Past |
| Paste valid signatures | `signaturesReceived += validCount`, push to `signatures[]` |
| Export | Trigger file download, no state change |
| Copy signatures | Write to clipboard, no state change |
| Rebroadcast | Show toast, no state change |
| Copy raw tx | Write to clipboard, no state change |
| Create tx | Append new `PendingBlockPayoutTx` to Pending list |
| Remove input (create flow) | Remove from draft `inputs[]` |

## What Comes Next (out of scope here)

- Replace mock data with real Tauri IPC calls to the orchestrator
- Real false claim proof validation (Rust/Tauri)
- Real signature validation against pubkeys
- Real Bitcoin broadcast via connected Bitcoin node
- Automatic expiration polling
