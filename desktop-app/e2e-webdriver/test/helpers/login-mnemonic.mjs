/** Demo regtest mnemonic — must match local asm-params / docs. */
export const DEMO_MNEMONIC = 'multiply toss magic exclude crawl obey garden black apart room village neglect'

/** Second Strata Administrator signer (canonical path) — asm-params keys[1]. */
export const DEMO_MNEMONIC_COSIGN =
	'multiply toss magic exclude crawl obey garden black apart room village absent'

/**
 * Full wallet connect + session auth until URL is /proposals.
 * @param {string} [mnemonic]
 */
export async function loginMnemonicToProposals(mnemonic = DEMO_MNEMONIC) {
	const ta = await $('textarea[data-testid="e2e-connect-mnemonic-textarea"]')
	await ta.waitForDisplayed({ timeout: 90000 })
	await ta.setValue(mnemonic)

	await $('button[data-testid="e2e-connect-palabras"]').click()
	await $('button[data-testid="e2e-connect-with-words"]').click()

	await $('//h1[contains(.,"Select authority")]').waitForDisplayed({ timeout: 60000 })
	// Membership check disables Continue until ASM confirms the signer; wait for Strata Admin.
	await browser.waitUntil(
		async () => {
			const badge = await $(
				'//button[.//p[contains(text(),"Strata Administrator")]]//span[contains(text(),"Available")]',
			)
			return badge.isDisplayed()
		},
		{
			timeout: 90000,
			timeoutMsg: 'Strata Administrator should show Available after ASM membership check',
		},
	)
	await $('//button[.//p[contains(text(),"Strata Administrator")]]').click()
	const authorityContinue = await $('button[data-testid="e2e-authority-select-continue"]')
	await authorityContinue.waitForClickable({ timeout: 30000 })
	await authorityContinue.click()

	await $('//h1[contains(.,"Authenticate session")]').waitForDisplayed({ timeout: 60000 })
	await $('button[data-testid="e2e-authenticate-submit"]').click()

	await browser.waitUntil(async () => (await browser.getUrl()).includes('/proposals'), {
		timeout: 90000,
		timeoutMsg: 'expected URL to contain /proposals after authentication',
	})
}
