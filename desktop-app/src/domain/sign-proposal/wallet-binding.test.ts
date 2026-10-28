import assert from 'node:assert/strict'
import { assertWalletPubkeyBinding, WALLET_BINDING_MISMATCH } from './wallet-binding.ts'

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
	assert.equal(e.message, WALLET_BINDING_MISMATCH)
	assert.ok(
		!e.message.includes('03ddeeff') && !e.message.includes('02aabbcc'),
		`the message names the session by Admin ID and prints no public key, got: ${e.message}`,
	)
}
assert.equal(threw, true, 'expected assertWalletPubkeyBinding to throw on pubkey mismatch')
console.log('wallet-binding: mismatch throws descriptive error OK')
