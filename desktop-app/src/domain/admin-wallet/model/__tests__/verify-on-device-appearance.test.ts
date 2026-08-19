// verify-on-device-appearance — pure model unit test (tsx + node:assert).
//
// The control is reused on two surfaces (the certificate modal's Step 2, and beside a receive
// address), and its states are re-entrable: a signer can verify, get a mismatch, and verify again.
// What is asserted here is the part that is easy to get wrong by hand — that success reads as
// finished without becoming a dead end, and that no alarm state inherits the success colour.

import assert from 'node:assert/strict'
import type { VerifyOnDeviceState } from '../../hooks/use-verify-on-device.ts'
import { verifyOnDeviceAppearance, verifyOnDeviceClassName } from '../verify-on-device-appearance.ts'

const IDLE_LABEL = 'Verify'
const BUSY_LABEL = 'Confirm on your Ledger…'

const idle: VerifyOnDeviceState = { status: 'idle' }
const verifying: VerifyOnDeviceState = { status: 'verifying' }
const verified: VerifyOnDeviceState = { status: 'verified', address: 'bcrt1qexample' }
const mismatch: VerifyOnDeviceState = { status: 'mismatch', address: 'bcrt1qother' }
const failed: VerifyOnDeviceState = { status: 'failed', message: 'device disconnected' }

// ── Success reads as finished, in the label as well as the colour ────────────

const onVerified = verifyOnDeviceAppearance(verified, IDLE_LABEL, BUSY_LABEL)
assert.equal(onVerified.label, 'Verified')
assert.equal(onVerified.icon, 'check')
assert.equal(onVerified.isBusy, false, 'a confirmed verification must stay clickable, not disabled')

const onIdle = verifyOnDeviceAppearance(idle, IDLE_LABEL, BUSY_LABEL)
assert.equal(onIdle.label, IDLE_LABEL)
assert.equal(onIdle.icon, 'shield')

const onVerifying = verifyOnDeviceAppearance(verifying, IDLE_LABEL, BUSY_LABEL)
assert.equal(onVerifying.label, BUSY_LABEL)
assert.equal(onVerifying.isBusy, true)

// The accessible name carries the state, so screen-reader users get it without the colour.
assert.notEqual(onVerified.label, onIdle.label)

// ── Only `verified` is green, on both variants ───────────────────────────────

const EMERALD = '#6ee7b7'
const SUCCESS_SURFACE = '#ecfdf5'

for (const variant of ['chip', 'primary'] as const) {
	const className = verifyOnDeviceClassName(verified, variant)
	assert.ok(className.includes(EMERALD), `${variant}: verified must carry the emerald border`)
	assert.ok(className.includes(SUCCESS_SURFACE), `${variant}: verified must carry the success surface`)

	// A mismatch is a security alarm. Inheriting the success colour from a previous run would put a
	// green "Verified" button next to a red mismatch panel — the two would contradict each other.
	for (const alarm of [mismatch, failed]) {
		const alarmClassName = verifyOnDeviceClassName(alarm, variant)
		assert.ok(!alarmClassName.includes(EMERALD), `${variant}/${alarm.status}: must not stay green`)
		assert.ok(!alarmClassName.includes(SUCCESS_SURFACE), `${variant}/${alarm.status}: must not stay green`)
	}

	// After an alarm the control has to look ready to be used again, not stuck in a spent state.
	assert.equal(verifyOnDeviceClassName(mismatch, variant), verifyOnDeviceClassName(idle, variant))
	assert.equal(verifyOnDeviceClassName(failed, variant), verifyOnDeviceClassName(idle, variant))

	// Verifying is the only state that blocks a second press.
	assert.ok(verifyOnDeviceClassName(verifying, variant).includes('cursor-wait'))
	assert.ok(!verifyOnDeviceClassName(verified, variant).includes('cursor-wait'))
}

// The alarm labels stay on the idle wording: the panel below says what went wrong, and a button
// labelled with the failure would leave nothing to press to try again.
assert.equal(verifyOnDeviceAppearance(mismatch, IDLE_LABEL, BUSY_LABEL).label, IDLE_LABEL)
assert.equal(verifyOnDeviceAppearance(failed, IDLE_LABEL, BUSY_LABEL).label, IDLE_LABEL)

console.log('verify-on-device-appearance: OK')
