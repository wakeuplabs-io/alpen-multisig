// device-copy — signer-facing copy tailored to the connected vendor.

import assert from 'node:assert/strict'
import { deviceCopy } from '../device-copy.ts'
import type { WalletVendor } from '../../wallet/types.ts'

const HARDWARE_NAME = /Trezor|Ledger/

// Trezor → named as Trezor, and pointed at the message text it renders.
const trezor = deviceCopy('trezor')
assert.equal(trezor.label, 'Trezor')
assert.equal(trezor.isHardware, true)
assert.match(trezor.verifyHint, /message text/)
assert.doesNotMatch(trezor.verifyHint, /Ledger/)
assert.doesNotMatch(trezor.broadcastHint, /Ledger/)
assert.doesNotMatch(trezor.verifyOnDeviceHint, /Ledger/)

// Ledger → named as Ledger, and pointed at BOTH values, since the device shows either the
// message text or the SHA-256 "Message hash" depending on model / app version (#402).
const ledger = deviceCopy('ledger')
assert.equal(ledger.label, 'Ledger')
assert.equal(ledger.isHardware, true)
assert.match(ledger.verifyHint, /Message hash/)
assert.match(ledger.verifyHint, /message text/)
assert.doesNotMatch(ledger.verifyHint, /Trezor/)
assert.doesNotMatch(ledger.broadcastHint, /Trezor/)
assert.doesNotMatch(ledger.verifyOnDeviceHint, /Trezor/)

// Software signers → no device screen, and no hardware vendor is ever named (#421).
for (const vendor of ['mnemonic', 'mock'] as const) {
	const copy = deviceCopy(vendor)
	assert.equal(copy.isHardware, false, `${vendor} must not claim a device screen`)
	for (const [field, text] of Object.entries(copy)) {
		if (typeof text !== 'string') {
			continue
		}
		assert.doesNotMatch(text, HARDWARE_NAME, `${vendor}.${field} must not name a hardware vendor`)
	}
}

// Every vendor names itself in the review prompt — guards against a hardcoded device
// name creeping back into a shared screen.
const vendors: WalletVendor[] = ['trezor', 'ledger', 'mnemonic', 'mock']
for (const vendor of vendors) {
	const copy = deviceCopy(vendor)
	assert.match(copy.reviewPrompt, new RegExp(copy.label, 'i'), `${vendor}.reviewPrompt must name its own signer`)
	// The pre-approval prompt must state that nothing leaves the app until the signer acts,
	// so it cannot contradict the "signature submitted" success state (#422).
	assert.match(copy.reviewPrompt, /Nothing is submitted until you/, `${vendor}.reviewPrompt must scope submission`)
}

// A vendor never borrows another vendor's label.
assert.doesNotMatch(deviceCopy('mnemonic').reviewPrompt, HARDWARE_NAME)

// No signer-facing copy ever mentions the sighash: no device displays it, so naming it in a
// comparison instruction points the signer at a value they can never see (#402).
for (const vendor of vendors) {
	for (const [field, text] of Object.entries(deviceCopy(vendor))) {
		if (typeof text !== 'string') {
			continue
		}
		assert.doesNotMatch(text, /sighash/i, `${vendor}.${field} must not mention the sighash`)
	}
}

console.log('device-copy: all assertions passed')
