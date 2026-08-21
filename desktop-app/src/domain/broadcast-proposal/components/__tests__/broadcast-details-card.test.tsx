// BroadcastDetailsCard — pure-logic contract tests.
//
// SCOPE: Only pure TypeScript / pure logic is covered here using the project's
// existing tsx test runner. DOM-rendering assertions require vitest +
// @testing-library/react (BLOCKED_BY_DEPENDENCY — not installed).
//
// What IS verified here:
//   1. The component exports correctly (module resolves without error).
//   2. The Funding Source card no longer renders a UTXO count (removed with the
//      Electrum-synced funding info — the count duplicated wallet-panel data and
//      could contradict the balance while a sync was in flight).
//   3. The broadcast button gate: disabled only while broadcasting, when signing is
//      unavailable, while funding info is loading, or when the wallet holds 0 sats
//      (total balance, confirmed + unconfirmed, refreshed after the on-mount sync).

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

// ── 1. Module export resolves ─────────────────────────────────────────────────
import { BroadcastDetailsCard } from '../broadcast-details-card.tsx'
assert.equal(typeof BroadcastDetailsCard, 'function', 'BroadcastDetailsCard must be exported')
console.log('BroadcastDetailsCard: module export OK')

// ── 2. UTXO count removed from the Funding Source card ───────────────────────
const __dirname = dirname(fileURLToPath(import.meta.url))
const cardSource = readFileSync(join(__dirname, '..', 'broadcast-details-card.tsx'), 'utf8')

assert.ok(!cardSource.includes('utxoCount'), 'card must not take a utxoCount prop anymore')
assert.ok(!cardSource.includes('UTXOs:'), 'card must not render a UTXO count line anymore')
assert.ok(
	cardSource.includes('e2e-admin-wallet-funding-address'),
	'funding address testid must be e2e-admin-wallet-funding-address (consumed by the e2e fund helper)',
)
console.log('BroadcastDetailsCard: UTXO count removed OK')

// ── 3. The button gate lives in the model, not inline in the JSX ─────────────
// The card used to spell the `disabled` expression out inline, and this test kept a hand-written
// copy of it — two places to drift. Both now point at one pure function, covered exhaustively in
// model/__tests__/broadcast-confirm-gate.test.ts.

assert.ok(
	cardSource.includes('isBroadcastConfirmDisabled({'),
	'the send button must delegate its disabled state to isBroadcastConfirmDisabled',
)
console.log('BroadcastDetailsCard: confirm gate delegated to the model OK')

// ── 4. Admin-wallet props are required ───────────────────────────────────────
// Issue #484: the cancel screen rendered this card without any admin-wallet prop, so
// `adminWalletInfo` was undefined, the gate matched `adminWalletInfo == null`, and Confirm & Send
// was disabled forever. Required keys make tsc reject a screen that forgets to wire them, so keep
// these props mandatory even though their values stay nullable.

const propsBlock = cardSource.slice(cardSource.indexOf('type Props = {'), cardSource.indexOf('/** bech32'))

for (const prop of ['canSign', 'canSignReason', 'adminWalletInfo', 'lastSyncedAt', 'syncError', 'phase']) {
	assert.ok(
		new RegExp(`\\n\\t${prop}: `).test(propsBlock),
		`${prop} must stay a required prop — optional (${prop}?:) is how #484 happened`,
	)
}
assert.ok(!cardSource.includes('canSign = true'), 'canSign must not default to true — an unwired screen must fail tsc')
console.log('BroadcastDetailsCard: admin-wallet props required OK')

console.log('All BroadcastDetailsCard pure-logic contract tests passed.')
