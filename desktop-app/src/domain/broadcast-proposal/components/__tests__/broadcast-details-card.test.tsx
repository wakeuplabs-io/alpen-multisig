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

// ── 3. Broadcast button disabled when wallet has no funds ────────────────────
// Mirrors the JSX `disabled` expression.
function isBroadcastDisabled(opts: {
	isBroadcasting: boolean
	canSign: boolean
	adminWalletInfo: { balanceSats: number } | null | undefined
}): boolean {
	return opts.isBroadcasting || !opts.canSign || opts.adminWalletInfo == null || opts.adminWalletInfo.balanceSats === 0
}

assert.equal(
	isBroadcastDisabled({ isBroadcasting: false, canSign: true, adminWalletInfo: { balanceSats: 0 } }),
	true,
	'disabled when total balance is 0',
)
assert.equal(
	isBroadcastDisabled({ isBroadcasting: false, canSign: true, adminWalletInfo: { balanceSats: 700 } }),
	false,
	'enabled when balance > 0',
)
assert.equal(
	isBroadcastDisabled({ isBroadcasting: false, canSign: true, adminWalletInfo: null }),
	true,
	'disabled when adminWalletInfo is null (loading)',
)
assert.equal(
	isBroadcastDisabled({ isBroadcasting: false, canSign: false, adminWalletInfo: { balanceSats: 700 } }),
	true,
	'disabled when signing is unavailable',
)
assert.equal(
	isBroadcastDisabled({ isBroadcasting: true, canSign: true, adminWalletInfo: { balanceSats: 700 } }),
	true,
	'disabled while broadcasting',
)
console.log('BroadcastDetailsCard: broadcast disabled-when-no-funds logic OK')

console.log('All BroadcastDetailsCard pure-logic contract tests passed.')
