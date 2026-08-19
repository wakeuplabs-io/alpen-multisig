/**
 * G9-B1 — device QA for the Admin ID Verification Certificate on Trezor. Not a CI spec.
 *
 * The Trezor half of the evidence PRD 06 §3.c.i and §4.2 need: the signer reads the Admin ID off
 * the device screen, both when signing the certificate and when verifying the address. Unlike the
 * Ledger, a Trezor renders message text unconditionally, so what is measured here is that the
 * shipped flow puts the right string in front of the signer on both steps.
 *
 * Needs the dockerised emulator up (`trezor-emu-docker/up.sh`, outside this repo) and the local
 * stack. Run by hand:
 *
 *   SKIP_E2E_BUILD=1 npm run qa:certificate-trezor
 *
 * Left out of the CI glob (`.qa.js`), same convention as the G5 specs.
 */

import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'

const EVIDENCE = path.resolve(process.cwd(), '../../../../issues/evidence')
const EMU = 'alpen-trezor-emu'

/** P2WPKH on any network the app connects to. */
const P2WPKH_RE = /^(bc|tb|bcrt)1q[qpzry9x8gf2tvdw0s3jn54khce6mua7l]{38}$/i
/** Base64, Bitcoin Core signmessage encoding: 65 bytes → 88 chars ending in '='. */
const CERTIFICATE_RE = /^[A-Za-z0-9+/]{87}=$/

function onDevice(script) {
	return execFileSync('docker', ['exec', '-i', EMU, 'python3', '-'], { input: script, encoding: 'utf8' }).trim()
}

/** Everything the device is painting right now, as one string. */
function deviceScreen() {
	try {
		return onDevice(`
from trezorlib.debuglink import DebugLink
from trezorlib.transport.udp import UdpTransport
d = DebugLink(UdpTransport("127.0.0.1:21325"), auto_interact=True)
d.open()
try:
    layout = d.read_layout()
    print(layout.title())
    print(layout.text_content())
finally:
    d.close()
`)
	} catch {
		return ''
	}
}

function pressYes() {
	try {
		onDevice(`
from trezorlib.debuglink import DebugLink
from trezorlib.transport.udp import UdpTransport
d = DebugLink(UdpTransport("127.0.0.1:21325"), auto_interact=True)
d.open()
try:
    d.press_yes()
finally:
    d.close()
`)
	} catch {
		/* the device may have moved on between read and press */
	}
}

async function shoot(name) {
	fs.mkdirSync(EVIDENCE, { recursive: true })
	await browser.saveScreenshot(path.join(EVIDENCE, `g9-b1-trezor-${name}.png`))
}

/**
 * Stands in for the human holding the Trezor: confirms each screen until `isDone()`, recording
 * what the device showed. The emulator ignores the mouse, so every confirmation goes over the
 * debug link.
 */
async function confirmOnDevice(isDone) {
	const seen = []
	const deadline = Date.now() + 120000
	while (Date.now() < deadline) {
		if (await isDone()) {
			return seen
		}
		const text = deviceScreen()
		if (text && seen[seen.length - 1] !== text) {
			seen.push(text)
		}
		pressYes()
		await browser.pause(400)
	}
	throw new Error(`the device never finished the flow. Screens seen:\n${seen.join('\n---\n')}`)
}

/** The device text, stripped of the wrapping the screen applies, for comparing against an address. */
function reassemble(screens) {
	return screens.join('').replace(/[\s|]/g, '')
}

describe('G9 — Admin ID Verification Certificate on Trezor', () => {
	it('signs the certificate and confirms the Admin ID on the device', async function () {
		this.timeout(420000)

		// ── Connect the Trezor emulator ──
		const connectTrezor = await $('button[data-testid="e2e-connect-trezor"]')
		await connectTrezor.waitForClickable({ timeout: 90000 })
		await connectTrezor.click()

		const connect = await $('button[data-testid="e2e-connect-with-words"]')
		await connect.waitForClickable({ timeout: 30000 })
		await connect.click()

		const adminIdValue = await $('[data-testid="e2e-connect-admin-id-value"]')
		await adminIdValue.waitForDisplayed({ timeout: 120000 })
		await browser.waitUntil(async () => P2WPKH_RE.test((await adminIdValue.getText()).trim()), {
			timeout: 120000,
			interval: 1000,
			timeoutMsg: 'the connect card never showed a P2WPKH Admin ID',
		})
		const adminId = (await adminIdValue.getText()).trim()
		await shoot('01-connected')

		// ── Step 1: sign the certificate ──
		const trigger = await $('[data-testid="e2e-connect-admin-id-verify"]')
		await trigger.waitForClickable({ timeout: 30000 })
		await trigger.click()

		const modal = await $('[data-testid="e2e-admin-id-certificate-modal"]')
		await modal.waitForDisplayed({ timeout: 15000 })
		await shoot('02-modal-unsigned')

		const message = await $('[data-testid="e2e-admin-id-certificate-message"]')
		expect((await message.getText()).trim()).toBe(`Admin ID: ${adminId}`)

		const sign = await $('[data-testid="e2e-admin-id-certificate-sign"]')
		await sign.waitForClickable({ timeout: 15000 })
		await sign.click()

		const chip = await $('[data-testid="e2e-admin-id-certificate-signed-chip"]')
		const signScreens = await confirmOnDevice(() => chip.isDisplayed())
		fs.writeFileSync(path.join(EVIDENCE, 'g9-b1-trezor-device-sign.txt'), signScreens.join('\n---\n'))

		// The requirement: the signer reads what they sign, off the device itself.
		expect(reassemble(signScreens)).toContain(adminId)

		await chip.waitForDisplayed({ timeout: 120000 })
		const certificate = (await (await $('[data-testid="e2e-admin-id-certificate-value"]')).getText()).trim()
		expect(certificate).toMatch(CERTIFICATE_RE)
		await shoot('03-modal-signed')

		// ── Step 2: confirm the same Admin ID on the device screen ──
		const verify = await $('[data-testid="e2e-wallet-verify-on-device"]')
		await verify.waitForClickable({ timeout: 15000 })
		await verify.click()

		const confirmed = await $('[data-testid="e2e-wallet-verify-on-device-result"]')
		const verifyScreens = await confirmOnDevice(() => confirmed.isDisplayed())
		fs.writeFileSync(path.join(EVIDENCE, 'g9-b1-trezor-device-address.txt'), verifyScreens.join('\n---\n'))
		expect(reassemble(verifyScreens)).toContain(adminId)

		await confirmed.waitForDisplayed({ timeout: 120000 })
		await shoot('04-verified-on-device')

		const mismatch = await $('[data-testid="e2e-wallet-verify-on-device-mismatch"]')
		expect(await mismatch.isExisting()).toBe(false)
	})
})
