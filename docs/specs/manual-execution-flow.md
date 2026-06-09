# Spec: Manual Execution Flow

## Objective

Define two complementary features that together enable full offline survivability for the
multisig signing process:

1. **Proposal detail utility buttons** — paste, copy, and download buttons added to the
   proposal detail view to support manual signature collection and export without the orchestrator.
2. **Manual proposal entry flow** — a standalone screen where a signer can paste a raw action hex,
   decode it, sign it locally, aggregate external signatures, and optionally broadcast — all without
   any orchestrator connection.

Together these features fulfill the architecture requirement that *"signers must be able to
construct, aggregate, and broadcast valid transactions manually if the backend is unavailable"*
(`docs/architecture/overview.md`).

## Background

The current proposal-detail screen (`domain/proposal-detail/`) shows a Sign or Broadcast CTA
but provides no in-place mechanism to:

- Import signatures contributed by signers out-of-band (email, Slack, etc.)
- Export the raw action hex or the signature bundle for offline coordination
- Work with a proposal that was never registered with the orchestrator

The manual entry flow is a new route that operates entirely in-memory — no `actionId`, no
orchestrator API calls — and reuses existing domain primitives (`computeSighash`,
`decodeActionHex`, `verifyThreshold`, the sign-proposal hardware wallet path).

## Scope

### Included

**Part A — Proposal detail utility buttons**

- Three icon-only buttons in the action row of `ProposalDetail` (alongside the existing
  Sign / Broadcast CTAs): **Paste signatures**, **Copy bundle**, **Download bundle**.
- Paste signatures modal: multi-line textarea, parse + validate, merge with existing signatures.
- Copy bundle: copies a JSON object containing `actionHex`, `seqNo`, `authority`, and
  `signatures` to the clipboard.
- Download bundle: triggers a `<a download>` save for the same JSON object as
  `proposal-<seqNo>.json`.
- New props on `ProposalDetail`: `onPasteSignatures(sigs: PastedSignature[])`.

**Part B — Manual proposal entry screen**

- New route: `/manual` (accessible from the proposals dashboard via a secondary "Enter manually"
  button).
- New screen: `ManualProposalScreen` at `desktop-app/src/screens/manual-proposal-screen.tsx`.
- New domain: `desktop-app/src/domain/manual-proposal/`.
- Step 1 — Import: paste raw `actionHex` + `seqNo` + `authority`, decode, show signer set
  change table + SPS-65 sighash, validate before proceeding.
- Step 2 — Sign & collect: view decoded proposal, sign with hardware wallet (reuses existing
  sign-proposal IPC path), paste signatures from other signers, track quorum locally.
- Step 3 — Broadcast: once quorum reached, trigger commit/reveal broadcast (reuses existing
  broadcast-proposal IPC path).
- All state is in-memory in a `useManualProposal` hook; nothing is persisted or sent to the
  orchestrator.
- Error states: invalid hex, decode failure, duplicate signature, wrong sighash for pasted sig,
  threshold not met on broadcast attempt.

### NOT included

- Changes to orchestrator HTTP API or backend domain.
- Saving manual proposals to the orchestrator or local disk.
- Multi-window or cross-session persistence of manual proposal state.
- Any changes to the existing orchestrator-backed proposal-detail flow beyond adding the
  utility buttons.

## Part A — Proposal Detail Utility Buttons

### Visual layout

The current action row in `ProposalDetail`:

```
[ Sign / Broadcast / "Waiting…" banner ]
```

Becomes:

```
[ Sign / Broadcast / "Waiting…" banner ]    [ paste ] [ copy ] [ download ]
```

The three icon-only buttons are right-aligned, secondary, and sized to match the existing
`CopyButton` style (`border border-[#e5e7eb] bg-white px-2.5 py-1.5`). They are always
visible regardless of proposal status (even for terminal proposals, export remains useful).

Icons:
- **Paste**: `ClipboardPasteIcon` (or the existing `CopyClipboardIcon` mirrored).
- **Copy**: existing `CopyClipboardIcon`.
- **Download**: `DownloadIcon`.

### Props change

```typescript
// domain/proposal-detail/components/proposal-detail.tsx
type Props = {
  proposal: Proposal
  signerPubkey: string | null
  decodedData: DecodedProposalData
  onSign: () => void
  onBroadcast: () => void
  onPasteSignatures: (sigs: PastedSignature[]) => void  // ← NEW
}

// domain/proposal-detail/model/pasted-signature.ts  ← NEW
export type PastedSignature = {
  signerPubkey: string
  signatureHex: string
}
```

