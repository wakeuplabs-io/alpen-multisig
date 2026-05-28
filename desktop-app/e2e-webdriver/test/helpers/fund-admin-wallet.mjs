/**
 * Pre-funds the Admin Wallet external address on regtest.
 *
 * From Phase 3.6, the commit transaction is always funded from the Admin Wallet (BDK).
 * The Admin Wallet external address (m/86'/0'/73'/0/0) must hold spendable UTXOs before
 * running the broadcast spec, or commit funding will fail with an insufficient-funds error.
 *
 * Reads BITCOIN_RPC_URL / BITCOIN_RPC_USER / BITCOIN_RPC_PASS from env (default: local regtest).
 * The Admin Wallet address is read from the wallet panel UI via data-testid.
 */

const DEFAULT_RPC_URL = process.env.BITCOIN_RPC_URL || 'http://127.0.0.1:18443'
const RPC_USER = process.env.BITCOIN_RPC_USER || 'rpcuser'
const RPC_PASS = process.env.BITCOIN_RPC_PASS || 'rpcpass'
const FUND_AMOUNT_BTC = 0.01

async function rpcCall(method, params = []) {
	const url = new URL(DEFAULT_RPC_URL)
	const resp = await fetch(url.toString(), {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json',
			Authorization: 'Basic ' + Buffer.from(`${RPC_USER}:${RPC_PASS}`).toString('base64'),
		},
		body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }),
	})
	const body = await resp.json()
	if (body.error) throw new Error(`RPC ${method} error: ${body.error.message}`)
	return body.result
}

/**
 * Reads the Admin Wallet external address from the wallet panel UI.
 * Requires the user to be logged in and on the proposals/wallet screen.
 *
 * @returns {Promise<string>} Admin Wallet external address at index 0
 */
async function readAdminWalletAddressFromUI() {
	const addressEl = await $('[data-testid="e2e-admin-wallet-external-address-0"]')
	await addressEl.waitForDisplayed({ timeout: 30000 })
	return addressEl.getText()
}

/**
 * Funds the Admin Wallet external address from the node coinbase wallet and mines
 * one confirmation block so the UTXO is immediately spendable.
 *
 * Call this before clicking "Confirm & Broadcast" in the broadcast spec.
 *
 * @param {string} [address] - Admin Wallet external address. If omitted, reads from UI.
 */
export async function fundAdminWallet(address) {
	const adminAddr = address ?? (await readAdminWalletAddressFromUI())

	// Send from the coinbase wallet (no wallet-scoped URL needed for sendtoaddress on regtest)
	await rpcCall('sendtoaddress', [adminAddr, FUND_AMOUNT_BTC])

	// Mine 1 block to confirm the UTXO
	const coinbaseAddr = await rpcCall('getnewaddress', [])
	await rpcCall('generatetoaddress', [1, coinbaseAddr])

	return adminAddr
}
