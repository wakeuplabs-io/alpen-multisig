// A hardware passphrase is never typed on, held by, or sent from this machine (#448).
//
// The Trezor passphrase unlocks a hidden wallet. Typing it here puts it in reach of a
// keylogger on the host, which is the whole reason the keys live on a device. The signer
// enters it on the device keypad instead, so there must be nowhere in the app for a
// host-side passphrase to appear.
//
// This is a source-contract test, like canonical-connect-paths: the paths it guards have no
// runtime seam to assert against, and the failure it prevents is a field or a payload key
// quietly coming back.

import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'

const appSrc = path.resolve(new URL('../..', import.meta.url).pathname)
const desktopRoot = path.resolve(appSrc, '..')

function read(relativePath: string): string {
	return fs.readFileSync(path.join(appSrc, relativePath), 'utf8')
}

function readDesktop(relativePath: string): string {
	return fs.readFileSync(path.join(desktopRoot, relativePath), 'utf8')
}

/**
 * Source with comments removed. The assertions below ban the *word* passphrase from code
 * that must never carry one, and prose explaining why it is absent would otherwise trip them.
 */
function code(source: string): string {
	return source.replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/.*$/gm, '')
}

const hwAdapter = read('wallet/hw-adapter.ts')
const connectPhase = read('domain/connect-wallet/components/connect-phase.tsx')
const hwWalletConnect = read('domain/connect-wallet/components/hw-wallet-connect.tsx')
const walletConnectScreen = read('screens/wallet-connect-screen.tsx')
const createWalletAdapter = read('wallet/create-wallet-adapter.ts')
const hwWalletCommands = readDesktop('src-tauri/src/commands/hw_wallet.rs')
const trezorAdapter = readDesktop('src-tauri/src/infrastructure/hw_wallet/trezor.rs')

// The hardware adapter is the only place that talks to the device commands. Not one of its
// IPC payloads may carry a passphrase — that is the wire the secret would travel on.
assert.ok(!/passphrase/i.test(code(hwAdapter)), 'the hardware adapter must not handle a passphrase at all')

// No control on the connect screen may collect one. The removed field was a
// `type="password"` input backed by local state, which is the shape a re-add would take.
assert.ok(!connectPhase.includes('type="password"'), 'the connect screen must not render a password input')
assert.ok(
	!/useState[^\n]*[Pp]assphrase|[Pp]assphrase[^\n]*useState/.test(code(connectPhase)),
	'the connect screen must not hold a passphrase in state',
)
assert.ok(
	!/htmlFor="[^"]*passphrase|(?<![\w-])id="[^"]*passphrase/i.test(code(connectPhase)),
	'the connect screen must not label a passphrase field',
)

// The wallet-method callbacks used to thread the secret from the screen down to the adapter.
assert.ok(!/passphrase/i.test(code(hwWalletConnect)), 'the connect flow must not pass a passphrase down')
assert.ok(
	!/selectAdapter\('trezor', \{ passphrase/.test(walletConnectScreen),
	'the connect screen must not build a Trezor adapter around a passphrase',
)
assert.ok(
	!/createHwAdapter\('(trezor|ledger)', /.test(createWalletAdapter),
	'hardware adapters take no passphrase argument',
)

// The Rust side: the IPC commands must not accept one, and the device handler must ask the
// device to prompt rather than answering with a host-supplied string.
assert.ok(!/passphrase: Option<String>/.test(hwWalletCommands), 'no hardware IPC command may accept a passphrase')
assert.ok(!/ack_passphrase/.test(code(trezorAdapter)), 'the Trezor adapter must never send a passphrase to the device')
assert.ok(/\.ack\(true\)/.test(code(trezorAdapter)), 'the Trezor adapter must ask the device to prompt on its keypad')

// The session must be droppable, or a hidden wallet stays reachable after disconnect.
assert.ok(/hw_wallet_end_session/.test(hwAdapter), 'disconnect must end the device session')
assert.ok(/pub fn forget_session/.test(trezorAdapter), 'the Trezor adapter must expose session invalidation')

console.log('no-host-passphrase: all assertions passed')
