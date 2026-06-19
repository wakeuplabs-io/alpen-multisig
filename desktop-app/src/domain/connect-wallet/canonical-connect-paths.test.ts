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

const connectModel = read('domain/connect-wallet/model/hw-wallet-connect.types.ts')
const connectHook = read('domain/connect-wallet/hooks/use-hw-wallet-connect.ts')
const walletTypes = read('wallet/types.ts')
const hwAdapter = read('wallet/hw-adapter.ts')
const mnemonicAdapter = read('wallet/mnemonic-adapter.ts')
const hwWalletCommands = readDesktop('src-tauri/src/commands/hw_wallet.rs')
const invokeCommands = readDesktop('src-tauri/src/commands/invoke.rs')
const loginMnemonicHelper = readDesktop('e2e-webdriver/test/helpers/login-mnemonic.mjs')
const e2ePackage = readDesktop('e2e-webdriver/package.json')

assert.ok(!connectModel.includes("'picking'"), 'connect state machine must not expose a picking phase')
assert.ok(!connectHook.includes('listAddresses'), 'connect hook must not enumerate selectable signer addresses')
assert.ok(!walletTypes.includes('listAddresses'), 'WalletAdapter must not expose derivation picking')
assert.ok(!hwAdapter.includes('listAddresses'), 'HW adapter must not expose derivation picking')
assert.ok(!mnemonicAdapter.includes('listAddresses'), 'Mnemonic adapter must not expose derivation picking')
assert.ok(mnemonicAdapter.includes('count: 1'), 'mnemonic connect must derive only the canonical Admin ID')
assert.ok(!hwWalletCommands.includes('list_hw_addresses'), 'Trezor address-list command must not be registered')
assert.ok(!hwWalletCommands.includes('list_ledger_addresses'), 'Ledger address-list command must not be registered')
assert.ok(!invokeCommands.includes('list_hw_addresses'), 'invoke handler must not register Trezor address listing')
assert.ok(!invokeCommands.includes('list_ledger_addresses'), 'invoke handler must not register Ledger address listing')
assert.ok(!loginMnemonicHelper.includes('pickingRowIndex'), 'e2e login helper must not choose a derivation row')
assert.ok(!loginMnemonicHelper.includes('e2e-picking-row'), 'e2e login helper must not click the picking grid')
assert.ok(!e2ePackage.includes('proposal-co-sign-row1'), 'row-1 co-sign spec must not remain as an npm script')

console.log('Canonical connect-path contract checks passed.')
