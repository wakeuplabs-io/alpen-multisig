# Alpen Multisig — Design System

> **Version:** 1.1 (final — approved April 2026)
> **Stack:** React 18 + TypeScript + TailwindCSS v3 + Tauri 2
> **Scope:** Walking Skeleton (Slice 0) through Slice 1 screens

---

## 1. Principles

This is a **security-critical governance tool**. Every design decision must reinforce:

1. **Trust** — Institutional, precise, serious. No playful decorations.
2. **Clarity** — Signers take irreversible on-chain actions. Information hierarchy must be unambiguous.
3. **Restraint** — Use color sparingly. When something is colored, it carries meaning.
4. **Light mode always** — No dark backgrounds. The full app runs on light surfaces.
5. **Brand alignment** — Consistent with Alpen's public identity (alpenlabs.io).

---

## 2. Color Tokens

### 2.1 CSS variables (globals.css)

```css
:root {
  /* ── Brand core ─────────────────────────────── */
  --color-black:        #0A0A0A;
  --color-white:        #FFFFFF;

  /* ── App backgrounds ────────────────────────── */
  --color-bg-base:      #F8F8FB;   /* page shell — near white, faint lavender */
  --color-bg-surface:   #F8F7FF;   /* card surface — very soft lavender, barely perceptible */
  --color-bg-elevated:  #FFFFFF;   /* modals, dropdowns, primary cards */
  --color-bg-rose:      #FAE8E0;   /* CTA sections only — use max once per screen */

  /* ── Text ───────────────────────────────────── */
  --color-text-primary:   #0A0A0A;
  --color-text-secondary: #6B7280;
  --color-text-muted:     #9CA3AF;
  --color-text-disabled:  #D1D5DB;

  /* ── Accent violet — ONLY for links & hover ─── */
  --color-accent:           #9480F5;
  --color-accent-hover:     #7C6FCD;
  --color-accent-surface:   #F8F7FF;
  --color-accent-border:    #DDD8FF;

  /* ── Borders ────────────────────────────────── */
  --color-border:           #E5E7EB;
  --color-border-strong:    #D1D5DB;
  --color-border-accent:    #DDD8FF;

  /* ── Proposal status ────────────────────────── */
  --color-pending:          #D97706;
  --color-pending-bg:       #FFFBEB;
  --color-pending-border:   #FDE68A;

  --color-approved:         #2563EB;
  --color-approved-bg:      #EFF6FF;
  --color-approved-border:  #BFDBFE;

  --color-enacted:          #059669;
  --color-enacted-bg:       #ECFDF5;
  --color-enacted-border:   #A7F3D0;

  --color-canceled:         #DC2626;
  --color-canceled-bg:      #FEF2F2;
  --color-canceled-border:  #FECACA;

  --color-expired:          #6B7280;
  --color-expired-bg:       #F9FAFB;
  --color-expired-border:   #E5E7EB;

  /* ── Feedback ───────────────────────────────── */
  --color-error:        #DC2626;
  --color-error-bg:     #FEF2F2;
  --color-success:      #059669;
  --color-success-bg:   #ECFDF5;
  --color-warning:      #D97706;
  --color-warning-bg:   #FFFBEB;

  /* ── Typography ─────────────────────────────── */
  --font-display: 'BIZ UDPMincho', serif;
  --font-body:    'Outfit', sans-serif;
  --font-mono:    'JetBrains Mono', 'ui-monospace', monospace;
}
```

### 2.2 Color usage rules

| Use case | Token | Notes |
|---|---|---|
| Page background | `--color-bg-base` | Always light, never pure white |
| Card / panel surface | `--color-bg-surface` | Barely-there lavender tint |
| Modal / elevated card | `--color-bg-elevated` | Pure white, always with border |
| CTA section | `--color-bg-rose` | Max 1 per screen |
| Body text | `--color-text-primary` | Near black |
| Labels, meta | `--color-text-secondary` | Gray |
| Placeholders, hints | `--color-text-muted` | Light gray |
| Links only | `--color-accent` | Violet — never for decoration |
| Link hover | `--color-accent-hover` | Darker violet |
| Default borders | `--color-border` | Light gray |
| Accent surface borders | `--color-border-accent` | Lavender-tinted |

