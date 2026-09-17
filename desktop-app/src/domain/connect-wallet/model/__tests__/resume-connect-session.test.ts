// Back from the offline generator (#411) must land on Authenticate with the session intact, and
// the wizard must never outlive the session wallet.
//
// The pure rules are asserted directly. The wiring that types cannot express — which screen tags
// the return step, which one forwards the session wallet — is asserted against source text,
// following the project's existing tsx-runner style: React rendering tests need vitest +
// @testing-library/react (BLOCKED_BY_DEPENDENCY — not installed).

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import {
	connectStateForWallet,
	hasAuthorityStep,
	readAuthorityStepFromLocationState,
	resolveManualBackNavigation,
} from '../resume-connect-session.ts'
import type { WalletAccountInfo } from '@/wallet/types'

const wallet: WalletAccountInfo = {
	deviceLabel: 'Software Wallet',
	derivationPath: "m/84'/1'/0'/0/0",
	addressSample: 'bc1qexampleadminidxxxxxxxxxxxxxxxxxxxxxx',
	publicKeyHex: '02' + 'ab'.repeat(32),
}

const seeded = connectStateForWallet(wallet)
assert.equal(seeded.phase, 'selected', 'a session wallet must open the wizard in the selected phase')
assert.equal(seeded.account, wallet)
assert.equal(seeded.selectedEntry?.index, 0)
assert.equal(seeded.selectedEntry?.derivationPath, wallet.derivationPath)
assert.equal(seeded.selectedEntry?.address, wallet.addressSample)
assert.equal(seeded.selectedEntry?.publicKeyHex, wallet.publicKeyHex)
console.log('connectStateForWallet: seeds the selected phase OK')

// No session wallet, no signer: the wizard goes back to Connect instead of stranding the user on
// a card for a device that is no longer connected, where Disconnect looks like it does nothing.
assert.deepEqual(connectStateForWallet(null), { phase: 'connect', account: null, selectedEntry: null })
console.log('connectStateForWallet: resets to connect without a wallet OK')

assert.equal(hasAuthorityStep(null), false)
assert.equal(hasAuthorityStep({}), false)
assert.equal(hasAuthorityStep({ authorityStep: null }), false)
assert.equal(hasAuthorityStep({ authorityStep: 'select-authority' }), true)
console.log('hasAuthorityStep: OK')

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
assert.ok(hook.includes('connectStateForWallet'), 'connect hook must derive its state from the session wallet')
assert.ok(hwConnect.includes('existingWallet'), 'HwWalletConnect must forward existingWallet')

// The reset is an effect, so only its dependency can be asserted from here: the hook has to
// re-run when the session wallet changes, or a cleared wallet never reaches the wizard.
assert.ok(hook.includes('}, [existingWallet])'), 'connect hook must reconcile when the session wallet changes')

// Disconnect ends the wizard, so the authority step falls back to selection and the tag Back
// left on the history entry goes with it. Left in place, the next signer connected on this
// screen would land straight on Authenticate for whichever authority happened to be selected,
// never having been asked to pick one.
const headerDisconnect = walletConnect.slice(
	walletConnect.indexOf('async function handleHeaderDisconnect'),
	walletConnect.indexOf('function handleSelectWalletMethod'),
)
assert.ok(headerDisconnect.length > 0, 'header disconnect handler must exist')
assert.ok(headerDisconnect.includes("setAuthorityStep('select-authority')"), 'disconnect must reset the authority step')
assert.ok(
	headerDisconnect.includes('replace: true') && headerDisconnect.includes('state: null'),
	'disconnect must clear the authorityStep tagged on the history entry',
)
console.log('disconnect resets the authority step: OK')

console.log('All resume-connect-session tests passed.')
