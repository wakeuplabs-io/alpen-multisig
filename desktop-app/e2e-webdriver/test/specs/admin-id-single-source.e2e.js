/**
 * Admin ID single-source e2e test — the falsifiable check behind the reversion to a
 * bitcoin address (spec: docs/specs/admin-id-as-bitcoin-address.md).
 *
 * The Admin ID is rendered in four places: the multisig-selection step, the
 * authenticate step, the session chip in the header, and the wallet panel that chip
 * opens. All four are supposed to resolve to the same `WalletAccountInfo.addressSample`,
 * captured once at connect. Nothing in the type system enforces that — they are separate
 * props on separate component trees — so the claim is checked against the running app.
 *
 * If the strings ever differ, the signer is comparing a different value against their
 * device screen depending on which one they happen to be looking at, which is precisely
 * the failure the address reversion exists to remove. Review caught one such divergence
 * that this spec, in its first form, was blind to: a screen labelling its chip from the
 * public key while the panel below it showed the address.
 *
 * Requires the full regtest stack (bitcoind, ASM, orchestrator, Postgres) and .env.
 */
import { DEMO_MNEMONIC } from '../helpers/login-mnemonic.mjs'
import { openWalletPanel } from '../helpers/wallet-panel.mjs'

/** P2WPKH — the output type the Admin ID is, on any network the app connects to. */
const P2WPKH_RE = /^(bc|tb|bcrt)1q[qpzry9x8gf2tvdw0s3jn54khce6mua7l]{38}$/i

describe('Strata Multisig — Admin ID is one address everywhere', () => {
	it('shows the same P2WPKH Admin ID at both connect steps, in the chip and in the panel', async function () {
		this.timeout(300000)

		// ── Connect with the demo mnemonic ──────────────────────────────────────
		const connectMnemonic = await $('button[data-testid="e2e-connect-mnemonic"]')
		await connectMnemonic.waitForClickable({ timeout: 90000 })
		await connectMnemonic.click()

		const ta = await $('textarea[data-testid="e2e-connect-mnemonic-textarea"]')
		await ta.waitForDisplayed({ timeout: 30000 })
		const selectAllKey = process.platform === 'darwin' ? 'Meta' : 'Control'
		await browser.waitUntil(
			async () => {
				await ta.click()
				await browser.keys([selectAllKey, 'a'])
				await browser.keys('Backspace')
				await ta.addValue(DEMO_MNEMONIC)
				return (await ta.getValue()) === DEMO_MNEMONIC
			},
			{ timeout: 15000, interval: 500, timeoutMsg: 'mnemonic textarea did not hold the expected words' },
		)
		await connectMnemonic.waitForClickable({ timeout: 30000 })
		await connectMnemonic.click()
		const connectWithWords = await $('button[data-testid="e2e-connect-with-words"]')
		await connectWithWords.waitForClickable({ timeout: 30000 })
		await connectWithWords.click()

		// ── Step 2 of 3: the Admin ID is shown before the membership check ──────
		await $('//h1[contains(.,"Select multisig")]').waitForDisplayed({ timeout: 60000 })
		const connectValue = await $('[data-testid="e2e-connect-admin-id-value"]')
		await connectValue.waitForDisplayed({ timeout: 30000 })
		const atSelection = (await connectValue.getText()).trim()

		expect(atSelection).toMatch(P2WPKH_RE)

		await browser.waitUntil(
			async () => {
				const badge = await $(
					'//button[.//p[contains(text(),"Strata Administrator")]]//span[contains(text(),"Available")]',
				)
				return badge.isDisplayed()
			},
			{ timeout: 90000, timeoutMsg: 'Strata Administrator should show Available after ASM membership check' },
		)
		await $('//button[.//p[contains(text(),"Strata Administrator")]]').click()
		const authorityContinue = await $('button[data-testid="e2e-authority-select-continue"]')
		await authorityContinue.waitForClickable({ timeout: 30000 })
		await authorityContinue.click()

		// ── Step 3 of 3: the same string, on a different component tree ─────────
		await $('//h1[contains(.,"Authenticate session")]').waitForDisplayed({ timeout: 60000 })
		const authenticateValue = await $('[data-testid="e2e-authenticate-admin-id-value"]')
		await authenticateValue.waitForDisplayed({ timeout: 30000 })
		const atAuthenticate = (await authenticateValue.getText()).trim()

		expect(atAuthenticate).toEqual(atSelection)

		await $('button[data-testid="e2e-authenticate-submit"]').click()
		await browser.waitUntil(async () => (await browser.getUrl()).includes('/proposals'), {
			timeout: 90000,
			timeoutMsg: 'expected URL to contain /proposals after authentication',
		})

		// ── After login: the wallet panel reads the session, not the connect flow ──
		await openWalletPanel()
		const panelValue = await $('[data-testid="e2e-wallet-admin-id-value"]')
		await panelValue.waitForDisplayed({ timeout: 30000 })
		const inPanel = (await panelValue.getText()).trim()

		expect(inPanel).toEqual(atSelection)

		// The Admin ID must appear once per surface. A second copy is what #413 reported,
		// and with the value being an address again a stale duplicate would read as a
		// second, fundable address rather than as a harmless repetition.
		const panelOccurrences = await $$(`//*[normalize-space(text())="${inPanel}"]`)
		expect(panelOccurrences.length).toEqual(1)

		// The header chip is the fourth surface, and the one review found disagreeing. It
		// shows the Admin ID truncated, so it is checked by its ends rather than in full.
		const chip = await $('[data-testid="e2e-session-chip-trigger"]')
		if (await chip.isExisting()) {
			const chipText = await chip.getText()
			expect(chipText).toContain(inPanel.slice(0, 10))
			expect(chipText).toContain(inPanel.slice(-8))
		}
	})
})
