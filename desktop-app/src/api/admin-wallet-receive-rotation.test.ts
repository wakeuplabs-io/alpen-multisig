import assert from 'node:assert/strict'

// R1.3 (receive rotation): acceptance test for the receive-address IPC bridge + hook export.
//
// Verifies nextAdminWalletReceiveAddress invokes the `admin_wallet_next_receive_address`
// command with no positional payload (the Rust command takes only WalletSession state), and
// that the receive-address hook is exported from the hooks barrel.

// Intercept Tauri's invoke before importing the bridge.
const calls: Array<{ cmd: string; args: unknown }> = []
;(globalThis as unknown as { window: unknown }).window = {
	__TAURI_INTERNALS__: {
		invoke: (cmd: string, args: unknown) => {
			calls.push({ cmd, args })
			return Promise.resolve({ index: 0, address: 'bcrt1pTEST', isUsed: false })
		},
	},
}

const { nextAdminWalletReceiveAddress } = await import('./admin-wallet.ts')

assert.equal(typeof nextAdminWalletReceiveAddress, 'function', 'nextAdminWalletReceiveAddress must be exported')

const result = await nextAdminWalletReceiveAddress()
const call = calls[calls.length - 1]
assert.equal(call.cmd, 'admin_wallet_next_receive_address', 'must invoke admin_wallet_next_receive_address')
assert.deepEqual(call.args, {}, 'receive-address command takes no positional args')
assert.equal(result.ok, true, 'happy-path result must be ok')
if (result.ok) {
	assert.equal(result.data.address, 'bcrt1pTEST')
	assert.equal(result.data.index, 0)
	assert.equal(result.data.isUsed, false)
}
console.log('admin-wallet-receive-rotation: command name + args OK')

// The hook must be exported from the hooks barrel.
const hooks = await import('@/domain/admin-wallet/hooks/index.ts')
assert.equal(
	typeof (hooks as Record<string, unknown>).useAdminWalletReceiveAddress,
	'function',
	'useAdminWalletReceiveAddress must be exported from the hooks barrel',
)
console.log('admin-wallet-receive-rotation: useAdminWalletReceiveAddress exported OK')
