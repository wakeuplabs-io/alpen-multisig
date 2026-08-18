/**
 * Admin ID Verification Certificate e2e test (PRD 06 §3.c.i, spec:
 * docs/specs/admin-id-verification-certificate.md).
 *
 * Walks the three normative wireframe states in order — unsigned, signed, copied — in the
 * running Tauri binary, on both sides of sign-in.
 *
 * What only an e2e can establish here: that the certificate reaches the *system*
 * clipboard as two usable lines. Everything the unit tests cover stops at the IPC
 * boundary; #428 was exactly a failure past it. A certificate that displays but does not
 * copy is useless — the whole point is handing it to someone else.
 *
 * Requires the full regtest stack and a graphical session (same as the other specs).
 */
import { DEMO_MNEMONIC } from '../helpers/login-mnemonic.mjs'
import { openWalletPanel } from '../helpers/wallet-panel.mjs'
import { readSystemClipboard, seedClipboard } from '../helpers/system-clipboard.mjs'

/** P2WPKH — the output type the Admin ID is, on any network the app connects to. */
const P2WPKH_RE = /^(bc|tb|bcrt)1q[qpzry9x8gf2tvdw0s3jn54khce6mua7l]{38}$/i
/** Base64, Bitcoin Core signmessage encoding: 65 bytes → 88 chars ending in '='. */
const CERTIFICATE_RE = /^[A-Za-z0-9+/]{87}=$/

const WAITING_LITERAL = 'Waiting for signature to generate Admin ID Verification Certificate...'

async function openCertificateModal(triggerTestId) {
	const trigger = await $(`[data-testid="${triggerTestId}"]`)
	await trigger.waitForClickable({ timeout: 30000 })
	await trigger.click()
	const modal = await $('[data-testid="e2e-admin-id-certificate-modal"]')
	await modal.waitForDisplayed({ timeout: 15000 })
	return modal
}

