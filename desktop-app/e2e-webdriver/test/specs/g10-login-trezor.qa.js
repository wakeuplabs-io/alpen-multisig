/**
 * G10-B2 — device QA for the login challenge on Trezor. Not a CI spec.
 *
 * The Trezor half of G10. A Trezor renders message text unconditionally, so unlike the Ledger it
 * never had the hash problem #402 reported — which is exactly why it is worth running: the
 * separator change must not degrade the screen on the device that was already fine, and the
 * orchestrator must still authenticate the signature it produces.
 *
 * Needs the dockerised emulator up (`trezor-emu-docker/up.sh`, outside this repo) and the local
 * stack. Run by hand:
 *
 *   SKIP_E2E_BUILD=1 npm run qa:login-trezor
 *
 * Left out of the CI glob (`.qa.js`), same convention as the G5 and G9 specs.
 */

import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'

const EVIDENCE = path.resolve(process.cwd(), '../../../../issues/evidence')
const EMU = 'alpen-trezor-emu'

/** P2WPKH on any network the app connects to. */
const P2WPKH_RE = /^(bc|tb|bcrt)1q[qpzry9x8gf2tvdw0s3jn54khce6mua7l]{38}$/i

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
	await browser.saveScreenshot(path.join(EVIDENCE, `g10-b2-trezor-${name}.png`))
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

/** Letters and digits alone: the device wraps and pads, and the `|` separators are its furniture. */
function alnum(text) {
	return text.replace(/[^a-z0-9]/gi, '')
}

describe('G10 — login challenge on Trezor', () => {
	it('renders the challenge as readable text and authenticates the session', async function () {
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

		// ── Step 2 of 3: pick the multisig ──
		await browser.waitUntil(
			async () => {
				const badge = await $(
					'//button[.//p[contains(text(),"Strata Administrator")]]//span[contains(text(),"Available")]',
				)
				return badge.isDisplayed()
			},
			{ timeout: 90000, timeoutMsg: 'Strata Administrator should show Available after the ASM membership check' },
		)
		await $('//button[.//p[contains(text(),"Strata Administrator")]]').click()
		const authorityContinue = await $('button[data-testid="e2e-authority-select-continue"]')
		await authorityContinue.waitForClickable({ timeout: 30000 })
		await authorityContinue.click()

		// ── Step 3 of 3: sign the challenge ──
		await $('//h1[contains(.,"Authenticate session")]').waitForDisplayed({ timeout: 60000 })
		await shoot('01-authenticate')
		await $('button[data-testid="e2e-authenticate-submit"]').click()

		const onProposals = async () => (await browser.getUrl()).includes('/proposals')
		const screens = await confirmOnDevice(onProposals)
		fs.writeFileSync(path.join(EVIDENCE, 'g10-b2-trezor-device-challenge.txt'), screens.join('\n---\n'))

		const rendered = alnum(screens.join(''))
		expect(rendered).toContain(alnum('Strata Session Authentication v1'))
		expect(rendered).toContain(alnum('Role: strata_administrator'))
		expect(rendered).toContain(alnum('Challenge:'))

		// The separators must not have cost the session: the orchestrator still has to accept a
		// signature over the new string.
		await browser.waitUntil(onProposals, {
			timeout: 90000,
			timeoutMsg: 'expected URL to contain /proposals after authenticating with the Trezor',
		})
		await shoot('02-authenticated')
	})
})
