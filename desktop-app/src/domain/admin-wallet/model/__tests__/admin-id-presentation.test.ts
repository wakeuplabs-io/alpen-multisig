// admin-id-presentation — pure model unit test (tsx + node:assert).
//
// PRD §4.1 as corrected by feedback issue #408: the Admin ID *is* the signer's
// compressed public key (33 bytes, 02/03 prefix), not a Bitcoin address. A bech32
// address or the connect placeholder 'Mnemonic signer' is NOT an Admin ID.

import assert from 'node:assert/strict'
import {
	isDisplayableAdminId,
	truncateAdminId,
	ADMIN_ID_LABEL,
	ADMIN_ID_SAFETY_CAPTION,
} from '../admin-id-presentation.ts'
import { adminIdVerifyCaption, matchesDeviceAddress } from '../../../../lib/admin-id.ts'

const PUBKEY = '02f8b7d1a1b9f0f4e8f1c2d3a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8'

// Compressed public keys are displayable, with and without the 0x prefix.
assert.equal(isDisplayableAdminId(PUBKEY), true, '02-prefixed key → true')
assert.equal(isDisplayableAdminId(`03${PUBKEY.slice(2)}`), true, '03-prefixed key → true')
assert.equal(isDisplayableAdminId(`0x${PUBKEY}`), true, '0x prefix → true')
assert.equal(isDisplayableAdminId(`  ${PUBKEY}  `), true, 'surrounding whitespace tolerated')

// Addresses are NOT the Admin ID anymore (#408).
assert.equal(isDisplayableAdminId('bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq'), false, 'bech32 address → false')
assert.equal(
	isDisplayableAdminId('bcrt1q9d4ywgfnd8h43da5tpcxcn6ajv590cg6d3tg6axemvljvt2k76zqnxz0jh'),
	false,
	'regtest address → false',
)

// Missing / empty / malformed / placeholder values are NOT displayable.
assert.equal(isDisplayableAdminId(undefined), false, 'undefined → false')
assert.equal(isDisplayableAdminId(''), false, 'empty → false')
assert.equal(isDisplayableAdminId('   '), false, 'whitespace → false')
assert.equal(isDisplayableAdminId('Mnemonic signer'), false, 'placeholder → false')
assert.equal(isDisplayableAdminId(`04${PUBKEY.slice(2)}`), false, 'uncompressed prefix → false')
assert.equal(isDisplayableAdminId(PUBKEY.slice(0, -2)), false, 'short key → false')
assert.equal(isDisplayableAdminId(`${PUBKEY}ff`), false, 'long key → false')
assert.equal(isDisplayableAdminId(`02${'z'.repeat(64)}`), false, 'non-hex → false')

// Truncation keeps both ends so the signer can compare against the device.
const truncated = truncateAdminId(PUBKEY)
assert.ok(truncated.startsWith(PUBKEY.slice(0, 10)), 'keeps the prefix')
assert.ok(truncated.endsWith(PUBKEY.slice(-8)), 'keeps the suffix')
assert.equal(truncateAdminId('02ab'), '02ab', 'short values are returned untouched')

// The verify-on-device caption names the connected signer and never claims the
// device renders the public key itself (#409 device limitation).
const trezorCaption = adminIdVerifyCaption('trezor')
assert.ok(trezorCaption.includes('Trezor'), 'names the connected device')
assert.ok(!trezorCaption.includes('Ledger'), 'never names the other vendor')
assert.ok(adminIdVerifyCaption('ledger').includes('Ledger'))
assert.ok(trezorCaption.includes('cannot display a raw public key'), 'states the device limitation')

// The device-address comparison backs the only verification a hardware signer can offer.
const ADDR = 'bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq'
assert.equal(matchesDeviceAddress(ADDR, ADDR), true, 'identical → match')
assert.equal(matchesDeviceAddress(ADDR, ADDR.toUpperCase()), true, 'bech32 is case-insensitive')
assert.equal(matchesDeviceAddress(ADDR, ` ${ADDR}\n`), true, 'device padding is tolerated')
assert.equal(
	matchesDeviceAddress(ADDR, 'tb1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq'),
	false,
	'different HRP → no match (wrong device app)',
)
assert.equal(matchesDeviceAddress(ADDR, `${ADDR.slice(0, -1)}x`), false, 'one character off → no match')
assert.equal(matchesDeviceAddress('', ADDR), false, 'no expected address → never a match')
assert.equal(matchesDeviceAddress(ADDR, ''), false, 'device returned nothing → no match')

// Copy literals are owned by the shared module (single audited source — architecture Rule 9).
assert.equal(ADMIN_ID_LABEL, 'Admin ID')
assert.equal(ADMIN_ID_SAFETY_CAPTION, 'For authentication only — it is a public key, not a payment address.')

console.log('admin-id-presentation: all assertions passed.')
