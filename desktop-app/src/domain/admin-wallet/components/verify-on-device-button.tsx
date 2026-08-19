import type { HwDeviceType, VerifyScriptType } from '../model/hw-device'
import { ShieldCheckMutedIcon, CheckEmeraldIcon, AlertTriangleIcon } from '@/assets/icons'
import { useVerifyOnDevice } from '../hooks/use-verify-on-device'
import { verifyOnDeviceAppearance, verifyOnDeviceClassName } from '../model/verify-on-device-appearance'
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
	/** Idle label. Defaults to the compact wording used beside a receive address. */
	label?: string
	/**
	 * `chip` is the small outlined affordance that sits next to an address. `primary`
	 * is the solid button the certificate wireframes give Step 2, where verifying on the
	 * device is half the point of the screen rather than an optional extra.
	 */
	variant?: 'chip' | 'primary'
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
	label = 'Verify on device',
	variant = 'chip',
}: VerifyOnDeviceButtonProps) {
	const { state, verify } = useVerifyOnDevice({ deviceType, network, expectedAddress })

	function handleVerify() {
		void verify(derivationPath, scriptType)
	}

	// A confirmed verification ends green, like the certificate's `Signed` chip — but it stays a
	// button, because a signer may legitimately want to check again. See the model for the reasoning.
	const appearance = verifyOnDeviceAppearance(state, label, `Confirm on your ${deviceCopy(deviceType).label}…`)

	return (
		<div className="mt-2">
			<button
				type="button"
				onClick={handleVerify}
				disabled={appearance.isBusy}
				aria-busy={appearance.isBusy}
				data-testid="e2e-wallet-verify-on-device"
				className={verifyOnDeviceClassName(state, variant)}
			>
				{appearance.icon === 'check' ? (
					<CheckEmeraldIcon width={12} height={12} />
				) : (
					<ShieldCheckMutedIcon width={12} height={12} />
				)}
				{appearance.label}
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
					className="mt-1.5 rounded-lg border border-danger-border bg-danger-surface px-3 py-2"
					data-testid="e2e-wallet-verify-on-device-mismatch"
				>
					<p className="inline-flex items-start gap-1.5 text-[11px] font-medium leading-[1.45] text-danger-strong">
						<AlertTriangleIcon width={12} height={12} className="mt-px shrink-0 text-danger" />
						<span>
							Your {deviceCopy(deviceType).label} showed a different {subject}. Do not use this signer until you find
							out why.
						</span>
					</p>
					<p className="mt-1.5 break-all font-mono text-[11px] leading-[1.45] text-danger-deep">{state.address}</p>
					<p className="mt-1 text-[11px] leading-[1.45] text-[#9ca3af]">
						On a Ledger this also happens when the wrong Bitcoin app is open (mainnet vs testnet) — check that first.
					</p>
				</div>
			)}

			{state.status === 'failed' && (
				<p
					aria-live="polite"
					className="mt-1.5 inline-flex items-start gap-1.5 text-[11px] leading-[1.45] text-danger-strong"
				>
					<AlertTriangleIcon width={12} height={12} className="mt-px shrink-0 text-danger" />
					<span>
						Could not verify the {subject}: {state.message}
					</span>
				</p>
			)}
		</div>
	)
}
