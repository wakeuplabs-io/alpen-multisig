// admin-id-presentation — pure model unit test (tsx + node:assert).
//
// PRD §4.1: after login the user MUST see their Admin ID and copy it. The
// Admin ID is the canonical BIP-84 auth address (wallet.addressSample). When
// no real address is available the connect flow substitutes the placeholder
// 'Mnemonic signer', which is NOT a copyable identity.

import assert from 'node:assert/strict'
import { isDisplayableAdminId, ADMIN_ID_LABEL, ADMIN_ID_SAFETY_CAPTION } from '../admin-id-presentation.ts'

// Real addresses are displayable.
assert.equal(isDisplayableAdminId('bcrt1q9d4ywgfnd8h43da5tpcxcn6ajv590cg6d3tg6axemvljvt2k76zqnxz0jh'), true)
assert.equal(isDisplayableAdminId('bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq'), true)

// Missing / empty / placeholder values are NOT displayable.
assert.equal(isDisplayableAdminId(undefined), false, 'undefined → false')
assert.equal(isDisplayableAdminId(''), false, 'empty → false')
assert.equal(isDisplayableAdminId('   '), false, 'whitespace → false')
assert.equal(isDisplayableAdminId('Mnemonic signer'), false, 'placeholder → false')

// Copy literals are owned here (single audited source — see architecture Rule 9).
assert.equal(ADMIN_ID_LABEL, 'Admin ID')
assert.equal(ADMIN_ID_SAFETY_CAPTION, 'For authentication only — never send funds to this address.')

console.log('admin-id-presentation: all assertions passed.')
