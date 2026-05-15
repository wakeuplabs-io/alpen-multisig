/**
 * Real-app smoke: mnemonic path through authority selection and session auth to /proposals.
 * Requires stack (bitcoind, ASM, orchestrator, Postgres) and .env — see README.md.
 */
const MNEMONIC =
	'multiply toss magic exclude crawl obey garden black apart room village neglect'

describe('Alpen Multisig wallet smoke', () => {
	it('connects with mnemonic and reaches /proposals', async () => {
		const ta = await $('textarea[data-testid="e2e-connect-mnemonic-textarea"]')
		await ta.waitForDisplayed({ timeout: 90000 })
		await ta.setValue(MNEMONIC)

		await $('button[data-testid="e2e-connect-palabras"]').click()
		await $('button[data-testid="e2e-connect-with-words"]').click()

		await $('//h1[contains(.,"Select your signer address")]').waitForDisplayed({ timeout: 90000 })
		await $('button[data-testid="e2e-picking-row-0"]').click()
		await $('button[data-testid="e2e-picking-continue"]').click()

		await $('//h1[contains(.,"Select authority")]').waitForDisplayed({ timeout: 60000 })
		await $('button[data-testid="e2e-authority-select-continue"]').click()

		await $('//h1[contains(.,"Authenticate session")]').waitForDisplayed({ timeout: 60000 })
		await $('button[data-testid="e2e-authenticate-submit"]').click()

		await browser.waitUntil(async () => (await browser.getUrl()).includes('/proposals'), {
			timeout: 90000,
			timeoutMsg: 'expected URL to contain /proposals after authentication',
		})
	})
})
