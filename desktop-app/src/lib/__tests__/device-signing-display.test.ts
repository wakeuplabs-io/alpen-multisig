// device-signing-display — what each signer shows for a message signature.

import assert from 'node:assert/strict'
import { deviceSigningDisplay } from '../device-signing-display.ts'

const message = 'Strata ASM Administration v1\nAction: ...'
const messageHash = 'ee020aa4a02d55a674aee20764aaa760d463559e7858c91f14f'

// Ledger → the SHA-256 "Message hash" it renders.
assert.deepEqual(deviceSigningDisplay('ledger', { message, messageHash }), {
	kind: 'hash',
	deviceLabel: 'Ledger',
	value: messageHash,
})

// Trezor → the message text it renders.
assert.deepEqual(deviceSigningDisplay('trezor', { message, messageHash }), {
	kind: 'text',
	deviceLabel: 'Trezor',
	value: message,
})

// Software signers → nothing to compare on a device.
assert.deepEqual(deviceSigningDisplay('mnemonic', { message, messageHash }), { kind: 'none' })
assert.deepEqual(deviceSigningDisplay('mock', { message, messageHash }), { kind: 'none' })

// Values still resolving → degrade to none rather than render a partial prompt.
assert.deepEqual(deviceSigningDisplay('ledger', { message, messageHash: null }), { kind: 'none' })
assert.deepEqual(deviceSigningDisplay('trezor', { message: null, messageHash }), { kind: 'none' })

console.log('device-signing-display: all assertions passed')