`onPasteSignatures` is called after the paste modal validates the incoming signatures. The
screen layer (`ProposalDetailScreen`) handles the actual `approveProposal` IPC call to
register each signature with the orchestrator.

### Paste signatures modal

Triggered by the paste icon button. Opens a `<dialog>` overlay.

**Input:** a multi-line textarea. Accepted formats:

1. **JSON array** — `[{ "signerPubkey": "…", "signatureHex": "…" }, …]`
2. **Single JSON object** — `{ "signerPubkey": "…", "signatureHex": "…" }`
3. **Newline-separated hex pairs** — `<pubkey> <signatureHex>` one per line

**Validation (blocking):**

| Check | Error |
|---|---|
| JSON parse failure | "Invalid format — expected JSON array or object" |
| `signerPubkey` not 33-byte compressed hex (66 chars) | "Invalid pubkey at index N" |
| `signatureHex` not 64-byte Schnorr hex (128 chars) | "Invalid signature at index N" |
| Signature already in `proposal.signatures` | "Duplicate: <truncated pubkey> already signed" |
| Signer not in `decodedData.allSigners` | Warning (non-blocking): "Pubkey not in known signer set" |

Signatures that pass all checks are listed in a preview table before confirming. The user
clicks **Import N signature(s)** to confirm, which calls `onPasteSignatures(validatedSigs)`.

### Copy bundle

Copies to clipboard:

```json
{
  "actionHex": "<hex>",
  "seqNo": 42,
  "authority": "strata_admin",
  "signatures": [
    { "signerPubkey": "…", "signatureHex": "…" }
  ]
}
```

Reuses the existing 2-second "Copied!" feedback pattern from `CopyButton`.

### Download bundle

Same JSON object, saved as `proposal-<seqNo>.json` via a temporary `<a href="data:…" download>`.
No Tauri IPC needed — uses browser `URL.createObjectURL`.

## Part B — Manual Proposal Entry Screen

### Route

```
/manual
```

Added to `App.tsx` alongside the existing proposal routes. No `actionId` URL segment — state
is entirely in-memory.

Navigation into the route:
- Secondary "Enter manually" button on the proposals dashboard (below the main CTA).
- Direct URL navigation.

Navigation out:
- Cancel at any step → back to `/proposals`.
- Broadcast success → navigate to the broadcast confirmation view (reusing the existing
  broadcast-done display pattern, but read from local state rather than orchestrator).

### Step flow

```
Step 1: Import         Step 2: Sign & Collect         Step 3: Broadcast
  [Paste action hex]  →  [View decoded + sign]       →  [Confirm & broadcast]
  [Enter seqNo]          [Paste other sigs]
  [Select authority]     [Track quorum]
```

A simple step indicator (1 · 2 · 3) is shown at the top, non-clickable — progression is
forward-only. Back button available between steps.

### Step 1 — Import

**Form fields:**

| Field | Type | Required | Notes |
|---|---|---|---|
| `actionHex` | `string` | yes | Hex-encoded ASM action payload |
| `seqNo` | `number` | yes | Sequence number (u64, must be ≥ 0) |
| `authority` | `Authority` | yes | Dropdown: same options as create-proposal form |

**Validation (blocking, on submit):**

1. `actionHex` must be valid hex (even length, hex chars only).
2. `decodeActionHex(actionHex)` must return a known action kind (`multisig_update`, `vk_update`,
   etc.) — not `unknown`.
3. `computeSighash(seqNo, actionHex)` must succeed.
4. `seqNo` must be a non-negative integer ≤ `Number.MAX_SAFE_INTEGER`.

On success, store `{ actionHex, seqNo, authority, sighashHex, decodedAction }` in
`useManualProposal` state and advance to Step 2.

**Inline error display** below the `actionHex` textarea using the same pattern as
`create-proposal` form errors.

### Step 2 — Sign & Collect

Displays the same decoded proposal view as `ProposalDetail`, reading from local state:
- Signer set change table (from `decodeActionHex` result).
- SPS-65 sighash.
- Signature list with quorum counter.

**Local quorum tracking:**

```typescript
type ManualSignature = {
  signerPubkey: string
  signatureHex: string
  source: 'local' | 'pasted'   // 'local' = signed by this device
}
```

Quorum is computed from the multisig config fetched via the same
`useMultisigConfig(authority)` hook already used in `use-decoded-proposal`.

**Actions available:**

