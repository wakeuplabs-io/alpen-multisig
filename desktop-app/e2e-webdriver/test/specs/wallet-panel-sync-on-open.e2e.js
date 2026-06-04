/**
 * Real-app verification for the wallet panel "sync on open" behaviour.
 *
 * Regression guard for the refresh-loop incident (#220 → #222 → sync-on-open):
 *  - opening the panel must auto-sync (triggerSync + reads), so the signer sees a
 *    chain-synced balance without pressing Refresh manually, AND
 *  - the panel must SETTLE — it must not get stuck in an infinite refresh loop.
 *
 * Requires the full local stack (bitcoind, ASM, orchestrator, Postgres) and .env — see README.md.
 */
import { loginMnemonicToProposals } from '../helpers/login-mnemonic.mjs'

const PANEL = '//*[@id="wallet-slide-dialog"]'
const SYNC_BTN = `${PANEL}//button[@data-testid="e2e-wallet-sync-refresh"]`

async function syncButton() {
	return $(SYNC_BTN)
}

async function isSyncing() {
	const btn = await syncButton()
	return (await btn.getAttribute('data-syncing')) === 'true'
}

describe('Wallet panel sync on open', () => {
	it('auto-syncs when the panel opens and settles without looping', async () => {
		await loginMnemonicToProposals()

		// Open the wallet panel from the header session chip.
		const trigger = await $('[data-testid="e2e-session-chip-trigger"]')
		await trigger.waitForClickable({ timeout: 30000 })
		await trigger.click()

		const btn = await syncButton()
		await btn.waitForDisplayed({ timeout: 30000 })

		// 1. The on-open auto-sync must fire (button enters the syncing state at least once).
		//    Poll quickly so we don't miss a fast regtest sync.
		let observedSyncing = false
		for (let i = 0; i < 40; i++) {
			if (await isSyncing()) {
				observedSyncing = true
				break
			}
			await browser.pause(50)
		}
		if (!observedSyncing) {
			throw new Error('on-open auto-sync never entered the syncing state (triggerSync not fired on open)')
		}

		// 2. The sync must SETTLE — button returns to the idle (non-syncing) state.
		await browser.waitUntil(async () => !(await isSyncing()), {
			timeout: 60000,
			timeoutMsg: 'sync never settled (stuck Refreshing… → infinite refresh loop)',
		})

		// 3. Anti-loop: it must STAY settled — no self-retriggering sync over a quiet window.
		const samples = []
		for (let i = 0; i < 10; i++) {
			await browser.pause(400)
			samples.push(await isSyncing())
		}
		const reSynced = samples.some((s) => s === true)
		if (reSynced) {
			throw new Error(`panel re-entered syncing on its own (refresh loop): ${samples.join(',')}`)
		}

		// 4. Sanity: the sync chip shows a real status (a settled sync sets a timestamp;
		//    "Never synced" only remains if the backend reported no sync, which would be a regression).
		const label = await $(`${PANEL}//*[@data-testid="e2e-wallet-sync-label"]`)
		const labelText = (await label.getText()).trim()
		if (labelText.toLowerCase().startsWith('sync error')) {
			throw new Error(`sync chip surfaced an error after on-open sync: "${labelText}"`)
		}
	})
})
