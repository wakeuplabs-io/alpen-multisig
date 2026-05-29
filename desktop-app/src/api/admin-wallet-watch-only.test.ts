import assert from 'node:assert/strict'

// Acceptance + regression test for the watch-only session-init bridge.
//
// Regression: walletSessionInitWatchOnly previously invoked with the payload flattened
// (`tauriCall(cmd, input)`), but the Rust command takes a single `input: WatchOnlyInitInput`
// parameter, which Tauri binds by name. The payload MUST be wrapped as `{ input }` (the same
// shape walletSessionInit uses) or Tauri fails to deserialise the argument and the command
// body never runs — the watch-only session silently never initialises.

// Intercept Tauri's invoke before importing the bridge. `@tauri-apps/api/core`'s `invoke`
// delegates to `window.__TAURI_INTERNALS__.invoke(cmd, args)` at call time.
const calls: Array<{ cmd: string; args: unknown }> = []
;(globalThis as unknown as { window: unknown }).window = {
	__TAURI_INTERNALS__: {
		invoke: (cmd: string, args: unknown) => {
			calls.push({ cmd, args })
			return Promise.resolve(null)
		},
	},
}

const { walletSessionInitWatchOnly, walletSessionInit, getAdminWalletCanSign } = await import('./admin-wallet.ts')

assert.equal(typeof walletSessionInitWatchOnly, 'function')
assert.equal(typeof getAdminWalletCanSign, 'function')

// Watch-only init must wrap the payload in { input } so Tauri can bind the `input` param.
await walletSessionInitWatchOnly({ xpub: 'tpubTEST' })
const watchOnlyCall = calls[calls.length - 1]
assert.equal(watchOnlyCall.cmd, 'wallet_session_init_watch_only')
assert.deepEqual(
	watchOnlyCall.args,
	{ input: { xpub: 'tpubTEST' } },
	'watch-only init payload must be wrapped in { input }',
)

// Consistency with the mnemonic path (which already wraps in { input }).
await walletSessionInit({ mnemonic: 'abandon about' })
const mnemonicCall = calls[calls.length - 1]
assert.deepEqual(
	mnemonicCall.args,
	{ input: { mnemonic: 'abandon about' } },
	'mnemonic init payload must be wrapped in { input }',
)

console.log('admin-wallet-watch-only: init payloads wrapped in { input } OK')
console.log('admin-wallet-watch-only: walletSessionInitWatchOnly / getAdminWalletCanSign exported OK')
