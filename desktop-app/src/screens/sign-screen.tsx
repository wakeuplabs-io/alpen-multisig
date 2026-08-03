import { useEffect, useMemo, useState } from 'react'
import { PendingExpiryCountdown } from '@/components/pending-expiry-countdown'
import { Navigate, useNavigate, useParams } from 'react-router-dom'
import { authorityFromRole, orchestratorAuthGetSession, getOrchestratorBaseUrl } from '@/api/orchestrator-auth'
import { authorityLabelForRole } from '@/lib/authority-label'
import { inferProposalTypeLabel } from '@/lib/proposal-type-label'
import { approveProposal, getProposalByActionId, type Proposal } from '@/api/proposals'
import { computeSighash, decodeActionHex, type DecodedAction } from '@/api/signing'
import { ShieldAccentIcon } from '@/assets/icons'
import { assertWalletPubkeyBinding } from '@/domain/sign-proposal/wallet-binding'
import { SignProposalView } from '@/domain/sign-proposal/components/sign-proposal-view'
import { useDeviceSigningMessage } from '@/hooks/use-device-signing-message'
import { useCurrentThreshold } from '@/domain/sign-proposal/hooks/use-current-threshold'
import { deviceSigningDisplay } from '@/lib/device-signing-display'
import { deviceCopy } from '@/lib/device-copy'
import { useSession } from '@/hooks/use-session'
import { Breadcrumbs } from '@/components/breadcrumbs'
import { DisconnectButton } from '@/components/disconnect-button'
import { ScreenShell } from '@/screens/screen-shell'
import type { SignSighashResult } from '@/wallet/types'
import { useWalletPanelData } from '@/domain/admin-wallet/hooks/use-wallet-panel-data'
import { WalletSessionControl } from '@/domain/admin-wallet/components/wallet-session-control'

