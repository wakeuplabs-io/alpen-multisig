// Defcon 1 is irreversible, so its two frontend gates get pinned: the type-to-confirm
// friction (AC 5) and the authority-keyed menu that is the only entry point to it (AC 1).

import assert from 'node:assert/strict'
import { matchesDefconConfirmation } from '../validators/defcon-1.ts'
import { getActionTypeOptions } from '../action-type-config.ts'

// AC 5 — case-insensitive, and nothing else. The contract's own rule is
// `input.toUpperCase() === "DEFCON 1"`, so there is deliberately no trim().
for (const accepted of ['DEFCON 1', 'defcon 1', 'Defcon 1']) {
	assert.equal(matchesDefconConfirmation(accepted), true, `must accept ${JSON.stringify(accepted)}`)
}
// The near-misses the contract's Edge Cases name by hand, plus the empty field.
for (const rejected of ['defcon1', 'DEFCON 1 ', ' DEFCON 1', 'DEFCON', '']) {
	assert.equal(matchesDefconConfirmation(rejected), false, `must reject ${JSON.stringify(rejected)}`)
}

// AC 1 — the council is offered Defcon 1 and nothing else...
assert.deepEqual(
	getActionTypeOptions('security_council').map((option) => option.actionType),
	['defcon_1'],
)

// ...and no other authority is offered it. `getActionTypeOptions` falls back to the Strata
// Administrator's menu for an unknown authority, so that fallback is checked too: it is the
// path a typo'd authority string would take.
for (const authority of ['strata_admin', 'sequencer_manager', 'alpen_admin', 'not_an_authority']) {
	const actionTypes = getActionTypeOptions(authority).map((option) => option.actionType)
	assert.ok(!actionTypes.includes('defcon_1'), `${authority} must not be offered defcon_1`)
}
