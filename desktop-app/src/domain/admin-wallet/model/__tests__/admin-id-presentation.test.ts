// admin-id-presentation — pure model unit test (tsx + node:assert).
//
// PRD 06 §3.b.ii.2: the Admin ID is a P2WPKH bitcoin address at m/84'/0'/73'/0/0.
// Every surface now renders that address, so the parallel change is contracted
// (spec: admin-id-as-bitcoin-address.md): the compressed public key that used to be
// accepted during the migration is rejected here, which is what stops the July
// rendering from creeping back in unnoticed.

import assert from 'node:assert/strict'
import {
	isDisplayableAdminId,
	truncateAdminId,
	ADMIN_ID_LABEL,
	ADMIN_ID_SAFETY_CAPTION,
} from '../admin-id-presentation.ts'
import { adminIdVerifyCaption, matchesDeviceAddress } from '@/lib/admin-id'

const PUBKEY = '02f8b7d1a1b9f0f4e8f1c2d3a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8'
const ADDR = 'bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq'
const REGTEST_ADDR = 'bcrt1q9d4ywgfnd8h43da5tpcxcn6ajv590cg6d3tg6axemvljvt2k76zqnxz0jh'

// ── Only an address is an Admin ID ───────────────────────────────────────────

assert.equal(isDisplayableAdminId(ADDR), true, 'bech32 address → true (PRD 06 §3.b.ii.2)')
assert.equal(isDisplayableAdminId(REGTEST_ADDR), true, 'regtest address → true')
assert.equal(isDisplayableAdminId('tb1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq'), true, 'testnet address → true')
assert.equal(isDisplayableAdminId(ADDR.toUpperCase()), true, 'bech32 is case-insensitive (BIP-173)')
assert.equal(isDisplayableAdminId(`  ${ADDR}  `), true, 'surrounding whitespace tolerated')

// The compressed public key was the Admin ID between PR #444 and PRD 06. Rejecting it
// is the point of this test: the reversion is undone the moment a surface can quietly
// pass a key again and still render.
assert.equal(isDisplayableAdminId(PUBKEY), false, 'compressed key → false (the July rendering, now reverted)')
assert.equal(isDisplayableAdminId(`03${PUBKEY.slice(2)}`), false, '03-prefixed key → false')
assert.equal(isDisplayableAdminId(`0x${PUBKEY}`), false, '0x-prefixed key → false')

assert.equal(isDisplayableAdminId(undefined), false, 'undefined → false')
assert.equal(isDisplayableAdminId(''), false, 'empty → false')
assert.equal(isDisplayableAdminId('   '), false, 'whitespace → false')
assert.equal(isDisplayableAdminId('Mnemonic signer'), false, 'placeholder → false')
assert.equal(isDisplayableAdminId('1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2'), false, 'base58 is not a supported shape')
assert.equal(isDisplayableAdminId('bc1'), false, 'HRP alone is not an address')
assert.equal(isDisplayableAdminId('bc1qbio'), false, 'b/i/o are outside the bech32 charset')

// ── The device shows the Admin ID itself ─────────────────────────────────────
// The indirection #409 objected to is gone: no supported signer can render a raw
// public key, but every one of them renders an address.

const trezorCaption = adminIdVerifyCaption('trezor')
assert.ok(trezorCaption.includes('Trezor'), 'names the connected device')
assert.ok(!trezorCaption.includes('Ledger'), 'never names the other vendor')
assert.ok(adminIdVerifyCaption('ledger').includes('Ledger'))
assert.ok(!trezorCaption.includes('derived'), 'the device shows the Admin ID, not something derived from it')

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
assert.equal(ADMIN_ID_SAFETY_CAPTION, 'For authentication only — never send funds to this address.')

console.log('admin-id-presentation: all assertions passed.')
