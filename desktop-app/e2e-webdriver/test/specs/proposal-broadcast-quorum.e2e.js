/**
 * Broadcast a multisig update proposal after quorum (commit/reveal via desktop env).
 * Run only when the "Quorum reached" list shows **Broadcast** (e.g. after
 * `test:e2e:proposal-add-signer` then `test:e2e:proposal-co-sign-row1`).
 * Uses address row #0 session (same as wallet smoke).
 */
import { DEMO_MNEMONIC, loginMnemonicToProposals } from '../helpers/login-mnemonic.mjs'

describe('Alpen Multisig proposal — broadcast after quorum', () => {
	it('prepares artifacts and confirms onchain broadcast', async function () {
		this.timeout(300000)

		await loginMnemonicToProposals(DEMO_MNEMONIC)

		await $('//h1[contains(.,"Proposals")]').waitForDisplayed({ timeout: 60000 })

		const broadcastBtn = await $('button[data-testid="e2e-proposal-broadcast-button"]')
		await broadcastBtn.waitForDisplayed({
			timeout: 120000,
			timeoutMsg:
				'No Broadcast in Quorum reached — run add-signer then co-sign-row1 first, or pick the first quorum card manually.',
		})
		await broadcastBtn.waitForClickable({ timeout: 30000 })
		await broadcastBtn.click()

		await $('//h1[contains(.,"Broadcast proposal")]').waitForDisplayed({ timeout: 60000 })

		const prepareBtn = await $('button[data-testid="e2e-broadcast-prepare"]')
		await prepareBtn.waitForClickable({ timeout: 60000 })
		await prepareBtn.click()

		const confirmBtn = await $('button[data-testid="e2e-broadcast-confirm"]')
		await confirmBtn.waitForDisplayed({ timeout: 180000 })
		await confirmBtn.waitForClickable({ timeout: 60000 })
		await confirmBtn.click()

		const done = await $('[data-testid="e2e-broadcast-done-banner"]')
		await done.waitForDisplayed({ timeout: 240000 })
	})
})
