import assert from 'node:assert/strict'
import { formatAdminWalletError } from '../format-admin-wallet-error.ts'
import type { AdminWalletError } from '@/api/admin-wallet.ts'

// Disabled → severity 'info'
const disabled = formatAdminWalletError({ type: 'Disabled' } satisfies AdminWalletError)
assert.equal(disabled.severity, 'info')
assert.ok(disabled.title.length > 0)
assert.ok(disabled.body.length > 0)

// RpcUnreachable → severity 'warning'
const rpcUnreachable = formatAdminWalletError({ type: 'RpcUnreachable', message: 'timeout' } satisfies AdminWalletError)
assert.equal(rpcUnreachable.severity, 'warning')

// ElectrumUnreachable → severity 'warning' (R2.2 Electrum sync)
const electrumUnreachable = formatAdminWalletError({
	type: 'ElectrumUnreachable',
	message: 'tcp://127.0.0.1:60401: connection refused',
} satisfies AdminWalletError)
assert.equal(electrumUnreachable.severity, 'warning')
assert.ok(electrumUnreachable.title.toLowerCase().includes('electrum'))

// RpcAuthFailed → severity 'warning'
const rpcAuthFailed = formatAdminWalletError({
	type: 'RpcAuthFailed',
	message: 'auth failed',
} satisfies AdminWalletError)
assert.equal(rpcAuthFailed.severity, 'warning')

// DescriptorParseError → severity 'fatal'
const descriptorParseError = formatAdminWalletError({
	type: 'DescriptorParseError',
	message: 'bad descriptor',
} satisfies AdminWalletError)
assert.equal(descriptorParseError.severity, 'fatal')

// RegtestGuardViolation → severity 'info'
const regtestViolation = formatAdminWalletError({
	type: 'RegtestGuardViolation',
	message: 'violation msg',
} satisfies AdminWalletError)
assert.equal(regtestViolation.severity, 'info')

// SyncIncomplete → severity 'warning', body === err.message
const syncMsg = 'sync partial at block 42'
const syncIncomplete = formatAdminWalletError({ type: 'SyncIncomplete', message: syncMsg } satisfies AdminWalletError)
assert.equal(syncIncomplete.severity, 'warning')
assert.equal(syncIncomplete.body, syncMsg)

// No dev/roadmap wording leaks into user-facing copy (R1.2 cleanup).
const variants: AdminWalletError[] = [
	{ type: 'Disabled' },
	{ type: 'RpcUnreachable', message: 'x' },
	{ type: 'ElectrumUnreachable', message: 'x' },
	{ type: 'RpcAuthFailed', message: 'x' },
	{ type: 'DescriptorParseError', message: 'x' },
	{ type: 'SyncIncomplete', message: 'x' },
	{ type: 'RegtestGuardViolation', message: 'x' },
	{ type: 'ReadOnly' },
	{ type: 'InvalidMnemonic', message: 'x' },
	{ type: 'Descriptor', message: 'x' },
	{ type: 'WalletCreation', message: 'x' },
]
const forbiddenWording = [/dev mnemonic/i, /palabras/i]
for (const variant of variants) {
	const view = formatAdminWalletError(variant)
	const text = `${view.title} ${view.body}`
	for (const pattern of forbiddenWording) {
		assert.ok(!pattern.test(text), `error copy for ${variant.type} must not contain ${pattern}`)
	}
}

// ── Phase 5 — fee bump variants (PRD §4.3.3) ─────────────────────────────────

// Every bump variant must produce a non-empty title and body.
const bumpVariants: AdminWalletError[] = [
	{ type: 'SignerNotAllowedOnNetwork' },
	{ type: 'InvalidTxid', message: 'x' },
	{ type: 'TxNotFound', message: 'x' },
	{ type: 'TxAlreadyConfirmed', message: 'x' },
	{ type: 'TxNotReplaceable', message: 'x' },
	{ type: 'GovernanceCommitNotReplaceable', message: 'x' },
	{ type: 'FeeTooLow', message: 'x' },
	{ type: 'FeeRateTooLow', message: 'x' },
	{ type: 'InvalidFeeRate', message: 'x' },
	{ type: 'InsufficientFunds', message: 'x' },
	{ type: 'BuildFailed', message: 'x' },
	{ type: 'SignFailed', message: 'x' },
	{ type: 'BroadcastFailed', message: 'x' },
]
for (const variant of bumpVariants) {
	const view = formatAdminWalletError(variant)
	assert.ok(view.title.length > 0, `${variant.type} must have a title`)
	assert.ok(view.body.length > 0, `${variant.type} must have a body`)
}

// Spot-check the high-signal copy for the dedicated guards.
const governance = formatAdminWalletError({ type: 'GovernanceCommitNotReplaceable', message: 'x' })
assert.ok(/reveal/i.test(governance.body), 'governance copy must explain the reveal dependency')

const notReplaceable = formatAdminWalletError({ type: 'TxNotReplaceable', message: 'x' })
assert.ok(/RBF/.test(notReplaceable.body), 'not-replaceable copy must mention RBF')

const feeTooLow = formatAdminWalletError({ type: 'FeeRateTooLow', message: 'at least 1100 sat/kvB required' })
assert.ok(feeTooLow.body.includes('at least 1100'), 'fee-too-low copy must carry the required rate')

console.log('format-admin-wallet-error: all assertions passed.')
