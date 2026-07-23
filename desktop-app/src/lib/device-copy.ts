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
	/** What to compare on the signer's screen instead of the sighash. */
	verifyHint: string
	/** Guidance while the app asks the signer to verify a derivation path / public key. */
	verifyOnDeviceHint: string
	/** What the signer is asked to approve when broadcasting the commit transaction. */
	broadcastHint: string
}

const DEVICE_COPY: Record<WalletVendor, DeviceCopy> = {
	trezor: {
		label: 'Trezor',
		isHardware: true,
		reviewPrompt: 'Review the payload, then approve it on your Trezor. Nothing is submitted until you approve there.',
		verifyHint:
			'Your Trezor shows the message it signs, not this sighash — it renders the message text. Compare that text on the device before approving.',
		verifyOnDeviceHint: 'Check your Trezor screen and confirm the selected path and public key shown by the device.',
		broadcastHint:
			'Broadcast is not the same as the login “Sign message”: your Trezor shows the commit transaction itself. Confirm every screen — inputs, outputs and fee — until the device is done.',
	},
	ledger: {
		label: 'Ledger',
		isHardware: true,
		reviewPrompt: 'Review the payload, then approve it on your Ledger. Nothing is submitted until you approve there.',
		verifyHint:
			'Your Ledger shows the message it signs, not this sighash — it renders the SHA-256 “Message hash”. Compare that value on the device before approving.',
		verifyOnDeviceHint: 'Check your Ledger screen and confirm the selected path and public key shown by the device.',
		broadcastHint:
			'Broadcast is not the same as the login “Sign message”: your Ledger first registers the Admin Wallet policy, then shows the commit transaction. Press right on every screen until the app is done.',
	},
	mnemonic: {
		label: 'Software Wallet',
		isHardware: false,
		reviewPrompt: 'Review the payload, then sign it with your software wallet. Nothing is submitted until you sign.',
		verifyHint:
			'Your software wallet signs the message locally — there is no device screen to compare this sighash against.',
		verifyOnDeviceHint:
			'Your software wallet has no device screen — the path and public key shown here are the ones it will use.',
		broadcastHint:
			'The commit transaction is signed locally by your software wallet — no device confirmation is asked.',
	},
	mock: {
		label: 'Mock',
		isHardware: false,
		reviewPrompt: 'Review the payload, then sign it with the mock signer. Nothing is submitted until you sign.',
		verifyHint: 'The mock signer signs locally — there is no device screen to compare this sighash against.',
		verifyOnDeviceHint:
			'The mock signer has no device screen — the path and public key shown here are the ones it will use.',
		broadcastHint: 'The commit transaction is signed locally by the mock signer — no device confirmation is asked.',
	},
}

export function deviceCopy(vendor: WalletVendor): DeviceCopy {
	return DEVICE_COPY[vendor]
}
