import type { DeviceSigningDisplay } from '@/lib/device-signing-display'

type Props = {
	display: DeviceSigningDisplay
}

/**
 * Shows what the connected hardware device displays for a message signature, so the signer
 * can compare it on-screen. Ledger renders `sha256(message)` ("Message hash"); Trezor renders
 * the message text. The BIP-137 sighash shown elsewhere in the UI never appears on the device,
 * so it must not be presented as the value to confirm. Software signers render nothing.
 */
export function DeviceSigningHint({ display }: Props) {
	if (display.kind === 'none') return null

	return (
		<div className="rounded-lg border border-[#d1fae5] bg-[#f0fdf4] px-4 py-3" data-testid="e2e-device-signing-hint">
			{display.kind === 'hash' ? (
				<>
					<p className="m-0 text-[0.65rem] font-medium uppercase tracking-[0.14em] text-[#059669]">
						Verify on {display.deviceLabel} — Message hash
					</p>
					<p className="m-0 mt-1 text-[0.75rem] text-[#6b7280]">
						Your {display.deviceLabel} shows this hash — it must match exactly (it differs from the sighash above).
					</p>
					<code className="mt-2 block break-all rounded border border-[#bbf7d0] bg-white px-2 py-1.5 font-mono text-mono-sm leading-5 text-[#334155]">
						{display.value}
					</code>
				</>
			) : (
				<>
					<p className="m-0 text-[0.65rem] font-medium uppercase tracking-[0.14em] text-[#059669]">
						Verify on {display.deviceLabel} — Message
					</p>
					<p className="m-0 mt-1 text-[0.75rem] text-[#6b7280]">
						Confirm this message on your {display.deviceLabel} — it must match exactly:
					</p>
					<pre className="m-0 mt-2 whitespace-pre-wrap break-all rounded border border-[#bbf7d0] bg-white px-2 py-1.5 font-mono text-mono-sm leading-5 text-[#334155]">
						{display.value}
					</pre>
				</>
			)}
		</div>
	)
}
