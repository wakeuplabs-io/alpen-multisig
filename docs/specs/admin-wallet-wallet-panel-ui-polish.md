# Admin Wallet — Wallet Panel UI Polish (R1.7)

**Phase:** R1.7 (see [`admin-wallet-implementation-plan.md`](./admin-wallet-implementation-plan.md))
**Scope:** Pure styling pass on the Admin Wallet slide-over panel — no logic, no new IPC, no new data.
**PRD ref:** PRD §4 visual quality; Alta WalletPanel handoff (`miniwallet/Alpen-v0.1-Alta-handoff/`).

## Goal

Bring the wallet slide-over from "raw Tailwind with placeholders" to a designed panel with clear visual hierarchy, consistent spacing rhythm, and typographic intent — so it reads as a product administrators trust with custody-level decisions.

## Design principles applied

1. **Three visual tiers:** Balance (elevated), Receive + Addresses (quiet utility), Footer (whisper). Equal treatment = no hierarchy.
2. **4pt spacing scale:** `px-5` gutter, `space-y-5` container rhythm — no arbitrary `18px` one-offs.
3. **One label token:** `text-[11px] font-medium uppercase tracking-[0.08em] text-[#9ca3af]` applied consistently to all section labels.
4. **Borderless card for balance:** `#f4f2ff` fill without border — fill alone defines the surface (fill + border = form field).
5. **Violet accent on BTC unit:** makes the balance typographically *designed*, not just labeled.

## In scope

- `wallet-panel-header.tsx` — font, subtitle contrast, watch-only badge, close button hit area
- `wallet-panel-content.tsx` — spacing: `px-5`, `space-y-5` container rhythm
- `wallet-balance.tsx` — borderless tinted card, 34px hero, violet BTC unit, quiet text toggle
- `receive-address-row.tsx` — hairline-border utility block (no fill)
- `addresses-with-balance-list.tsx` — chevron SVG, drop `#` column header
- `address-row.tsx` — drop index cell, `tabular-nums` right-aligned balance, 60%→100% copy opacity
- `sync-chip.tsx` — ghost refresh button, pulse dot (motion-safe), label contrast

## Out of scope

- QR code for receive address (Phase 7) — noted as the highest-value deferred improvement
- Any new IPC commands or Rust changes
- Send, fee-bump, tx list (Phases 5–6)

## Design tokens

| Token | Value | Usage |
|---|---|---|
| `font-sans` | `Outfit` | Headings, labels, UI text |
| `font-mono` | `JetBrains Mono` | Addresses, amounts, subtitle |
| `font-['BIZ_UDPMincho']` | BIZ UDPMincho | Balance hero number |
| `#f4f2ff` | Violet-tinted surface | Balance card fill (no border) |
| `#f3f4f6` | Hairline | Section dividers, receive border |
| `#9480f5` | Violet accent | BTC/sats unit, refresh button text, pulse dot |
| `#4b5563` | Subtitle text | Header subtitle (from `#6b7280`) |
| `#9ca3af` | Muted | Labels, secondary amounts |

## Component specs

### Header
- Title: `font-sans text-[15px] font-medium tracking-[0.02em] text-[#111827]`
- Subtitle: `font-mono text-[11px] text-[#4b5563]`
- Watch-only badge: `inline-flex items-center gap-1 text-[11px] text-[#9ca3af] bg-[#f3f4f6] rounded-full px-2 py-0.5` + eye SVG (14×14)
- Close button: `p-2 rounded-lg`

### Balance card
- Outer: `bg-[#f4f2ff] rounded-2xl p-5` — no border
- Hero: `font-['BIZ_UDPMincho'] text-[34px] font-normal leading-none text-[#111827]`
- Unit: `font-sans text-[13px] font-medium text-[#9480f5] ml-2`
- Secondary line: `font-mono text-[12px] text-[#9ca3af] mt-1.5`
- Unconfirmed line (if present): `font-mono text-[12px] text-[#6b7280] mt-1`
- Toggle: quiet text link below, `text-[12px] text-[#9480f5] underline underline-offset-2 hover:text-[#7c6fcd] mt-2`
- Loading state: same card surface wrapping skeletons

### Receive address block
- Container: `border border-[#f3f4f6] rounded-xl px-4 py-3` (no fill)
- Label: shared label token
- Address + Copy row: `flex items-center justify-between gap-2 mt-1.5`
- Loading/empty states: same container

### Addresses section
- Toggle: chevron SVG (`rotate-180` when expanded) + `Addresses with balance · {count}` (literal preserved)
- Count: `text-[#9ca3af]`
- No `#` index column
- `<tbody>`: `divide-y divide-[#f3f4f6]`

### Address rows
- No index cell rendered
- Balance: `text-right font-mono text-[13px] tabular-nums`
- Copy: `opacity-60 group-hover:opacity-100 group-focus-within:opacity-100 transition-opacity`
- Row hover: `group hover:bg-[#fafafa] transition-colors`

### Sync footer
- Label: `text-[12px] text-[#6b7280]`
- Pulse dot (isRefreshing): `motion-safe:animate-pulse w-1.5 h-1.5 rounded-full bg-[#9480f5]` (`aria-hidden`)
- Refresh button: `rounded-md px-2 py-1 text-[11px] font-medium text-[#9480f5] hover:bg-[#f4f2ff] disabled:opacity-50`

## Done when

- Balance card: borderless violet surface, 34px hero, violet BTC unit, quiet toggle.
- Receive address: hairline-border utility block (not a card), address + copy.
- Addresses: chevron toggle, no `#` column, right-aligned tabular mono balances, fade-in copy.
- Sync footer: ghost violet refresh button, pulse dot during sync.
- Loading states: match final layout proportions (no layout shift).
- `npm run lint`, `npm run format:check`, `npm run build`, `npm test` all pass.
