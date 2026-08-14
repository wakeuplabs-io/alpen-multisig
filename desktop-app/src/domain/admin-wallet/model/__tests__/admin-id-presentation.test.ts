// admin-id-presentation — pure model unit test (tsx + node:assert).
//
// PRD 06 §3.b.ii.2: the Admin ID is a P2WPKH bitcoin address at m/84'/0'/73'/0/0.
// The app still renders the compressed public key on every surface, so this module
// is in the *expand* step of a parallel change (spec: admin-id-as-bitcoin-address.md):
// it accepts either shape, and the captions are chosen from the shape of the value
// rather than being one constant. The surfaces migrate one per commit; the pubkey
// branch is deleted in the contract step, and this file then pins its rejection.

import assert from 'node:assert/strict'
import { isDisplayableAdminId, truncateAdminId, ADMIN_ID_LABEL } from '../admin-id-presentation.ts'
import { adminIdShape, adminIdSafetyCaption, adminIdVerifyCaption, matchesDeviceAddress } from '@/lib/admin-id'

const PUBKEY = '02f8b7d1a1b9f0f4e8f1c2d3a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8'
const ADDR = 'bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq'
const REGTEST_ADDR = 'bcrt1q9d4ywgfnd8h43da5tpcxcn6ajv590cg6d3tg6axemvljvt2k76zqnxz0jh'

// ── Shape detection — what the widened contract is built on ──────────────────

assert.equal(adminIdShape(ADDR), 'address', 'mainnet bech32 → address')
assert.equal(adminIdShape(REGTEST_ADDR), 'address', 'regtest bech32 → address')
assert.equal(adminIdShape('tb1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq'), 'address', 'testnet bech32 → address')
assert.equal(adminIdShape(ADDR.toUpperCase()), 'address', 'bech32 is case-insensitive (BIP-173)')
assert.equal(adminIdShape(PUBKEY), 'pubkey', 'compressed key → pubkey')
assert.equal(adminIdShape(undefined), 'none', 'undefined → none')
assert.equal(adminIdShape('Mnemonic signer'), 'none', 'placeholder → none')
assert.equal(adminIdShape('1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2'), 'none', 'base58 is not a supported shape')
assert.equal(adminIdShape('bc1'), 'none', 'HRP alone is not an address')
assert.equal(adminIdShape('bc1qbio'), 'none', 'b/i/o are outside the bech32 charset')

// ── Displayability — both shapes, during expand ──────────────────────────────

assert.equal(isDisplayableAdminId(ADDR), true, 'bech32 address → true (PRD 06 §3.b.ii.2)')
assert.equal(isDisplayableAdminId(REGTEST_ADDR), true, 'regtest address → true')
assert.equal(isDisplayableAdminId(PUBKEY), true, 'compressed key → still true while surfaces migrate')
assert.equal(isDisplayableAdminId(`03${PUBKEY.slice(2)}`), true, '03-prefixed key → true')
assert.equal(isDisplayableAdminId(`0x${PUBKEY}`), true, '0x prefix → true')
assert.equal(isDisplayableAdminId(`  ${ADDR}  `), true, 'surrounding whitespace tolerated')

assert.equal(isDisplayableAdminId(undefined), false, 'undefined → false')
assert.equal(isDisplayableAdminId(''), false, 'empty → false')
assert.equal(isDisplayableAdminId('   '), false, 'whitespace → false')
assert.equal(isDisplayableAdminId('Mnemonic signer'), false, 'placeholder → false')
assert.equal(isDisplayableAdminId(`04${PUBKEY.slice(2)}`), false, 'uncompressed prefix → false')
assert.equal(isDisplayableAdminId(PUBKEY.slice(0, -2)), false, 'short key → false')
assert.equal(isDisplayableAdminId(`${PUBKEY}ff`), false, 'long key → false')
assert.equal(isDisplayableAdminId(`02${'z'.repeat(64)}`), false, 'non-hex → false')

// ── Safety caption follows the shape ─────────────────────────────────────────
// An address that must never receive funds and a public key that cannot receive
// them at all are different warnings. Showing either one against the wrong value
// would be a lie on a security-relevant caption.

assert.equal(adminIdSafetyCaption(ADDR), 'For authentication only — never send funds to this address.')
assert.equal(adminIdSafetyCaption(PUBKEY), 'For authentication only — it is a public key, not a payment address.')
assert.equal(
	adminIdSafetyCaption(undefined),
	'For authentication only — never send funds to this address.',
	'unknown values get the address caption: it is the requirement, and the stricter warning',
)

// ── Verify-on-device caption follows the shape too ───────────────────────────
// With the Admin ID as an address the device shows the Admin ID *itself*; with a
// public key it can only show an address derived from it (#409).

const trezorForKey = adminIdVerifyCaption('trezor', PUBKEY)
assert.ok(trezorForKey.includes('Trezor'), 'names the connected device')
assert.ok(!trezorForKey.includes('Ledger'), 'never names the other vendor')
assert.ok(trezorForKey.includes('cannot display a raw public key'), 'states the device limitation for a key')
assert.ok(adminIdVerifyCaption('ledger', PUBKEY).includes('Ledger'))

const trezorForAddress = adminIdVerifyCaption('trezor', ADDR)
assert.ok(trezorForAddress.includes('Trezor'), 'names the connected device')
assert.ok(
	!trezorForAddress.includes('cannot display a raw public key'),
	'the indirection is gone once the Admin ID is the address the device shows',
)

// ── Truncation and device comparison are shape-agnostic ──────────────────────

const truncated = truncateAdminId(ADDR)
assert.ok(truncated.startsWith(ADDR.slice(0, 10)), 'keeps the prefix')
assert.ok(truncated.endsWith(ADDR.slice(-8)), 'keeps the suffix')
assert.equal(truncateAdminId('02ab'), '02ab', 'short values are returned untouched')

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

console.log('admin-id-presentation: all assertions passed.')
