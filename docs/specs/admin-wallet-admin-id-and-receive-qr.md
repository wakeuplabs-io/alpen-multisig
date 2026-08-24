# Spec: Admin Wallet — Admin ID visibility + Receive QR (Phase 7)

**Program:** Admin Wallet — Phase 7
**PRD source:** [`docs/0-prd/03-prd-update.md`](../0-prd/03-prd-update.md) §4.1, §4.3.4.1
**Plan:** [`admin-wallet-implementation-plan.md`](./admin-wallet-implementation-plan.md) §Phase 7
**Compliance matrix:** [`admin-wallet-prd-compliance.md`](./admin-wallet-prd-compliance.md)

## Objective

Close the open Phase 7 PRD gaps so a signed-in signer can **see and trust their identity and receive flow** inside the wallet panel:

- **§4.1** — After logging in, the user MUST be able to **see their Admin ID and copy it to the clipboard**. Today only a *truncated* signer label appears in the `SessionChip` and the panel subtitle; the full Admin ID is never shown and cannot be copied → matrix **FAIL**.
- **§4.3.4.1** — The receive address MUST be shown **in both text and QR code formats**, and **clicking the address text or the QR MUST copy the address**. Today the receive row shows text + an icon copy button only; no QR → matrix **PARTIAL**.

This is a **frontend-only** phase. The Admin ID is already available client-side as `wallet.addressSample` (the canonical BIP-84 `m/84'/0'/73'/0/0` entry produced on connect), and the receive address is already provided by the panel (`panel.receiveAddress`). No Rust/IPC/backend change is required.

## Scope

**In scope:**
- An **Admin ID row** at the top of the wallet panel: full address (wrapped, monospace), a clear **"Admin ID"** label, a **copy-to-clipboard** control, and a **signer-safety caption** marking it auth-only.
- A **receive QR** under the existing receive address text, with **click-to-copy on both the text and the QR** (PRD §4.3.4.1.1).
- A shared, dependency-light **`QrCode`** UI primitive (wrapping `qrcode.react` → inline SVG) and a shared **`useClipboardCopy`** hook for click-to-copy + "Copied" feedback.
- New `qrcode.react` dependency (chosen approach: MIT, no transitive runtime deps, renders inline SVG; pre-bundled fine under the `optimizeDeps` es2022 target).
- Architecture wiring test (Rule 9) + two pure model unit tests.

**NOT in scope (explicitly deferred):**
- **§4.2 / §4.3.4.2 — view/verify on the hardware-wallet screen → Phase 8** (requires device adapters).
- **QR for the Admin ID.** PRD mandates QR only for the *receive* address (§4.3.4.1). The plan's loose "QR for receive/Admin ID" phrasing is **intentionally narrowed**: the Admin ID is auth-only and **must never receive funds**, so rendering a scannable QR for it would invite mis-sends. The Admin ID gets **text + copy only**. (Documented deviation; PRD MUST is still satisfied.)
- Any change to the connect flow, session lifecycle, or receive-rotation logic (rotation already shipped in R1.3).

## Technical Design

### Data flow (no new transport)

```
use-hw-wallet-connect ──onConnected({ addressSample = Admin ID })──▶ wallet session
        │
screen (e.g. proposals-dashboard) ── wallet.addressSample ──▶ WalletSessionControl
        │                                                          │
        │                              (existing) signer label ◀───┤
        └──────────────────────── adminId prop ───────────────────▶ WalletPanelContent ──▶ AdminIdRow
                                                                     │
                                  panel.receiveAddress ─────────────▶ ReceiveAddressRow ──▶ QrCode + copy
```

`WalletSessionControl` already receives `addressSample`; it forwards it as a new `adminId` prop to `WalletPanelContent`. No change to `useWalletPanelData` or any hook/IPC.

### Production code (reusable)

