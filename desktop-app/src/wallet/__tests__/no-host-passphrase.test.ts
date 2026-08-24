// A hardware passphrase is never typed on, held by, or sent from this machine (#448).
//
// The Trezor passphrase unlocks a hidden wallet. Typing it here puts it in reach of a
// keylogger on the host, which is the whole reason the keys live on a device. The signer
// enters it on the device keypad instead, so there must be nowhere in the app for a
// host-side passphrase to appear.
//
// What that does *not* forbid is choosing which wallet to open. The passphrase is a
// per-session parameter rather than device state, so the host selects the wallet on every
// connection by how it answers PassphraseRequest — an empty string opens the standard wallet,
// deferring to the keypad opens a hidden one. An earlier revision of this file banned the
// empty-string answer and allowed only one connect control, which removed the choice rather
// than the secret; those assertions were replaced, not relaxed.
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

assert.ok(
	connectPhase.includes('e2e-passphrase-on-device'),
	'the connect screen must still tell the signer where the passphrase is entered',
)

// Two controls start a Trezor connection, and they must open *different* wallets. One seed backs
// the standard wallet plus a distinct wallet per passphrase, and the host picks which by how it
// answers PassphraseRequest, so the choice is real (issues/evidence/G5-B0-PROTOCOL.md).
//
// What this guards is the earlier defect, not the second button: two CTAs wired to the same
// argument would be the rival-CTA problem for real, promising a choice the app does not make.
// So the kinds are counted rather than the calls.
const connectKinds = code(connectPhase).match(/onConnectTrezor\(\s*'(standard|hidden)'\s*\)/g) ?? []
assert.equal(connectKinds.length, 2, 'the connect screen must offer exactly two Trezor wallet choices')
assert.equal(new Set(connectKinds).size, 2, 'the two Trezor controls must open different wallets')

// A bare call would leave the wallet to the default and make the two controls identical again.
assert.ok(!/onConnectTrezor(\?\.)?\(\s*\)/.test(code(connectPhase)), 'a Trezor control must name the wallet it opens')

const messageLine = code(connectPhase)
	.split('\n')
	.find((line) => line.includes('e2e-passphrase-on-device'))
assert.ok(messageLine !== undefined, 'the passphrase message must carry its test id')
assert.ok(!/<button|onClick=/.test(messageLine), 'the passphrase message must be copy, not a button')

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

// The Rust side. Both answers to PassphraseRequest must be present, because they are what makes
// the two controls above mean different things:
//
//   Standard -> ack_passphrase("")   an empty string is the absence of a passphrase
//   Hidden   -> ack(true)            the device prompts on its own keypad
//
// Neither carries a secret from this machine, which is the property this file exists to protect.
assert.ok(/\.ack\(true\)/.test(code(trezorAdapter)), 'the Trezor adapter must ask the device to prompt on its keypad')
assert.ok(
	/ack_passphrase\(String::new\(\)\)/.test(code(trezorAdapter)),
	'the Trezor adapter must answer with an empty passphrase to open the standard wallet',
)

// The empty string is the *only* passphrase this app may send. Anything else -- a variable, a
// literal, a parameter -- is a host-supplied secret, which is the whole defect.
const ackArguments = code(trezorAdapter).match(/ack_passphrase\(([^)]*\)?[^)]*)\)/g) ?? []
for (const call of ackArguments) {
	assert.equal(call, 'ack_passphrase(String::new())', `the Trezor adapter must not send a passphrase: ${call}`)
}

// Connecting must start its own device session, for the wallet it was asked for. Without this a
// session opened under one passphrase would be inherited by the next connection, which the signer
// never authorised -- and the kind must travel with it, or a later operation on a lost session
// would be answered for the other wallet.
assert.ok(
	/pub fn connect\([^)]*\)[^{]*\{[^}]*start_session\(kind\)/s.test(code(trezorAdapter)),
	'connect must start a clean device session for the wallet it was asked for',
)

console.log('no-host-passphrase: all assertions passed')
