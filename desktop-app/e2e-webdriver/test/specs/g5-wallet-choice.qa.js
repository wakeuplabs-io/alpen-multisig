/**
 * G5-B6 — device QA, not a CI spec.
 *
 * Drives the real binary against the dockerised Trezor emulator to prove the two connect
 * actions reach two different wallets. Needs the emulator up with the passphrase enabled
 * (`trezor-emu-docker/up.sh --passphrase`), so it is run by hand and left out of test:e2e:all.
 *
 * Captures screenshots into issues/evidence/.
 */

import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'

const EVIDENCE = path.resolve(process.cwd(), '../../../../issues/evidence')
const EMU = 'alpen-trezor-emu'

/** Type a passphrase on the emulator keypad over the debug link. */
function typeOnDevice(passphrase) {
	const script = `
from trezorlib.debuglink import DebugLink
from trezorlib.transport.udp import UdpTransport
d = DebugLink(UdpTransport("127.0.0.1:21325"), auto_interact=True)
d.open()
try:
    d.input(${JSON.stringify(passphrase)})
    print("typed")
finally:
    d.close()
`
	return execFileSync('docker', ['exec', '-i', EMU, 'python3', '-'], { input: script, encoding: 'utf8' })
}

/** Whatever the device is showing right now. */
function deviceScreen() {
	const script = `
from trezorlib.debuglink import DebugLink
from trezorlib.transport.udp import UdpTransport
d = DebugLink(UdpTransport("127.0.0.1:21325"), auto_interact=True)
d.open()
try:
    print(d.read_layout().title())
finally:
    d.close()
`
	try {
		return execFileSync('docker', ['exec', '-i', EMU, 'python3', '-'], { input: script, encoding: 'utf8' }).trim()
	} catch {
		return ''
	}
}

async function shoot(name) {
	fs.mkdirSync(EVIDENCE, { recursive: true })
	await browser.saveScreenshot(path.join(EVIDENCE, `g5-448-b6-${name}.png`))
}

async function selectTrezor() {
	const chip = await $('button[data-testid="e2e-connect-trezor"]')
	await chip.waitForClickable({ timeout: 60000 })
	await chip.click()
}

async function readAdminId() {
	const value = await $('[data-testid="e2e-connect-admin-id-value"]')
	await value.waitForDisplayed({ timeout: 90000 })
	return (await value.getText()).trim()
}

describe('G5 — the two Trezor connect actions open different wallets', () => {
	it('opens the standard wallet, then a hidden one, and they differ', async function () {
		this.timeout(600000)

		// 1. Standard wallet: no keypad may appear.
		await selectTrezor()
		const connect = await $('button[data-testid="e2e-connect-with-words"]')
		await connect.waitForClickable({ timeout: 60000 })
		await connect.click()

		const standardId = await readAdminId()
		await shoot('standard-connected')
		console.log(`STANDARD_ADMIN_ID=${standardId}`)

		const screenAfterStandard = deviceScreen()
		console.log(`DEVICE_SCREEN_AFTER_STANDARD=${screenAfterStandard}`)
		if (/passphrase/i.test(screenAfterStandard)) {
			throw new Error('the standard wallet asked for a passphrase on the device keypad')
		}

		// 2. Reload back to the connect screen and take the hidden wallet.
		await browser.execute(() => window.location.assign('/'))
		await selectTrezor()

		const hidden = await $('button[data-testid="e2e-connect-hidden-wallet"]')
		await hidden.waitForClickable({ timeout: 60000 })
		await hidden.click()

		// The device puts its keypad up while the app still says "Detecting…".
		await browser.waitUntil(async () => /passphrase/i.test(deviceScreen()), {
			timeout: 60000,
			interval: 2000,
			timeoutMsg: 'the device never asked for a passphrase on its keypad',
		})
		await shoot('hidden-device-prompt')
		typeOnDevice('hidden1')

		const hiddenId = await readAdminId()
		await shoot('hidden-connected')
		console.log(`HIDDEN_ADMIN_ID=${hiddenId}`)

		if (hiddenId === standardId) {
			throw new Error(`both actions opened the same wallet (${hiddenId})`)
		}
	})
})
