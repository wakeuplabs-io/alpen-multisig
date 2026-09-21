import { execFileSync } from 'node:child_process'

const EMU = 'alpen-trezor-emu'

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

/**
 * A THP device (Safe 7) asks the signer to allow this computer the first time the app opens a
 * channel to it in a run. Confirms that dialog until `isDone()`, and presses nothing else: a V1
 * device never shows it, so on a Safe 3 this only waits.
 */
export async function allowPairingUntil(isDone, timeout = 120000) {
	const deadline = Date.now() + timeout
	while (Date.now() < deadline) {
		if (await isDone()) {
			return
		}
		if (/to pair with this Trezor/i.test(onDevice('    print(d.read_layout().text_content())'))) {
			onDevice('    d.press_yes()')
		}
		await browser.pause(500)
	}
	throw new Error('the Trezor flow never finished')
}
