import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

/**
 * Helpers for interacting with the Admin Wallet panel in e2e tests.
 *
 * The wallet panel slides in from the right when the session chip is clicked.
 * It shows: balance, receive address, addresses with balance, and sync controls.
 */

const __dirname = fileURLToPath(new URL('.', import.meta.url))
const FAUCET_URL = process.env.REGTEST_DEV_API_URL ?? 'http://127.0.0.1:3001'
const MINE_URL = FAUCET_URL

/**
 * Opens the wallet panel by clicking the session chip trigger.
 * Waits for the wallet slide dialog to be visible.
 */
export async function openWalletPanel() {
	const chip = await $('button[data-testid="e2e-session-chip-trigger"]')
	await chip.waitForDisplayed({ timeout: 30000 })
	await chip.waitForClickable({ timeout: 10000 })
	await chip.click()

	// Wait for the slide panel to appear
	const panel = await $('#wallet-slide-dialog')
	await panel.waitForDisplayed({ timeout: 5000 })
}

/**
 * Closes the wallet panel by pressing Escape.
 */
export async function closeWalletPanel() {
	await browser.keys(['Escape'])
	// Small delay for the close animation
	await browser.pause(350)
}

/**
 * Reads the receive address from the wallet panel.
 * Uses the `title` attribute which holds the full address (the visible span is truncated).
 *
 * @returns {Promise<string>} The current receive address
 */
export async function readReceiveAddress() {
	const addrEl = await $('[data-testid="e2e-wallet-receive-address-value"]')
	await addrEl.waitForDisplayed({ timeout: 15000 })
	const title = await addrEl.getAttribute('title')
	if (title && title.trim()) {
		return title.trim()
	}
	// Fallback: try getText
	return (await addrEl.getText()).trim()
}

/**
 * Sends BTC from the regtest-dev-api faucet to the given address.
 * The faucet also mines 1 block to confirm the transaction.
 *
 * @param {string} address - Address to fund
 * @param {string} amountBtc - Amount in BTC (default: 0.01)
 */
export function fundAddressViaFaucet(address, amountBtc = '0.01') {
	const result = spawnSync(
		'curl',
		[
			'-sf',
			'-X',
			'POST',
			`${FAUCET_URL}/faucet`,
			'-H',
			'Content-Type: application/json',
			'-d',
			`{"address":"${address}","amount_btc":${amountBtc}}`,
		],
		{ encoding: 'utf8', env: process.env },
	)
	if (result.status !== 0) {
		throw new Error(result.stderr || result.stdout || `faucet call failed (${result.status})`)
	}
	return result.stdout.trim()
}

/**
 * Sends BTC directly via bitcoin-cli to the given address WITHOUT mining.
 * This creates an unconfirmed transaction so the wallet shows unconfirmed balance.
 *
 * @param {string} address - Address to fund
 * @param {string} amountBtc - Amount in BTC (default: 0.01)
 */
export function sendToAddressUnconfirmed(address, amountBtc = '0.01') {
	const result = spawnSync(
		'docker',
		[
			'exec',
			'staging-bitcoin-1',
			'bitcoin-cli',
			'-regtest',
			'-rpcuser=user',
			'-rpcpassword=password',
			'sendtoaddress',
			address,
			amountBtc,
		],
		{ encoding: 'utf8', env: process.env },
	)
	if (result.status !== 0) {
		throw new Error(result.stderr || result.stdout || `sendtoaddress failed (${result.status})`)
	}
	return result.stdout.trim()
}

/**
 * Mines regtest blocks via the regtest-dev-api HTTP endpoint.
 *
 * @param {number} count - Number of blocks to mine (default: 1)
 */
export function mineRegtestBlocks(count = 1) {
	const result = spawnSync('curl', ['-sf', '-X', 'POST', `${MINE_URL}/mine?count=${count}`], {
		encoding: 'utf8',
		env: process.env,
	})
	if (result.status !== 0) {
		throw new Error(result.stderr || result.stdout || `mine call failed (${result.status})`)
	}
	return result.stdout.trim()
}

