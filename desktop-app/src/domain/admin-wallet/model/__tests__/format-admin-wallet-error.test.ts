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

console.log('format-admin-wallet-error: all assertions passed.')
