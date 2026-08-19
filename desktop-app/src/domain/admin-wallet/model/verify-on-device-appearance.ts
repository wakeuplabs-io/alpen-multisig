import type { VerifyOnDeviceState } from '../hooks/use-verify-on-device'

/**
 * How the verify-on-device control looks and reads for a given state.
 *
 * Pulled out of the component because the interesting part is a decision, not markup: a
 * verification that succeeded has to *look* finished — the certificate modal's Step 1 closes on a
 * green `Signed` chip, and a Step 2 that stays identical after confirming reads as unfinished — but
 * unlike signing, verifying is a security check worth repeating, so the control stays a live
 * button rather than becoming a chip. Green is a state, not an ending.
 */
export type VerifyOnDeviceAppearance = {
	/** Visible label, which is also the accessible name — the state must not ride on colour alone. */
	label: string
	/** `check` once confirmed, `shield` otherwise. Same reason: colour is never the only signal. */
	icon: 'shield' | 'check'
	/** Verifying is in flight; the control is disabled while the device is waiting. */
	isBusy: boolean
}

export type VerifyOnDeviceVariant = 'chip' | 'primary'

/**
 * `verified` is the only state that turns green. `mismatch` and `failed` must fall back to the
 * neutral look even when a previous run succeeded: a green "Verified" button sitting above a red
 * mismatch panel contradicts the alarm, which is worse than the inconsistency this change fixes.
 */
export function verifyOnDeviceAppearance(
	state: VerifyOnDeviceState,
	idleLabel: string,
	busyLabel: string,
): VerifyOnDeviceAppearance {
	if (state.status === 'verifying') {
		return { label: busyLabel, icon: 'shield', isBusy: true }
	}
	if (state.status === 'verified') {
		return { label: 'Verified', icon: 'check', isBusy: false }
	}
	return { label: idleLabel, icon: 'shield', isBusy: false }
}

/**
 * Tailwind classes per variant and state. Both variants gain the same success branch: re-checking a
 * receive address on the device is as legitimate as re-checking the Admin ID, so the affordance
 * should not differ between the two surfaces that use this control.
 */
export function verifyOnDeviceClassName(state: VerifyOnDeviceState, variant: VerifyOnDeviceVariant): string {
	const isVerified = state.status === 'verified'
	const isBusy = state.status === 'verifying'

	if (variant === 'primary') {
		const base = 'inline-flex items-center gap-1.5 rounded-lg px-4 py-1.5 text-label font-medium transition'
		if (isBusy) {
			return `${base} cursor-wait bg-[#9ca3af] text-white`
		}
		if (isVerified) {
			return `${base} border border-[#6ee7b7] bg-[#ecfdf5] text-[#065f46] hover:bg-[#d1fae5]`
		}
		return `${base} bg-[#111827] text-white hover:bg-[#374151]`
	}

	const base = 'inline-flex items-center gap-1.5 rounded-md border px-2.5 py-1 text-[11px] font-medium transition'
	if (isBusy) {
		return `${base} cursor-wait border-[#e5e7eb] text-[#9ca3af]`
	}
	if (isVerified) {
		return `${base} border-[#6ee7b7] bg-[#ecfdf5] text-[#065f46] hover:bg-[#d1fae5]`
	}
	return `${base} border-[#ddd6fe] text-[#7c6cf0] hover:border-[#c4b5fd] hover:bg-[#faf9ff]`
}