### 2.3 What NOT to do

- ❌ No dark mode / dark backgrounds anywhere in the app
- ❌ No violet backgrounds, decorations, or fills — violet is for links/hover only
- ❌ No pure `#000000` — always use `--color-black` (`#0A0A0A`)
- ❌ No additional brand colors beyond what's defined here
- ❌ No rose (`--color-bg-rose`) used as a repeating pattern

---

## 3. Typography

### 3.1 Font imports

```css
@import url('https://fonts.googleapis.com/css2?family=BIZ+UDPMincho:wght@400&display=swap');
@import url('https://fonts.googleapis.com/css2?family=Outfit:wght@400;500;600;700&display=swap');
@import url('https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400&display=swap');
```

### 3.2 Type scale

| Role | Size | Weight | Family | Line height | Used for |
|---|---|---|---|---|---|
| display-lg | 36px | 400 | BIZ UDPMincho | 1.15 | Screen titles (connect wallet, auth) |
| display-md | 28px | 400 | BIZ UDPMincho | 1.2 | Dashboard section headings |
| display-sm | 22px | 400 | BIZ UDPMincho | 1.25 | Card headings, authority name |
| heading | 18px | 600 | Outfit | 1.3 | Subsection labels |
| body-lg | 16px | 400 | Outfit | 1.6 | Primary body content |
| body | 14px | 400 | Outfit | 1.6 | Secondary content, descriptions |
| body-sm | 13px | 400 | Outfit | 1.5 | Meta, timestamps, captions |
| label | 12px | 500 | Outfit | 1.4 | Form labels, badge text |
| mono | 13px | 400 | JetBrains Mono | 1.5 | Addresses, keys, hashes, hex |
| mono-sm | 11px | 400 | JetBrains Mono | 1.6 | Inline address display |

### 3.3 Typography rules

- **BIZ UDPMincho** — `font-weight: 400` always. Never bold. Screen/card titles only.
- **Outfit** — default `400`. Use `500` for emphasis, `600` for headings, `700` sparingly.
- **JetBrains Mono** — always for: derivation paths, public keys, signatures, hex, addresses.
- Never use body font for crypto values.

---

## 4. Spacing & Layout

### 4.1 App shell

```
┌─────────────────────────────────────────────────┐
│  Sidebar (240px)  │  Main content (flex-1)       │
│  bg: #FFFFFF      │  bg: #F8F8FB                 │
│  border-right:    │  max-width: 800px centered    │
│  1px #E5E7EB      │  padding: 40px 40px           │
└─────────────────────────────────────────────────┘
```

### 4.2 Border radius

| Token | Value | Use |
|---|---|---|
| rounded-sm | 6px | Badges, pills |
| rounded-md | 8px | Buttons, inputs, nav items |
| rounded-lg | 12px | Cards, panels |
| rounded-xl | 16px | Main screen cards, modals |
| rounded-full | 9999px | Status dots, quorum bar |

---

## 5. Component Specs

### 5.1 Button

```
Primary:
  background:    #0A0A0A
  color:         #FFFFFF
  border:        none
  border-radius: 8px
  padding:       10px 20px
  font:          Outfit 500 14px
  hover bg:      #2A2A2A
  active:        scale(0.98)
  disabled:      opacity 0.38, cursor not-allowed

Secondary:
  background:    transparent
  color:         #0A0A0A
  border:        1px solid #0A0A0A
  border-radius: 8px
  padding:       10px 20px
  font:          Outfit 500 14px
  hover bg:      #F8F7FF
  hover border:  #DDD8FF

Ghost / Link:
  background:    transparent
  color:         #9480F5
  border:        none
  font:          Outfit 400 14px
  hover color:   #7C6FCD
  text-decoration: underline on hover

Destructive:
  background:    #DC2626
  color:         #FFFFFF
  border:        none
  border-radius: 8px
  padding:       10px 20px
  font:          Outfit 500 14px
  hover bg:      #B91C1C

Sizes:
  sm:  padding 6px 14px,  font 13px
  md:  padding 10px 20px, font 14px  ← default
  lg:  padding 12px 24px, font 15px
```

