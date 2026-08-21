// isBroadcastConfirmDisabled — the "Confirm & Send" gate, shared by the send-proposal and
// send-cancel screens.
//
// Regression net for issue #484: the cancel screen rendered BroadcastDetailsCard without
// `adminWalletInfo`, so the gate saw `undefined`, matched `adminWalletInfo == null`, and the
// button was disabled forever — a queued update could never be cancelled from the UI.

import assert from 'node:assert/strict'
import { isBroadcastConfirmDisabled } from '../broadcast-proposal.ts'
import type { BroadcastConfirmGateInput } from '../broadcast-proposal.ts'

/** Everything green: funded wallet, signing available, nothing in flight. */
const enabled: BroadcastConfirmGateInput = {
	isBroadcasting: false,
	canSign: true,
	targetQueued: true,
	adminWalletInfo: { balanceSats: 50_000 },
}

function gate(overrides: Partial<BroadcastConfirmGateInput>): boolean {
	return isBroadcastConfirmDisabled({ ...enabled, ...overrides })
}

// ── Enabled ──────────────────────────────────────────────────────────────────

assert.equal(gate({}), false, 'funded + canSign + queued target must enable the button')
assert.equal(gate({ targetQueued: null }), false, 'targetQueued null (not checked yet) must not block')
assert.equal(
	gate({ targetQueued: undefined }),
	false,
	'targetQueued undefined (send-proposal screen, no cancel target) must not block',
)
assert.equal(gate({ adminWalletInfo: { balanceSats: 1 } }), false, 'any non-zero balance is enough to try')

// ── Disabled ─────────────────────────────────────────────────────────────────

assert.equal(gate({ isBroadcasting: true }), true, 'no double submit while a broadcast is in flight')
assert.equal(gate({ canSign: false }), true, 'signing unavailable must block')
assert.equal(gate({ targetQueued: false }), true, 'a cancel whose target left the ASM queue must block')
assert.equal(gate({ adminWalletInfo: { balanceSats: 0 } }), true, 'an unfunded admin wallet cannot pay the commit')
assert.equal(gate({ adminWalletInfo: null }), true, 'wallet info still loading must block')
assert.equal(gate({ adminWalletInfo: undefined }), true, 'wallet info unavailable must block')

console.log('isBroadcastConfirmDisabled: gate truth table OK')
