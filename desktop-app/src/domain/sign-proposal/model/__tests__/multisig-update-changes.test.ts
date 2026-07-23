// multisig-update-changes — the review screen lists only what actually changes (#423).

import assert from 'node:assert/strict'
import { multisigUpdateChanges } from '../multisig-update-changes.ts'

const KEY_A = '02aaaa'
const KEY_B = '02bbbb'

function update(overrides: { addKeys?: string[]; removeKeys?: string[]; newThreshold?: number } = {}) {
	return {
		kind: 'multisig_update' as const,
		role: 'StrataAdministrator',
		addKeys: overrides.addKeys ?? [],
		removeKeys: overrides.removeKeys ?? [],
		newThreshold: overrides.newThreshold ?? 2,
	}
}

// The reported bug: adding a signer without touching the threshold must not
// report a "New threshold".
const addOnly = multisigUpdateChanges(update({ addKeys: [KEY_A], newThreshold: 2 }), 2)
assert.equal(addOnly.showThreshold, false, 'unchanged threshold must not be presented as new')
assert.deepEqual(addOnly.addKeys, [KEY_A])
assert.equal(addOnly.hasAnyChange, true)

// A real threshold change is still shown.
const raised = multisigUpdateChanges(update({ addKeys: [KEY_A], newThreshold: 3 }), 2)
assert.equal(raised.showThreshold, true)
assert.equal(raised.newThreshold, 3)

// Threshold-only change: no members move, the threshold is the whole proposal.
const thresholdOnly = multisigUpdateChanges(update({ newThreshold: 3 }), 2)
assert.equal(thresholdOnly.showThreshold, true)
assert.deepEqual(thresholdOnly.addKeys, [])
assert.deepEqual(thresholdOnly.removeKeys, [])
assert.equal(thresholdOnly.hasAnyChange, true)

// Current threshold unknown (still loading, or the config read failed) → show it.
// Withholding a value we cannot prove unchanged is the worse failure for a signer.
const unknown = multisigUpdateChanges(update({ addKeys: [KEY_A], newThreshold: 2 }), null)
assert.equal(unknown.showThreshold, true, 'unknown current threshold must fail open')

// Removals are surfaced the same way.
const removal = multisigUpdateChanges(update({ removeKeys: [KEY_B], newThreshold: 2 }), 2)
assert.deepEqual(removal.removeKeys, [KEY_B])
assert.equal(removal.showThreshold, false)
assert.equal(removal.hasAnyChange, true)

// Blank key entries never count as a change.
const blanks = multisigUpdateChanges(update({ addKeys: ['', '   '], removeKeys: [''], newThreshold: 2 }), 2)
assert.deepEqual(blanks.addKeys, [])
assert.deepEqual(blanks.removeKeys, [])
assert.equal(blanks.hasAnyChange, false, 'a proposal that changes nothing visible must say so')

console.log('multisig-update-changes: all assertions passed')
