# Spec: Admin ID is a bitcoin address (G7)

**Program:** Admin ID rework — loop G7 of 3 (G8 = Verification Certificate, G9 = device QA + compliance close-out)
**PRD source:** [`docs/0-prd/06-prd-hardware-signer-and-block-payouts-update.md`](../0-prd/06-prd-hardware-signer-and-block-payouts-update.md) §3.b.ii.2, §4.a
**Supersedes:** [`admin-wallet-admin-id-and-receive-qr.md`](./admin-wallet-admin-id-and-receive-qr.md) §"Update — feedback 2026-07-01"
**Issues:** #408 (reversed), #410, #412, #413
**Compliance matrix:** [`admin-wallet-prd-compliance.md`](./admin-wallet-prd-compliance.md)

## Objective

PRD snapshot 06 §3.b.ii.2 states the Admin ID is a **P2WPKH bitcoin address** derived at
`m/84'/0'/73'/0/0`. The app today renders the **compressed public key** in every Admin ID surface —
a deviation introduced in July (PR #444) answering #408/#412. The PRD never stopped saying
"address"; the subprotocol maintainer has now ruled, and this loop undoes considered work at the
client's explicit request → matrix §4.1 currently **PASS against the wrong requirement**.

Make the Admin ID the address again, in one place and with one string, so that:

- the signer sees the same value in the app and on their hardware signer screen (which is what
  unblocks #409 and the certificate work in G8), and
- the "Compressed public key" section and the duplicated Admin ID rendering disappear (#408, #413).

**The derivation path does not change** (`hw_wallet/trezor.rs:13`, `ledger.rs:33`). No re-derivation,
no identity churn, no re-enrolment of existing signers. **The backend does not change:**
`is_signer_member_for_authority` (`orchestrator-be/src/infrastructure/asm_role_membership.rs:28-55`)
compares a compressed pubkey **recovered from the nonce signature**, never a string parsed off a
screen. The Admin ID's internal identity stays the key; only its presentation becomes the address.

**This is a frontend-only loop.** Rust already returns both values from connect
(`trezor.rs:330-336` derives the P2WPKH address next to the key).

## Scope

**In scope:**

- `src/lib/admin-id.ts` becomes the single audited source for the new rule: an Admin ID is a bitcoin
  address; the safety caption and the verify caption follow.
- The connect flow (pre-sign-in) shows the address at the multisig-selection step and the
  authenticate step, **once** — the separate compressed-public-key row on the authenticate step is
  removed (#408).
- The wallet panel (post-login) shows the address; the now-redundant **"Address on device"** block is
  removed (#413), because the value above it *is* that address.
- The nine screens that pass `adminId={wallet.publicKeyHex}` are repointed at the address.
- The two test guards that encode the opposite rule are inverted, each one travelling in the same
  commit as the change it describes (see **Sequencing**).
- Docs: the July "Update" section of the receive-QR spec is superseded in place; the matrix §4.1 is
  repointed at PRD 06.

**NOT in scope (explicitly deferred):**

- **The Admin ID Verification Certificate** (PRD 06 §3.c.i, §4.a) → **G8**. G7 deliberately ships no
  new affordance; it only corrects what the existing ones display.
- **Device QA on Trezor/Ledger and the compliance flip to PASS** → **G9**. §4.2 stays **PARTIAL**
  when G7 merges.
- **Everything Payout Administrator**, including the **P2TR** Admin ID at `m/86'/0'/73'/0/0`
  (PRD 06 §3.b.ii.1). Not implemented today — the path is BIP-84 for every authority. Recorded in
  the matrix as **DEFER**, destination: the `block_payouts` program.
- **QR for the Admin ID stays forbidden.** PRD 06 §3.b.ii.2 is explicit that the Admin ID must not
  sign bitcoin transactions and it must not receive funds. The Admin ID being an address again makes
  a scannable QR *more* dangerous, not less. Rule 9's ban is kept and its justification updated.
- Any change to the session lifecycle, the auth challenge, the ASM membership check, or Rust.

## Technical Design

### Data flow — one string, already (established in B0)

```
adapter.connect() ──▶ WalletAccountInfo.addressSample          (m/84'/0'/73'/0/0, P2WPKH)
        │
use-hw-wallet-connect:57-62 ──▶ canonicalEntry.address
        │                            │
        │                            ├──▶ ConnectAdminIdCard            (connect step 2)
        │                            └──▶ AuthenticateSessionPhase      (connect step 3)
        │
use-hw-wallet-connect:67-74 ──onConnected({ addressSample })──▶ wallet-session-provider
                                                                    │
                            screens (×9) ── wallet.addressSample ────┴──▶ WalletPanelContent ──▶ AdminIdRow
```

The pre-sign-in card and the post-login panel already resolve to **the same
`WalletAccountInfo.addressSample`**, captured once at connect time. The reversion is therefore a
**presentation swap** (`publicKeyHex` → `addressSample`) at the leaves, not a data-flow change; no
new plumbing is required to keep the two placements in sync.

`publicKeyHex` is **not** removed from the session — the auth challenge and G8's certificate
self-check both still need it. It simply stops being what "Admin ID" means on screen.

### Production code

| File | Responsibility (one sentence) |
|---|---|
| `src/lib/admin-id.ts` | Own the rule that an Admin ID is a bitcoin address, plus the label, the safety caption and the verify caption as the single audited literals. |
| `src/lib/device-copy.ts` | Drop the per-vendor copy that explains the device "cannot render a raw public key" — the device now renders the Admin ID itself. |
| `src/domain/admin-wallet/model/admin-id-presentation.ts` | Keep re-exporting from `@/lib/admin-id` (unchanged surface, corrected docblock). |
| `src/domain/connect-wallet/hooks/use-hw-wallet-connect.ts` | Publish the address as the Admin ID on the session; stop mirroring `publicKeyHex` into `xpubOrFingerprint`. |
| `src/domain/connect-wallet/components/connect-admin-id-card.tsx` | Present the address at the multisig-selection step. |
| `src/domain/connect-wallet/components/authenticate-session-phase.tsx` | Present the address at the authenticate step; the `compressedPublicKey` prop becomes `adminId` and the second, duplicated row is deleted. |
| `src/domain/admin-wallet/components/admin-id-row.tsx` | Present the address once, with the safety caption and the verify affordance; the "Address on device" block is deleted. |
| `src/domain/admin-wallet/components/wallet-session-control.tsx` | Feed the session chip from the address. |
| `src/screens/*.tsx` (×9) | Pass `adminId={wallet.addressSample}`. |

### Sequencing — expand/contract, so every commit is atomic

`lib/admin-id.ts` is shared by the connect flow and the wallet panel, and two guards in the tree
assert the *opposite* rule on purpose. Flipping the shared rule first would leave an intermediate
commit that compiles but renders `Unknown` in every surface that has not migrated yet — green CI,
broken app. Migrating one surface first would leave the other failing its guard.

The way out is **parallel change**: widen the contract, migrate the surfaces one at a time, then
narrow it. Every commit builds, passes the full suite, and leaves the app working.

| Step | What it does | Why it is safe on its own |
|---|---|---|
| **Expand** | `isDisplayableAdminId` accepts an address **or** a compressed pubkey; the safety caption is chosen from the value's shape instead of being one constant | Pure widening. Every caller still passes a pubkey and still gets today's exact behaviour and today's exact caption |
| **Migrate — connect** | The connect flow's two surfaces pass the address; the duplicated pubkey row goes | The panel keeps passing a pubkey, which the widened rule still accepts. Connect is correct end-to-end and demo-able |
| **Migrate — panel** | The panel, the session chip and the nine screens pass the address; "Address on device" goes; Rule 9's `addressSample` prohibition is inverted **in this same commit** | Nothing passes a pubkey any more. Demo-able |
| **Contract** | The rule narrows to address-only; the pubkey branch, its caption and the transitional guard assertions are deleted; the test now asserts a raw pubkey is **rejected** | Nothing depends on the widened behaviour, so removing it changes nothing observable — and the narrowed test is what stops the regression from coming back |

The two guards are never "fixed later": each guard assertion moves in the same commit as the change
that makes it true. There is no batch whose job is to repair a tree an earlier batch broke.

### Definition of done per step — `red → green → refactor`

No step is finished at green. Each one runs the full cycle:

1. **Red** — the test describing the change is written first and fails for the right reason. On the
   React surfaces, where the repo has no vitest/RTL, the equivalent is the structural assertion in
   `architecture.test.ts` or the WebDriver spec: added first, watched to fail.
2. **Green** — the smallest implementation that satisfies it, plus the full local CI checklist.
3. **Refactor** — suite green, **behaviour unchanged**, cleaning up what this step just touched.

The refactor beat lands in the **same commit** as the step. Splitting it into a follow-up would
reintroduce exactly the non-atomic sequencing the expand/contract design exists to avoid.

What the third beat looks for, cheapest first: duplication the step introduced (typically the same
copy literal or derivation appearing on two surfaces — it goes to the single audited source in
`lib/`, the pattern the repo already uses for the §4.3.5 send copy); names the change made
inaccurate; the `sdd` Phase 6 thresholds (files over 200 lines, functions over 40); and
simplification — conditionals the change made redundant, props left without a reader, dead code from
the previous shape. In this spec's design the **contract** step is that last category promoted to a
step of its own, which is why it carries a commit rather than hiding inside B3.

Two boundaries keep the refactor from becoming undeclared scope:

- **Only what the step touched.** Pre-existing debt noticed in passing is recorded as a candidate,
  not fixed on the side; if it is large it earns its own step and its own commit.
- **Behaviour and refactor never mix inside the third beat.** If cleaning up surfaces a behaviour
  change, it goes back to *red* — test first.

### The validation rule

`isDisplayableAdminId` currently requires 33-byte compressed-pubkey hex (`lib/pubkey.ts`), which
**rejects addresses on purpose**. Its final form is: non-empty, not the `'Mnemonic signer'`
placeholder, and bech32-shaped (`bc1` / `tb1` / `bcrt1` + bech32 charset). It reaches that form
through the expand/contract steps above, accepting both shapes in between.

This is a **display guard, not a consensus validator** — and the spec says so out loud. There is no
address validator in the frontend today (destination validation for Send BTC lives in Rust), and
adding a bech32 dependency to gate a string the device itself produced would be theatre. The
authority on the Admin ID's correctness is the device, via verify-on-device; this check only stops
the UI from labelling a placeholder or an empty string as an identity.

### Copy literals (restored, not invented)

`issues/extracted/images/image5.jpg` is a screenshot of the pre-#408 build showing the target state
verbatim. The literals return to:

- `ADMIN_ID_LABEL` = `Admin ID` (unchanged).
- `ADMIN_ID_SAFETY_CAPTION` = **"For authentication only — never send funds to this address."**
- `adminIdVerifyCaption(vendor)` loses the indirection ("shows the address derived from this key and
  path — hardware signers cannot display a raw public key") and simply states that the device shows
  the Admin ID itself, for comparison.

### Architecture compliance (`architecture.test.ts` — Rule 9)

Rule 9 is rewritten, not deleted. It keeps enforcing the same three properties under the corrected
requirement:

| Assertion | Before | After |
|---|---|---|
| `wallet-session-control.tsx` must not surface `addressSample` as the Admin ID (`:295-298`) | required by #408 | **inverted** — it must surface exactly that |
| `lib/admin-id.ts` owns the caption literal (`:304-306`) | `it is a public key, not a payment address.` | `never send funds to this address.` — during expand both literals are required, and the old one is dropped at contract |
| Admin ID surfaces must not render `QrCode` (`:311-317`) | kept | **kept**, justification updated |
| `authority-selection-phase.tsx` renders `<ConnectAdminIdCard adminId={adminId} />` (`:320-322`, #410) | kept | **kept** |
| `admin-id-row.tsx` carries `e2e-wallet-admin-id-verify-address` + `expectedAddress={verify.address}` (`:326-334`) | required the separate address block | **replaced** — `expectedAddress` is now the Admin ID itself |
| `model/admin-id-presentation.ts` re-exports from `@/lib/admin-id` (`:301-303`) | kept | **kept** |

### Test helpers

None. Tests target the pure model functions and the structural wiring assertions only. No new Tauri
commands, no exposed test utilities.

## Test Cases

**Pure model — `lib/admin-id.ts`** (`model/__tests__/admin-id-presentation.test.ts`, rewritten):

- `isDisplayableAdminId('bc1q…')` → **true** (this is the assertion currently inverted at `:25-30`);
- `isDisplayableAdminId('bcrt1q…')` / `tb1q…` → true (regtest and testnet);
- `isDisplayableAdminId('02…64 hex')` → **false** — a raw pubkey is no longer an Admin ID, so the
  regression cannot silently come back;
- `isDisplayableAdminId('')` / `undefined` / `'Mnemonic signer'` → false;
- `ADMIN_ID_SAFETY_CAPTION` and `ADMIN_ID_LABEL` equal the exact literals above;
- `adminIdVerifyCaption(vendor)` no longer contains `'cannot display a raw public key'`;
- `matchesDeviceAddress` unchanged (case-insensitive per BIP-173) — it now compares the Admin ID
  against the device, rather than a derived proxy.

**Copy — `lib/__tests__/device-copy.test.ts`:** the per-vendor hints no longer claim the Admin ID on
screen is a public key.

**Architecture wiring — `architecture.test.ts` Rule 9:** the six assertions in the table above.

**Manual / E2E (non-blocking, repo convention):** extend
`desktop-app/e2e-webdriver/test/specs/admin-wallet-panel.e2e.js` to assert the panel's Admin ID value
is bech32-shaped, and add a connect-flow assertion that the value at step 2, step 3 and the panel is
**byte-identical**. This is the falsifiable check for B0's single-source claim — if the three differ,
the loop stops and re-plumbs before continuing.

> React components cannot be unit-tested here (no vitest/RTL in the repo). They are covered by the
> structural Rule 9 plus the optional WebDriver spec, consistent with the rest of `admin-wallet`.

## Module structure

No new modules. All changes land in existing locations and respect the dependency directions already
enforced by Rules 1–3:

- **`src/lib/`** (pure, no React, no transport): `admin-id.ts`, `device-copy.ts`.
- **`domain/*/components/`** (presentational, no `@/api/*`, no `@tauri-apps/api/core`):
  `admin-id-row.tsx`, `connect-admin-id-card.tsx`, `authenticate-session-phase.tsx`,
  `wallet-session-control.tsx`.
- **`domain/connect-wallet/hooks/`**: `use-hw-wallet-connect.ts`.
- **`screens/`**: route-level prop wiring only.

`src/lib/pubkey.ts` stays — the auth flow and G8 still consume compressed pubkeys. It just stops
being the Admin ID's validator.

## Out-of-scope follow-ups (tracked for the matrix)

- **G8** — Admin ID Verification Certificate (PRD 06 §3.c.i, §4.a). §4.2 stays **PARTIAL** until then.
- **G9** — device QA + compliance close-out; §4.1 and §4.2 → **PASS** there, with evidence.
- **Payout Administrator P2TR Admin ID** (PRD 06 §3.b.ii.1) → matrix row **DEFER**, destination the
  `block_payouts` program.
- On merge, update [`admin-wallet-prd-compliance.md`](./admin-wallet-prd-compliance.md): repoint the
  header **PRD source** from `03-prd-update.md` to `06-…`, and correct the §4.1 note, which today
  records the compressed-public-key rendering as the requirement.