- **Sign** (black primary button) — triggers hardware wallet signing flow using the existing
  `sign-proposal` IPC path. Passes `sighashHex`; result is stored as a `ManualSignature`
  with `source: 'local'`.
- **Paste signatures** (icon button) — same paste modal defined in Part A. Validated
  signatures are stored with `source: 'pasted'`.
- **Copy bundle** / **Download bundle** — same as Part A, reads from local state.

When `collectedSignatures >= requiredSignatures`, a "Quorum reached" banner appears and the
**Broadcast** button becomes active.

### Step 3 — Broadcast

Reuses the `broadcast-proposal` domain, passing a synthetic `Proposal` object constructed
from local state:

```typescript
const syntheticProposal: Proposal = {
  actionId: `manual-${sighashHex.slice(0, 16)}`,   // local identifier only
  seqNo,
  authority,
  status: 'approved',
  requiredSignatures,
  actionHex,
  actionType: derivedActionType,
  signatures: localSignatures.map(s => ({ signerPubkey: s.signerPubkey, signatureHex: s.signatureHex })),
  broadcastStatus: 'idle',
  commitTxid: null,
  revealTxid: null,
  broadcastError: null,
  kind: 'manual',
  targetActionId: null,
  activationHeight: null,
  updateIdInQueue: null,
  cancelProposal: null,
}
```

The broadcast IPC call (`broadcastProposal`) accepts this synthetic proposal; the Tauri
backend does not validate `actionId` format — it reads `actionHex` and `signatures` directly.

**Before submitting**, the screen shows the same `BroadcastDetailsCard` confirmation
(commit address, amount, estimated fee, admin wallet balance) that the orchestrator-backed
flow shows.

On broadcast success, the screen shows commit TXID + reveal TXID with copy buttons. No
orchestrator `PATCH` is issued (there is no registered proposal to update).

**Error handling:** same `BroadcastPhase` / `BroadcastErrorCode` model as the existing
`use-broadcast-proposal` hook. The `resubmit-reveal` recovery path works because the
signed reveal is held in the Tauri process's in-memory store keyed by the synthetic
`actionId`.

### Domain layout

```
desktop-app/src/
├── screens/
│   └── manual-proposal-screen.tsx          ← NEW
└── domain/
    └── manual-proposal/
        ├── components/
        │   ├── manual-import-form.tsx       ← NEW (Step 1)
        │   ├── manual-sign-collect.tsx      ← NEW (Step 2, wraps ProposalDetail)
        │   └── paste-signatures-modal.tsx   ← NEW (shared by Part A + Step 2)
        ├── hooks/
        │   └── use-manual-proposal.ts       ← NEW (full step state machine)
        └── model/
            └── manual-proposal.types.ts     ← NEW
```

`paste-signatures-modal.tsx` is the single implementation of the paste modal used by both
the orchestrator-backed `ProposalDetail` (Part A) and the manual flow (Part B).

### `useManualProposal` hook contract

```typescript
type ManualStep = 'import' | 'sign-collect' | 'broadcast'

type UseManualProposalReturn = {
  step: ManualStep
  // Step 1 state
  importForm: { actionHex: string; seqNo: string; authority: string }
  importErrors: Record<string, string>
  handleImportChange: (field: string, value: string) => void
  handleImportSubmit: () => void
  // Step 2 state
  decodedData: DecodedProposalData
  localSignatures: ManualSignature[]
  requiredSignatures: number
  hasQuorum: boolean
  handleSign: () => void
  handlePasteSignatures: (sigs: PastedSignature[]) => void
  handleBack: () => void
  handleAdvanceToBroadcast: () => void
  // Step 3 state
  broadcastPhase: BroadcastPhase
  broadcastError: BroadcastErrorCode | null
  handleConfirmBroadcast: () => void
  commitTxid: string | null
  revealTxid: string | null
}
```

## ProposalDetail Props Change — Screen Integration

`ProposalDetailScreen` already handles `onSign` and `onBroadcast` navigation. It must also:

1. Receive `onPasteSignatures` from the new prop and call `approveProposal` for each valid
   signature, then reload the proposal from the orchestrator.
2. Render the three utility icon buttons by passing `proposal.actionHex` and
   `proposal.signatures` as needed.

The presentational `ProposalDetail` component remains unaware of the orchestrator; it only
calls `onPasteSignatures(validatedSigs)` after the modal confirms.

## Test Cases

### Unit — `domain/manual-proposal/`

1. **Import form — valid hex advances to step 2** — `handleImportSubmit` with well-formed
   `actionHex` + `seqNo` + `authority` transitions `step` to `'sign-collect'`.
