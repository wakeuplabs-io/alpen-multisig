import { useState } from 'react'
import { Navigate, useNavigate } from 'react-router-dom'
import { LogOutMutedIcon, ShieldPurpleIcon } from '@/assets/icons'
import { SignProposalView } from '@/domain/sign-proposal/components/sign-proposal-view'
import { useSession } from '@/hooks/use-session'
import { ScreenShell } from '@/screens/screen-shell'
import { AuthRole } from '@/types/auth-role'
import type { SignSighashResult } from '@/wallet/types'

// TODO: remove the mock data
export function SignPocScreen() {
	const navigate = useNavigate()
	const { wallet, adapter, selectedRole, sessionTimeLabel, disconnectSession } = useSession()
	const [isSigning, setIsSigning] = useState(false)
	const [signError, setSignError] = useState<string | null>(null)
	const [signResult, setSignResult] = useState<SignSighashResult | null>(null)
	const [copyFeedbackVisible, setCopyFeedbackVisible] = useState(false)

	if (wallet === null) {
		return <Navigate to="/" replace />
	}

	const authorityLabel =
		selectedRole === AuthRole.StrataAdministrator ? 'Alpen Administrator' : 'Alpen Sequencer Manager'

	const signerLabel = wallet.addressSample
		? `${wallet.addressSample.slice(0, 5)}...${wallet.addressSample.slice(-6)}`
		: 'Unknown'

	const sighashHex = 'a1b2c3d4e5f60789129ab34c5d6e7f89a1b2c3d4e5f60789129ab34c5d6e7f89'

	async function handleBack() {
		await disconnectSession()
	}

	async function handleSignWithHw() {
		setIsSigning(true)
		setSignError(null)
		setSignResult(null)
		try {
			const result = await adapter.signSighash(sighashHex)
			setSignResult(result)
		} catch (e) {
			setSignError(String(e))
		} finally {
			setIsSigning(false)
		}
	}

	async function handleCopySighash() {
		try {
			await navigator.clipboard.writeText(sighashHex)
			setCopyFeedbackVisible(true)
			setTimeout(() => setCopyFeedbackVisible(false), 450)
		} catch (error) {
			setSignError(`Unable to copy sighash: ${String(error)}`)
		}
	}

	return (
		<ScreenShell
			headerContent={
				<>
					<span className="inline-flex items-center gap-1.5 rounded-md border border-[#e4dfff] bg-[#f5f3ff] px-2.5 py-1.25 text-[12px] font-medium text-[#7c6fcd]">
						<ShieldPurpleIcon width={12} height={12} className="block shrink-0" />
						{authorityLabel}
					</span>
					<span className="inline-flex items-center gap-2 rounded-full border border-[#e5e7eb] bg-[#f8f8fb] px-3 py-1.25 text-[12px]">
						<span className="font-mono text-[11px] font-medium text-[#111827]">Session · {sessionTimeLabel}</span>
						<span className="h-3 w-px bg-[#e5e7eb]" aria-hidden="true" />
						<span className="font-mono text-[11px] text-[#6b7280]">{signerLabel}</span>
					</span>
					<button
						type="button"
						className="inline-flex items-center gap-1.5 rounded-lg border border-[#e5e7eb] bg-white px-2.5 py-1.25 text-[12px] font-medium text-[#6b7280] transition hover:border-[#fca5a5] hover:bg-[#fef2f2] hover:text-[#b91c1c]"
						onClick={() => void handleBack()}
					>
						<LogOutMutedIcon width={12} height={12} className="block shrink-0" />
						Disconnect
					</button>
				</>
			}
		>
			<div className="mx-auto w-full max-w-[760px]">
				<button
					type="button"
					className="inline-flex items-center gap-1.5 text-sm text-[#6b7280] transition hover:text-[#111827]"
					onClick={() => navigate('/proposals')}
				>
					← Back
				</button>

				<h1 className="m-0 mt-3 font-['BIZ_UDPMincho'] text-[44px] leading-[1.05] tracking-[-0.01em] text-[#0a0a0a]">
					Sign proposal
				</h1>
				<p className="m-0 mt-1 text-[13px] text-[#6b7280]">
					Review the payload, then confirm on your Trezor. Nothing is sent until you sign.
				</p>

				<div className="mt-5">
					<SignProposalView
						authorityLabel={authorityLabel}
						proposalIdLabel="#43"
						proposalTypeLabel="Verification key update"
						proposalTitle="Rotate verification key (Q2 2026)"
						beforeValue="0x04e1b2c3d4e5f60789129ab34c5d6e7f89a1b2c3d4e5f60789129ab34c5d6e7f"
						afterValue="0x09f8e7d6c5b4a3921807f6e5d4c3b2a19080706f5e4d3c2b1a0980706f5e4d3c"
						sighashHex={sighashHex}
						signResult={signResult}
						isSigning={isSigning}
						error={signError}
						copyFeedbackVisible={copyFeedbackVisible}
						onCopySighash={() => void handleCopySighash()}
						onSign={() => void handleSignWithHw()}
					/>
				</div>
			</div>
		</ScreenShell>
	)
}
