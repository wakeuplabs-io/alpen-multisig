/**
 * G5-B6 — the refusal case, and the screen itself.
 *
 * Run with the emulator's passphrase switched OFF (`up.sh --no-passphrase`). Asking for a
 * hidden wallet there must stop with a message, not connect to the standard wallet and call
 * it success — the firmware never emits PassphraseRequest, so nothing the host sends can
 * produce a prompt (case 4 in G5-B0-PROTOCOL.md).
 *
 * Device QA, not a CI spec.
 */

import fs from 'node:fs'
import path from 'node:path'

const EVIDENCE = path.resolve(process.cwd(), '../../../../issues/evidence')

async function shoot(name) {
	fs.mkdirSync(EVIDENCE, { recursive: true })
	await browser.saveScreenshot(path.join(EVIDENCE, `g5-448-b6-${name}.png`))
}

describe('G5 — a hidden wallet is refused when the device cannot open one', () => {
	it('shows both actions, and refuses the hidden one with the passphrase off', async function () {
		this.timeout(600000)

		const chip = await $('button[data-testid="e2e-connect-trezor"]')
		await chip.waitForClickable({ timeout: 60000 })
		await chip.click()

		// Both actions on the idle screen: this is the control the request asked for.
		const connect = await $('button[data-testid="e2e-connect-with-words"]')
		const hidden = await $('button[data-testid="e2e-connect-hidden-wallet"]')
		await connect.waitForDisplayed({ timeout: 30000 })
		await hidden.waitForDisplayed({ timeout: 30000 })
		await hidden.scrollIntoView()
		await shoot('both-actions')
		console.log(`HIDDEN_LABEL=${await hidden.getText()}`)

		await hidden.click()

		// It must fail, and say why. Landing on the Admin ID screen would mean it silently
		// opened the standard wallet, which is the defect this case exists for.
		await browser.waitUntil(
			async () => {
				const onAdminId = await $$('[data-testid="e2e-connect-admin-id-value"]')
				if (onAdminId.length > 0) {
					throw new Error('the app connected to a wallet instead of refusing the hidden one')
				}
				return /passphrase switched off/i.test(await $('body').getText())
			},
			{ timeout: 120000, interval: 1000, timeoutMsg: 'no refusal message appeared' },
		)

		await shoot('hidden-refused-passphrase-off')
		console.log('REFUSED_AS_EXPECTED')
	})
})