2. **Import form — invalid hex shows error** — non-hex `actionHex` sets `importErrors.actionHex`.
3. **Import form — unknown action kind shows error** — `decodeActionHex` returning `unknown`
   sets `importErrors.actionHex` with a decode-failure message.
4. **Paste modal — JSON array parsed correctly** — valid `[{ signerPubkey, signatureHex }]`
   resolves to `ManualSignature[]`.
5. **Paste modal — duplicate signature rejected** — a pubkey already in `localSignatures`
   produces a blocking error.
6. **Paste modal — wrong-length pubkey blocked** — a 64-char (non-compressed) pubkey produces
   a blocking error.
7. **Quorum reached when sigs ≥ required** — `hasQuorum` becomes `true` at threshold.
8. **Quorum not reached below threshold** — adding one fewer than required leaves
   `hasQuorum: false`.

### Unit — `paste-signatures-modal.tsx`

9. **Newline-pair format parsed** — `"<pubkey> <sig>\n<pubkey2> <sig2>"` produces two entries.
10. **Single JSON object accepted** — `{ signerPubkey, signatureHex }` (no array) resolves to
    one entry.
11. **Non-signer pubkey shows warning, not error** — `decodedData.allSigners` not containing
    the pubkey renders a warning row but does not block import.

### Unit — `ProposalDetail` copy / download

12. **Copy bundle writes correct JSON** — `navigator.clipboard.writeText` called with JSON
    containing `actionHex`, `seqNo`, `authority`, `signatures`.
13. **Download triggers anchor click** — a temporary `<a>` element with `download="proposal-N.json"`
    is created and clicked.

### Integration — manual flow (jsdom)

14. **Full step 1→2→broadcast path** — mock `decodeActionHex`, `computeSighash`,
    `signSighash`, `broadcastProposal` IPC. Assert that `broadcastProposal` is called with
    `actionHex` and accumulated `signatures`.
15. **Broadcast unavailable before quorum** — `handleAdvanceToBroadcast` is a no-op (or
    returns an error) when `hasQuorum: false`.

## Module Impact

```
desktop-app/src/
├── App.tsx                                              ← MODIFIED: add /manual route
├── screens/
│   ├── manual-proposal-screen.tsx                      ← NEW
│   └── proposal-detail-screen.tsx                      ← MODIFIED: wire onPasteSignatures
└── domain/
    ├── proposal-detail/
    │   └── components/
    │       └── proposal-detail.tsx                     ← MODIFIED: add utility buttons + onPasteSignatures prop
    └── manual-proposal/
        ├── components/
        │   ├── manual-import-form.tsx                  ← NEW
        │   ├── manual-sign-collect.tsx                 ← NEW
        │   └── paste-signatures-modal.tsx              ← NEW (shared)
        ├── hooks/
        │   └── use-manual-proposal.ts                  ← NEW
        └── model/
            └── manual-proposal.types.ts                ← NEW
```

## Dependencies and Ordering

- **Part A can ship independently** of Part B. The paste modal component is written once
  inside `domain/manual-proposal/components/` and imported by `ProposalDetail`.
- **Part B depends on Part A** only for `paste-signatures-modal.tsx`.
- No orchestrator changes required — the broadcast IPC already accepts `actionHex` +
  `signatures` directly.
- `use-decoded-proposal` hook is reused as-is in `useManualProposal` Step 2.
- `BroadcastDetailsCard` and `use-broadcast-proposal` are reused as-is in Step 3.
- Routing change (`App.tsx`) is the only shared file touched by Part B.

## Open Questions

1. **Authority dropdown in manual import** — should the authority field be a free-text
   input or a fixed enum dropdown? Dropdown is safer (prevents typos that corrupt the
   sighash); recommend dropdown matching the existing `create-proposal` authority selector.
2. **Quorum source without orchestrator** — `requiredSignatures` comes from
   `useMultisigConfig(authority)`, which today calls a Tauri IPC that fetches from the
   orchestrator. In a fully offline scenario this call will fail. Mitigaton: fall back to a
   manual "required signatures" input field in Step 1 if the IPC fails, with a visible
   "could not fetch multisig config" warning.
3. **`kind: 'manual'` in Proposal type** — the synthetic `Proposal` needs a new union
   member on `ProposalKind`. Confirm whether `ProposalKind` lives in `api/proposals.ts`
   (frontend-only) or also in the Tauri backend types. If only frontend, the change is local.
