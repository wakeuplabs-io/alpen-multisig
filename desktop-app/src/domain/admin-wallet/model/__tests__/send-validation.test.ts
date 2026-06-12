// send-validation — pure model tests (Phase 6, PRD §4.3.5.2 / §4.3.5.5).

import assert from 'node:assert/strict'
import { canConfirmSend, parseAmountSats } from '../send-validation.ts'

// ── parseAmountSats ──────────────────────────────────────────────────────────

// Accepts plain digit strings (sats are integers).
assert.equal(parseAmountSats('0'), 0)
assert.equal(parseAmountSats('1'), 1)
assert.equal(parseAmountSats('30000'), 30_000)
assert.equal(parseAmountSats('  30000  '), 30_000, 'surrounding whitespace is trimmed')

// Rejects everything that is not a plain non-negative integer.
for (const bad of ['', '   ', '-1', '1.5', '1e3', '0x10', '1,000', '1_000', 'abc', '10 sats', '+5']) {
	assert.equal(parseAmountSats(bad), null, `${JSON.stringify(bad)} must parse to null`)
}

// Rejects unsafe integers (would lose precision crossing IPC).
assert.equal(parseAmountSats('9007199254740993'), null, 'beyond Number.MAX_SAFE_INTEGER must be rejected')

console.log('parseAmountSats OK')

// ── canConfirmSend (P6.1 interim gate, PRD §4.3.5.5) ─────────────────────────

const ready = { isDestinationValid: true, amountSats: 30_000, isFeeReady: true, isSubmitting: false }

assert.equal(canConfirmSend(ready), true, 'all-valid gate must enable Confirm')

// Each gating dimension toggled independently must disable Confirm.
assert.equal(
	canConfirmSend({ ...ready, isDestinationValid: false }),
	false,
	'unvalidated destination must disable (P6.2 — backend-authoritative, §4.3.5.1)',
)
assert.equal(canConfirmSend({ ...ready, amountSats: null }), false, 'unparseable amount must disable')
assert.equal(canConfirmSend({ ...ready, amountSats: 0 }), false, 'zero amount must disable (§4.3.5.2)')
assert.equal(canConfirmSend({ ...ready, isFeeReady: false }), false, 'missing fee presets must disable')
assert.equal(canConfirmSend({ ...ready, isSubmitting: true }), false, 'mid-submit must disable (no double send)')

console.log('canConfirmSend OK')
console.log('send-validation.test.ts PASS')
