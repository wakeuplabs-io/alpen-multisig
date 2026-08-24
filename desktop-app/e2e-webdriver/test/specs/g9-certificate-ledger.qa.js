/**
 * G9-B1 — device QA for the Admin ID Verification Certificate on Ledger. Not a CI spec.
 *
 * This is the evidence the compliance matrix has been waiting for since G7: PRD 06 §3.c.i and
 * §4.2 both require the signer to read the Admin ID **off the device screen**, which no app-side
 * test can establish. B0 measured that the device renders the message text
 * (issues/evidence/G9-B0-LEDGER-MEASUREMENT.md); this spec walks the shipped UI end to end and
 * captures what a signer actually sees.
 *
 * Needs Speculos up (`./scripts/ledger-up.sh <bitcoin.elf>`) and the app pointed at it:
 *
 *   LEDGER_SPECULOS_URL=http://localhost:5001 SKIP_E2E_BUILD=1 \
 *     npm run qa:certificate-ledger
 *
 * Left out of the CI glob (`.qa.js`), same convention as the G5 specs.
 */

import fs from 'node:fs'
import path from 'node:path'

const EVIDENCE = path.resolve(process.cwd(), '../../../../issues/evidence')
const SPECULOS = process.env.LEDGER_SPECULOS_URL ?? 'http://localhost:5001'

/** P2WPKH on any network the app connects to. */
const P2WPKH_RE = /^(bc|tb|bcrt)1q[qpzry9x8gf2tvdw0s3jn54khce6mua7l]{38}$/i
/** Base64, Bitcoin Core signmessage encoding: 65 bytes → 88 chars ending in '='. */
const CERTIFICATE_RE = /^[A-Za-z0-9+/]{87}=$/

async function shoot(name) {
	fs.mkdirSync(EVIDENCE, { recursive: true })
	await browser.saveScreenshot(path.join(EVIDENCE, `g9-b1-ledger-${name}.png`))
}

/** The text lines the emulated device is painting right now. */
async function deviceScreen() {
	const res = await fetch(`${SPECULOS}/events?currentscreenonly=true`)
	const body = await res.json()
	return (body.events ?? []).map((e) => e.text).join(' ')
}

/** Save what the device screen looks like, alongside the app screenshot. */
async function shootDevice(name) {
	fs.mkdirSync(EVIDENCE, { recursive: true })
	const res = await fetch(`${SPECULOS}/screenshot`)
	const bytes = Buffer.from(await res.arrayBuffer())
	fs.writeFileSync(path.join(EVIDENCE, `g9-b1-device-${name}.png`), bytes)
}

async function pressDevice(button) {
	await fetch(`${SPECULOS}/button/${button}`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ action: 'press-and-release' }),
	})
}

/**
 * Stands in for the human holding the Ledger: pages through the review screens and approves.
 * Speculos' automation rules only cover the PSBT flow, so a message signature would otherwise
 * sit forever waiting for a button that never comes.
 *
 * Shoots every screen matching `payloadRe` — the pages carrying what the signer is asked to
 * read, which is what the requirement is about, and which the device splits across pages.
 * Returns every distinct screen it saw.
 */
async function approveOnDevice(shotName, isDone, payloadRe) {
	const seen = []
	const deadline = Date.now() + 120000
	let shot = 0
	while (Date.now() < deadline) {
		if (await isDone()) {
			return seen
		}
		const text = (await deviceScreen()).trim()
		if (text && seen[seen.length - 1] !== text) {
			seen.push(text)
			if (payloadRe.test(text)) {
				await shootDevice(`${shotName}-${String(++shot).padStart(2, '0')}`)
			}
		}
		// 'Sign message' ends the signing flow; 'Confirm' ends the address check.
		const lower = text.toLowerCase()
		const isAction = ['approve', 'sign message', 'confirm'].some((a) => lower.includes(a))
		await pressDevice(isAction ? 'both' : 'right')
		await browser.pause(200)
	}
	throw new Error(`the device never finished the flow. Screens seen:\n${seen.join('\n')}`)
}

/**
 * Leaves the emulator on its idle screen. A previous run that ended mid-flow parks the device on a
 * review or confirmation screen, and the app's next request then fails on a device that is not
 * where it expects it — which is a dirty-fixture failure, not a finding.
 */
async function settleDevice() {
	const deadline = Date.now() + 20000
	while (Date.now() < deadline) {
		const text = (await deviceScreen()).trim()
		if (/app is ready/i.test(text)) {
			return
		}
		// Reject/Cancel resolves a pending flow; anything else just pages forward.
		await pressDevice(/cancel|reject/i.test(text) ? 'both' : 'right')
		await browser.pause(200)
	}
	throw new Error(`the emulator would not return to its idle screen (showing: ${await deviceScreen()})`)
}

describe('G9 — Admin ID Verification Certificate on Ledger', () => {
	it('signs the certificate and confirms the Admin ID on the device', async function () {
		this.timeout(420000)

		await settleDevice()

		// ── Connect the Ledger (Speculos) ──
		// Two steps: the chip picks the connection method, the action button below performs it.
		const connectLedger = await $('button[data-testid="e2e-connect-ledger"]')
		await connectLedger.waitForClickable({ timeout: 90000 })
		await connectLedger.click()

		const connect = await $('button[data-testid="e2e-connect-with-words"]')
		await connect.waitForClickable({ timeout: 30000 })
		await connect.click()

		// The connect card shows the Admin ID the device derived.
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

		// While the device waits for approval it must be showing the message text — the claim B0
		// established, re-checked here through the shipped flow rather than the Rust port.
		const chip = await $('[data-testid="e2e-admin-id-certificate-signed-chip"]')
		const signScreens = await approveOnDevice('01-message-page', () => chip.isDisplayed(), /Admin ID/)
		const signText = signScreens.join('\n')
		expect(signText.toLowerCase()).not.toContain('hash')
		expect(signText).toContain('Admin ID')

		await chip.waitForDisplayed({ timeout: 120000 })
		const certificate = (await (await $('[data-testid="e2e-admin-id-certificate-value"]')).getText()).trim()
		expect(certificate).toMatch(CERTIFICATE_RE)
		await shoot('03-modal-signed')

		// ── Step 2: confirm the same Admin ID on the device screen ──
		const verify = await $('[data-testid="e2e-wallet-verify-on-device"]')
		await verify.waitForClickable({ timeout: 15000 })
		await verify.click()

		const confirmed = await $('[data-testid="e2e-wallet-verify-on-device-result"]')
		const verifyScreens = await approveOnDevice('02-address-page', () => confirmed.isDisplayed(), /Address/)

		// The requirement §3.c.i is about: the signer reads the Admin ID off the device itself.
		// The device wraps it across lines, so compare the reassembled payload.
		const rendered = verifyScreens.join('').replace(/[\s|]/g, '')
		expect(rendered).toContain(adminId)

		await confirmed.waitForDisplayed({ timeout: 120000 })
		await shoot('04-verified-on-device')

		const mismatch = await $('[data-testid="e2e-wallet-verify-on-device-mismatch"]')
		expect(await mismatch.isExisting()).toBe(false)
	})
})
