/**
 * Admin Wallet panel e2e test — validates the full wallet panel lifecycle:
 *
 * 1. Login → open wallet panel → read rotating receive address
 * 2. Fund the receive address → mine → validate confirmed balance + address list
 * 3. Fund again (new receive address) → sync → validate balance increase
 * 4. Send unconfirmed BTC → validate unconfirmed balance appears
 * 5. Mine → confirm → validate balance resolution
 * 6. Fund once more → additional validations on the admin wallet
 *
 * Requires the full regtest stack (bitcoind, ASM, orchestrator, Postgres) and .env.
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
	readUnconfirmedBalance,
	waitForUnconfirmedBalance,
	expandAddressesList,
	readAddressRows,
	waitForAddressCount,
	waitForBalanceIncrease,
	waitForNoUnconfirmedBalance,
} from '../helpers/wallet-panel.mjs'

const FUND_AMOUNT_BTC = '0.01'
const BTC_ADDRESS_RE = /^(tb1|bcrt1|[123mn])[a-zA-HJ-NP-Z0-9]{25,62}$/

describe('Strata Multisig — Admin Wallet panel', () => {
	it('funds, syncs, and validates confirmed and unconfirmed balances', async function () {
		this.timeout(300000)

		// ── Step 1: Login and reach /proposals ──────────────────────────────────
		await loginMnemonicToProposals(DEMO_MNEMONIC)
		await $('//h1[contains(.,"Proposals")]').waitForDisplayed({ timeout: 60000 })

		// ── Step 2: Open wallet panel and read the first receive address ────────
		await openWalletPanel()

		const firstAddress = await readReceiveAddress()
		expect(firstAddress).toMatch(BTC_ADDRESS_RE)

		// Read initial balance before funding
		const initialBalance = await readBalanceText()

		// ── Step 3: Fund the first receive address (confirmed via faucet) ───────
		fundAddressViaFaucet(firstAddress, FUND_AMOUNT_BTC)
		mineRegtestBlocks(2)

		// ── Step 4: Sync wallet and validate confirmed balance increased ────────
		await triggerWalletSync()
		await waitForWalletSyncDone()

		// Close and reopen panel to trigger balance re-fetch
		await closeWalletPanel()
		await browser.pause(500)
		await openWalletPanel()

		await waitForBalanceIncrease(initialBalance, 30000)

		// Verify no unconfirmed balance (faucet confirmed immediately)
		const unconfAfterFirst = await readUnconfirmedBalance()
		expect(unconfAfterFirst).toBeNull()

		// ── Step 5: Expand addresses list and validate ──────────────────────────
		await expandAddressesList()
		await browser.waitUntil(
			async () => {
				const rows = await readAddressRows()
				return rows.length >= 1
			},
			{ timeout: 20000, timeoutMsg: 'Expected at least 1 address row' },
		)

		const rowsAfterFirst = await readAddressRows()
		expect(rowsAfterFirst.length).toBeGreaterThanOrEqual(1)
		const hasFirstAddress = rowsAfterFirst.some((r) => r.address === firstAddress)
		expect(hasFirstAddress).toBeTruthy()

		const balanceAfterFirst = await readBalanceText()

		// ── Step 6: Close and reopen panel to get a new receive address ─────────
		await closeWalletPanel()
		await browser.pause(500)
		await openWalletPanel()

		const secondAddress = await readReceiveAddress()
		expect(secondAddress).not.toEqual(firstAddress)

		// ── Step 7: Fund the second receive address (confirmed via faucet) ──────
		fundAddressViaFaucet(secondAddress, FUND_AMOUNT_BTC)

		// ── Step 8: Sync and validate balance increase ──────────────────────────
		await triggerWalletSync()
		await waitForWalletSyncDone()

		await closeWalletPanel()
		await browser.pause(500)
		await openWalletPanel()

		await waitForBalanceIncrease(balanceAfterFirst, 30000)

		const unconfAfterSecond = await readUnconfirmedBalance()
		expect(unconfAfterSecond).toBeNull()

		await expandAddressesList()
		await browser.waitUntil(
			async () => {
				const rows = await readAddressRows()
				return rows.length >= 2
			},
			{ timeout: 20000, timeoutMsg: 'Expected at least 2 address rows' },
		)

		const rowsAfterSecond = await readAddressRows()
		expect(rowsAfterSecond.length).toBeGreaterThanOrEqual(2)

		const balanceAfterSecond = await readBalanceText()

		// ── Step 9: Send UNCONFIRMED BTC to third address ───────────────────────
		const thirdAddress = await readReceiveAddress()
		expect(thirdAddress).not.toEqual(secondAddress)

		// Send via bitcoin-cli WITHOUT mining — this creates an unconfirmed tx
		sendToAddressUnconfirmed(thirdAddress, FUND_AMOUNT_BTC)

		// ── Step 10: Sync and validate unconfirmed balance appears ──────────────
		await triggerWalletSync()
		await waitForWalletSyncDone()

		await closeWalletPanel()
		await browser.pause(500)
		await openWalletPanel()

		// Confirmed balance should NOT have increased yet (transaction is unconfirmed)
		const balanceBeforeMine = await readBalanceText()
		const confirmedBefore = parseFloat(balanceBeforeMine)
		const confirmedAfterSecond = parseFloat(balanceAfterSecond)
		expect(confirmedBefore).toEqual(confirmedAfterSecond)

		// Unconfirmed balance SHOULD appear
		const unconfBeforeMine = await waitForUnconfirmedBalance(30000)
		expect(unconfBeforeMine.toLowerCase()).toContain('unconfirmed')

		// ── Step 11: Mine blocks to confirm the unconfirmed transaction ─────────
		mineRegtestBlocks(6)

		// ── Step 12: Sync and validate balance resolution ───────────────────────
		await triggerWalletSync()
		await waitForWalletSyncDone()

		await closeWalletPanel()
		await browser.pause(500)
		await openWalletPanel()

		// Confirmed balance should now have increased
		await waitForBalanceIncrease(balanceAfterSecond, 30000)

		// Unconfirmed should clear
		await waitForNoUnconfirmedBalance(60000)

		// ── Step 13: Fund the (rotated) receive address a fourth time ───────────
		const fourthAddress = await readReceiveAddress()
		expect(fourthAddress).not.toEqual(thirdAddress)

		// Read balance BEFORE funding
		const balanceBeforeFourth = await readBalanceText()

		fundAddressViaFaucet(fourthAddress, FUND_AMOUNT_BTC)

		// ── Step 14: Sync and validate final balance ────────────────────────────
		await triggerWalletSync()
		await waitForWalletSyncDone()

		await closeWalletPanel()
		await browser.pause(500)
		await openWalletPanel()

		await waitForBalanceIncrease(balanceBeforeFourth, 30000)

		const unconfAfterFourth = await readUnconfirmedBalance()
		expect(unconfAfterFourth).toBeNull()

		// All addresses should show in the list
		await expandAddressesList()
		await browser.waitUntil(
			async () => {
				const rows = await readAddressRows()
				return rows.length >= 4
			},
			{ timeout: 20000, timeoutMsg: 'Expected at least 4 address rows' },
		)

		const rowsAfterFourth = await readAddressRows()
		expect(rowsAfterFourth.length).toBeGreaterThanOrEqual(4)

		// ── Step 15: Final validation — balance text is a valid BTC amount ──────
		const finalBalance = await readBalanceText()
		expect(finalBalance).toMatch(/^\d+\.\d+$/)

		// Close the panel
		await closeWalletPanel()
	})
})
