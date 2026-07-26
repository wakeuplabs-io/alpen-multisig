import { CopyButton } from '@/components/copy-button'
import { DeviceSigningHint } from '@/components/device-signing-hint'
import { ADMIN_ID_LABEL } from '@/lib/admin-id'
import type { SigningStepInfo } from '@/contexts/session-context'
import { useDeviceMessageDisplay } from '@/hooks/use-device-message-display'
import type { WalletVendor } from '@/wallet/types'

type Props = {
	authorityLabel: string
	adapterLabel: string
	/** The Admin ID: the signer's compressed public key (#408). */
	compressedPublicKey: string
	isAuthenticating: boolean
	authError: string | null
	authOkMessage: string | null
	signingStep: SigningStepInfo | null
	walletVendor: WalletVendor
	onBackToAuthority: () => void
	onAuthenticate: () => void
	onManualProposal: () => void
}

export function AuthenticateSessionPhase({
	authorityLabel,
	adapterLabel,
	compressedPublicKey,
	isAuthenticating,
	authError,
	authOkMessage,
	signingStep,
	walletVendor,
	onBackToAuthority,
	onAuthenticate,
	onManualProposal,
}: Props) {
	const deviceDisplay = useDeviceMessageDisplay(walletVendor, signingStep?.challengeMessage ?? null)
	const authenticateButtonClassName = isAuthenticating
		? 'inline-flex items-center justify-center rounded-lg border border-[#0a0a0a] bg-[#a3a3a3] px-5 py-2 text-body font-medium text-white'
		: 'inline-flex items-center justify-center rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-5 py-2 text-body font-medium text-white transition hover:bg-[#2a2a2a]'

	function getButtonLabel() {
		if (!isAuthenticating) return `Authenticate with ${adapterLabel}`
		if (signingStep) {
			return `Signing authentication (${signingStep.step} of ${signingStep.totalSteps})…`
		}
		return 'Authenticating…'
	}

	return (
		<div className="mx-auto w-full max-w-150">
			<div className="mb-3 flex items-center justify-between">
				<button
					type="button"
					className="inline-flex items-center gap-1 text-body text-[#666] transition hover:text-[#0a0a0a]"
					onClick={onBackToAuthority}
				>
					<span aria-hidden="true">←</span>
					Back
				</button>
				<p className="m-0 w-22 text-right text-[0.68rem] font-medium uppercase tracking-[0.22em] tabular-nums text-[#9ca3af]">
					Step 3 of 3
				</p>
			</div>

			<h1 className="m-0 font-display text-[2.15rem] font-normal leading-[1.1] tracking-[-0.01em] text-[#0a0a0a]">
				Authenticate session
			</h1>
			<p className="mb-0 mt-3 text-[0.88rem] leading-[1.55] text-[#6b7280]">
				Your {adapterLabel} will sign an authentication challenge to prove control of this Admin ID. This requires{' '}
				<strong className="font-medium text-[#374151]">1 signature</strong> for the coordination backend.
			</p>

			<div className="mt-5 rounded-xl border border-[#e5e7eb] bg-white px-5 py-4">
				<p className="m-0 text-[0.65rem] font-medium uppercase tracking-[0.14em] text-[#9ca3af]">
					You are about to sign
				</p>
				<div className="mt-3 grid gap-3">
					<div>
						<p className="m-0 text-label text-[#9ca3af]">Multisig</p>
						<p className="m-0 mt-1 text-body font-medium text-[#111827]">{authorityLabel}</p>
					</div>
					<div>
						<p className="m-0 text-label text-[#9ca3af]">{ADMIN_ID_LABEL}</p>
						<div className="mt-1 flex items-start justify-between gap-2">
							<p
								className="m-0 min-w-0 break-all font-mono text-label text-[#111827]"
								data-testid="e2e-authenticate-admin-id-value"
							>
								{compressedPublicKey}
							</p>
							<CopyButton text={compressedPublicKey} variant="icon" />
						</div>
					</div>
					<div className="grid grid-cols-2 gap-3">
						<div>
							<p className="m-0 text-label text-[#9ca3af]">Session expiry</p>
							<p className="m-0 mt-1 text-body font-medium text-[#111827]">30 minutes</p>
						</div>
						<div>
							<p className="m-0 text-label text-[#9ca3af]">Challenge nonce</p>
							<p className="m-0 mt-1 font-mono text-label text-[#111827]">Generated per request</p>
						</div>
					</div>
				</div>
			</div>

			{signingStep && (
				<div className="mt-4">
					<p className="m-0 mb-2 text-[0.65rem] font-medium uppercase tracking-[0.14em] text-[#059669]">
						Signing on device — step {signingStep.step} of {signingStep.totalSteps}
					</p>
					{deviceDisplay.kind === 'none' ? (
						<div className="rounded-lg border border-[#d1fae5] bg-[#f0fdf4] px-4 py-3">
							<p className="m-0 text-[0.75rem] text-[#6b7280]">
								Confirm this message on your {adapterLabel} — it must match exactly:
							</p>
							<pre className="m-0 mt-2 whitespace-pre-wrap break-all font-mono text-[0.72rem] leading-[1.6] text-[#111827]">
								{signingStep.challengeMessage}
							</pre>
						</div>
					) : (
						<DeviceSigningHint display={deviceDisplay} />
					)}
				</div>
			)}

			{authOkMessage && <p className="mt-4 text-[0.85rem] text-[#166534]">{authOkMessage}</p>}
			{authError && <p className="mt-3 text-[0.85rem] text-[#b91c1c]">{authError}</p>}

			<div className="mt-4">
				<button
					type="button"
					data-testid="e2e-authenticate-submit"
					className={`${authenticateButtonClassName} w-full`}
					onClick={onAuthenticate}
					disabled={isAuthenticating}
				>
					{getButtonLabel()}
				</button>
			</div>

			<div className="mt-3 text-center">
				<button
					type="button"
					className="text-label text-[#9ca3af] transition hover:text-[#6b7280]"
					onClick={onManualProposal}
				>
					Enter proposal manually (offline)
				</button>
			</div>
		</div>
	)
}
