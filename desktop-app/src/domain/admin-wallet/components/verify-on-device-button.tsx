import type { HwDeviceType, VerifyScriptType } from '../model/hw-device'
import { ShieldCheckMutedIcon, CheckEmeraldIcon, AlertTriangleIcon } from '@/assets/icons'
import { useVerifyOnDevice } from '../hooks/use-verify-on-device'
import { deviceCopy } from '@/lib/device-copy'

export type VerifyOnDeviceButtonProps = {
	deviceType: HwDeviceType
	network: string
	derivationPath: string
	scriptType: VerifyScriptType
	/** Short label of what is being verified (e.g. 'receive address', 'Admin ID'). */
	subject: string
	/**
	 * Address the app expects the device to render. When set, the address the device
	 * actually shows is compared against it and a difference is surfaced as an alarm.
	 */
	expectedAddress?: string
}

/**
 * Verify-on-device affordance (Phase 8, PRD §4.2 / §4.3.4.2): asks the connected
 * device to display the address so the signer can compare it on the device screen.
 * Surfaces verifying / confirmed / mismatch / failed states — a mismatch means the
 * device rendered a different address than the app expected, which is a security
 * alarm, not a transport error. Never renders signing material.
 */
export function VerifyOnDeviceButton({
	deviceType,
	network,
	derivationPath,
	scriptType,
	subject,
	expectedAddress,
}: VerifyOnDeviceButtonProps) {
	const { state, verify } = useVerifyOnDevice({ deviceType, network, expectedAddress })
	const isVerifying = state.status === 'verifying'

	function handleVerify() {
		void verify(derivationPath, scriptType)
	}

	return (
		<div className="mt-2">
			<button
				type="button"
				onClick={handleVerify}
				disabled={isVerifying}
				data-testid="e2e-wallet-verify-on-device"
				className={`inline-flex items-center gap-1.5 rounded-md border px-2.5 py-1 text-[11px] font-medium transition ${
					isVerifying
						? 'cursor-wait border-[#e5e7eb] text-[#9ca3af]'
						: 'border-[#ddd6fe] text-[#7c6cf0] hover:border-[#c4b5fd] hover:bg-[#faf9ff]'
				}`}
			>
				<ShieldCheckMutedIcon width={12} height={12} />
				{isVerifying ? `Confirm on your ${deviceCopy(deviceType).label}…` : 'Verify on device'}
			</button>

			{state.status === 'verified' && (
				<p
					aria-live="polite"
					className="mt-1.5 inline-flex items-start gap-1.5 text-[11px] leading-[1.45] text-[#059669]"
					data-testid="e2e-wallet-verify-on-device-result"
				>
					<CheckEmeraldIcon width={12} height={12} className="mt-px shrink-0" />
					<span>
						Confirmed the {subject} on your {deviceCopy(deviceType).label}.
					</span>
				</p>
			)}

			{state.status === 'mismatch' && (
				<div
					aria-live="assertive"
					className="mt-1.5 rounded-lg border border-[#fecaca] bg-[#fef2f2] px-3 py-2"
					data-testid="e2e-wallet-verify-on-device-mismatch"
				>
					<p className="inline-flex items-start gap-1.5 text-[11px] font-medium leading-[1.45] text-[#b91c1c]">
						<AlertTriangleIcon width={12} height={12} className="mt-px shrink-0 text-[#dc2626]" />
						<span>
							Your {deviceCopy(deviceType).label} showed a different {subject}. Do not use this signer until you find
							out why.
						</span>
					</p>
					<p className="mt-1.5 break-all font-mono text-[11px] leading-[1.45] text-[#7f1d1d]">{state.address}</p>
					<p className="mt-1 text-[11px] leading-[1.45] text-[#9ca3af]">
						On a Ledger this also happens when the wrong Bitcoin app is open (mainnet vs testnet) — check that first.
					</p>
				</div>
			)}

			{state.status === 'failed' && (
				<p
					aria-live="polite"
					className="mt-1.5 inline-flex items-start gap-1.5 text-[11px] leading-[1.45] text-[#b45309]"
				>
					<AlertTriangleIcon width={12} height={12} className="mt-px shrink-0 text-[#d97706]" />
					<span>
						Could not verify the {subject}: {state.message}
					</span>
				</p>
			)}
		</div>
	)
}
