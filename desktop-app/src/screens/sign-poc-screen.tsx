import { useEffect, useMemo, useState } from 'react'
import { Navigate, useNavigate, useParams } from 'react-router-dom'
import { orchestratorAuthGetSession, ORCHESTRATOR_BASE_URL } from '@/api/orchestrator-auth'
import { approveProposal, getProposalByActionId, type Proposal } from '@/api/proposals'
import { computeSighash } from '@/api/signing'
import { LogOutMutedIcon, ShieldPurpleIcon } from '@/assets/icons'
import { SignProposalView } from '@/domain/sign-proposal/components/sign-proposal-view'
import { useSession } from '@/hooks/use-session'
import { ScreenShell } from '@/screens/screen-shell'
import { AuthRole } from '@/types/auth-role'
import type { SignSighashResult } from '@/wallet/types'

export function SignPocScreen() {
	const navigate = useNavigate()
	const { actionId } = useParams<{ actionId: string }>()
	const { wallet, adapter, selectedRole, sessionTimeLabel, disconnectSession, ensureOrchestratorSession } = useSession()
	const [isSigning, setIsSigning] = useState(false)
	const [isLoading, setIsLoading] = useState(true)
	const [proposal, setProposal] = useState<Proposal | null>(null)
	const [sighashHex, setSighashHex] = useState('')
	const [loadError, setLoadError] = useState<string | null>(null)
	const [signError, setSignError] = useState<string | null>(null)
	const [signResult, setSignResult] = useState<SignSighashResult | null>(null)
	const [copyFeedbackVisible, setCopyFeedbackVisible] = useState(false)
	const [signerPubkey, setSignerPubkey] = useState<string | null>(null)

	const authorityLabel =
		selectedRole === AuthRole.StrataAdministrator ? 'Alpen Administrator' : 'Alpen Sequencer Manager'

	const signerLabel = wallet?.addressSample
		? `${wallet.addressSample.slice(0, 5)}...${wallet.addressSample.slice(-6)}`
		: 'Unknown'

	const signerAlreadySigned =
		proposal !== null &&
		signerPubkey !== null &&
		proposal.signatures.some((signature) => signature.signerPubkey.toLowerCase() === signerPubkey.toLowerCase())
	const canSign = proposal?.status === 'pending' && !signerAlreadySigned
	const proposalTypeLabel = useMemo(() => {
		if (proposal === null) {
			return 'Proposal update'
		}
		if (proposal.authority.toLowerCase().includes('sequencer')) {
			return 'Sequencer update'
		}
		if (proposal.actionHex.toLowerCase().startsWith('0x01')) {
			return 'Verification key update'
		}
		return 'Signer update'
	}, [proposal])

	const proposalTitle = useMemo(() => {
		if (proposal === null) {
			return 'Proposal'
		}
		return `Proposal #${proposal.seqNo} - ${proposalTypeLabel}`
	}, [proposal, proposalTypeLabel])

	const decodedActionHex = useMemo(() => {
		if (proposal === null) {
			return ''
		}
		return proposal.actionHex.startsWith('0x') ? proposal.actionHex.slice(2) : proposal.actionHex
	}, [proposal])

	const beforeValue = decodedActionHex.length > 0 ? `0x${decodedActionHex.slice(0, 64)}` : 'Unavailable'
	const afterValue = decodedActionHex.length > 64 ? `0x${decodedActionHex.slice(64, 128)}` : 'Unavailable'

	useEffect(() => {
		let mounted = true

		async function loadProposal() {
			if (actionId === undefined) {
				setLoadError('Missing proposal id in route.')
				setIsLoading(false)
				return
			}

			setIsLoading(true)
			setLoadError(null)
			setSignError(null)
			setSignResult(null)

			try {
				await ensureOrchestratorSession()
				const currentSession = await orchestratorAuthGetSession()
				if (!currentSession.ok) {
					throw new Error(currentSession.error)
				}
				setSignerPubkey(currentSession.data?.signerPubkey ?? null)

				const proposalResult = await getProposalByActionId({
					baseUrl: ORCHESTRATOR_BASE_URL,
					actionId,
				})
				if (!proposalResult.ok) {
					throw new Error(proposalResult.error)
				}

				const nextProposal = proposalResult.data
				const sighashResult = await computeSighash(nextProposal.seqNo, nextProposal.actionHex)
				if (!sighashResult.ok) {
					throw new Error(sighashResult.error)
				}

				if (!mounted) {
					return
				}

				setProposal(nextProposal)
				setSighashHex(sighashResult.data.sighashHex)
			} catch (error) {
				if (!mounted) {
					return
				}
				setLoadError(String(error))
			} finally {
				if (mounted) {
					setIsLoading(false)
				}
			}
		}

		void loadProposal()

		return () => {
			mounted = false
		}
	}, [actionId, ensureOrchestratorSession])

	async function handleBack() {
		await disconnectSession()
	}

	async function handleSignWithHw() {
		if (!canSign) {
			setSignError(
				signerAlreadySigned
					? 'You already signed this proposal. Additional signatures from the same signer are not allowed.'
					: 'This proposal is no longer pending and cannot be signed.',
			)
			return
		}
		if (sighashHex.length === 0) {
			setSignError('Sighash is not available yet. Please retry loading this proposal.')
			return
		}
		if (signerPubkey === null) {
			setSignError('No authenticated signer in session. Re-authenticate and try again.')
			return
		}
		setIsSigning(true)
		setSignError(null)
		setSignResult(null)
		try {
			const signed = await adapter.signSighash(sighashHex)
			const approved = await approveProposal({
				baseUrl: ORCHESTRATOR_BASE_URL,
				actionId: proposal.actionId,
				signerPubkey,
				signatureHex: signed.signatureHex,
			})
			if (!approved.ok) {
				throw new Error(approved.error)
			}
			setSignResult(signed)
			navigate('/proposals')
		} catch (e) {
			setSignError(String(e))
		} finally {
			setIsSigning(false)
		}
	}

	async function handleCopySighash() {
		if (sighashHex.length === 0) {
			setSignError('No sighash available to copy.')
			return
		}
		try {
			await navigator.clipboard.writeText(sighashHex)
			setCopyFeedbackVisible(true)
			setTimeout(() => setCopyFeedbackVisible(false), 450)
		} catch (error) {
			setSignError(`Unable to copy sighash: ${String(error)}`)
		}
	}

	if (wallet === null) {
		return <Navigate to="/" replace />
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

				{isLoading ? (
					<div className="mt-5 rounded-xl border border-[#e5e7eb] bg-white px-4 py-3 text-sm text-[#6b7280]">
						Loading proposal...
					</div>
				) : null}

				{loadError ? (
					<div className="mt-5 rounded-xl border border-[#fecaca] bg-[#fef2f2] px-4 py-3">
						<p className="m-0 text-sm text-[#991b1b]">{loadError}</p>
						<button
							type="button"
							className="mt-3 inline-flex items-center rounded-md border border-[#991b1b] bg-white px-3 py-1.5 text-xs font-medium text-[#991b1b] transition hover:bg-[#fef2f2]"
							onClick={() => navigate('/proposals')}
						>
							Back to proposals
						</button>
					</div>
				) : null}

				{!isLoading && proposal !== null && proposal.status !== 'pending' ? (
					<div className="mt-5 rounded-xl border border-[#fecaca] bg-[#fef2f2] px-4 py-3">
						<p className="m-0 text-sm font-medium text-[#991b1b]">
							This proposal is no longer pending and cannot be signed.
						</p>
					</div>
				) : null}

				{!isLoading && signerAlreadySigned ? (
					<div className="mt-5 rounded-xl border border-[#fde68a] bg-[#fffbeb] px-4 py-3">
						<p className="m-0 text-sm font-medium text-[#92400e]">
							You already signed this proposal. Additional signatures from the same signer are not allowed.
						</p>
					</div>
				) : null}

				{!isLoading && proposal !== null && loadError === null ? (
					<div className="mt-5">
						<SignProposalView
							authorityLabel={authorityLabel}
							proposalIdLabel={`#${proposal.seqNo}`}
							proposalTypeLabel={proposalTypeLabel}
							proposalTitle={proposalTitle}
							beforeValue={beforeValue}
							afterValue={afterValue}
							sighashHex={sighashHex}
							signResult={signResult}
							isSigning={isSigning}
							error={signError}
							copyFeedbackVisible={copyFeedbackVisible}
							onCopySighash={() => void handleCopySighash()}
							onSign={() => void handleSignWithHw()}
						/>
					</div>
				) : null}
			</div>
		</ScreenShell>
	)
}
