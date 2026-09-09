// V4 Phase 2 §4.2 — the destination change shown to a signer who never opened the create form.
//
// Three claims, all of them about what the reader is left with rather than about markup.

import assert from 'node:assert/strict'
import { buildSafeHarbourChange } from '../build-safe-harbour-change.ts'

const INSTALLED = {
	address: 'bcrt1p0xlxvlhemja6c4dqv22uapctqupfhlxm9h8z3k2e72q4k9hcz7vqc8gma6',
	addressHex: '0479be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798',
}
const PROPOSED = {
	address: 'bcrt1pqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqn4zdly',
	addressHex: `04${'02'.repeat(32)}`,
}

// Claim 1: before enactment, both sides are shown, and they are the two different destinations.
const pending = buildSafeHarbourChange({ installed: INSTALLED, proposed: PROPOSED, isEnacted: false })
assert.deepEqual(pending, { from: INSTALLED, to: PROPOSED }, 'claim 1: a pending rotation shows what it replaces')

// Claim 2: after enactment there is no "before" to show. `installed` is the live value, so it is
// already the destination this proposal put there — rendering both would print one address twice
// and read as a rotation that changed nothing.
const enacted = buildSafeHarbourChange({ installed: PROPOSED, proposed: PROPOSED, isEnacted: true })
assert.deepEqual(
	enacted,
	{ from: null, to: PROPOSED },
	'claim 2: an enacted rotation shows the installed destination once, not twice',
)

// Claim 3: with no live read there is no comparison, and half of one is worse than none — a lone
// address with no stated role is unreadable on a screen whose purpose is "this replaces that".
assert.equal(
	buildSafeHarbourChange({ installed: null, proposed: PROPOSED, isEnacted: false }),
	null,
	'claim 3: an unreadable installed destination suppresses the section rather than degrading it',
)

console.log('build-safe-harbour-change: all assertions passed')
