// SignerAdminId — a listed signer reads as its Admin ID, never as its public key (feedback item 60, AC 1).

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { adminIdDisplay } from '../../lib/admin-id.ts'
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
