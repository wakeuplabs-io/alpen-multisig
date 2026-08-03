import type { DeviceSigningDisplay } from '@/lib/device-signing-display'

type Props = {
	display: DeviceSigningDisplay
}

const LABEL_CLASS = 'm-0 text-[0.65rem] font-medium uppercase tracking-[0.14em] text-[#059669]'
const HINT_CLASS = 'm-0 mt-1 text-[0.75rem] text-[#6b7280]'
const VALUE_CLASS =
	'm-0 mt-2 block break-all rounded border border-[#bbf7d0] bg-white px-2 py-1.5 font-mono text-mono-sm leading-5 text-[#334155]'

/**
 * Shows what the connected hardware device displays for a message signature, so the signer
 * can compare it on-screen. A Ledger renders either the message text or the SHA-256
 * "Message hash" depending on the model and Bitcoin app version, so both are shown and the
 * signer compares whichever one appears; a Trezor renders the message text. Software signers
 * render nothing.
 */
export function DeviceSigningHint({ display }: Props) {
	if (display.kind === 'none') return null

	return (
		<div className="rounded-lg border border-[#d1fae5] bg-[#f0fdf4] px-4 py-3" data-testid="e2e-device-signing-hint">
			<HintBody display={display} />
		</div>
	)
}

function HintBody({ display }: { display: Exclude<DeviceSigningDisplay, { kind: 'none' }> }) {
	if (display.kind === 'hash-and-text') {
		return (
			<>
				<p className={LABEL_CLASS}>Verify on {display.deviceLabel}</p>
				<p className={HINT_CLASS}>
					Your {display.deviceLabel} shows one of the two values below — the message text or the SHA-256 “Message hash”,
					depending on the model and Bitcoin app version. Whichever one appears must match exactly.
				</p>
				<p className={`${LABEL_CLASS} mt-3`}>Message</p>
				<pre className={`${VALUE_CLASS} whitespace-pre-wrap`}>{display.text}</pre>
				<p className={`${LABEL_CLASS} mt-3`}>Message hash</p>
				<code className={VALUE_CLASS}>{display.hash}</code>
			</>
		)
	}

	return (
		<>
			<p className={LABEL_CLASS}>Verify on {display.deviceLabel} — Message</p>
			<p className={HINT_CLASS}>Confirm this message on your {display.deviceLabel} — it must match exactly:</p>
			<pre className={`${VALUE_CLASS} whitespace-pre-wrap`}>{display.value}</pre>
		</>
	)
}
