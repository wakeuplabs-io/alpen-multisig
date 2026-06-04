import { Navigate, useLocation, useNavigate, useParams } from 'react-router-dom'
import { ORCHESTRATOR_BASE_URL } from '@/api/orchestrator-auth'
import { approveProposal, reportBroadcastProgress } from '@/api/proposals'
import { LogOutMutedIcon, ShieldPurpleIcon } from '@/assets/icons'
import type { ImportBroadcastState } from '@/domain/proposal-detail/components/import-bundle-modal'
import type { PastedSignature } from '@/domain/proposal-detail/model/pasted-signature'
import { ActivationCountdown } from '@/domain/cancel-proposal/components/activation-countdown'
import { ProposalDetail } from '@/domain/proposal-detail/components/proposal-detail'
import { useDecodedProposal } from '@/domain/proposal-detail/hooks/use-decoded-proposal'
import { useProposalDetail } from '@/domain/proposal-detail/hooks/use-proposal-detail'
import { useBlockHeight } from '@/hooks/use-block-height'
import { useSession } from '@/hooks/use-session'
import { ScreenShell } from '@/screens/screen-shell'
import { authorityLabelForRole } from '@/lib/authority-label'
import { useWalletPanelData } from '@/domain/admin-wallet/hooks/use-wallet-panel-data'
import { WalletSessionControl } from '@/domain/admin-wallet/components/wallet-session-control'

const CANCELABLE_AUTHORITIES = ['alpen_admin', 'strata_admin']

type LocationState = { signerPubkey?: string | null }

