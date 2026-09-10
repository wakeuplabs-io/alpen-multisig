import assert from 'node:assert/strict'
import { decodedActionAuthorizingAuthority } from '../authorizing-authority.ts'

assert.equal(
	decodedActionAuthorizingAuthority({
		kind: 'multisig_update',
		role: 'security_council',
		addKeys: [],
		removeKeys: [],
		newThreshold: 2,
	}),
	'strata_admin',
	'a Security Council rotation is authorized by the Strata Administrator',
)

assert.equal(
	decodedActionAuthorizingAuthority({
		kind: 'multisig_update',
		role: 'strata_admin',
		addKeys: [],
		removeKeys: [],
		newThreshold: 2,
	}),
	'strata_admin',
	'a regular administrator signer update remains self-authorized',
)

assert.equal(
	decodedActionAuthorizingAuthority({ kind: 'defcon_3' }),
	'security_council',
	'a Defcon action remains authorized by the Security Council',
)

console.log('authorizing-authority: all assertions passed')
