// device-copy — signer-facing copy tailored to the connected vendor.

import assert from 'node:assert/strict'
import { deviceCopy } from '../device-copy.ts'
import type { WalletVendor } from '../../wallet/types.ts'

const HARDWARE_NAME = /Trezor|Ledger/

/**
 * Every signer-facing string in a DeviceCopy, including the ones nested inside
 * `passphraseOnDevice`. Flattening matters: a nested block that the sweeps below skipped
 * would be exactly where a stray vendor name or a sighash reference survives.
 */
function copyStrings(copy: object, prefix = ''): [string, string][] {
	return Object.entries(copy).flatMap(([field, value]) => {
		if (typeof value === 'string') {
			return [[`${prefix}${field}`, value] as [string, string]]
		}
		if (value !== null && typeof value === 'object') {
			return copyStrings(value, `${prefix}${field}.`)
		}
		return []
	})
}

// Trezor → named as Trezor, and pointed at the message text it renders.
const trezor = deviceCopy('trezor')
assert.equal(trezor.label, 'Trezor')
assert.equal(trezor.isHardware, true)
assert.match(trezor.verifyHint, /message text/)
assert.doesNotMatch(trezor.verifyHint, /Ledger/)
assert.doesNotMatch(trezor.broadcastHint, /Ledger/)
assert.doesNotMatch(trezor.verifyOnDeviceHint, /Ledger/)

// Ledger → named as Ledger, and pointed at BOTH values. Since G10 every message the app signs is
// printable ASCII, so current Bitcoin app versions show the text (#402) — but older models and app
// versions still answer with the SHA-256 "Message hash", and the app cannot tell in advance, so the
// hint has to keep naming both.
const ledger = deviceCopy('ledger')
assert.equal(ledger.label, 'Ledger')
assert.equal(ledger.isHardware, true)
assert.match(ledger.verifyHint, /Message hash/)
assert.match(ledger.verifyHint, /message text/)
assert.doesNotMatch(ledger.verifyHint, /Trezor/)
assert.doesNotMatch(ledger.broadcastHint, /Trezor/)
assert.doesNotMatch(ledger.verifyOnDeviceHint, /Trezor/)

// The verify-on-device hint backs the connect flow's verify step, which now compares the
// Admin ID itself (PRD 06 §3.b.ii.2). Claiming the device shows something *derived* from
// the value on screen would send the signer looking for a second string that no longer
// exists — the indirection #409 objected to, described after it was removed.
for (const vendor of ['trezor', 'ledger'] as const) {
	const { verifyOnDeviceHint } = deviceCopy(vendor)
	assert.doesNotMatch(verifyOnDeviceHint, /derived from/, `${vendor}: the device shows the Admin ID itself`)
	assert.doesNotMatch(verifyOnDeviceHint, /raw public key/, `${vendor}: the Admin ID is not a public key`)
}

// Software signers → no device screen, and no hardware vendor is ever named (#421).
for (const vendor of ['mnemonic', 'mock'] as const) {
	const copy = deviceCopy(vendor)
	assert.equal(copy.isHardware, false, `${vendor} must not claim a device screen`)
	for (const [field, text] of copyStrings(copy)) {
		assert.doesNotMatch(text, HARDWARE_NAME, `${vendor}.${field} must not name a hardware vendor`)
	}
	assert.equal(copy.passphraseOnDevice, undefined, `${vendor} has no device keypad to offer`)
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
	for (const [field, text] of copyStrings(deviceCopy(vendor))) {
		assert.doesNotMatch(text, /sighash/i, `${vendor}.${field} must not mention the sighash`)
	}
}

// On-device passphrase entry is a Trezor-only affordance (#448): Ledger unlocks a passphrase
// wallet by PIN on the device itself, and software signers have no device at all. Describing it
// anywhere else would promise a keypad that is not there.
assert.equal(ledger.passphraseOnDevice, undefined, 'Ledger must not describe on-device passphrase entry')
assert.ok(trezor.passphraseOnDevice, 'Trezor must say where its passphrase is entered')
// This renders while the app is connecting and the device may already be waiting on its keypad.
// It has to name the device and point the signer at it — that is the whole job of the line, and
// it is why the message lives in the connecting state rather than on the idle screen.
assert.match(trezor.passphraseOnDevice, /Trezor/)
assert.match(trezor.passphraseOnDevice, /on the device/)
// One line, not a paragraph: it sits under "Looking for a Trezor on USB." in a status box.
assert.ok(trezor.passphraseOnDevice.length < 90, 'the connecting line must stay a single short sentence')

console.log('device-copy: all assertions passed')
