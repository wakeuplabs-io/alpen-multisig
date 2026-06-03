import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const addressRowSource = readFileSync(join(__dirname, '..', '..', 'components', 'address-row.tsx'), 'utf8')

assert.ok(
	addressRowSource.includes('formatBtcFromSats(confirmedSats)'),
	'AddressRow must format confirmed sats as hero',
)
assert.ok(addressRowSource.includes('formatUnconfirmedBalanceLine'), 'AddressRow must use formatUnconfirmedBalanceLine')
assert.ok(!addressRowSource.includes('balanceSats'), 'AddressRow must not use legacy balanceSats prop')

const listSource = readFileSync(join(__dirname, '..', '..', 'components', 'addresses-with-balance-list.tsx'), 'utf8')
assert.ok(listSource.includes('Addresses with balance ·'), 'accordion header must use R1.6 copy')

console.log('address-row-contract: all assertions passed.')
