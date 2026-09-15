// device-signing-display — what each signer shows for a message signature.

import assert from 'node:assert/strict'
import { deviceSigningDisplay, signingMessageSection } from '../device-signing-display.ts'

const message = 'Strata ASM Administration v1\nAction: ...'
const messageHash = 'ee020aa4a02d55a674aee20764aaa760d463559e7858c91f14f'

// Ledger → both values: which one a device shows turns on the model, the app version and the
// message itself — the governance message still renders as a hash on some builds — and the app cannot
// tell which in advance (#402 — a Nano X was observed rendering the full text).
// The hash is upper-cased to match the device, which prints it with "%02X".
assert.deepEqual(deviceSigningDisplay('ledger', { message, messageHash }), {
	kind: 'hash-and-text',
	deviceLabel: 'Ledger',
	hash: messageHash.toUpperCase(),
	text: message,
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
// Ledger needs both values, so either one missing degrades.
assert.deepEqual(deviceSigningDisplay('ledger', { message, messageHash: null }), { kind: 'none' })
assert.deepEqual(deviceSigningDisplay('ledger', { message: null, messageHash }), { kind: 'none' })
assert.deepEqual(deviceSigningDisplay('trezor', { message: null, messageHash }), { kind: 'none' })

console.log('device-signing-display: all assertions passed')

// signingMessageSection — the message appears exactly once on a signing screen: inside the device
// hint when there is one, as its own section when there is not, and nowhere before it resolves.
assert.equal(signingMessageSection(deviceSigningDisplay('ledger', { message, messageHash }), message), null)
assert.equal(signingMessageSection(deviceSigningDisplay('trezor', { message, messageHash }), message), null)
assert.equal(signingMessageSection(deviceSigningDisplay('mnemonic', { message, messageHash }), message), message)
assert.equal(signingMessageSection(deviceSigningDisplay('mock', { message, messageHash }), message), message)
assert.equal(signingMessageSection(deviceSigningDisplay('mnemonic', { message: null, messageHash: null }), null), null)
