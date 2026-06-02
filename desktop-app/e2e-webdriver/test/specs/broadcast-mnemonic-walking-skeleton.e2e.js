/**
 * E2E walking skeleton — mnemonic-driven broadcast on real Tauri binary.
 *
 * Full flow: Palabras login → approve proposal → broadcast → commit→reveal→confirmed.
 * No device prompt appears (mnemonic path, not hardware wallet).
 *
 * Run: npx wdio run wdio.conf.js --spec ./test/specs/broadcast-mnemonic-walking-skeleton.e2e.js
 */
import { DEMO_MNEMONIC, loginMnemonicToProposals } from '../helpers/login-mnemonic.mjs'
import { mineWhileWaitingForBroadcastDone } from '../helpers/mine-regtest-blocks.mjs'
import { fundAdminWallet } from '../helpers/fund-admin-wallet.mjs'

describe('Alpen Multisig — mnemonic walking skeleton broadcast', () => {
	it('regtest e2e mnemonic walking skeleton', async function () {
		this.timeout(300000)

		// 1. Palabras login
		await loginMnemonicToProposals(DEMO_MNEMONIC)

		await $('//h1[contains(.,"Proposals")]').waitForDisplayed({ timeout: 60000 })

		// 2. Approve proposal — click first Broadcast in Quorum reached
		const broadcastBtn = await $('button[data-testid="e2e-proposal-broadcast-button"]')
		await broadcastBtn.waitForDisplayed({
			timeout: 120000,
			timeoutMsg: 'No Broadcast in Quorum reached — run add-signer then co-sign-row1 first.',
		})
		await broadcastBtn.waitForClickable({ timeout: 30000 })
		await broadcastBtn.click()

		await $('//h1[contains(.,"Broadcast proposal")]').waitForDisplayed({ timeout: 60000 })

		// 4. Prepare broadcast (auto-runs on mount); wait for confirm step
		// The mnemonic signer returns instantly; if a device prompt were shown, prepare would hang.
		// Presence of the confirm button proves prepare finished without device interaction.
		const confirmBtn = await $('button[data-testid="e2e-broadcast-confirm"]')
		await confirmBtn.waitForClickable({
			timeout: 180000,
			timeoutMsg: 'Prepare broadcast should finish and enable Confirm & Broadcast',
		})

		// 5. Fund Admin Wallet before broadcasting
		await fundAdminWallet()

		// 6. Confirm & Broadcast
		await confirmBtn.click()

		// 7. Wait for commit→reveal→confirmed (regtest mining drives confirmations)
		await mineWhileWaitingForBroadcastDone('[data-testid="e2e-broadcast-done-banner"]')
	})
})
