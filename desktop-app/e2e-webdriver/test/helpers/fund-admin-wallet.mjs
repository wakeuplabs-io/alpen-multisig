import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

/**
 * Pre-funds the Admin Wallet external address on regtest.
 *
 * From Phase 3.6, the commit transaction is always funded from the Admin Wallet (BDK).
 * The Admin Wallet external address (m/86'/0'/73'/0/0) must hold spendable UTXOs before
 * "Confirm & Broadcast", or commit funding fails with an insufficient-funds error.
 *
 * Funds via `bitcoin-cli -rpcwallet=asm-runner` (the same wallet-scoped path the rest of the
 * e2e stack uses — see `runtests/mine-blocks.sh` and `mine-regtest-blocks.mjs`), not raw HTTP RPC.
 */

const __dirname = fileURLToPath(new URL('.', import.meta.url))
const FUND_AMOUNT_BTC = '0.01'

function runtestsDir() {
	if (process.env.ALPEN_RUNTESTS_DIR) {
		return process.env.ALPEN_RUNTESTS_DIR
	}
	return path.resolve(__dirname, '../../../../../runtests')
}

function autotestEnvDefaults() {
	if (process.env.ALPEN_AUTOTEST_DIR) {
		return path.join(process.env.ALPEN_AUTOTEST_DIR, 'env.defaults.sh')
	}
	return path.resolve(__dirname, '../../../../../autotest/env.defaults.sh')
}

/**
 * Reads the Admin Wallet external address from the broadcast screen UI.
 * The "Funding Source" card renders the address once prepare-broadcast has run.
 *
 * @returns {Promise<string>} Admin Wallet external address at index 0
 */
async function readAdminWalletAddressFromUI() {
	const addressEl = await $('[data-testid="e2e-admin-wallet-external-address-0"]')
	await addressEl.waitForDisplayed({ timeout: 30000 })
	return (await addressEl.getText()).trim()
}

/**
 * Sends `FUND_AMOUNT_BTC` from the `asm-runner` wallet to `address` and mines one block to confirm.
 * Uses the wallet-scoped `bitcoin-cli -rpcwallet=asm-runner` path so it works regardless of how
 * many wallets bitcoind has loaded.
 *
 * @param {string} address - Admin Wallet external address to fund
 */
function fundAddressViaCli(address) {
	const envDefaults = autotestEnvDefaults()
	const runtests = runtestsDir()
	const result = spawnSync(
		'bash',
		[
			'-lc',
			`set -euo pipefail
[ -f "${envDefaults}" ] && source "${envDefaults}"
source "${runtests}/env.sh"
alpen_wait_bitcoind_rpc
bitcoin-cli $CLI listwallets | grep -q asm-runner || \
  bitcoin-cli $CLI loadwallet asm-runner 2>/dev/null || \
  bitcoin-cli $CLI createwallet asm-runner
bitcoin-cli $CLI -rpcwallet=asm-runner sendtoaddress "${address}" ${FUND_AMOUNT_BTC}
CHANGE=$(bitcoin-cli $CLI -rpcwallet=asm-runner getnewaddress)
bitcoin-cli $CLI generatetoaddress 1 "$CHANGE" >/dev/null
echo "OK funded ${address} with ${FUND_AMOUNT_BTC} BTC"`,
		],
		{ encoding: 'utf8', env: process.env },
	)
	if (result.status !== 0) {
		throw new Error(result.stderr || result.stdout || `fund-admin-wallet failed (${result.status})`)
	}
	return result.stdout.trim()
}

/**
 * Funds the Admin Wallet external address and mines one confirmation block so the UTXO is
 * immediately spendable. Call this before clicking "Confirm & Broadcast" in the broadcast spec.
 *
 * @param {string} [address] - Admin Wallet external address. If omitted, reads it from the UI.
 */
export async function fundAdminWallet(address) {
	const adminAddr = address ?? (await readAdminWalletAddressFromUI())
	fundAddressViaCli(adminAddr)
	return adminAddr
}
