/**
 * Creates a Defcon 1 proposal as the Security Council.
 *
 * Requires full stack + same mnemonic session as wallet-smoke — see README.md. The demo mnemonic
 * works for the council because the local stack maps that role to the same signer pair as the
 * Strata administrator, so no second mnemonic is needed.
 *
 * What this covers that a unit test cannot: the four canonical signing lines reach the screen
 * through the real IPC renderer — the same one the device signs over — and the type-to-confirm
 * gate really holds the submit control shut in a running app.
 */
import { DEMO_MNEMONIC, loginMnemonicToProposals } from '../helpers/login-mnemonic.mjs'

const SECURITY_COUNCIL = 'Security Council'

/** The message the signer compares against their device. Sequence is filled from the chain. */
const CANONICAL_LINES = ['Strata ASM Administration v1', 'Action: Defcon 1', 'Authorized By: Strata Security Council']

describe('Strata Multisig proposal — Defcon 1', () => {
	it('offers Defcon 1 only to the council, renders the four lines, and gates on type-to-confirm', async function () {
		this.timeout(180000)
		await loginMnemonicToProposals(DEMO_MNEMONIC, SECURITY_COUNCIL)

		// Do not use browser.url('/proposals/create'): Tauri's asset protocol resolves paths as
		// static files (no SPA fallback). In-app navigation matches production WebView behavior.
		const createNav = await $('button[data-testid="e2e-dashboard-create-proposal"]')
		await createNav.waitForClickable({ timeout: 60000 })
		await createNav.click()

		await $('//h1[contains(.,"Create")]').waitForDisplayed({ timeout: 60000 })

		// AC 1 — the council's menu offers Defcon 1, and offers nothing else.
		const defconCard = await $('//button[.//p[contains(text(),"DEFCON 1")]]')
		await defconCard.waitForDisplayed({ timeout: 60000 })
		await defconCard.click()

		const signerUpdateCard = await $('//button[.//p[contains(text(),"Signer update")]]')
		await expect(signerUpdateCard).not.toBeExisting()

		// AC 4 — the four lines, verbatim, with no `Action Details:` block. The sequence number is
		// auto-detected from chain, so the message resolves without typing anything.
		const message = await $('[data-testid="e2e-defcon-1-signing-message"]')
		await message.waitForDisplayed({ timeout: 60000 })
		await browser.waitUntil(async () => (await message.getText()).startsWith(CANONICAL_LINES[0]), {
			timeout: 60000,
			timeoutMsg: 'the signing message should resolve once the sequence number is known',
		})

		const lines = (await message.getText()).split('\n')
		await expect(lines.length).toBe(4)
		await expect(lines.slice(0, 3)).toEqual(CANONICAL_LINES)
		await expect(lines[3]).toMatch(/^Sequence: \d+$/)

		// AC 5 — the gate. Empty and near-miss both hold the control shut; the correct word in
		// lower case opens it, because the match is case-insensitive and nothing else.
		const preview = await $('button[data-testid="e2e-create-proposal-preview"]')
		await expect(preview).toBeDisabled()

		const confirm = await $('input[data-testid="e2e-defcon-1-confirm"]')
		await confirm.waitForDisplayed({ timeout: 30000 })
		await setConfirmValue(confirm, 'defcon1')
		await expect(preview).toBeDisabled()

		await setConfirmValue(confirm, 'defcon 1')
		await preview.waitForClickable({ timeout: 30000 })
		await preview.click()

		await $('//h1[contains(.,"Review")]').waitForDisplayed({ timeout: 60000 })

		const signBtn = await $('button[data-testid="e2e-create-proposal-sign-submit"]')
		await signBtn.waitForClickable({ timeout: 60000 })
		await signBtn.click()

		const success = await $('[data-testid="e2e-proposal-signature-success"]')
		await success.waitForDisplayed({ timeout: 120000 })
	})
})

/**
 * Replace the field's contents. `setValue` on a controlled input can concatenate onto what is
 * already there, which would turn a near-miss into a different near-miss and pass by accident.
 */
async function setConfirmValue(input, value) {
	const selectAllKey = process.platform === 'darwin' ? 'Meta' : 'Control'
	await browser.waitUntil(
		async () => {
			await input.click()
			await browser.keys([selectAllKey, 'a'])
			await browser.keys('Backspace')
			await input.addValue(value)
			return (await input.getValue()) === value
		},
		{ timeout: 15000, interval: 500, timeoutMsg: `confirm field did not hold ${JSON.stringify(value)}` },
	)
}