| File | Responsibility (one sentence) |
|---|---|
| `src/components/qr-code.tsx` | Render a string as an accessible inline-SVG QR code (wraps `qrcode.react`). |
| `src/hooks/use-clipboard-copy.ts` | Provide `{ copied, copy }` — write text to the clipboard and flash a 2s "copied" flag. |
| `src/domain/admin-wallet/model/build-receive-qr-value.ts` | Map a receive address to the exact string encoded in the QR and copied on click (the bare address — **not** a BIP-21 URI). |
| `src/domain/admin-wallet/model/admin-id-presentation.ts` | Decide whether an `addressSample` is a displayable Admin ID, and own the Admin ID label + safety-caption copy literals. |
| `src/domain/admin-wallet/components/admin-id-row.tsx` | Present the Admin ID (full address + copy + safety caption); render nothing usable when not displayable. |
| `src/domain/admin-wallet/components/receive-address-row.tsx` | *(extend)* Present the receive address as text **and** QR, both click-to-copy. |

`CopyButton` (existing) is refactored in Phase 6 to consume `useClipboardCopy` so there is a single copy-feedback implementation (no behavior change).

### Test helpers
None required. Tests target the pure model functions and the component-wiring assertions only. No new Tauri commands, no exposed test utilities.

### Component sketch

- **`AdminIdRow`** — placed at the **top of `WalletPanelContent`**, above the balance, visually separated (it is identity, not funds):
  - label `Admin ID`,
  - full address wrapped in monospace (`break-all`), `data-testid="e2e-wallet-admin-id-value"`,
  - `CopyButton` (`variant="labeled"`),
  - caption: **"For authentication only — never send funds to this address."**
  - When `isDisplayableAdminId(adminId)` is false (missing / `"Mnemonic signer"` placeholder) → show `Unknown`, no copy control.
- **`ReceiveAddressRow`** (extended) — keeps the existing label + text + icon copy, and adds:
  - a `QrCode` of `buildReceiveQrValue(address)`, wrapped in a `button` (click → copy, `aria-label="Copy address"`), `data-testid="e2e-wallet-receive-qr"`,
  - the address **text** becomes a copy trigger too (button/role), sharing `useClipboardCopy` feedback,
  - unchanged loading / empty states.

### Architecture compliance (extend `architecture.test.ts` — Rule 9)
- `wallet-panel-content.tsx` renders `AdminIdRow` and forwards an `adminId` prop.
- `wallet-session-control.tsx` passes `addressSample` into the panel content as `adminId`.
- `receive-address-row.tsx` renders `QrCode` (real QR — Rule 4 already forbids placeholder copy).
- `admin-id-presentation.ts` contains the exact safety-caption literal (single audited source, mirroring the §4.3.5 send-copy pattern).
- `admin-id-row.tsx` does **not** import/render `QrCode` (enforces the "no QR on Admin ID" safety decision).

## Test Cases

**Pure model — `build-receive-qr-value.ts`** (`test:model-build-receive-qr-value`):
- returns the address unchanged for a typical bech32 address;
- empty input → empty string;
- does **not** prepend `bitcoin:` or any URI scheme (copy must yield exactly the address).

**Pure model — `admin-id-presentation.ts`** (`test:model-admin-id-presentation`):
- `isDisplayableAdminId('bc1q…')` → true;
- `isDisplayableAdminId('')` / `undefined` → false;
- `isDisplayableAdminId('Mnemonic signer')` → false (placeholder, not an address);
- exported safety caption equals the exact PRD-safety literal; label equals `Admin ID`.

**Architecture wiring — `architecture.test.ts` Rule 9:** all five structural assertions above pass.

**Manual / E2E (non-blocking, repo convention):** extend `desktop-app/e2e-webdriver/test/specs/admin-wallet-panel.e2e.js` to assert the Admin ID value + receive QR render after login, and that the copy controls are present. Run on demand like the fee-bump spec; not a CI gate.

> The React components themselves cannot be unit-tested here (no vitest/RTL in the repo — see existing hook tests). They are covered by the structural Rule 9 + the optional WebDriver e2e, consistent with the rest of `admin-wallet`.

