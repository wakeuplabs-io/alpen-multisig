// multisig-update-target — the authority a decoded action modifies (Constraint 2), not the
// session's authority.
//
// The detail view builds its Before/After signer table from `getMultisigConfig(proposal.authority)`.
// For a council rotation the proposal's authority is `strata_admin` (the administrator), so without
// this function that call reads the administrator's config and the table renders the wrong signers
// under "Before". Commit 6 uses this value to suppress that table rather than show a false one;
// Phase 3 uses the same value to fetch the right config instead of suppressing.

import assert from 'node:assert/strict'
import { multisigUpdateTargetAuthority } from '../multisig-update-target.ts'

assert.equal(
	multisigUpdateTargetAuthority({
		kind: 'multisig_update',
		role: 'security_council',
		addKeys: [],
		removeKeys: [],
		newThreshold: 2,
	}),
	'security_council',
)

// The no-regression case: for the three self-rotating authorities `role === proposal.authority`
// always holds, so the guard built on this function is a no-op for everything shipped so far.
assert.equal(
	multisigUpdateTargetAuthority({
		kind: 'multisig_update',
		role: 'strata_admin',
		addKeys: [],
		removeKeys: [],
		newThreshold: 2,
	}),
	'strata_admin',
)

assert.equal(multisigUpdateTargetAuthority({ kind: 'defcon_3' }), null)
assert.equal(
	multisigUpdateTargetAuthority({ kind: 'vk_update', authority: 'strata_admin', typeId: 1, conditionHex: '' }),
	null,
)
assert.equal(multisigUpdateTargetAuthority({ kind: 'unknown', rawHex: 'ff' }), null)
assert.equal(multisigUpdateTargetAuthority(null), null)

console.log('multisig-update-target: all assertions passed.')
