// SignerAdminId — a listed signer reads as its Admin ID, never as its public key (feedback item 60, AC 1).

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { renderToStaticMarkup } from 'react-dom/server'
import { adminIdDisplay, includesAdminId } from '../../lib/admin-id.ts'
import { SignerAdminId } from '../signer-admin-id.tsx'

const ADMIN_ID = 'bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080'

assert.equal(typeof SignerAdminId, 'function', 'SignerAdminId must be exported')

assert.equal(adminIdDisplay(undefined, 'truncated'), '…', 'a key still resolving reads as an ellipsis')
assert.equal(adminIdDisplay('', 'truncated'), 'Unknown', 'a key with no Admin ID reads Unknown')
assert.equal(adminIdDisplay(ADMIN_ID, 'full'), ADMIN_ID)
assert.equal(adminIdDisplay(ADMIN_ID, 'truncated'), 'bcrt1qw508…7kygt080')
assert.equal(
	adminIdDisplay('0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798', 'full'),
	'Unknown',
	'a public key is never displayable as an Admin ID',
)
console.log('SignerAdminId: display rules OK')

const __dirname = dirname(fileURLToPath(import.meta.url))
const source = readFileSync(join(__dirname, '..', 'signer-admin-id.tsx'), 'utf8')
assert.ok(!source.includes('truncatePubkey'), 'the component must not fall back to printing the public key')
console.log('SignerAdminId: no public-key fallback OK')

// A signer the ASM lists is already an Admin ID: printed as given, with no resolution step.
const listed = renderToStaticMarkup(<SignerAdminId adminId={ADMIN_ID} variant="full" />)
assert.ok(listed.includes(`>${ADMIN_ID}</span>`), 'an ASM-listed signer prints its Admin ID at once')
const notAnId = renderToStaticMarkup(
	<SignerAdminId adminId="0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798" />,
)
assert.ok(notAnId.includes('>Unknown</span>'), 'a key passed where an Admin ID belongs reads Unknown, not the key')
console.log('SignerAdminId: ASM-listed signers OK')

assert.equal(includesAdminId([ADMIN_ID], ADMIN_ID.toUpperCase()), true, 'membership ignores case')
assert.equal(includesAdminId([ADMIN_ID], ''), false, 'an empty Admin ID is in no set')
console.log('includesAdminId: OK')
