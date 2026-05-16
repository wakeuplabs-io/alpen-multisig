/** Demo regtest mnemonic — must match local asm-params / docs. */
export const DEMO_MNEMONIC = 'multiply toss magic exclude crawl obey garden black apart room village neglect'

/**
 * Full wallet connect + session auth until URL is /proposals.
 * @param {string} [mnemonic]
 * @param {{ pickingRowIndex?: number }} [opts] — BIP-84 row index (default `0`). Use `1` for co-signer / “#1” session.
 */
export async function loginMnemonicToProposals(mnemonic = DEMO_MNEMONIC, opts = {}) {
	const pickingRowIndex = opts.pickingRowIndex ?? 0
	const ta = await $('textarea[data-testid="e2e-connect-mnemonic-textarea"]')
	await ta.waitForDisplayed({ timeout: 90000 })
	await ta.setValue(mnemonic)

	await $('button[data-testid="e2e-connect-palabras"]').click()
	await $('button[data-testid="e2e-connect-with-words"]').click()

	await $('//h1[contains(.,"Select your signer address")]').waitForDisplayed({ timeout: 90000 })
	await $(`button[data-testid="e2e-picking-row-${pickingRowIndex}"]`).click()
	await $('button[data-testid="e2e-picking-continue"]').click()

	await $('//h1[contains(.,"Select authority")]').waitForDisplayed({ timeout: 60000 })
	await $('button[data-testid="e2e-authority-select-continue"]').click()

	await $('//h1[contains(.,"Authenticate session")]').waitForDisplayed({ timeout: 60000 })
	await $('button[data-testid="e2e-authenticate-submit"]').click()

	await browser.waitUntil(async () => (await browser.getUrl()).includes('/proposals'), {
		timeout: 90000,
		timeoutMsg: 'expected URL to contain /proposals after authentication',
	})
}