---

### 5.2 Card

```
Default card (proposals, main content):
  background:    #FFFFFF
  border:        1px solid #E5E7EB
  border-radius: 12px
  padding:       24px

Surface card (informational panels, auth steps):
  background:    #F8F7FF
  border:        1px solid #DDD8FF
  border-radius: 12px
  padding:       24px

Clickable card hover:
  border-color:  #DDD8FF
  background:    #F8F7FF
  cursor:        pointer
  transition:    all 150ms ease
```

---

### 5.3 Input

```
Default:
  background:    #FFFFFF
  border:        1px solid #E5E7EB
  border-radius: 8px
  padding:       10px 12px
  font:          Outfit 400 14px
  color:         #0A0A0A
  placeholder:   #9CA3AF

Focus:
  border-color:  #9480F5
  box-shadow:    0 0 0 3px rgba(148,128,245,0.10)
  outline:       none

Error:
  border-color:  #DC2626
  box-shadow:    0 0 0 3px rgba(220,38,38,0.10)

Disabled:
  background:    #F8F8FB
  color:         #D1D5DB
  cursor:        not-allowed

Monospace variant (hex, addresses):
  font-family:   JetBrains Mono
  font-size:     13px
  letter-spacing: 0.02em
```

---

### 5.4 Badge / Status pill

```
font:          Outfit 500 12px
padding:       3px 10px
border-radius: 6px
border:        1px solid
display:       inline-flex, align-items center, gap 5px

Variants:
  pending:   bg #FFFBEB  text #D97706  border #FDE68A
  approved:  bg #EFF6FF  text #2563EB  border #BFDBFE
  enacted:   bg #ECFDF5  text #059669  border #A7F3D0
  canceled:  bg #FEF2F2  text #DC2626  border #FECACA
  expired:   bg #F9FAFB  text #6B7280  border #E5E7EB
  neutral:   bg #F5F3FF  text #7C6FCD  border #E4DFFF  ← authority label

Each badge includes a 7px colored dot matching its text color.
```

---

### 5.5 Quorum progress bar

```
Track:
  height:        6px
  background:    #E5E7EB
  border-radius: 9999px
  width:         100%

Fill variants:
  collecting:    background #0A0A0A   (default)
  reached:       background #059669   (quorum met)
  expiring:      background #DC2626   (< 24h remaining)

transition: width 300ms ease

Always show "N / M signatures" text label alongside the bar.
```

---

### 5.6 Address picker row

```
Layout:
  display:               grid
  grid-template-columns: 2.5rem 1fr auto
  align-items:           center
  gap:                   10px
  padding:               11px 14px
  border-radius:         8px
  border:                1px solid #E5E7EB
  background:            #FFFFFF
  margin-bottom:         6px
  cursor:                pointer

Columns:
  index:    Outfit 12px    color #9CA3AF
  path:     JetBrains Mono 11px  color #6B7280
  address:  JetBrains Mono 11px  color #0A0A0A

Selected state:
  border-color:  #DDD8FF
  background:    #F8F7FF
  index color:   #7C6FCD   ← accent hover (violet)
  path color:    #7C6FCD   ← accent hover (violet)
  address color: #0A0A0A   ← unchanged

Hover (unselected):
  border-color: #D1D5DB
  background:   #F8F8FB
```

---

### 5.7 Monospace value display

```
background:    #F8F8FB
border:        1px solid #EEECFA
border-radius: 8px
padding:       10px 12px
font:          JetBrains Mono 11px
color:         #0A0A0A
word-break:    break-all
line-height:   1.6

Copy button (top-right, absolute):
  Outfit 12px 500
  color:        #9CA3AF
  hover color:  #9480F5
```

---

### 5.8 Sidebar

