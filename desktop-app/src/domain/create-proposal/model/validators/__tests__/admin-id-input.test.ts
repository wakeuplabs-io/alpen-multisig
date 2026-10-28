// AC 3: the signer-update input takes an Admin ID on the chain's network. A compressed public key,
// the format it used to take, is refused with its own message (docs/specs/asm-v0.5-signer-address.md).

import assert from 'node:assert/strict'
import {
	ADMIN_ID_IS_PUBKEY_MESSAGE,
	ADMIN_ID_SHAPE_MESSAGE,
	adminIdInputError,
	chainAdminIdHrp,
	validateSignerKeyInput,
} from '../types.ts'

// The generator point: its compressed key and its Admin ID on regtest and mainnet (BIP-173).
const PUBKEY = '0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798'
const REGTEST_ID = 'bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080'
const MAINNET_ID = 'bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4'
// 2·G on regtest, a current signer.
const CURRENT = 'bcrt1qq6hag67dl53wl99vzg42z8eyzfz2xlkvwk6f7m'

assert.equal(adminIdInputError(PUBKEY, 'bcrt'), ADMIN_ID_IS_PUBKEY_MESSAGE, 'a pasted public key names itself')
assert.equal(adminIdInputError(`0x${PUBKEY}`, 'bcrt'), ADMIN_ID_IS_PUBKEY_MESSAGE, 'with the 0x prefix too')
assert.equal(adminIdInputError('not an address', 'bcrt'), ADMIN_ID_SHAPE_MESSAGE, 'anything else is a shape error')
assert.match(adminIdInputError(MAINNET_ID, 'bcrt') ?? '', /bcrt1q/, "another network's Admin ID is refused")
assert.equal(adminIdInputError(` ${REGTEST_ID.toUpperCase()} `, 'bcrt'), null, 'case and padding do not matter')
assert.equal(adminIdInputError(MAINNET_ID, null), null, 'with no current set to read the network from, only the shape')

assert.equal(chainAdminIdHrp([CURRENT]), 'bcrt', 'the HRP comes from the current signer set')
assert.equal(chainAdminIdHrp(null), null, 'an unread set gives no HRP')

const validate = (key: string) =>
	validateSignerKeyInput({ key, currentSigners: [CURRENT], existingKeysToAdd: [], keysToRemove: [] })
assert.equal(validate(REGTEST_ID), null, 'a regtest Admin ID is accepted')
assert.equal(validate(PUBKEY), ADMIN_ID_IS_PUBKEY_MESSAGE, 'the add-signer field refuses a public key')
assert.match(validate(MAINNET_ID) ?? '', /network/, 'and an Admin ID for another network')
assert.equal(
	validate(CURRENT.toUpperCase()),
	'Signer already exists in the current set',
	'members compare case-insensitively',
)

console.log('admin id input: all assertions passed')
