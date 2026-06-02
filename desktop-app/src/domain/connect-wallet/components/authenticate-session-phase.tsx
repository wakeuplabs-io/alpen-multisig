import type { SigningStepInfo } from '@/contexts/session-context'

type Props = {
	authorityLabel: string
	adapterLabel: string
	signerAddress: string
	compressedPublicKey: string
	isAuthenticating: boolean
	authError: string | null
	authOkMessage: string | null
	signingStep: SigningStepInfo | null
	onBackToAuthority: () => void
	onAuthenticate: () => void
}

export function AuthenticateSessionPhase({
	authorityLabel,
	adapterLabel,
	signerAddress,
	compressedPublicKey,
	isAuthenticating,
	authError,
	authOkMessage,
	signingStep,
	onBackToAuthority,
	onAuthenticate,
}: Props) {
	const authenticateButtonClassName = isAuthenticating
		? 'inline-flex items-center justify-center rounded-lg border border-[#0a0a0a] bg-[#a3a3a3] px-5 py-2 text-sm font-medium text-white'
		: 'inline-flex items-center justify-center rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-5 py-2 text-sm font-medium text-white transition hover:bg-[#2a2a2a]'

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
					className="inline-flex items-center gap-1 text-sm text-[#666] transition hover:text-[#0a0a0a]"
					onClick={onBackToAuthority}
				>
					<span aria-hidden="true">←</span>
					Back
				</button>
				<p className="m-0 w-22 text-right text-[0.68rem] font-medium uppercase tracking-[0.22em] tabular-nums text-[#9ca3af]">
					Step 3 of 3
				</p>
			</div>

			<h1 className="m-0 font-['BIZ_UDPMincho'] text-[2.15rem] font-normal leading-[1.1] tracking-[-0.01em] text-[#0a0a0a]">
				Authenticate session
			</h1>
			<p className="mb-0 mt-3 text-[0.88rem] leading-[1.55] text-[#6b7280]">
				Your {adapterLabel} will sign an authentication challenge to prove control of this address. This requires{' '}
				<strong className="font-medium text-[#374151]">2 signatures</strong>: one for the on-chain session and one for
				the coordination backend.
			</p>

			<div className="mt-5 rounded-xl border border-[#e5e7eb] bg-white px-5 py-4">
				<p className="m-0 text-[0.65rem] font-medium uppercase tracking-[0.14em] text-[#9ca3af]">
					You are about to sign
				</p>
				<div className="mt-3 grid gap-3">
					<div>
						<p className="m-0 text-xs text-[#9ca3af]">Authority</p>
						<p className="m-0 mt-1 text-sm font-medium text-[#111827]">{authorityLabel}</p>
					</div>
					<div>
						<p className="m-0 text-xs text-[#9ca3af]">Signer address</p>
						<p className="m-0 mt-1 font-mono text-xs text-[#111827]">{signerAddress}</p>
					</div>
					<div>
						<p className="m-0 text-xs text-[#9ca3af]">Compressed public key</p>
						<p className="m-0 mt-1 break-all font-mono text-xs text-[#111827]">{compressedPublicKey}</p>
					</div>
					<div className="grid grid-cols-2 gap-3">
						<div>
							<p className="m-0 text-xs text-[#9ca3af]">Session expiry</p>
							<p className="m-0 mt-1 text-sm font-medium text-[#111827]">30 minutes</p>
						</div>
						<div>
							<p className="m-0 text-xs text-[#9ca3af]">Challenge nonce</p>
							<p className="m-0 mt-1 font-mono text-xs text-[#111827]">Generated per request</p>
						</div>
					</div>
				</div>
			</div>

			{signingStep && (
				<div className="mt-4 rounded-lg border border-[#d1fae5] bg-[#f0fdf4] px-4 py-3">
					<p className="m-0 text-[0.65rem] font-medium uppercase tracking-[0.14em] text-[#059669]">
						Signing on device — step {signingStep.step} of {signingStep.totalSteps}
					</p>
					<p className="m-0 mt-1 text-[0.75rem] text-[#6b7280]">
						Confirm this hash on your {adapterLabel} — it must match exactly:
					</p>
					<p className="m-0 mt-2 break-all font-mono text-[0.72rem] leading-[1.6] text-[#111827]">
						{signingStep.challengeHex}
					</p>
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
		</div>
	)
}
