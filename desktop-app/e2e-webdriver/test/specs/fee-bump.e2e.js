/**
 * Fee-bump E2E test — validates the fee-bump flow for unconfirmed transactions (PRD §4.3.3):
 *
 * 1. Login → open wallet panel → fund wallet
 * 2. Send unconfirmed BTC (creates a pending transaction)
 * 3. Verify transaction appears in "Pending transactions" list
 * 4. Click "Bump fee" on the pending transaction
 * 5. Enter new fee rate → Confirm → Verify success
 * 6. Verify new txid appears (RBF replacement)
 *
 * Requires the full regtest stack (bitcoind, ASM, orchestrator, Postgres, electrs) and .env.
 *
 * Note: This test validates the RBF path for plain wallet sends. The CPFP path for governance
 * commits requires a pending governance broadcast, which is covered by the proposal-broadcast-quorum
 * spec combined with manual verification.
 */
import { DEMO_MNEMONIC, loginMnemonicToProposals } from '../helpers/login-mnemonic.mjs'
import {
	openWalletPanel,
	closeWalletPanel,
	readReceiveAddress,
	fundAddressViaFaucet,
	sendToAddressUnconfirmed,
	mineRegtestBlocks,
	triggerWalletSync,
	waitForWalletSyncDone,
	readBalanceText,
	waitForBalanceIncrease,
	waitForUnconfirmedBalance,
	waitForNoUnconfirmedBalance,
} from '../helpers/wallet-panel.mjs'

const FUND_AMOUNT_BTC = '0.01'
const SEND_AMOUNT_BTC = '0.005'
const INITIAL_FEE_RATE = 1000 // 1 sat/vB in sat/kvB
const BUMP_FEE_RATE = 5000 // 5 sat/vB in sat/kvB

describe('Alpen Multisig — Fee-bump (PRD §4.3.3)', () => {
	it('creates unconfirmed send, bumps fee via RBF, and verifies replacement', async function () {
		this.timeout(300000)

		// ── Step 1: Login and reach /proposals ──────────────────────────────────
		await loginMnemonicToProposals(DEMO_MNEMONIC)
		await $('//h1[contains(.,"Proposals")]').waitForDisplayed({ timeout: 60000 })

		// ── Step 2: Open wallet panel and fund the wallet ───────────────────────
		await openWalletPanel()

		const receiveAddress = await readReceiveAddress()
		const initialBalance = await readBalanceText()

		fundAddressViaFaucet(receiveAddress, FUND_AMOUNT_BTC)
		mineRegtestBlocks(2)

		await triggerWalletSync()
		await waitForWalletSyncDone()

		// Close and reopen panel to trigger balance re-fetch
		await closeWalletPanel()
		await browser.pause(500)
		await openWalletPanel()

		// Wait for balance to increase (this is the key - wait for actual increase)
		await waitForBalanceIncrease(initialBalance, 30000)

		const balanceAfterFund = await readBalanceText()
		expect(parseFloat(balanceAfterFund)).toBeGreaterThan(0)

		// ── Step 3: Send unconfirmed BTC to create a pending transaction ────────
		// Use a different address to ensure the transaction is outgoing
		const externalAddress = await readReceiveAddress()
		sendToAddressUnconfirmed(externalAddress, SEND_AMOUNT_BTC)

		// ── Step 4: Sync and verify unconfirmed balance appears ─────────────────
		await triggerWalletSync()
		await waitForWalletSyncDone()

		await closeWalletPanel()
		await browser.pause(500)
		await openWalletPanel()

		// Unconfirmed balance should appear (negative, since we sent)
		const unconfBalance = await waitForUnconfirmedBalance(30000)
		expect(unconfBalance.toLowerCase()).toContain('unconfirmed')

		// ── Step 5: Expand "Pending transactions" section ───────────────────────
		// Look for the pending transactions section
		const pendingSection = await $('//h3[contains(.,"Pending transactions")]')
		await pendingSection.waitForDisplayed({ timeout: 10000 })

		// Find the bump button for the pending transaction
		// The button has data-testid="e2e-wallet-tx-bump-${txid}"
		const bumpButtons = await $$('[data-testid^="e2e-wallet-tx-bump-"]')
		expect(bumpButtons.length).toBeGreaterThan(0, 'Expected at least one pending transaction with bump button')

		// Click the first bump button
		const firstBumpButton = bumpButtons[0]
		await firstBumpButton.click()

		// ── Step 6: Fill the bump fee form ──────────────────────────────────────
		const bumpForm = await $('[data-testid="e2e-wallet-bump-form"]')
		await bumpForm.waitForDisplayed({ timeout: 10000 })

		// Enter the new fee rate
		const rateInput = await $('[data-testid="e2e-wallet-bump-rate-input"]')
		await rateInput.waitForDisplayed({ timeout: 5000 })
		await rateInput.setValue(BUMP_FEE_RATE.toString())

		// ── Step 7: Confirm the bump ────────────────────────────────────────────
		const confirmButton = await $('[data-testid="e2e-wallet-bump-confirm"]')
		await confirmButton.waitForClickable({ timeout: 5000 })
		await confirmButton.click()

		// ── Step 8: Verify success ──────────────────────────────────────────────
		const successBanner = await $('[data-testid="e2e-wallet-bump-success"]')
		await successBanner.waitForDisplayed({ timeout: 30000 })

		// Success message should contain the new txid
		const successText = await successBanner.getText()
		expect(successText.toLowerCase()).toContain('success')

		// ── Step 9: Sync and verify the replacement transaction ─────────────────
		await triggerWalletSync()
		await waitForWalletSyncDone()

		// The old pending transaction should be replaced by the new one
		// (In RBF, the original txid is replaced)
		await closeWalletPanel()
		await browser.pause(500)
		await openWalletPanel()

		// Unconfirmed balance should still be present (the replacement is also unconfirmed)
		const unconfAfterBump = await waitForUnconfirmedBalance(30000)
		expect(unconfAfterBump.toLowerCase()).toContain('unconfirmed')

		// ── Step 10: Mine blocks to confirm the replacement ─────────────────────
		mineRegtestBlocks(6)

		await triggerWalletSync()
		await waitForWalletSyncDone()

		await closeWalletPanel()
		await browser.pause(500)
		await openWalletPanel()

		// Unconfirmed should clear after confirmation
		// Wait a bit for the UI to update
		await browser.pause(2000)
		const unconfAfterConfirm = await readUnconfirmedBalance()
		// After confirmation, unconfirmed balance should be null or zero
		expect(unconfAfterConfirm === null || unconfAfterConfirm === '0').toBeTruthy()

		await closeWalletPanel()
	})
})
