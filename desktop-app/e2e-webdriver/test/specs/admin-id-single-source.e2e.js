/**
 * Admin ID single-source e2e test — the falsifiable check behind the reversion to a
 * bitcoin address (spec: docs/specs/admin-id-as-bitcoin-address.md).
 *
 * The Admin ID is rendered in three places: the multisig-selection step, the
 * authenticate step, and the wallet panel after login. All three are supposed to
 * resolve to the same `WalletAccountInfo.addressSample`, captured once at connect.
 * Nothing in the type system enforces that — the three are separate props on
 * separate component trees — so the claim is checked against the running app.
 *
 * If the three strings ever differ, the signer is comparing a different value against
 * their device screen depending on which screen they happen to be looking at, which is
 * precisely the failure the address reversion exists to remove.
 *
 * Requires the full regtest stack (bitcoind, ASM, orchestrator, Postgres) and .env.
 */
import { DEMO_MNEMONIC } from '../helpers/login-mnemonic.mjs'
import { openWalletPanel } from '../helpers/wallet-panel.mjs'

/** Bech32 segwit address, any of the networks the app connects to. */
const BECH32_RE = /^(bc|tb|bcrt)1[qpzry9x8gf2tvdw0s3jn54khce6mua7l]{11,}$/i

describe('Strata Multisig — Admin ID is one address everywhere', () => {
	it('shows the same bech32 Admin ID at both connect steps and in the wallet panel', async function () {
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

		expect(atSelection).toMatch(BECH32_RE)

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
	})
})
