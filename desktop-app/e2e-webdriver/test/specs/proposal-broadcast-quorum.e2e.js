/**
 * Broadcast a multisig update proposal after quorum (commit/reveal via orchestrator).
 * Run only when the "Quorum reached" list shows **Broadcast**.
 * Uses the canonical Admin ID session (same as wallet smoke).
 *
 * Mnemonic walking skeleton gate (slice a): Palabras login → unified flow →
 * phase progress commit→reveal→confirmed → no device affordance.
 */
import { DEMO_MNEMONIC, loginMnemonicToProposals } from '../helpers/login-mnemonic.mjs'
import { mineWhileWaitingForBroadcastDone } from '../helpers/mine-regtest-blocks.mjs'
import { fundAdminWallet } from '../helpers/fund-admin-wallet.mjs'

describe('Alpen Multisig proposal — broadcast after quorum', () => {
	it('prepares artifacts and confirms onchain broadcast', async function () {
		this.timeout(300000)

		await loginMnemonicToProposals(DEMO_MNEMONIC)

		await $('//h1[contains(.,"Proposals")]').waitForDisplayed({ timeout: 60000 })

		const broadcastBtn = await $('button[data-testid="e2e-proposal-broadcast-button"]')
		await broadcastBtn.waitForDisplayed({
			timeout: 120000,
			timeoutMsg:
				'No Broadcast in Quorum reached — run add-signer then co-sign-mnemonic first, or pick the first quorum card manually.',
		})
		await broadcastBtn.waitForClickable({ timeout: 30000 })
		await broadcastBtn.click()

		await $('//h1[contains(.,"Broadcast proposal")]').waitForDisplayed({ timeout: 60000 })

		// Screen auto-runs prepare on mount; wait for the confirm step (prepare only shows as Retry on error).
		const confirmBtn = await $('button[data-testid="e2e-broadcast-confirm"]')
		await confirmBtn.waitForClickable({
			timeout: 180000,
			timeoutMsg: 'Prepare broadcast should finish and enable Confirm & Broadcast',
		})

		// Fund Admin Wallet before broadcasting — required from Phase 3.6 (sole commit funder).
		await fundAdminWallet()

		await confirmBtn.click()

		// --- Mnemonic walking skeleton gate assertions ---

		// Gate 1: Phase progress renders Commit → Reveal → Enactment steps. After submit the
		// command returns within seconds, so the heading may already read "Submitted — awaiting
		// confirmation…" (decoupled submit/confirm) instead of "Broadcasting…".
		const phaseProgress = await $('//h3[contains(.,"Broadcasting") or contains(.,"awaiting confirmation")]')
		await phaseProgress.waitForDisplayed({
			timeout: 30000,
			timeoutMsg: 'Phase progress should show the broadcasting/awaiting-confirmation heading after Confirm & Broadcast',
		})

		const commitStep = await $('//p[contains(text(),"Commit")]')
		await commitStep.waitForDisplayed({
			timeout: 10000,
			timeoutMsg: 'Phase progress should render Commit step',
		})

		const revealStep = await $('//p[contains(text(),"Reveal")]')
		await revealStep.waitForDisplayed({
			timeout: 10000,
			timeoutMsg: 'Phase progress should render Reveal step',
		})

		// Step 3 reads "Broadcasted" while broadcasting and "Awaiting block" once submitted.
		const broadcastedStep = await $('//p[contains(text(),"Broadcasted") or contains(text(),"Awaiting block")]')
		await broadcastedStep.waitForDisplayed({
			timeout: 10000,
			timeoutMsg: 'Phase progress should render the Broadcasted / Awaiting block step',
		})

		// Gate 2: No device prompt affordance appears (mnemonic signer returns instantly)
		const bodyText = await $('body').getText()
		const devicePromptPatterns = [/confirm on device/i, /connect your.*hardware/i]
		for (const pattern of devicePromptPatterns) {
			if (pattern.test(bodyText)) {
				throw new Error(`Device prompt found: "${pattern.source}" should not appear in mnemonic flow`)
			}
		}

		// Orchestrator: commit → wait conf → reveal → wait conf. Regtest must mine during that wait.
		await mineWhileWaitingForBroadcastDone('[data-testid="e2e-broadcast-done-banner"]')

		// Gate 3: Phase progress shows completion state
		const doneHeading = await $('//h3[contains(.,"Reveal confirmed") or contains(.,"Proposal enacted")]')
		await doneHeading.waitForDisplayed({
			timeout: 10000,
			timeoutMsg: 'Phase progress should show completion heading after broadcast done',
		})
	})
})