/**
 * Triggers a wallet sync by clicking the refresh button in the sync chip.
 */
export async function triggerWalletSync() {
	const syncBtn = await $('[data-testid="e2e-wallet-sync-refresh"]')
	await syncBtn.waitForDisplayed({ timeout: 10000 })
	await syncBtn.waitForClickable({ timeout: 5000 })
	await syncBtn.click()
}

/**
 * Waits for the wallet sync to complete (sync label no longer shows "Syncing").
 *
 * @param {number} timeoutMs - Max wait time in ms (default: 30000)
 */
export async function waitForWalletSyncDone(timeoutMs = 30000) {
	const deadline = Date.now() + timeoutMs
	while (Date.now() < deadline) {
		const syncLabel = await $('[data-testid="e2e-wallet-sync-label"]')
		if (await syncLabel.isDisplayed()) {
			const text = await syncLabel.getText()
			if (!text.toLowerCase().includes('syncing')) {
				return
			}
		}
		await browser.pause(1000)
	}
	throw new Error('Wallet sync did not complete within timeout')
}

/**
 * Reads the confirmed balance text from the wallet panel.
 *
 * @returns {Promise<string>} The balance text (e.g. "0.01000000 BTC")
 */
export async function readBalanceText() {
	const balanceEl = await $('[data-testid="e2e-wallet-balance-primary"]')
	await balanceEl.waitForDisplayed({ timeout: 10000 })
	return (await balanceEl.getText()).trim()
}

/**
 * Reads the unconfirmed balance line from the wallet panel.
 * Returns null if no unconfirmed balance is shown.
 *
 * @returns {Promise<string|null>} The unconfirmed line text or null
 */
export async function readUnconfirmedBalance() {
	try {
		const unconfEl = await $('[data-testid="e2e-wallet-balance-unconfirmed"]')
		if (await unconfEl.isDisplayed()) {
			return (await unconfEl.getText()).trim()
		}
	} catch {
		// Element not found — no unconfirmed balance
	}
	return null
}

/**
 * Expands the "Addresses with balance" section in the wallet panel.
 * Idempotent: does nothing if already expanded.
 */
export async function expandAddressesList() {
	const toggle = await $('[data-testid="e2e-wallet-addresses-toggle"]')
	await toggle.waitForDisplayed({ timeout: 10000 })

	// Check if already expanded by looking for the table
	try {
		const table = await $('[data-testid="e2e-wallet-addresses-list"]')
		if (await table.isDisplayed()) {
			return // Already expanded
		}
	} catch {
		// Table not found — needs expansion
	}

	await toggle.waitForClickable({ timeout: 5000 })
	await toggle.click()
	// Wait for the table to appear
	const table = await $('[data-testid="e2e-wallet-addresses-list"]')
	await table.waitForDisplayed({ timeout: 5000 })
}

/**
 * Reads all address rows from the expanded addresses list.
 *
 * @returns {Promise<Array<{address: string, balance: string}>>}
 */
export async function readAddressRows() {
	const rows = []
	// Wait for the list to render
	await browser.pause(1000)

	// Find all address row elements by their testid pattern
	const table = await $('[data-testid="e2e-wallet-addresses-list"]')
	if (!(await table.isDisplayed())) {
		return rows
	}

	// Get all tr elements with e2e-wallet-address-row- prefix
	const rowElements = await $$('tr[data-testid^="e2e-wallet-address-row-"]')
	for (const row of rowElements) {
		const testid = await row.getAttribute('data-testid')
		const address = testid.replace('e2e-wallet-address-row-', '')
		const balanceCell = await row.$('td:last-child div:first-child')
		const balance = await balanceCell.getText()
		rows.push({ address, balance })
	}

	return rows
}

