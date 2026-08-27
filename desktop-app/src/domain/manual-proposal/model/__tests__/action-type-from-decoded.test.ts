// action-type-from-decoded — the offline path names the action it decoded (AC 15a).
//
// This replaced a hex-prefix guess that returned `multisig_update` for anything not starting `01`,
// so an imported Defcon 1 read as *Signer update* on the screen a signer reaches precisely when the
// orchestrator cannot tell them what they are holding.
//
// The assertion that matters is the `unknown` arm: it is the one a `default:` would have swallowed.

import assert from 'node:assert/strict'
import { actionTypeFromDecoded } from '../action-type-from-decoded.ts'

assert.equal(actionTypeFromDecoded({ kind: 'defcon_1' }), 'defcon_1')
assert.equal(
	actionTypeFromDecoded({ kind: 'vk_update', authority: 'strata_admin', typeId: 1, conditionHex: '' }),
	'vk_update',
)
assert.equal(
	actionTypeFromDecoded({ kind: 'multisig_update', role: 'r', addKeys: [], removeKeys: [], newThreshold: 2 }),
	'multisig_update',
)
assert.equal(actionTypeFromDecoded({ kind: 'unknown', rawHex: 'ff' }), 'unknown')

console.log('action-type-from-decoded: all assertions passed.')
