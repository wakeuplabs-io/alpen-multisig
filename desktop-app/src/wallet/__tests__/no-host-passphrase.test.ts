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
const walletTypes = read('wallet/types.ts')
const hwDispatch = readDesktop('src-tauri/src/infrastructure/hw_wallet/hw_psbt_signer.rs')
const hwWalletMod = readDesktop('src-tauri/src/infrastructure/hw_wallet/mod.rs')

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

// The passphrase block is copy, not a control. A button beside "Connect wallet" would run the
// same handler -- connecting is what makes the device prompt -- while implying the two open
// different wallets, which is the inference a signer should never be invited to make.
const passphraseBlock = connectPhase.slice(
	connectPhase.indexOf('e2e-passphrase-on-device'),
	connectPhase.indexOf('{mnemonicError'),
)
assert.ok(passphraseBlock.length > 0, 'the connect screen must still say where the passphrase is entered')
assert.ok(!/<button|onClick=/.test(passphraseBlock), 'the passphrase block must be copy, not a second connect button')

// The wallet-method callbacks used to thread the secret from the screen down to the adapter.
assert.ok(!/passphrase/i.test(code(hwWalletConnect)), 'the connect flow must not pass a passphrase down')
assert.ok(
	!/selectAdapter\('trezor', \{ passphrase/.test(walletConnectScreen),
	'the connect screen must not build a Trezor adapter around a passphrase',
)
assert.ok(
	!/createHwAdapter\(\s*'(trezor|ledger)'\s*,/.test(createWalletAdapter),
	'hardware adapters take no passphrase argument',
)

// The adapter contract itself. `WalletAdapter.passphrase` used to expose the held secret to
// anything holding an adapter; `WalletAdapterOptions.passphrase` survives for the software
// wallet, where it is a BIP39 passphrase that never leaves the host by design.
assert.ok(
	!/WalletAdapter\s*=\s*\{[^}]*passphrase/s.test(code(walletTypes)),
	'a connected adapter must not carry a passphrase',
)

// The Rust dispatch layer between the commands and the device adapters. It may still talk
// *about* passphrase entry — `supports_passphrase_entry` is how the UI knows to offer it —
// but nothing may carry the secret itself.
const CARRIES_A_PASSPHRASE = /\bpassphrase\s*:\s*(&\s*str|String|Option\s*<)/
for (const [name, source] of [
	['hw_psbt_signer', hwDispatch],
	['hw_wallet/mod', hwWalletMod],
	['commands/hw_wallet', hwWalletCommands],
	['trezor', trezorAdapter],
] as const) {
	assert.ok(!CARRIES_A_PASSPHRASE.test(code(source)), `${name} must not thread a passphrase to the device`)
}

// The Rust side: the IPC commands must not accept one, and the device handler must ask the
// device to prompt rather than answering with a host-supplied string.
assert.ok(!/ack_passphrase/.test(code(trezorAdapter)), 'the Trezor adapter must never send a passphrase to the device')
assert.ok(/\.ack\(true\)/.test(code(trezorAdapter)), 'the Trezor adapter must ask the device to prompt on its keypad')

// Connecting must start its own device session. Without this a session opened under one
// passphrase would be inherited by the next connection, which the signer never authorised.
assert.ok(
	/pub fn connect\([^)]*\)[^{]*\{[^}]*forget_session\(\)/s.test(code(trezorAdapter)),
	'connect must start a clean device session',
)

console.log('no-host-passphrase: all assertions passed')