export function ProposalDetailScreen() {
	const navigate = useNavigate()
	const location = useLocation()
	const { actionId } = useParams<{ actionId: string }>()
	const { wallet, selectedRole, sessionTimeLabel, sessionWarning, disconnectSession } = useSession()
	const signerPubkey: string | null = (location.state as LocationState)?.signerPubkey ?? null

	const authorityLabel = authorityLabelForRole(selectedRole)
	const panel = useWalletPanelData()

	const { proposal, isLoading, error, reload } = useProposalDetail(ORCHESTRATOR_BASE_URL, actionId ?? '')
	const decodedData = useDecodedProposal(proposal)
	const currentBlockHeight = useBlockHeight()

	async function handleBack() {
		await disconnectSession()
	}

	async function handlePasteSignatures(sigs: PastedSignature[], broadcastState: ImportBroadcastState) {
		if (!actionId || !proposal) return
		for (const sig of sigs) {
			const res = await approveProposal({ baseUrl: ORCHESTRATOR_BASE_URL, actionId, ...sig })
			if (!res.ok) {
				console.warn(`Failed to import signature for ${sig.signerPubkey}: ${res.error}`)
			}
		}
		const hasNewBroadcastData =
			broadcastState.broadcastStatus !== null ||
			broadcastState.commitTxid !== null ||
			broadcastState.revealTxid !== null
		if (hasNewBroadcastData) {
			const effectiveBroadcastStatus = broadcastState.broadcastStatus ?? proposal.broadcastStatus
			// Sync broadcast state without proposalStatus so this call always succeeds.
			await reportBroadcastProgress({
				baseUrl: ORCHESTRATOR_BASE_URL,
				actionId,
				broadcastStatus: effectiveBroadcastStatus,
				commitTxid: broadcastState.commitTxid ?? undefined,
				revealTxid: broadcastState.revealTxid ?? undefined,
			})
			// If reveal is confirmed, attempt enacted transition in a separate call.
			// Backend validates against ASM — 409 means not yet confirmed; non-fatal,
			// reload reconcile will transition when ASM confirms.
			if (effectiveBroadcastStatus === 'reveal_confirmed') {
				await reportBroadcastProgress({
					baseUrl: ORCHESTRATOR_BASE_URL,
					actionId,
					broadcastStatus: 'reveal_confirmed',
					proposalStatus: 'enacted',
				})
			}
		}
		reload()
	}

	async function handleCheckEnacted(): Promise<{ ok: true } | { ok: false; error: string }> {
		if (!actionId) return { ok: false, error: 'No action ID' }
		const res = await reportBroadcastProgress({
			baseUrl: ORCHESTRATOR_BASE_URL,
			actionId,
			broadcastStatus: 'reveal_confirmed',
			proposalStatus: 'enacted',
		})
		if (res.ok) {
			reload()
			return { ok: true }
		}
		return { ok: false, error: res.error }
	}

	if (wallet === null) {
		return <Navigate to="/" replace />
	}

	if (actionId === undefined) {
		return <Navigate to="/proposals" replace />
	}

	return (
		<ScreenShell
			headerContent={
				<>
					<span className="inline-flex items-center gap-1.5 rounded-md border border-[#e4dfff] bg-[#f5f3ff] px-2.5 py-1.25 text-[12px] font-medium text-[#7c6fcd]">
						<ShieldPurpleIcon width={12} height={12} className="block shrink-0" />
						{authorityLabel}
					</span>
					<WalletSessionControl
						panel={panel}
						sessionTimeLabel={sessionTimeLabel}
						sessionWarning={sessionWarning}
						addressSample={wallet.addressSample}
					/>
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
			<div className="mx-auto w-full max-w-190">
				<button
					type="button"
					className="inline-flex items-center gap-1.5 text-sm text-[#6b7280] transition hover:text-[#111827]"
					onClick={() => navigate('/proposals')}
				>
					← Back to proposals
				</button>

				<h1 className="m-0 mt-3 font-['BIZ_UDPMincho'] text-[44px] leading-[1.05] tracking-[-0.01em] text-[#0a0a0a]">
					Proposal detail
				</h1>
				<p className="m-0 mt-1 text-[13px] text-[#6b7280]">Review signatures, action payload, and broadcast status.</p>

				<div className="mt-6">
					{isLoading && (
						<div className="animate-pulse space-y-3 rounded-xl border border-[#e5e7eb] bg-white p-6 shadow-sm">
							<div className="h-7 w-48 rounded-lg bg-[#f3f4f6]" />
							<div className="h-4 w-64 rounded-md bg-[#f3f4f6]" />
							<div className="mt-4 h-1.5 w-full rounded-full bg-[#f3f4f6]" />
							<div className="mt-6 h-20 w-full rounded-lg bg-[#f3f4f6]" />
						</div>
					)}

					{error && (
						<div className="rounded-xl border border-[#fecaca] bg-[#fef2f2] px-4 py-3">
							<p className="m-0 text-sm text-[#991b1b]">{error}</p>
							<button
								type="button"
								className="mt-2 rounded-md border border-[#991b1b] bg-white px-3 py-1 text-xs font-medium text-[#991b1b] transition hover:bg-[#fef2f2]"
								onClick={reload}
							>
								Retry
							</button>
						</div>
					)}

					{proposal && (
						<>
							<ProposalDetail
								proposal={proposal}
								signerPubkey={signerPubkey}
								decodedData={decodedData}
								onSign={() => navigate(`/proposals/${actionId}/sign`)}
								onBroadcast={() => navigate(`/proposals/${actionId}/broadcast`)}
								onPasteSignatures={(sigs, broadcastState) => void handlePasteSignatures(sigs, broadcastState)}
								onCheckEnacted={handleCheckEnacted}
								onManualExecute={() =>
									navigate('/manual', {
										state: {
											prefill: {
												actionHex: proposal.actionHex,
												seqNo: proposal.seqNo,
												authority: proposal.authority,
												signatures: proposal.signatures.map((s) => ({
													signerPubkey: s.signerPubkey,
													signatureHex: s.signatureHex,
												})),
											},
										},
									})
								}
							/>

							{/* Activation countdown */}
							{proposal.activationHeight !== null && proposal.status === 'approved' && (
								<div className="mt-4 rounded-xl border border-[#fde68a] bg-[#fffbeb] px-4 py-3">
									<ActivationCountdown
										activationHeight={proposal.activationHeight}
										currentHeight={currentBlockHeight}
									/>
								</div>
							)}

							{/* In-progress cancel banner */}
							{proposal.cancelProposal !== null && (
								<div className="mt-4 flex items-center justify-between gap-3 rounded-xl border border-[#fde68a] bg-[#fffbeb] px-4 py-3">
									<p className="m-0 text-[13px] text-[#d97706]">
										⚠ Cancellation in progress — {proposal.cancelProposal.signatures.length} /{' '}
										{proposal.cancelProposal.requiredSignatures} cancel signatures collected.
									</p>
									<button
										type="button"
										className="shrink-0 text-[13px] font-medium text-[#d97706] transition hover:text-[#b45309]"
										onClick={() => navigate(`/proposals/${actionId}/cancel`, { state: { signerPubkey } })}
									>
										View cancel →
									</button>
								</div>
							)}

							{/* Cancel CTA */}
							{proposal.status === 'approved' &&
								proposal.kind !== 'cancel' &&
								CANCELABLE_AUTHORITIES.includes(proposal.authority) &&
								proposal.cancelProposal === null && (
									<button
										type="button"
										className="mt-4 w-full rounded-xl border border-[#dc2626] bg-white px-4 py-2.5 text-sm font-medium text-[#dc2626] transition hover:bg-[#fef2f2]"
										onClick={() => navigate(`/proposals/${actionId}/cancel`, { state: { signerPubkey } })}
									>
										Cancel this proposal
									</button>
								)}
						</>
					)}
				</div>
			</div>
		</ScreenShell>
	)
}