```
width:         240px
background:    #FFFFFF
border-right:  1px solid #E5E7EB
padding:       24px 16px
display:       flex flex-col
height:        100vh

Logo area:
  font:          BIZ UDPMincho 18px 400
  color:         #0A0A0A
  padding-bottom: 16px
  border-bottom: 1px solid #E5E7EB
  margin-bottom: 20px

Nav item:
  padding:       9px 10px
  border-radius: 7px
  font:          Outfit 400 13px
  color:         #6B7280
  margin-bottom: 2px

Nav item active:
  background:   #0A0A0A
  color:        #FFFFFF

Nav item hover:
  background:   #F8F8FB
  color:        #0A0A0A

Authority badge:
  background:   #F8F8FB
  border:       1px solid #E5E7EB
  border-radius: 8px
  padding:      8px 10px
  margin-top:   16px
  label font:   Outfit 10px #9CA3AF
  name font:    Outfit 500 12px #0A0A0A

Wallet address (bottom):
  margin-top:   auto
  padding-top:  14px
  border-top:   1px solid #E5E7EB
  font:         JetBrains Mono 10px
  color:        #9CA3AF
  word-break:   break-all
  line-height:  1.5
```

---

### 5.9 Toast / Notification

```
background:    #FFFFFF
border:        1px solid #E5E7EB
border-radius: 10px
padding:       12px 16px
min-width:     280px
max-width:     360px
position:      top-right, stacked

Left accent bar (3px wide, full height, border-radius left):
  success: #059669
  error:   #DC2626
  warning: #D97706
  info:    #9480F5
```

---

## 6. Logo Usage

Source files live in [`./assets/`](./assets/) (kebab-case). Each mark is available as **SVG** (default for UI) and **PNG** (raster use).

| Asset | Context |
|---|---|
| `black-alpen-lockup` | Default — all light backgrounds (horizontal lockup) |
| `white-alpen-lockup` | Only if ever used on a dark surface |
| `black-alpen-lockup-stacked` / `white-alpen-lockup-stacked` | Stacked (vertical) lockup when the layout calls for it |
| `black-alpen-icon` / `white-alpen-icon` | Compact spaces, favicon |
| `black-alpen-wordmark` / `white-alpen-wordmark` | Wordmark only when the full lockup does not fit (rare) |

File names use the pattern `<asset>.svg` and `<asset>.png` (for example `black-alpen-lockup.svg`).

Rules:
- Always use **lockup** (icon + wordmark). Icon-only only when space is under 120px wide.
- Never recolor, stretch, or add effects.
- Minimum lockup width: 100px. Minimum icon: 24px.

---

## 7. Motion

```css
/* Interactive elements */
transition: all 150ms ease;

/* Clickable card hover */
transform: translateY(-1px);
transition: transform 150ms ease, border-color 150ms ease;

/* Button press */
transform: scale(0.98);
transition: transform 100ms ease;

/* Modal entry */
animation: fadeScaleIn 150ms ease;
@keyframes fadeScaleIn {
  from { opacity: 0; transform: scale(0.97); }
  to   { opacity: 1; transform: scale(1); }
}

/* Progress bar */
transition: width 300ms ease;

/* Toast entry */
animation: slideInRight 200ms ease;
```

Never: bounces, spring physics, animations over 300ms, particle effects.

---

## 8. Iconography

Library: **Lucide React** (`npm install lucide-react`)

| Context | Icon | Size |
|---|---|---|
| Hardware wallet | `Usb` | 20px |
| Copy | `Copy` | 16px |
| Success | `Check` | 16px |
| Warning / expiry | `AlertTriangle` | 16px |
| Time remaining | `Clock` | 14px |
| Proposal | `FileText` | 16px |
| Signing | `PenLine` | 16px |
| Broadcast | `Send` | 16px |
| External link | `ArrowUpRight` | 14px |
| Disconnect | `LogOut` | 16px |
| Chevron | `ChevronRight` | 14px |

Icon sizing: inline with text = 16px, standalone = 20px, empty state = 40px color `#9CA3AF`.

---

## 9. Tailwind config

