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
 * Uses the regtest-dev-api faucet endpoint (HTTP) — same as local-stack.sh --fund.
 */

const __dirname = fileURLToPath(new URL('.', import.meta.url))
const FAUCET_URL = process.env.REGTEST_DEV_API_URL ?? 'http://127.0.0.1:3001'
const FUND_AMOUNT_BTC = '0.01'

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
 * Sends FUND_AMOUNT_BTC from the regtest-dev-api faucet to address.
 * The faucet also mines 1 block to confirm the transaction.
 *
 * @param {string} address - Admin Wallet external address to fund
 */
function fundAddressViaFaucet(address) {
  const result = spawnSync(
    'curl',
    [
      '-sf',
      '-X', 'POST',
      `${FAUCET_URL}/faucet`,
      '-H', 'Content-Type: application/json',
      '-d', `{"address":"${address}","amount_btc":${FUND_AMOUNT_BTC}}`,
    ],
    { encoding: 'utf8', env: process.env },
  )
  if (result.status !== 0) {
    throw new Error(
      result.stderr || result.stdout || `faucet call failed (${result.status})`,
    )
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
  fundAddressViaFaucet(adminAddr)
  return adminAddr
}
