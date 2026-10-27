// useSessionAdminId — which rendering of the session's Admin ID a surface shows, and which one the device check gets.
//
// The hook needs a React renderer (not installed); its decision lives in `sessionAdminId`.

import assert from 'node:assert/strict'
import { sessionAdminId } from '../use-session-admin-id.ts'

const KEY = '0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798'
// The generator point's P2WPKH program, as a Trezor names it and as regtest does.
const DEVICE = 'bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4'
const CHAIN = 'bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080'

assert.deepEqual(
	sessionAdminId(DEVICE, KEY, new Map([[KEY, CHAIN]])),
	{ display: CHAIN, device: DEVICE },
	'the surface shows the chain rendering; the device check keeps the device one',
)
assert.deepEqual(
	sessionAdminId(DEVICE, `0x${KEY.toUpperCase()}`, new Map([[KEY, CHAIN]])),
	{ display: CHAIN, device: DEVICE },
	'the key is looked up normalized',
)
assert.deepEqual(
	sessionAdminId(DEVICE, KEY, new Map()),
	{ display: undefined, device: DEVICE },
	'while the key resolves the surface shows nothing yet, never the device rendering it is about to replace',
)
assert.deepEqual(
	sessionAdminId(DEVICE, KEY, new Map([[KEY, '']])),
	{ display: DEVICE, device: DEVICE },
	'a key with no Admin ID falls back to the device rendering',
)
assert.deepEqual(
	sessionAdminId(DEVICE, undefined, new Map()),
	{ display: DEVICE, device: DEVICE },
	'no key to resolve: the device rendering is all there is',
)
assert.deepEqual(
	sessionAdminId(undefined, undefined, new Map()),
	{ display: '', device: undefined },
	'no session values: shown as unknown, not as still resolving',
)
console.log('useSessionAdminId: sessionAdminId OK')