```js
// tailwind.config.js
const { fontFamily } = require('tailwindcss/defaultTheme')

module.exports = {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      fontFamily: {
        display: ['BIZ UDPMincho', ...fontFamily.serif],
        body:    ['Outfit', ...fontFamily.sans],
        mono:    ['JetBrains Mono', ...fontFamily.mono],
      },
      colors: {
        brand: {
          black:             '#0A0A0A',
          white:             '#FFFFFF',
          accent:            '#9480F5',
          'accent-hover':    '#7C6FCD',
          lavender:          '#F8F7FF',
          'lavender-border': '#DDD8FF',
          rose:              '#FAE8E0',
        },
        surface: {
          base:     '#F8F8FB',
          card:     '#F8F7FF',
          elevated: '#FFFFFF',
        },
        status: {
          pending:       '#D97706',
          'pending-bg':  '#FFFBEB',
          approved:      '#2563EB',
          'approved-bg': '#EFF6FF',
          enacted:       '#059669',
          'enacted-bg':  '#ECFDF5',
          canceled:      '#DC2626',
          'canceled-bg': '#FEF2F2',
          expired:       '#6B7280',
          'expired-bg':  '#F9FAFB',
        },
      },
      borderRadius: {
        sm:  '6px',
        md:  '8px',
        lg:  '12px',
        xl:  '16px',
      },
      boxShadow: {
        card:          '0 1px 4px rgba(0,0,0,0.06)',
        modal:         '0 8px 32px rgba(0,0,0,0.10)',
        'focus-accent':'0 0 0 3px rgba(148,128,245,0.10)',
        'focus-error': '0 0 0 3px rgba(220,38,38,0.10)',
      },
    },
  },
  plugins: [],
}
```

---

## 10. CSS global baseline

```css
/* src/index.css */
@import url('https://fonts.googleapis.com/css2?family=BIZ+UDPMincho:wght@400&display=swap');
@import url('https://fonts.googleapis.com/css2?family=Outfit:wght@400;500;600;700&display=swap');
@import url('https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400&display=swap');
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  html, body {
    font-family: 'Outfit', sans-serif;
    font-size: 14px;
    color: #0A0A0A;
    background: #F8F8FB;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }

  h1, h2, h3 {
    font-family: 'BIZ UDPMincho', serif;
    font-weight: 400;
  }

  code, pre, .mono {
    font-family: 'JetBrains Mono', ui-monospace, monospace;
  }

  * { box-sizing: border-box; }
}
```

---

## 11. Screen inventory — Slice 0 (Walking Skeleton)

| Screen | Route | Task |
|---|---|---|
| Wallet Connect | `/` | Refactor existing PoC |
| Address Picker | `/` (step 2) | Refactor existing PoC |
| Authority Selector | `/authority` | Build from scratch |
| Session Auth | `/auth` | Build from scratch |
| Proposals Dashboard | `/dashboard` | Build from scratch |
| Proposal Detail | `/dashboard/:id` | Build from scratch |
| Create Proposal | `/dashboard/new` | Build from scratch |
| Sign Proposal | `/dashboard/:id/sign` | Refactor existing PoC |
| Export / Broadcast | `/dashboard/:id/broadcast` | Build from scratch |

---

## 12. Accessibility baseline

- All interactive elements keyboard-navigable
- Focus ring: `box-shadow: 0 0 0 3px rgba(148,128,245,0.10)` on all inputs
- Color is never the only state indicator — always pair with text label or icon
- Min contrast: 4.5:1 body text, 3:1 large text
- All monospace values (addresses, hashes) must have a copy button

---

## 13. Do's and Don'ts

| Do | Don't |
|---|---|
| BIZ UDPMincho for screen titles only | Mix display and body in same element |
| Soft lavender `#F8F7FF` for surface cards | Use violet for backgrounds or emphasis |
| Monospace for all crypto values | Show addresses in Outfit |
| Status colors strictly for proposal states | Reuse status colors for other purposes |
| White sidebar with black active item | Use dark backgrounds anywhere |
| Violet only for links and hover states | Use violet for buttons or headings |
| Pair every color indicator with text label | Rely on color alone to communicate state |
| Lucide icons consistently | Mix icon libraries |
