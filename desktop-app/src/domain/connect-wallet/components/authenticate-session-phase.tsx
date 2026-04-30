type Props = {
	authorityLabel: string
	signerAddress: string
	isAuthenticating: boolean
	authError: string | null
	authOkMessage: string | null
	onBackToAuthority: () => void
	onAuthenticate: () => void
}

export function AuthenticateSessionPhase({
	authorityLabel,
	signerAddress,
	isAuthenticating,
	authError,
	authOkMessage,
	onBackToAuthority,
	onAuthenticate,
}: Props) {
	const authenticateButtonClassName = isAuthenticating
		? 'rounded-lg border border-[#0a0a0a] bg-[#a3a3a3] px-5 py-2 text-sm font-medium text-white'
		: 'rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-5 py-2 text-sm font-medium text-white transition hover:bg-[#2a2a2a]'

	return (
		<div className="w-full max-w-[760px] pb-28">
			<div className="mb-3 flex items-center justify-between">
				<button
					type="button"
					className="inline-flex items-center gap-1 text-sm text-[#666] transition hover:text-[#0a0a0a]"
					onClick={onBackToAuthority}
				>
					<span aria-hidden="true">←</span>
					Back
				</button>
				<p className="m-0 text-[0.68rem] font-medium uppercase tracking-[0.22em] text-[#9ca3af]">Step 4 of 4</p>
			</div>

			<h1 className="m-0 font-['BIZ_UDPMincho'] text-[2.15rem] font-normal leading-[1.1] tracking-[-0.01em] text-[#0a0a0a]">
				Authenticate session
			</h1>
			<p className="mb-0 mt-3 text-[0.88rem] leading-[1.55] text-[#6b7280]">
				Your Trezor will sign a structured challenge to prove control of this address. No password, no biometrics.
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

			{authOkMessage && <p className="mt-4 text-[0.85rem] text-[#166534]">{authOkMessage}</p>}
			{authError && <p className="mt-3 text-[0.85rem] text-[#b91c1c]">{authError}</p>}

			<div className="fixed inset-x-0 bottom-0 z-20 border-t border-[#e5e7eb] bg-white/95 px-4 py-3 backdrop-blur-sm">
				<div className="mx-auto flex w-full max-w-[1000px] items-center justify-end gap-2">
					<button
						type="button"
						className={authenticateButtonClassName}
						onClick={onAuthenticate}
						disabled={isAuthenticating}
					>
						{isAuthenticating ? 'Authenticating...' : 'Authenticate with Trezor'}
					</button>
				</div>
			</div>
		</div>
	)
}
