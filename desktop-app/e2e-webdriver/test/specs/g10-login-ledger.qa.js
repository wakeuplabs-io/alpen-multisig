/**
 * G10-B2 — device QA for the login challenge on Ledger. Not a CI spec.
 *
 * B0 measured the two message shapes through the Rust port and found that the ` | ` format
 * renders as text where the newline format renders a SHA-256 "Message hash"
 * (issues/evidence/G10-B0-CHALLENGE-MEASUREMENT.md). This spec is the other half: it drives the
 * **shipped login flow** end to end and asserts on what the device paints while the signer is
 * being asked to approve — which is what #402 reported and what no app-side test can establish.
 *
 * It also proves the change did not break authentication: the run only passes if the orchestrator
 * accepts the signature over the new string and the app lands on /proposals.
 *
 * Needs Speculos up (`./scripts/ledger-up.sh <bitcoin.elf>`), the full regtest stack, and the app
 * pointed at the emulator:
 *
 *   LEDGER_SPECULOS_URL=http://localhost:5001 SKIP_E2E_BUILD=1 npm run qa:login-ledger
 *
 * Left out of the CI glob (`.qa.js`), same convention as the G5 and G9 specs.
 */

import fs from 'node:fs'
import path from 'node:path'

const EVIDENCE = path.resolve(process.cwd(), '../../../../issues/evidence')
const SPECULOS = process.env.LEDGER_SPECULOS_URL ?? 'http://localhost:5001'

/** P2WPKH on any network the app connects to. */
const P2WPKH_RE = /^(bc|tb|bcrt)1q[qpzry9x8gf2tvdw0s3jn54khce6mua7l]{38}$/i

async function shoot(name) {
	fs.mkdirSync(EVIDENCE, { recursive: true })
	await browser.saveScreenshot(path.join(EVIDENCE, `g10-b2-ledger-${name}.png`))
}

/** The text lines the emulated device is painting right now. */
async function deviceScreen() {
	const res = await fetch(`${SPECULOS}/events?currentscreenonly=true`)
	const body = await res.json()
	return (body.events ?? []).map((e) => e.text).join(' ')
}

async function shootDevice(name) {
	fs.mkdirSync(EVIDENCE, { recursive: true })
	const res = await fetch(`${SPECULOS}/screenshot`)
	const bytes = Buffer.from(await res.arrayBuffer())
	fs.writeFileSync(path.join(EVIDENCE, `g10-b2-device-${name}.png`), bytes)
}

async function pressDevice(button) {
	await fetch(`${SPECULOS}/button/${button}`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ action: 'press-and-release' }),
	})
}

/**
 * Stands in for the human holding the Ledger: pages through the review screens and approves,
 * shooting every screen that carries part of the challenge. Speculos' automation rules only
 * cover the PSBT flow, so a message signature would otherwise wait forever for a button press.
 */
async function approveOnDevice(isDone) {
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
			if (/message/i.test(text)) {
				await shootDevice(`challenge-${String(++shot).padStart(2, '0')}`)
			}
		}
		const lower = text.toLowerCase()
		await pressDevice(['approve', 'sign message'].some((a) => lower.includes(a)) ? 'both' : 'right')
		await browser.pause(200)
	}
	throw new Error(`the device never finished the flow. Screens seen:\n${seen.join('\n')}`)
}

/**
 * Leaves the emulator on its idle screen. A previous run that ended mid-flow parks the device on
 * a review screen, and the app's next request then fails on a dirty fixture rather than on a
 * finding.
 */
async function settleDevice() {
	const deadline = Date.now() + 20000
	while (Date.now() < deadline) {
		const text = (await deviceScreen()).trim()
		if (/app is ready/i.test(text)) {
			return
		}
		await pressDevice(/cancel|reject/i.test(text) ? 'both' : 'right')
		await browser.pause(200)
	}
	throw new Error(`the emulator would not return to its idle screen (showing: ${await deviceScreen()})`)
}

describe('G10 — login challenge on Ledger', () => {
	it('renders the challenge as readable text and authenticates the session', async function () {
		this.timeout(420000)

		await settleDevice()

		// ── Connect the Ledger (Speculos) ──
		const connectLedger = await $('button[data-testid="e2e-connect-ledger"]')
		await connectLedger.waitForClickable({ timeout: 90000 })
		await connectLedger.click()

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
		const screens = await approveOnDevice(onProposals)
		const seen = screens.join('\n')

		// The finding #402 reported, checked where it happens. The device wraps mid-word and
		// paginates, so the payload is reassembled before comparing, on letters and digits alone —
		// pagination markers and the `|` separators are the device's own furniture.
		const alnum = (text) => text.replace(/[^a-z0-9]/gi, '')
		const rendered = alnum(seen)

		expect(seen.toLowerCase()).not.toContain('message hash')
		expect(rendered).toContain(alnum('Strata Session Authentication v1'))
		expect(rendered).toContain(alnum('Role: strata_administrator'))
		expect(rendered).toContain(alnum('Challenge:'))

		// Authentication itself: the orchestrator has to accept a signature over the new string,
		// otherwise the readable screen bought nothing.
		await browser.waitUntil(onProposals, {
			timeout: 90000,
			timeoutMsg: 'expected URL to contain /proposals after authenticating with the Ledger',
		})
		await shoot('02-authenticated')
	})
})