export function SignScreen() {
	const navigate = useNavigate()
	const { actionId } = useParams<{ actionId: string }>()
	const {
		wallet,
		adapter,
		selectedRole,
		sessionTimeLabel,
		sessionWarning,
		disconnectSession,
		ensureOrchestratorSession,
	} = useSession()
	const panel = useWalletPanelData()
	const [isSigning, setIsSigning] = useState(false)
	const [isLoading, setIsLoading] = useState(true)
	const [proposal, setProposal] = useState<Proposal | null>(null)
	const [sighashHex, setSighashHex] = useState('')
	const [loadError, setLoadError] = useState<string | null>(null)
	const [signError, setSignError] = useState<string | null>(null)
	const [signResult, setSignResult] = useState<SignSighashResult | null>(null)
	const [signerPubkey, setSignerPubkey] = useState<string | null>(null)
	const [decodedAction, setDecodedAction] = useState<DecodedAction | null>(null)
	const [showQuorumPrompt, setShowQuorumPrompt] = useState(false)

	const authorityLabel = authorityLabelForRole(selectedRole)

	const signerAlreadySigned =
		proposal !== null &&
		signerPubkey !== null &&
		proposal.signatures.some((signature) => signature.signerPubkey.toLowerCase() === signerPubkey.toLowerCase())
	const canSign = proposal?.status === 'pending' && !signerAlreadySigned
	const proposalTypeLabel = useMemo(() => {
		if (proposal === null) return 'Proposal update'
		return inferProposalTypeLabel(proposal)
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

	const { message: deviceMessage, messageHash: deviceMessageHash } = useDeviceSigningMessage(
		proposal?.seqNo ?? null,
		decodedActionHex || null,
	)
	const currentThreshold = useCurrentThreshold(
		authorityFromRole(selectedRole),
		decodedAction?.kind === 'multisig_update',
	)
	const deviceDisplay = deviceSigningDisplay(adapter.vendor, {
		message: deviceMessage,
		messageHash: deviceMessageHash,
	})

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
					baseUrl: getOrchestratorBaseUrl(),
					actionId,
				})
				if (!proposalResult.ok) {
					throw new Error(proposalResult.error)
				}

				const nextProposal = proposalResult.data
				if (nextProposal.authority !== authorityFromRole(selectedRole)) {
					setLoadError(
						'This proposal belongs to a different authority than your current role. Go back and select the correct role.',
					)
					setIsLoading(false)
					return
				}
				const sighashResult = await computeSighash(nextProposal.seqNo, nextProposal.actionHex)
				if (!sighashResult.ok) {
					throw new Error(sighashResult.error)
				}

				if (!mounted) {
					return
				}

				setProposal(nextProposal)
				setSighashHex(sighashResult.data.sighashHex)

				const decodedResult = await decodeActionHex(nextProposal.actionHex)
				setDecodedAction(
					decodedResult.ok
						? decodedResult.data
						: { kind: 'unknown', rawHex: nextProposal.actionHex.replace(/^0x/i, '') },
				)
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
	}, [actionId, ensureOrchestratorSession, selectedRole])

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
			const signed = await adapter.signSighash(sighashHex, {
				seqno: proposal.seqNo,
				actionHex: decodedActionHex,
			})
			assertWalletPubkeyBinding(signerPubkey, signed.publicKeyHex)
			const approved = await approveProposal({
				baseUrl: getOrchestratorBaseUrl(),
				actionId: proposal.actionId,
				signerPubkey,
				signatureHex: signed.signatureHex,
			})
			if (!approved.ok) {
				throw new Error(approved.error)
			}
			setSignResult(signed)
			const quorumReached =
				approved.data.status === 'approved' || approved.data.signatures.length >= approved.data.requiredSignatures
			if (quorumReached) {
				setShowQuorumPrompt(true)
			} else {
				navigate('/proposals')
			}
		} catch (e) {
			setSignError(String(e))
		} finally {
			setIsSigning(false)
		}
	}

	if (wallet === null) {
		return <Navigate to="/" replace />
	}

	return (
		<ScreenShell
			authorityBadge={
				<span className="inline-flex items-center gap-1.5 rounded-md border border-accent-border bg-bg-surface px-2.5 py-1.25 text-label font-medium text-accent-hover">
					<ShieldAccentIcon width={12} height={12} className="block shrink-0" />
					{authorityLabel}
				</span>
			}
			headerContent={
				<>
					<WalletSessionControl
						panel={panel}
						sessionTimeLabel={sessionTimeLabel}
						sessionWarning={sessionWarning}
						adminId={wallet.publicKeyHex}
						adminIdAddress={wallet.addressSample}
					/>
					<DisconnectButton onClick={() => void handleBack()} />
				</>
			}
		>
			<div className="mx-auto w-full max-w-190">
				<Breadcrumbs />

				<h1 className="m-0 mt-3 font-display text-[44px] leading-[1.05] tracking-[-0.01em] text-[#0a0a0a]">
					Sign proposal
				</h1>
				<p className="m-0 mt-1 text-body-sm text-[#6b7280]">{deviceCopy(adapter.vendor).reviewPrompt}</p>

				{isLoading ? (
					<div className="mt-5 rounded-xl border border-[#e5e7eb] bg-white px-4 py-3 text-body text-[#6b7280]">
						Loading proposal...
					</div>
				) : null}

				{loadError ? (
					<div className="mt-5 rounded-xl border border-danger-border bg-danger-surface px-4 py-3">
						<p className="m-0 text-body text-danger-deep">{loadError}</p>
						<button
							type="button"
							className="mt-3 inline-flex items-center rounded-md border border-danger-deep bg-white px-3 py-1.5 text-label font-medium text-danger-deep transition hover:bg-danger-surface"
							onClick={() => navigate('/proposals')}
						>
							Back to proposals
						</button>
					</div>
				) : null}

				{!isLoading && proposal !== null && proposal.status !== 'pending' ? (
					<div className="mt-5 rounded-xl border border-danger-border bg-danger-surface px-4 py-3">
						<p className="m-0 text-body font-medium text-danger-deep">
							This proposal is no longer pending and cannot be signed.
						</p>
					</div>
				) : null}

				{!isLoading && signerAlreadySigned ? (
					<div className="mt-5 rounded-xl border border-accent-border bg-highlight-surface px-4 py-3">
						<p className="m-0 text-body font-medium text-emphasis">
							You already signed this proposal. Additional signatures from the same signer are not allowed.
						</p>
					</div>
				) : null}

				{!isLoading && proposal !== null && proposal.status === 'pending' && (
					<div className="mt-4 rounded-xl border border-accent-border bg-highlight-surface px-4 py-2.5">
						<PendingExpiryCountdown expiresAtMs={proposal.expiresAtMs} />
					</div>
				)}

				{showQuorumPrompt && actionId !== undefined && (
					<div
						className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 px-4"
						onClick={(e) => {
							if (e.target === e.currentTarget) {
								setShowQuorumPrompt(false)
								navigate('/proposals')
							}
						}}
					>
						<div className="w-full max-w-120 rounded-2xl border border-[#e5e7eb] bg-white p-6 shadow-xl">
							<h2 className="m-0 font-display text-display-sm font-normal text-[#0a0a0a]">Quorum reached</h2>
							<p className="m-0 mt-2 text-body text-[#6b7280]">
								This proposal now has enough signatures. Do you want to broadcast the Bitcoin transaction now?
							</p>
							<div className="mt-5 flex gap-3">
								<button
									type="button"
									className="flex-1 rounded-xl border border-[#111827] bg-[#111827] px-4 py-2.5 text-body font-medium text-white transition hover:bg-black"
									onClick={() => navigate(`/proposals/${actionId}/broadcast`)}
								>
									Broadcast now
								</button>
								<button
									type="button"
									className="flex-1 rounded-xl border border-[#e5e7eb] bg-white px-4 py-2.5 text-body font-medium text-[#374151] transition hover:border-[#d1d5db] hover:bg-[#f9fafb]"
									onClick={() => {
										setShowQuorumPrompt(false)
										navigate('/proposals')
									}}
								>
									Later
								</button>
							</div>
						</div>
					</div>
				)}

				{!isLoading && proposal !== null && loadError === null ? (
					<div className="mt-5">
						<SignProposalView
							authorityLabel={authorityLabel}
							proposalIdLabel={`#${proposal.seqNo}`}
							proposalTypeLabel={proposalTypeLabel}
							proposalTitle={proposalTitle}
							decodedAction={decodedAction}
							currentThreshold={currentThreshold}
							deviceDisplay={deviceDisplay}
							signResult={signResult}
							isSigning={isSigning}
							error={signError}
							walletVendor={adapter.vendor}
							onSign={() => void handleSignWithHw()}
						/>
					</div>
				) : null}
			</div>
		</ScreenShell>
	)
}