## Module structure

All new code lives in existing locations and respects the dependency-direction rules already enforced by `architecture.test.ts`:
- **`model/`** (pure, no React, no transport): `build-receive-qr-value.ts`, `admin-id-presentation.ts`.
- **`components/`** (presentational, no `@/api/*`, no `@tauri-apps/api/core`): `admin-id-row.tsx`, extended `receive-address-row.tsx`.
- **shared primitives** (`src/components/`, `src/hooks/`): `qr-code.tsx`, `use-clipboard-copy.ts` — generic, reusable, no domain/transport coupling.

Dependency direction is preserved: components depend on pure model helpers and shared primitives; model helpers depend on nothing React/transport. The Admin ID reaches the panel by **prop drilling an existing value** (`addressSample`), not by adding a data dependency to `useWalletPanelData`.

---

## Update — PRD 06 (2026-08-14) — the section below is superseded

**The Admin ID is a bitcoin address again.** PRD snapshot 06 §3.b.ii.2 restores the P2WPKH
address at `m/84'/0'/73'/0/0`, reversing the compressed-public-key rendering that the July
section below specified and that PR #444 shipped.

Read the July section as history, not as contract. What it recorded is still worth keeping:
it explains *why* the app went the other way for five weeks, and its device-capability table
(#409) is the measurement the reversion rests on — no supported signer can render a raw
compressed public key, which is precisely why an address is the shape that lets the device
show the Admin ID itself.

What changed and where:

| July section says | Now | Where |
|---|---|---|
| The Admin ID is the compressed public key everywhere (#408, #412) | It is the P2WPKH address, on all three surfaces | [`admin-id-as-bitcoin-address.md`](./admin-id-as-bitcoin-address.md) |
| The device confirms the key **indirectly**, via a separate "Address on device" block | The device shows the Admin ID **itself**; the block is gone, and its duplicate reading of the same value was #413 | same spec, B3 |
| `isDisplayableAdminId` validates a compressed pubkey | It validates a bech32 address, and **rejects** a raw key so the July rendering cannot return unnoticed | `src/lib/admin-id.ts` |
| Safety caption: "it is a public key, not a payment address" | "never send funds to this address" | same |

Unchanged, and deliberately so:

- **The derivation path.** `m/84'/…/73'/0/0`, exactly as before. No signer is re-derived or
  re-enrolled by this reversal.
- **The backend.** `is_signer_member_for_authority` compares a compressed pubkey **recovered
  from the nonce signature**; it never parsed a displayed string, so nothing there moves.
- **No QR on the Admin ID.** The July decision (see In/NOT-in-scope above) stands with a
  stronger reason: the Admin ID is a real address now, so a scannable code would invite the
  mis-send it was always meant to prevent.
- **The receive-QR half of this spec** (§4.3.4.1) is untouched by any of this.

Why the reversal happened at all: the maintainer's ruling, the updated PRD and the wireframes
arrived through a channel outside the tracker, so #408, #409, #410 and #412 record no
explanation for it. This section is that explanation.

Still open after G7: the **Admin ID Verification Certificate** (PRD 06 §3.c.i, §4.a) → G8, and
**device QA plus the §4.1/§4.2 compliance flip** → G9.

---

## Update — feedback 2026-07-01 (#408, #409, #410, #412)

Alpen corrected a requirement that this spec encoded from the pre-update PRD: **the Admin ID
*is* the signer's compressed public key**, not the BIP-84 address derived from it. Everything
below supersedes the "canonical BIP-84 auth address" framing used above.

### What changed

- **#408 / #412 — the Admin ID is the compressed public key everywhere.** The address rendering
  and the separate "Compressed public key" block on the authenticate-session screen are gone; a
  single `Admin ID` field carries the key. Same value in the wallet panel (`AdminIdRow`), the
  session chip and the offline/manual screen. Presentation rules moved to `src/lib/admin-id.ts`
  (shared by the connect flow and the Admin Wallet) and `isDisplayableAdminId` now validates a
  compressed pubkey (33 bytes, `02`/`03`, optional `0x`) instead of "any non-placeholder string".
  The safety caption changed accordingly: the Admin ID is a public key, **not** a payment address.
- **#410 — order of the sign-in flow.** The Admin ID is now shown on the *multisig selection*
  step (`ConnectAdminIdCard`), rendered while the canonical signer-set membership check is still
  running. The signer verifies the identity the app derived before the app judges it.
- The BIP-84 derivation itself is unchanged (`m/84'/…/73'/0/0`); only what the UI presents changed.

### Device capability (#409) — the signer cannot see the raw key on the device

Alpen asked for a button that displays the Admin ID **on the hardware signer's screen**.
Neither supported device can render a raw compressed public key:

| Device | What the API can display | Evidence |
|---|---|---|
| Trezor | `GetAddress` with `show_display = true` → an **address**; `GetPublicKey` with `show_display = true` → an **xpub** (base58 extended key, not the 33-byte hex) | `src-tauri/src/infrastructure/hw_wallet/trezor.rs` (`get_xpub`, `verify_address`) |
| Ledger | `get_wallet_address(..., display = true)` → an **address** via a wallet policy; `get_extended_pubkey(path, display = true)` → an **xpub** | `src-tauri/src/infrastructure/hw_wallet/ledger.rs` (`verify_address_with`), `ledger_bitcoin_client` 0.6.2 |

**Decision:** keep the existing verify-on-device affordance, which shows the **P2WPKH address
derived from the same key and path**, and make that comparison actually possible. Confirming that
address proves the device holds the key behind the displayed Admin ID — `address = bech32(hash160(pubkey))`
is a deterministic function of that exact key. Claiming the device "shows the Admin ID" would be
false. Showing the xpub instead was rejected: it is a different value from the one on screen, so it
cannot be compared visually.

This is an app-level constraint imposed by the device firmware/APIs — it cannot be lifted from
our side. Revisit if a vendor adds a pubkey-display screen.

### How the verification works in the UI

For **hardware sessions only** (`AdminIdRow` renders this block only when a `verify` context exists):

1. Under the Admin ID key, the panel shows an **"Address on device"** block with the address derived
   from that key (`wallet.addressSample`), plus the per-vendor note from `adminIdVerifyCaption`
   (`src/lib/admin-id.ts`). Without it the signer would have a hex on screen and a `bc1q…` on the
   device, with nothing to compare.
2. `verify_address_on_device` now **returns the exact string the device rendered** — Trezor's
   `GetAddress` and Ledger's `get_wallet_address` both already produce it; it used to be discarded.
3. `useVerifyOnDevice` compares it against the expected address (`matchesDeviceAddress` — trims and
   lowercases, since bech32 is case-insensitive per BIP-173) and lands on a dedicated **`mismatch`**
   state, surfaced as a security alarm rather than a transport error.

`wallet.addressSample` is device-accurate for the Admin ID without re-encoding: the adapters derive
it with the HRP of the path's own coin type (`trezor.rs` → `KnownHrp::Mainnet` for coin `0'`;
`ledger.rs` → `hrp_from_path`, `Testnets` for coin `1'`), which is exactly what each device renders.
This is unlike the *receive* address, which comes from BDK on the real network (`bcrt1…`) and must be
re-encoded by `device_verify_address` (`src-tauri/src/commands/admin_wallet.rs`).

## Out-of-scope follow-ups (tracked for the matrix)
- §4.2 + §4.3.4.2 HW verify → **Phase 8**.
- On merge, update [`admin-wallet-prd-compliance.md`](./admin-wallet-prd-compliance.md): §4.1 **FAIL → PASS**, §4.3.4.1 QR + click-to-copy **FAIL/PARTIAL → PASS** (HW-verify rows remain Phase 8).
