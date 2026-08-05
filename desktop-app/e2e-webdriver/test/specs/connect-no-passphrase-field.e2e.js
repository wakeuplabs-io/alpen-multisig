/**
 * #448 — the connect screen must never ask for a Trezor passphrase on this machine.
 *
 * The passphrase unlocks a hidden wallet. Typing it on the host puts it in reach of a
 * keylogger, which is the whole reason the keys live on a device, so it is entered on the
 * Trezor keypad instead. This spec is the guard that the field does not come back: it selects
 * the Trezor connection method on the real binary and asserts there is nowhere to type it.
 *
 * What replaces the field is copy, not a control: connecting is what makes the device prompt,
 * so a second button beside "Connect wallet" would run the same handler while implying the two
 * open different wallets. This spec pins that too — one connect action, not two.
 *
 * Requires the app binary only — it never leaves the connect screen. See README.md.
 */

describe('Strata Multisig connect — no passphrase typed on the host', () => {
	it('offers the Trezor method without a passphrase field', async function () {
		this.timeout(120000)

		// The connect screen is the landing route; no login needed.
		const trezorChip = await $('button[data-testid="e2e-connect-trezor"]')
		await trezorChip.waitForClickable({ timeout: 60000 })
		await trezorChip.click()

		// Whatever the Trezor branch renders, none of it may collect the secret.
		const passwordInputs = await $$('input[type="password"]')
		if (passwordInputs.length > 0) {
			throw new Error(`connect screen renders ${passwordInputs.length} password input(s); the passphrase must be entered on the device`)
		}

		const legacyField = await $$('#trezor-passphrase')
		if (legacyField.length > 0) {
			throw new Error('the removed "Passphrase (optional)" field is back on the connect screen')
		}

		// The screen still has to be usable — a blank Trezor branch would pass the checks above.
		const connectButton = await $('button[data-testid="e2e-connect-with-words"]')
		await connectButton.waitForDisplayed({ timeout: 30000 })

		// And it has to say where the passphrase *is* entered, as text rather than a rival CTA.
		const passphraseBlock = await $('[data-testid="e2e-passphrase-on-device"]')
		await passphraseBlock.waitForDisplayed({ timeout: 30000 })
		const buttonsInBlock = await passphraseBlock.$$('button')
		if (buttonsInBlock.length > 0) {
			throw new Error('the passphrase block renders a button; connecting is the only action on this screen')
		}
	})
})
