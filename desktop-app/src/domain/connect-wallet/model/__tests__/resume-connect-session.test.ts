import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import {
	readAuthorityStepFromLocationState,
	resolveManualBackNavigation,
	selectedStateFromWallet,
} from '../resume-connect-session.ts'
import type { WalletAccountInfo } from '@/wallet/types'

const wallet: WalletAccountInfo = {
	deviceLabel: 'Software Wallet',
	derivationPath: "m/84'/1'/0'/0/0",
	addressSample: 'bc1qexampleadminidxxxxxxxxxxxxxxxxxxxxxx',
	publicKeyHex: '02' + 'ab'.repeat(32),
}

const seeded = selectedStateFromWallet(wallet)
assert.equal(seeded.phase, 'selected', 'existing wallet must open in selected phase')
assert.equal(seeded.account, wallet)
assert.equal(seeded.selectedEntry.index, 0)
assert.equal(seeded.selectedEntry.derivationPath, wallet.derivationPath)
assert.equal(seeded.selectedEntry.address, wallet.addressSample)
assert.equal(seeded.selectedEntry.publicKeyHex, wallet.publicKeyHex)
console.log('selectedStateFromWallet: seeds selected phase OK')

assert.equal(readAuthorityStepFromLocationState(null), 'select-authority')
assert.equal(readAuthorityStepFromLocationState({}), 'select-authority')
assert.equal(readAuthorityStepFromLocationState({ authorityStep: 'authenticate-session' }), 'authenticate-session')
assert.equal(readAuthorityStepFromLocationState({ authorityStep: 'select-authority' }), 'select-authority')
assert.equal(readAuthorityStepFromLocationState({ authorityStep: 'nope' }), 'select-authority')
console.log('readAuthorityStepFromLocationState: OK')

assert.deepEqual(resolveManualBackNavigation(null), { kind: 'history' })
assert.deepEqual(resolveManualBackNavigation({ prefill: {} }), { kind: 'history' })
assert.deepEqual(
	resolveManualBackNavigation({
		returnTo: { path: '/', authorityStep: 'authenticate-session' },
	}),
	{ kind: 'returnTo', path: '/', authorityStep: 'authenticate-session' },
)
assert.deepEqual(
	resolveManualBackNavigation({
		returnTo: { path: '', authorityStep: 'authenticate-session' },
	}),
	{ kind: 'history' },
)
console.log('resolveManualBackNavigation: OK')

const here = dirname(fileURLToPath(import.meta.url))
const screensDir = join(here, '../../../../screens')
const walletConnect = readFileSync(join(screensDir, 'wallet-connect-screen.tsx'), 'utf8')
const manual = readFileSync(join(screensDir, 'manual-proposal-screen.tsx'), 'utf8')
const hook = readFileSync(join(here, '../../hooks/use-hw-wallet-connect.ts'), 'utf8')
const hwConnect = readFileSync(join(here, '../../components/hw-wallet-connect.tsx'), 'utf8')

assert.ok(
	walletConnect.includes("authorityStep: 'authenticate-session'"),
	'offline entry must tag returnTo Authenticate',
)
assert.ok(walletConnect.includes('existingWallet={wallet}'), 'connect screen must pass session wallet for rehydrate')
assert.ok(walletConnect.includes('readAuthorityStepFromLocationState'), 'connect screen must restore authority step')
assert.ok(manual.includes('resolveManualBackNavigation'), 'manual screen must use explicit Back resolution')
assert.ok(manual.includes('handleDisconnect'), 'Disconnect handler must not be named handleBack')
assert.ok(!manual.includes('async function handleBack('), 'manual screen must not keep disconnect as handleBack')
assert.ok(hook.includes('selectedStateFromWallet'), 'connect hook must seed from existing wallet')
assert.ok(hook.includes('existingWallet'), 'connect hook must accept existingWallet')
assert.ok(hwConnect.includes('existingWallet'), 'HwWalletConnect must forward existingWallet')

console.log('All resume-connect-session tests passed.')
