/** Demo regtest mnemonic — must match local asm-params / docs. */
export const DEMO_MNEMONIC = 'multiply toss magic exclude crawl obey garden black apart room village neglect'

/** Second Strata Administrator signer (canonical path) — asm-params keys[1]. */
export const DEMO_MNEMONIC_COSIGN = 'multiply toss magic exclude crawl obey garden black apart room village absent'

/**
 * Full wallet connect + session auth until URL is /proposals.
 * @param {string} [mnemonic]
 */
export async function loginMnemonicToProposals(mnemonic = DEMO_MNEMONIC) {
	const ta = await $('textarea[data-testid="e2e-connect-mnemonic-textarea"]')
	await ta.waitForDisplayed({ timeout: 90000 })
	// The textarea ships pre-filled with a dev default (DEMO_MNEMONIC) and now mounts only
	// after an async capability check (useMnemonicSigningEnabled). On that late controlled-input
	// mount, WDIO setValue can concatenate onto the existing value instead of replacing it,
	// producing a duplicated/invalid mnemonic. Clear deterministically, type, and verify the
	// field holds exactly the requested words before continuing.
	const selectAllKey = process.platform === 'darwin' ? 'Meta' : 'Control'
	await browser.waitUntil(
		async () => {
			await ta.click()
			await browser.keys([selectAllKey, 'a'])
			await browser.keys('Backspace')
			await ta.addValue(mnemonic)
			return (await ta.getValue()) === mnemonic
		},
		{
			timeout: 15000,
			interval: 500,
			timeoutMsg: 'mnemonic textarea did not hold the expected words after entry',
		},
	)

	const connectMnemonic = await $('button[data-testid="e2e-connect-mnemonic"]')
	await connectMnemonic.waitForClickable({ timeout: 30000 })
	await connectMnemonic.click()

	const connectWithWords = await $('button[data-testid="e2e-connect-with-words"]')
	await connectWithWords.waitForClickable({ timeout: 30000 })
	await connectWithWords.click()

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
