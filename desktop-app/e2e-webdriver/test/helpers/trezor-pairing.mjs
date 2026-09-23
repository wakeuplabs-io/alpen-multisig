import { execFileSync } from 'node:child_process'

const EMU = 'alpen-trezor-emu'

/** The pairing code closes the device's text, in two groups: "…code on <host> 123 456". */
const PAIRING_CODE_RE = /(\d{3}) (\d{3})\s*$/

/** Runs a debug-link script inside the emulator container (outside this repo, see the QA specs). */
function onDevice(body) {
	const script = `
from trezorlib.debuglink import DebugLink
from trezorlib.transport.udp import UdpTransport
d = DebugLink(UdpTransport("127.0.0.1:21325"), auto_interact=True)
d.open()
try:
${body}
finally:
    d.close()
`
	try {
		return execFileSync('docker', ['exec', '-i', EMU, 'python3', '-'], { input: script, encoding: 'utf8' }).trim()
	} catch {
		return ''
	}
}

/** The pairing code the device is showing, or null. */
export function pairingCodeOnDevice() {
	const match = onDevice('    print(d.read_layout().text_content())').match(PAIRING_CODE_RE)
	return match ? `${match[1]}${match[2]}` : null
}

/** Types `code` into the app's pairing form and submits it. */
export async function enterPairingCode(code) {
	await $('[data-testid="e2e-trezor-pairing-code"]').setValue(code)
	await $('button[data-testid="e2e-trezor-pairing-submit"]').click()
}

/**
 * A THP device (Safe 7) pairs the first time the app opens a channel to it in a run: it asks the
 * signer to allow this computer, then shows a code the signer types into the app. Stands in for
 * that signer until `isDone()`. Pressing "yes" on the code screen would cancel the pairing, so
 * only the allow dialog is ever confirmed. A V1 device (Safe 3) never pairs, so there this only
 * waits. With `enterCode: false` the code is left for the caller to type.
 */
export async function allowPairingUntil(isDone, { enterCode = true, timeout = 120000 } = {}) {
	const deadline = Date.now() + timeout
	while (Date.now() < deadline) {
		if (await isDone()) {
			return
		}
		const screen = onDevice('    print(d.read_layout().text_content())')
		const code = screen.match(PAIRING_CODE_RE)
		const form = await $('[data-testid="e2e-trezor-pairing-code"]')
		if (/to pair with this Trezor/i.test(screen)) {
			onDevice('    d.press_yes()')
		} else if (enterCode && code && (await form.isDisplayed()) && (await form.isEnabled())) {
			await enterPairingCode(`${code[1]}${code[2]}`)
		}
		await browser.pause(500)
	}
	throw new Error('the Trezor flow never finished')
}
