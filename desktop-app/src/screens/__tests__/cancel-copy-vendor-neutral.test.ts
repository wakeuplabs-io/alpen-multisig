// The cancel flow must name the signer that is actually connected.
//
// Issue #487: signing a cancel proposal with the Software Wallet still showed "Sign on hardware
// wallet" — the signer was told to use a device they do not have. Every other flow already routes
// its signer-facing copy through `deviceCopy(vendor)` (#420/#421/#426); cancel was the one left
// with the string hardcoded.
//
// Source-text assertions, following the project's existing tsx-runner style — React rendering
// tests need vitest + @testing-library/react (BLOCKED_BY_DEPENDENCY — not installed).

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const srcDir = join(dirname(fileURLToPath(import.meta.url)), '..', '..')

const cancelDetailsCardPath = join(srcDir, 'domain', 'cancel-proposal', 'components', 'cancel-details-card.tsx')

const files = {
	'screens/cancel-proposal-screen.tsx': readFileSync(join(srcDir, 'screens', 'cancel-proposal-screen.tsx'), 'utf8'),
	'screens/cancel-proposal-sign-screen.tsx': readFileSync(
		join(srcDir, 'screens', 'cancel-proposal-sign-screen.tsx'),
		'utf8',
	),
	'domain/cancel-proposal/components/cancel-details-card.tsx': readFileSync(cancelDetailsCardPath, 'utf8'),
}

for (const [name, source] of Object.entries(files)) {
	// No hardcoded device family: the vendor decides what the signer is called.
	assert.ok(!/hardware wallet/i.test(source), `${name}: must not name "hardware wallet" — use deviceCopy(vendor).label`)

	// And no vendor named outright either — a Trezor signer must never read "Ledger".
	assert.ok(!/\b(Trezor|Ledger)\b/.test(source), `${name}: must not hardcode a vendor name`)

	assert.ok(source.includes("from '@/lib/device-copy'"), `${name}: must import the shared device copy`)
	assert.ok(source.includes('deviceCopy('), `${name}: must derive its signer label from deviceCopy(vendor)`)
}
console.log('cancel flow: signer copy derived from the connected vendor OK')

// The card takes the vendor as a required prop, so `tsc` rejects a screen that forgets to wire it —
// that is what caught the same class of omission in broadcast-details-card (#484).
const cardSource = files['domain/cancel-proposal/components/cancel-details-card.tsx']
const propsBlock = cardSource.slice(cardSource.indexOf('type Props = {'), cardSource.indexOf('export function'))
assert.ok(
	/\n\twalletVendor: WalletVendor\n/.test(propsBlock),
	'walletVendor must stay a required prop — optional (walletVendor?:) is how #487 survives a refactor',
)
console.log('CancelDetailsCard: walletVendor is a required prop OK')

// The screen has to pass the connected adapter's vendor, not a literal.
assert.ok(
	files['screens/cancel-proposal-screen.tsx'].includes('walletVendor={adapter.vendor}'),
	'cancel-proposal-screen.tsx: must pass the connected adapter vendor into CancelDetailsCard',
)
console.log('cancel-proposal-screen: card wired to the session adapter OK')
