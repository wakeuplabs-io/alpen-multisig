import type { WalletVendor } from '@/wallet/types'

/**
 * Signer-facing copy tailored to the vendor actually connected, so no screen ever names a
 * device the signer does not have in hand (feedback 2026-07-01 #420/#421/#426).
 *
 * Every string here is presentation only — what a device *displays* for a message it signs
 * lives in `deviceSigningDisplay` (`@/lib/device-signing-display`) and is not duplicated.
 */
export type DeviceCopy = {
	/** Human name of the signer ('Trezor', 'Ledger', 'Software Wallet', 'Mock'). */
	label: string
	/** Whether there is a physical device screen to confirm on. */
	isHardware: boolean
	/** Subtitle above a payload awaiting approval, before anything is submitted. */
	reviewPrompt: string
	/** What the signer compares on the device screen — or why there is nothing to compare. */
	verifyHint: string
	/** Guidance while the app asks the signer to verify a derivation path / public key. */
	verifyOnDeviceHint: string
	/** What the signer is asked to approve when broadcasting the commit transaction. */
	broadcastHint: string
	/**
	 * Shown while the app is connecting, for vendors that ask for a passphrase on the device
	 * (#448). Ledger unlocks a passphrase wallet by PIN before the app sees anything, and
	 * software signers have no device at all.
	 *
	 * It belongs to the connecting state, not to the idle screen. That is the moment the
	 * device is holding up its keypad while the app can only say "Detecting device…", and the
	 * moment the signer is looking at the wrong screen waiting for something to happen. Before
	 * connecting there is nothing to act on, and the screen already promises twice over that
	 * no password is typed here.
	 */
	passphraseOnDevice?: string
	/**
	 * Label for the second connect action, on vendors where one seed backs more than one
	 * wallet (#448). Present only for Trezor: a Ledger has nothing for the host to select,
	 * and software signers take their passphrase in the mnemonic form.
	 *
	 * Names the device, because that is the whole point of the action — the passphrase is
	 * typed there and never here.
	 */
	hiddenWallet?: string
}

const DEVICE_COPY: Record<WalletVendor, DeviceCopy> = {
	trezor: {
		label: 'Trezor',
		isHardware: true,
		reviewPrompt: 'Review the payload, then approve it on your Trezor. Nothing is submitted until you approve there.',
		verifyHint:
			'Your Trezor renders the message text it signs. Compare that text on the device, character for character, before approving.',
		verifyOnDeviceHint:
			'Check your Trezor screen: it displays the address derived from the selected path — hardware signers cannot render a raw public key, so this address is what proves the key matches.',
		broadcastHint:
			'Broadcast is not the same as the login “Sign message”: your Trezor shows the commit transaction itself. Confirm every screen — inputs, outputs and fee — until the device is done.',
		passphraseOnDevice: 'If your Trezor asks for your passphrase, enter it on the device.',
		hiddenWallet: 'Enter passphrase on Trezor',
	},
	ledger: {
		label: 'Ledger',
		isHardware: true,
		reviewPrompt: 'Review the payload, then approve it on your Ledger. Nothing is submitted until you approve there.',
		verifyHint:
			'Depending on the model and Bitcoin app version, your Ledger renders either the message text or its SHA-256 “Message hash”. Compare whichever one appears on the device before approving.',
		verifyOnDeviceHint:
			'Check your Ledger screen: it displays the address derived from the selected path — hardware signers cannot render a raw public key, so this address is what proves the key matches.',
		broadcastHint:
			'Broadcast is not the same as the login “Sign message”: your Ledger first registers the Admin Wallet policy, then shows the commit transaction. Press right on every screen until the app is done.',
	},
	mnemonic: {
		label: 'Software Wallet',
		isHardware: false,
		reviewPrompt: 'Review the payload, then sign it with your software wallet. Nothing is submitted until you sign.',
		verifyHint:
			'Your software wallet signs the message locally — there is no device screen to compare the message against.',
		verifyOnDeviceHint:
			'Your software wallet has no device screen — the Admin ID shown here is the public key it will use.',
		broadcastHint:
			'The commit transaction is signed locally by your software wallet — no device confirmation is asked.',
	},
	mock: {
		label: 'Mock',
		isHardware: false,
		reviewPrompt: 'Review the payload, then sign it with the mock signer. Nothing is submitted until you sign.',
		verifyHint: 'The mock signer signs locally — there is no device screen to compare the message against.',
		verifyOnDeviceHint:
			'The mock signer has no device screen — the path and public key shown here are the ones it will use.',
		broadcastHint: 'The commit transaction is signed locally by the mock signer — no device confirmation is asked.',
	},
}

export function deviceCopy(vendor: WalletVendor): DeviceCopy {
	return DEVICE_COPY[vendor]
}
