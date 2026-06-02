/**
 * Second multisig party: cosign mnemonic at canonical Admin ID, signs the first pending proposal.
 * Run only after a pending signer-update proposal exists (e.g. `npm run test:e2e:proposal-add-signer`).
 */
import { DEMO_MNEMONIC_COSIGN, loginMnemonicToProposals } from '../helpers/login-mnemonic.mjs'

describe('Alpen Multisig proposal — co-sign with second mnemonic', () => {
	it('opens first pending Sign flow and submits second signature', async function () {
		this.timeout(180000)

		await loginMnemonicToProposals(DEMO_MNEMONIC_COSIGN)

		await $('//h1[contains(.,"Proposals")]').waitForDisplayed({ timeout: 60000 })

		const signFromList = await $('button[data-testid="e2e-proposal-sign-button"]')
		await signFromList.waitForDisplayed({
			timeout: 120000,
			timeoutMsg:
				'No pending proposal with Sign — create one first: npm run test:e2e:proposal-add-signer',
		})
		await signFromList.waitForClickable({ timeout: 30000 })
		await signFromList.click()

		await $('//h1[contains(.,"Sign proposal")]').waitForDisplayed({ timeout: 60000 })

		const signBtn = await $('button[data-testid="e2e-sign-proposal-submit"]')
		await signBtn.waitForClickable({ timeout: 60000 })
		await signBtn.click()

		await browser.waitUntil(
			async () => {
				const u = await browser.getUrl()
				return u.includes('/proposals') && !u.includes('/sign')
			},
			{
				timeout: 120000,
				timeoutMsg: 'expected return to /proposals after co-sign (no /sign in URL)',
			},
		)
	})
})
