/**
 * #566 — device QA for pairing a Trezor Safe 7 with the code on its screen. Not a CI spec.
 *
 * A Safe 7 speaks THP and, with release firmware, pairs only by CodeEntry: it shows a 6-digit
 * code the signer types into the app. The other Trezor specs pair through the shared helper;
 * this one covers what they cannot — a wrong code is refused, and connecting again pairs.
 *
 * Needs the dockerised Safe 7 emulator (`trezor-emu-docker/up.sh --model T3W1 --wipe`, outside
 * this repo) and the local stack. Run by hand:
 *
 *   SKIP_E2E_BUILD=1 npm run qa:trezor-pairing
 */

import fs from 'node:fs'
import path from 'node:path'

import { allowPairingUntil, enterPairingCode, pairingCodeOnDevice } from '../helpers/trezor-pairing.mjs'

const EVIDENCE = path.resolve(process.cwd(), '../../../../issues/evidence')

async function shoot(name) {
	fs.mkdirSync(EVIDENCE, { recursive: true })
	await browser.saveScreenshot(path.join(EVIDENCE, `566-pairing-${name}.png`))
}

describe('Trezor Safe 7 — pairing with the code on its screen', () => {
	it('refuses a wrong code, then pairs with the right one', async function () {
		this.timeout(300000)

		const chip = await $('button[data-testid="e2e-connect-trezor"]')
		await chip.waitForClickable({ timeout: 90000 })
		await chip.click()

		const connect = await $('button[data-testid="e2e-connect-with-words"]')
		await connect.waitForClickable({ timeout: 30000 })
		await connect.click()

		// Allow the pairing on the device, and stop once the app asks for the code.
		const form = await $('[data-testid="e2e-trezor-pairing-code"]')
		await allowPairingUntil(() => form.isDisplayed(), { enterCode: false })
		const code = pairingCodeOnDevice()
		if (code === null) {
			throw new Error('the app asked for a pairing code the device is not showing')
		}
		await shoot('01-code-requested')

		await enterPairingCode(code === '000000' ? '111111' : '000000')
		await browser.waitUntil(async () => /does not match/i.test(await $('body').getText()), {
			timeout: 60000,
			timeoutMsg: 'a wrong pairing code was not refused',
		})
		await shoot('02-wrong-code-refused')

		// The device dropped that pairing; connecting again starts a new one with a new code.
		await connect.waitForClickable({ timeout: 30000 })
		await connect.click()
		const adminIdValue = await $('[data-testid="e2e-connect-admin-id-value"]')
		await allowPairingUntil(() => adminIdValue.isDisplayed())
		await shoot('03-paired')
	})
})
