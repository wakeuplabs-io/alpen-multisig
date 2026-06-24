// Admin wallet capability DTO — Zod parsing and graceful degradation.
//
// Behaviors:
// 1. New DTO {canSign, signerKind, reason?} parses correctly
// 2. Legacy bare-bool degrades to signerKind: 'none'
// 3. reason is only present when canSign === false
// 4. Invalid signerKind values are rejected

import assert from 'node:assert/strict'
import { adminWalletCapabilitySchema } from '@/api/ipc-schemas'

// ── Behavior 1: New DTO parses correctly ─────────────────────────────────────

const hardwareResult = adminWalletCapabilitySchema.parse({ canSign: true, signerKind: 'hardware' })
assert.equal(hardwareResult.canSign, true)
assert.equal(hardwareResult.signerKind, 'hardware')
assert.equal(hardwareResult.reason, undefined)
console.log('parses hardware signer capability OK')

const ledgerResult = adminWalletCapabilitySchema.parse({ canSign: true, signerKind: 'ledger' })
assert.equal(ledgerResult.canSign, true)
assert.equal(ledgerResult.signerKind, 'hardware')
console.log('maps ledger signerKind to hardware OK')

const trezorResult = adminWalletCapabilitySchema.parse({ canSign: true, signerKind: 'trezor' })
assert.equal(trezorResult.canSign, true)
assert.equal(trezorResult.signerKind, 'hardware')
console.log('maps trezor signerKind to hardware OK')

const mnemonicResult = adminWalletCapabilitySchema.parse({ canSign: true, signerKind: 'mnemonic' })
assert.equal(mnemonicResult.canSign, true)
assert.equal(mnemonicResult.signerKind, 'mnemonic')
console.log('parses mnemonic signer capability OK')

// ── Behavior 2: Cannot-sign with reason ──────────────────────────────────────

const cannotSignResult = adminWalletCapabilitySchema.parse({
	canSign: false,
	signerKind: 'none',
	reason: 'No session initialized',
})
assert.equal(cannotSignResult.canSign, false)
assert.equal(cannotSignResult.signerKind, 'none')
assert.equal(cannotSignResult.reason, 'No session initialized')
console.log('parses cannot-sign with reason OK')

// ── Behavior 2b: reason as null (Rust Option<String> serialization) ──────────

const nullReasonResult = adminWalletCapabilitySchema.parse({
	canSign: true,
	signerKind: 'mnemonic',
	reason: null,
})
assert.equal(nullReasonResult.canSign, true)
assert.equal(nullReasonResult.signerKind, 'mnemonic')
assert.equal(nullReasonResult.reason, undefined)
console.log('parses reason: null as undefined OK')

// ── Behavior 3: Graceful degradation of legacy bare-bool ─────────────────────

const legacyTrueResult = adminWalletCapabilitySchema.parse(true)
assert.equal(legacyTrueResult.canSign, true)
assert.equal(legacyTrueResult.signerKind, 'none')
assert.equal(legacyTrueResult.reason, undefined)
console.log('degrades legacy bare-bool true OK')

const legacyFalseResult = adminWalletCapabilitySchema.parse(false)
assert.equal(legacyFalseResult.canSign, false)
assert.equal(legacyFalseResult.signerKind, 'none')
assert.equal(legacyFalseResult.reason, undefined)
console.log('degrades legacy bare-bool false OK')

// ── Behavior 3b: adminWalletAccountPath (device-specific BIP-86 account path) ─

const withAccountPath = adminWalletCapabilitySchema.parse({
	canSign: true,
	signerKind: 'trezor',
	adminWalletAccountPath: "m/86'/0'/73'",
})
assert.equal(withAccountPath.adminWalletAccountPath, "m/86'/0'/73'")
console.log('parses adminWalletAccountPath OK')

// Absent (software session) → null
const noAccountPath = adminWalletCapabilitySchema.parse({ canSign: true, signerKind: 'mnemonic' })
assert.equal(noAccountPath.adminWalletAccountPath, null)
// Legacy bare-bool → null
assert.equal(adminWalletCapabilitySchema.parse(true).adminWalletAccountPath, null)
console.log('defaults adminWalletAccountPath to null OK')

// ── Behavior 4: Invalid signerKind rejected ──────────────────────────────────

try {
	adminWalletCapabilitySchema.parse({ canSign: true, signerKind: 'biometric' })
	assert.fail('should have rejected invalid signerKind')
} catch {
	console.log('rejects invalid signerKind OK')
}

console.log('All admin wallet capability schema tests passed.')
