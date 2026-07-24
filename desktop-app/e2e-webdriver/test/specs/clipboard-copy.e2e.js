/**
 * Clipboard copy e2e test (feedback 2026-07-01 #428).
 *
 * Alpen reported that a "Copy" button next to a wallet address did not put the
 * address on the clipboard. Unit tests cannot catch that: the failure is in the
 * app → OS clipboard hand-off. This spec clicks a real copy affordance in the
 * running Tauri binary and reads the *system* clipboard back.
 *
 * Requires the full regtest stack and a graphical session (same as the other specs).
 */
import { execFileSync, spawn } from 'node:child_process'
import { DEMO_MNEMONIC, loginMnemonicToProposals } from '../helpers/login-mnemonic.mjs'
import { openWalletPanel, readReceiveAddress, waitForWalletSyncDone } from '../helpers/wallet-panel.mjs'

const BTC_ADDRESS_RE = /^(tb1|bcrt1|[123mn])[a-zA-HJ-NP-Z0-9]{25,62}$/

/** Reads the system clipboard, preferring Wayland and falling back to X11. */
function readSystemClipboard() {
	const readers = [
		['wl-paste', ['--no-newline']],
		['xclip', ['-selection', 'clipboard', '-o']],
		['xsel', ['--clipboard', '--output']],
	]
	for (const [bin, args] of readers) {
		try {
			return execFileSync(bin, args, { encoding: 'utf8', timeout: 5000 }).trim()
		} catch {
			// Try the next reader — the session may be Wayland-only or X11-only.
		}
	}
	throw new Error('no clipboard reader available (install wl-clipboard, xclip or xsel)')
}

/**
 * Puts a known value on the clipboard so a no-op copy is distinguishable from a
 * stale hit. Clipboard writers stay alive to *serve* the selection, so they must
 * run detached rather than being waited on (a synchronous call just times out).
 */
async function seedClipboard(value) {
	const writers = [
		['wl-copy', [value]],
		['xclip', ['-selection', 'clipboard', '-i']],
	]
	for (const [bin, args] of writers) {
		const child = spawn(bin, args, { stdio: ['pipe', 'ignore', 'ignore'], detached: true })
		let spawnFailed = false
		child.on('error', () => {
			spawnFailed = true
		})
		child.stdin.end(value)
		child.unref()

		// Give the writer a moment to take ownership, then confirm it stuck.
		await new Promise((resolve) => setTimeout(resolve, 500))
		if (!spawnFailed && readSystemClipboard() === value) {
			return
		}
	}
	throw new Error('no clipboard writer available (install wl-clipboard or xclip)')
}

describe('Alpen Multisig — clipboard copy', () => {
	it('puts the receive address on the system clipboard', async function () {
		this.timeout(300000)

		await loginMnemonicToProposals(DEMO_MNEMONIC)
		await $('//h1[contains(.,"Proposals")]').waitForDisplayed({ timeout: 60000 })

		await openWalletPanel()

		// The panel paints a cached receive address first and advances to the next unused
		// one once the background sync lands (earlier specs consume addresses by funding
		// them). Let the sync settle so the row is not mid-rotation when we click.
		await waitForWalletSyncDone()

		expect(await readReceiveAddress()).toMatch(BTC_ADDRESS_RE)

		// A sentinel proves the assertion below is not passing on stale clipboard content.
		const sentinel = `SENTINEL-${Date.now()}`
		await seedClipboard(sentinel)
		expect(readSystemClipboard()).toBe(sentinel)

		const copyButton = await $('[data-testid="e2e-wallet-receive-address-value"]')
		await copyButton.waitForClickable({ timeout: 10000 })
		await copyButton.click()

		// The copy goes through Tauri IPC, so give the round-trip a moment to land.
		await browser.waitUntil(() => readSystemClipboard() !== sentinel, {
			timeout: 10000,
			interval: 250,
			timeoutMsg: 'the copy button never reached the system clipboard',
		})

		// The card copies the bare address it is rendering at click time — deliberately not
		// a BIP-21 URI (see build-receive-qr-value.ts). Compare against the address read
		// back from the row instead of a snapshot taken earlier: a sync that rotates the
		// receive address mid-test would otherwise fail the run on a stale expectation.
		await browser.waitUntil(async () => readSystemClipboard() === (await readReceiveAddress()), {
			timeout: 10000,
			interval: 250,
			timeoutMsg: 'clipboard should hold the receive address the row is showing',
		})
	})
})
