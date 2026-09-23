// Contract Constraint 5: no Defcon 3 surface may reuse Defcon 1's copy. The two levels sweep the
// same funds, but one is cancelable until it activates and the other is not, so a shared string is
// a wrong statement on one of the two screens.
//
// This asserts difference and nothing stronger. Forbidding the word "irreversible" in the Defcon 3
// body would be wrong — the sweep genuinely cannot be undone once it activates — and searching for
// "cancelable" would pin phrasing rather than behaviour. What the assertion guards is the real
// regression: someone "deduplicating" the copy by collapsing the two levels onto one string.

import assert from 'node:assert/strict'
import { COUNCIL_DASHBOARD_SAFE_HARBOR_NOTE, DEFCON_COPY, type DefconCopy } from '../defcon-copy.ts'

const fields = Object.keys(DEFCON_COPY.defcon_1) as (keyof DefconCopy)[]
assert.ok(fields.length > 0, 'the copy table must not be empty')

for (const field of fields) {
	assert.notEqual(
		DEFCON_COPY.defcon_3[field],
		DEFCON_COPY.defcon_1[field],
		`defcon_3.${field} must not reuse defcon_1's wording`,
	)
}

// The confirmation gate (case-insensitive, near-misses refused, neither level's string arming the
// other) is asserted through the schema in defcon-confirm-gate.test.ts, the matcher's only caller.

// The dashboard banner is not a per-level note (AC 15): it must name both levers and never be
// Defcon-1-only.
assert.ok(COUNCIL_DASHBOARD_SAFE_HARBOR_NOTE.includes('Defcon 1'), 'the dashboard note must name Defcon 1')
assert.ok(COUNCIL_DASHBOARD_SAFE_HARBOR_NOTE.includes('Defcon 3'), 'the dashboard note must name Defcon 3')
assert.notEqual(
	COUNCIL_DASHBOARD_SAFE_HARBOR_NOTE,
	DEFCON_COPY.defcon_1.safeHarborNote,
	'the dashboard note must not reuse the Defcon 1 form note',
)

console.log('defcon-copy: the two levels never share a string')