describe('Strata Multisig — Admin ID Verification Certificate', () => {
	it('signs, shows and copies the certificate, before and after sign-in', async function () {
		this.timeout(420000)

		// ── Before sign-in (#410): the certificate is offered on the connect card ──
		const connectMnemonic = await $('button[data-testid="e2e-connect-mnemonic"]')
		await connectMnemonic.waitForClickable({ timeout: 90000 })
		await connectMnemonic.click()

		const ta = await $('textarea[data-testid="e2e-connect-mnemonic-textarea"]')
		await ta.waitForDisplayed({ timeout: 30000 })
		const selectAllKey = process.platform === 'darwin' ? 'Meta' : 'Control'
		await browser.waitUntil(
			async () => {
				await ta.click()
				await browser.keys([selectAllKey, 'a'])
				await browser.keys('Backspace')
				await ta.addValue(DEMO_MNEMONIC)
				return (await ta.getValue()) === DEMO_MNEMONIC
			},
			{ timeout: 15000, interval: 500, timeoutMsg: 'mnemonic textarea did not hold the expected words' },
		)
		await connectMnemonic.waitForClickable({ timeout: 30000 })
		await connectMnemonic.click()
		const connectWithWords = await $('button[data-testid="e2e-connect-with-words"]')
		await connectWithWords.waitForClickable({ timeout: 30000 })
		await connectWithWords.click()

		await $('//h1[contains(.,"Select multisig")]').waitForDisplayed({ timeout: 60000 })
		const connectAdminId = (await (await $('[data-testid="e2e-connect-admin-id-value"]')).getText()).trim()
		expect(connectAdminId).toMatch(P2WPKH_RE)

		await openCertificateModal('e2e-connect-admin-id-verify')

		// Wireframe 1 — unsigned: the message box holds the exact bytes that will be
		// signed, and the result box says what it is waiting for.
		const preSignInMessage = await $('[data-testid="e2e-admin-id-certificate-message"]')
		await browser.waitUntil(async () => (await preSignInMessage.getText()).includes(connectAdminId), {
			timeout: 15000,
			timeoutMsg: 'the modal should show the Admin ID it is about to sign',
		})
		expect((await preSignInMessage.getText()).trim()).toEqual(`Admin ID: ${connectAdminId}`)
		expect(await (await $('[data-testid="e2e-admin-id-certificate-value"]')).getText()).toContain(WAITING_LITERAL)

		// No visible close button in the wireframes: Escape is the way out.
		await browser.keys('Escape')
		await $('[data-testid="e2e-admin-id-certificate-modal"]').waitForDisplayed({ timeout: 10000, reverse: true })

		// ── Finish signing in from where the pre-sign-in checks left off ──────────
		// The session is already connected at this point, so the flow continues through
		// authority selection rather than restarting at the connect screen.
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

		await $('//h1[contains(.,"Authenticate session")]').waitForDisplayed({ timeout: 60000 })
		await $('button[data-testid="e2e-authenticate-submit"]').click()
		await browser.waitUntil(async () => (await browser.getUrl()).includes('/proposals'), {
			timeout: 90000,
			timeoutMsg: 'expected URL to contain /proposals after authentication',
		})

		// ── After login: the same modal from the wallet panel ──────────────────────
		await openWalletPanel()

		const panelAdminId = (await (await $('[data-testid="e2e-wallet-admin-id-value"]')).getText()).trim()
		expect(panelAdminId).toEqual(connectAdminId)

		await openCertificateModal('e2e-wallet-admin-id-verify')
		const message = (await (await $('[data-testid="e2e-admin-id-certificate-message"]')).getText()).trim()
		expect(message).toEqual(`Admin ID: ${panelAdminId}`)

		// Wireframe 2 — signed: Sign is replaced by the Signed chip, and the result box
		// carries the certificate itself.
		const signButton = await $('[data-testid="e2e-admin-id-certificate-sign"]')
		await signButton.waitForClickable({ timeout: 15000 })
		await signButton.click()

		const signedChip = await $('[data-testid="e2e-admin-id-certificate-signed-chip"]')
		await signedChip.waitForDisplayed({ timeout: 60000 })
		expect(await signedChip.getText()).toContain('Signed')
		expect(await signButton.isExisting()).toBe(false)

		const certificate = (await (await $('[data-testid="e2e-admin-id-certificate-value"]')).getText()).trim()
		expect(certificate).toMatch(CERTIFICATE_RE)
		// The header byte the wireframe pins (31 + recid) — base64 'I'..'K' as the first
		// character. A raw device signature would start elsewhere.
		expect(certificate.charAt(0)).toMatch(/[IJK]/)

		// Step 2 is present and, in a mnemonic session, disabled with the reason (D3).
		const noDevice = await $('[data-testid="e2e-admin-id-certificate-no-device"]')
		expect(await noDevice.isDisplayed()).toBe(true)
		expect(await noDevice.getText()).toContain('mnemonic')

		// Wireframe 3 — copied: both lines reach the system clipboard, in order, with
		// nothing for the reader to strip before verifying.
		const sentinel = `SENTINEL-${Date.now()}`
		await seedClipboard(sentinel)
		expect(readSystemClipboard()).toBe(sentinel)

		const copyButton = await $('[data-testid="e2e-admin-id-certificate-copy"]')
		await copyButton.waitForClickable({ timeout: 10000 })
		await copyButton.click()

		await browser.waitUntil(() => readSystemClipboard() !== sentinel, {
			timeout: 10000,
			interval: 250,
			timeoutMsg: 'the copy button never reached the system clipboard',
		})

		const copied = readSystemClipboard().split('\n')
		expect(copied.length).toBe(2)
		expect(copied[0]).toEqual(message)
		expect(copied[1]).toEqual(certificate)

		await $('//*[contains(text(),"Copied to clipboard")]').waitForDisplayed({ timeout: 10000 })
	})
})
