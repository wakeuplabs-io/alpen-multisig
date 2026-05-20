import assert from 'node:assert/strict'
import { assertWalletPubkeyBinding } from './wallet-binding.ts'

// matching pubkeys (same case) must not throw
assertWalletPubkeyBinding('02aabbcc', '02aabbcc')
console.log('wallet-binding: matching pubkeys OK')

// matching pubkeys (different case) must not throw
assertWalletPubkeyBinding('02AABBCC', '02aabbcc')
console.log('wallet-binding: case-insensitive match OK')

// mismatched pubkeys must throw with a descriptive message
let threw = false
try {
	assertWalletPubkeyBinding('02aabbcc', '03ddeeff')
} catch (e) {
	threw = true
	assert.ok(e instanceof Error)
	assert.ok(
		e.message.includes('03ddeeff') && e.message.includes('02aabbcc'),
		`error message should include both pubkeys, got: ${e.message}`,
	)
}
assert.equal(threw, true, 'expected assertWalletPubkeyBinding to throw on pubkey mismatch')
console.log('wallet-binding: mismatch throws descriptive error OK')
