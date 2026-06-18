import type { HwDeviceType, VerifyScriptType } from '../model/hw-device'
import { ShieldCheckMutedIcon, CheckEmeraldIcon, AlertTriangleIcon } from '@/assets/icons'
import { useVerifyOnDevice } from '../hooks/use-verify-on-device'

export type VerifyOnDeviceButtonProps = {
	deviceType: HwDeviceType
	network: string
	derivationPath: string
	scriptType: VerifyScriptType
	/** Short label of what is being verified (e.g. 'receive address', 'Admin ID'). */
	subject: string
}

const DEVICE_LABEL: Record<HwDeviceType, string> = {
	trezor: 'Trezor',
	ledger: 'Ledger',
}

/**
 * Verify-on-device affordance (Phase 8, PRD §4.2 / §4.3.4.2): asks the connected
 * device to display the address so the signer can compare it on the device screen.
 * Surfaces verifying / confirmed / failed states; never renders signing material.
 */
export function VerifyOnDeviceButton({
	deviceType,
	network,
	derivationPath,
	scriptType,
	subject,
}: VerifyOnDeviceButtonProps) {
	const { state, verify } = useVerifyOnDevice({ deviceType, network })
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
				{isVerifying ? `Confirm on your ${DEVICE_LABEL[deviceType]}…` : 'Verify on device'}
			</button>

			{state.status === 'verified' && (
				<p
					aria-live="polite"
					className="mt-1.5 inline-flex items-start gap-1.5 text-[11px] leading-[1.45] text-[#059669]"
				>
					<CheckEmeraldIcon width={12} height={12} className="mt-px shrink-0" />
					<span>
						Confirmed the {subject} on your {DEVICE_LABEL[deviceType]}.
					</span>
				</p>
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