/**
 * Waits for the addresses list to show a specific count of rows.
 *
 * @param {number} expectedCount - Expected number of address rows
 * @param {number} timeoutMs - Max wait time in ms (default: 30000)
 */
export async function waitForAddressCount(expectedCount, timeoutMs = 30000) {
	const deadline = Date.now() + timeoutMs
	while (Date.now() < deadline) {
		try {
			const table = await $('[data-testid="e2e-wallet-addresses-list"]')
			if (await table.isDisplayed()) {
				const rowElements = await $$('tr[data-testid^="e2e-wallet-address-row-"]')
				if (rowElements.length === expectedCount) {
					return
				}
			}
		} catch {
			// Table not yet visible
		}
		await browser.pause(1000)
	}
	throw new Error(`Expected ${expectedCount} address rows within ${timeoutMs}ms`)
}

/**
 * Waits for the balance to show a specific value (or contain a substring).
 *
 * @param {string} expectedText - Expected balance text or substring
 * @param {number} timeoutMs - Max wait time in ms (default: 30000)
 */
export async function waitForBalance(expectedText, timeoutMs = 30000) {
	const deadline = Date.now() + timeoutMs
	while (Date.now() < deadline) {
		try {
			const balance = await readBalanceText()
			if (balance.includes(expectedText)) {
				return balance
			}
		} catch {
			// Balance element not ready yet
		}
		await browser.pause(1000)
	}
	// Final attempt to get actual balance for error message
	let actual = '(unknown)'
	try {
		actual = await readBalanceText()
	} catch {
		actual = '(element not found)'
	}
	throw new Error(`Balance did not show "${expectedText}" within ${timeoutMs}ms (actual: "${actual}")`)
}

/**
 * Waits for the balance to increase from a previous value.
 * Parses the BTC amount and compares numerically.
 *
 * @param {string} previousBalance - The previous balance text (e.g. "0.05998991")
 * @param {number} timeoutMs - Max wait time in ms (default: 30000)
 */
export async function waitForBalanceIncrease(previousBalance, timeoutMs = 30000) {
	const prevAmount = parseFloat(previousBalance)
	const deadline = Date.now() + timeoutMs
	while (Date.now() < deadline) {
		try {
			const balance = await readBalanceText()
			const amount = parseFloat(balance)
			if (!isNaN(amount) && amount > prevAmount) {
				return balance
			}
		} catch {
			// Balance element not ready yet
		}
		await browser.pause(1000)
	}
	// Final attempt to get actual balance for error message
	let actual = '(unknown)'
	try {
		actual = await readBalanceText()
	} catch {
		actual = '(element not found)'
	}
	throw new Error(`Balance did not increase from "${previousBalance}" within ${timeoutMs}ms (actual: "${actual}")`)
}

/**
 * Waits for unconfirmed balance to appear.
 *
 * @param {number} timeoutMs - Max wait time in ms (default: 30000)
 * @returns {Promise<string>} The unconfirmed balance text
 */
export async function waitForUnconfirmedBalance(timeoutMs = 30000) {
	const deadline = Date.now() + timeoutMs
	while (Date.now() < deadline) {
		const unconf = await readUnconfirmedBalance()
		if (unconf !== null) {
			return unconf
		}
		await browser.pause(1000)
	}
	throw new Error(`Unconfirmed balance did not appear within ${timeoutMs}ms`)
}

/**
 * Waits for unconfirmed balance to disappear (all confirmed).
 *
 * @param {number} timeoutMs - Max wait time in ms (default: 60000)
 */
export async function waitForNoUnconfirmedBalance(timeoutMs = 60000) {
	const deadline = Date.now() + timeoutMs
	while (Date.now() < deadline) {
		const unconf = await readUnconfirmedBalance()
		if (unconf === null) {
			return
		}
		await browser.pause(1000)
	}
	throw new Error(`Unconfirmed balance did not clear within ${timeoutMs}ms`)
}
