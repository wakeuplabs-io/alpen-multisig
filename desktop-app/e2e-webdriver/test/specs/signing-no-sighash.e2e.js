/**
 * #402 — the signing UI must never show the SPS-65 sighash.
 *
 * No hardware device renders it, so a signer comparing it against their device screen is
 * comparing against nothing. This spec walks a real create+sign flow and asserts the value is
 * absent from the review step, which is the screen the client reported.
 *
 * The device-accurate hint itself (`e2e-device-signing-hint`) only renders for hardware
 * sessions, so this spec asserts its absence for the mnemonic signer — the other half needs a
 * Ledger and is covered by manual QA.
 *
 * Requires full stack + same mnemonic session as wallet-smoke — see README.md.
 */
import { DEMO_MNEMONIC, loginMnemonicToProposals } from '../helpers/login-mnemonic.mjs'

// Deterministic, on-curve, and not in the staging signer set. Chosen over a fixed fixture that
// later became a real member — the reason `proposal-add-signer` no longer reaches the preview.
const NEW_SIGNER = '02bc0269811cd8173a66d4573c6ceb89b10aae14b6160ebb924766eda723486071'

describe('Strata Multisig signing — no SPS-65 sighash on screen', () => {
	it('reaches the review step without showing the sighash', async function () {
		this.timeout(180000)
		await loginMnemonicToProposals(DEMO_MNEMONIC)

		const createNav = await $('button[data-testid="e2e-dashboard-create-proposal"]')
		await createNav.waitForClickable({ timeout: 60000 })
		await createNav.click()

		await $('//h1[contains(.,"Create")]').waitForDisplayed({ timeout: 60000 })
		await $('//button[.//p[contains(text(),"Signer update")]]').click()

		const title = await $('input[data-testid="e2e-create-proposal-title"]')
		await title.waitForDisplayed({ timeout: 30000 })
		// The title must not contain the word this spec searches for, or the assertion finds its
		// own fixture and fails on a screen that is perfectly clean.
		await title.setValue(`E2E device value ${Date.now()}`)

		const pubkeyIn = await $('input[data-testid="e2e-new-signer-pubkey-input"]')
		await pubkeyIn.waitForDisplayed({ timeout: 60000 })
		await pubkeyIn.setValue(NEW_SIGNER)
		await $('button[data-testid="e2e-new-signer-add-button"]').click()

		await browser.waitUntil(
			async () => (await $('span[data-testid="e2e-added-signer-value"]').getText()) === NEW_SIGNER,
			{ timeout: 15000, timeoutMsg: 'added signer pubkey should appear in list' },
		)

		const previewBtn = await $('button[data-testid="e2e-create-proposal-preview"]')
		await previewBtn.waitForClickable({ timeout: 60000 })
		await previewBtn.click()

		await $('//h1[contains(.,"Review")]').waitForDisplayed({ timeout: 60000 })

		const body = await $('body').getText()
		const offending = body.split('\n').filter((line) => /sighash/i.test(line))
		if (offending.length > 0) {
			throw new Error(`the review step still mentions the sighash: ${JSON.stringify(offending)}`)
		}

		// A mnemonic signer has no device screen, so there is nothing to compare and the hint
		// must stay absent rather than render an empty prompt.
		const hint = await $('[data-testid="e2e-device-signing-hint"]')
		if (await hint.isExisting()) {
			throw new Error('device signing hint rendered for a software signer')
		}
	})
})
