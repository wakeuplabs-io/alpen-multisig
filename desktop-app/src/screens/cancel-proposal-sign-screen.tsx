import { useEffect, useState } from 'react'
import { Navigate, useNavigate, useParams } from 'react-router-dom'
import { buildCancelActionHex } from '@/api/action-builder'
import { getOrchestratorBaseUrl } from '@/api/orchestrator-auth'
import { orchestratorAuthGetSession } from '@/api/orchestrator-auth'
import {
	approveProposal,
	createCancelProposal,
	getProposalByActionId,
	getNextSeqNo,
	type Proposal,
} from '@/api/proposals'
import { computeSighash } from '@/api/signing'
import { ShieldAccentIcon } from '@/assets/icons'
import { authorityLabelForRole } from '@/lib/authority-label'
import { deviceSigningDisplay } from '@/lib/device-signing-display'
import { useSession } from '@/hooks/use-session'
import { useDeviceSigningMessage } from '@/hooks/use-device-signing-message'
import { DeviceSigningHint } from '@/components/device-signing-hint'
import { Breadcrumbs } from '@/components/breadcrumbs'
import { DisconnectButton } from '@/components/disconnect-button'
import { ScreenShell } from '@/screens/screen-shell'
import type { SignSighashResult } from '@/wallet/types'
import { useWalletPanelData } from '@/domain/admin-wallet/hooks/use-wallet-panel-data'
import { WalletSessionControl } from '@/domain/admin-wallet/components/wallet-session-control'

type Flow = 'initiate' | 'approve'

export function CancelProposalSignScreen() {
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
	const authorityLabel = authorityLabelForRole(selectedRole)
	const panel = useWalletPanelData()

	const [isLoading, setIsLoading] = useState(true)
	const [loadError, setLoadError] = useState<string | null>(null)
	const [isSigning, setIsSigning] = useState(false)
	const [signError, setSignError] = useState<string | null>(null)
	const [signResult, setSignResult] = useState<SignSighashResult | null>(null)

	// Signing context resolved on load
	const [flow, setFlow] = useState<Flow>('initiate')
	const [parentProposal, setParentProposal] = useState<Proposal | null>(null)
	const [cancelActionHex, setCancelActionHex] = useState('')
	const [cancelSeqNo, setCancelSeqNo] = useState(0)
	const [cancelActionId, setCancelActionId] = useState<string | null>(null)
	const [sighashHex, setSighashHex] = useState('')
	const [signerPubkey, setSignerPubkey] = useState<string | null>(null)
	const [alreadySigned, setAlreadySigned] = useState(false)

	const { message: deviceMessage, messageHash: deviceMessageHash } = useDeviceSigningMessage(
		cancelActionHex ? cancelSeqNo : null,
		cancelActionHex ? cancelActionHex.replace(/^0x/i, '') : null,
	)
	const deviceDisplay = deviceSigningDisplay(adapter.vendor, {
		message: deviceMessage,
		messageHash: deviceMessageHash,
	})

	useEffect(() => {
		let mounted = true

		async function load() {
			if (!actionId) {
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
				const session = await orchestratorAuthGetSession()
				if (!session.ok) throw new Error(session.error)
				const pubkey = session.data?.signerPubkey ?? null
				if (mounted) setSignerPubkey(pubkey)

				// Fetch the parent (target) proposal
				const parentRes = await getProposalByActionId({ baseUrl: getOrchestratorBaseUrl(), actionId })
				if (!parentRes.ok) throw new Error(parentRes.error)
				const parent = parentRes.data
				if (mounted) setParentProposal(parent)

				if (parent.cancelProposal !== null) {
					// Approve flow: cancel proposal already exists, fetch it for actionHex + seqNo
					const cancelRes = await getProposalByActionId({
						baseUrl: getOrchestratorBaseUrl(),
						actionId: parent.cancelProposal.actionId,
					})
					if (!cancelRes.ok) throw new Error(cancelRes.error)
					const cancelProp = cancelRes.data

					const sighashRes = await computeSighash(cancelProp.seqNo, cancelProp.actionHex)
					if (!sighashRes.ok) throw new Error(sighashRes.error)

					const signed =
						pubkey !== null && cancelProp.signatures.some((s) => s.signerPubkey.toLowerCase() === pubkey.toLowerCase())

					if (mounted) {
						setFlow('approve')
						setCancelActionHex(cancelProp.actionHex)
						setCancelSeqNo(cancelProp.seqNo)
						setCancelActionId(cancelProp.actionId)
						setSighashHex(sighashRes.data.sighashHex)
						setAlreadySigned(signed)
					}
				} else {
					// Initiate flow: build the cancel action hex from the target proposal
					const seqNoRes = await getNextSeqNo({ baseUrl: getOrchestratorBaseUrl() })
					if (!seqNoRes.ok) throw new Error(seqNoRes.error)
					const nextSeqNo = seqNoRes.data

					const cancelHexRes = await buildCancelActionHex(parent.actionHex)
					if (!cancelHexRes.ok) throw new Error(cancelHexRes.error)
					const cancelHex = cancelHexRes.data.actionHex

					const sighashRes = await computeSighash(nextSeqNo, cancelHex)
					if (!sighashRes.ok) throw new Error(sighashRes.error)

					if (mounted) {
						setFlow('initiate')
						setCancelActionHex(cancelHex)
						setCancelSeqNo(nextSeqNo)
						setSighashHex(sighashRes.data.sighashHex)
					}
				}
			} catch (e) {
				if (mounted) setLoadError(String(e))
			} finally {
				if (mounted) setIsLoading(false)
			}
		}

		void load()
		return () => {
			mounted = false
		}
	}, [actionId, ensureOrchestratorSession])

	async function handleBack() {
		await disconnectSession()
	}

	async function handleSign() {
		if (alreadySigned) {
			setSignError('You have already signed this cancel proposal.')
			return
		}
		if (!sighashHex) {
			setSignError('Sighash not ready. Reload the page.')
			return
		}
		if (!signerPubkey) {
			setSignError('No authenticated signer in session. Re-authenticate and try again.')
			return
		}

		setIsSigning(true)
		setSignError(null)
		setSignResult(null)

		try {
			const signed = await adapter.signSighash(sighashHex, {
				seqno: cancelSeqNo,
				actionHex: cancelActionHex,
			})

			if (flow === 'initiate') {
				const res = await createCancelProposal({
					baseUrl: getOrchestratorBaseUrl(),
					targetActionId: actionId!,
					seqNo: cancelSeqNo,
					actionHex: cancelActionHex,
					signerPubkey,
					signatureHex: signed.signatureHex,
				})
				if (!res.ok) throw new Error(res.error)
			} else {
				if (!cancelActionId) throw new Error('Cancel proposal action ID is missing.')
				const res = await approveProposal({
					baseUrl: getOrchestratorBaseUrl(),
					actionId: cancelActionId,
					signerPubkey,
					signatureHex: signed.signatureHex,
				})
				if (!res.ok) throw new Error(res.error)
			}

			setSignResult(signed)
			navigate(`/proposals/${actionId}/cancel`, { state: { signerPubkey } })
		} catch (e) {
			setSignError(String(e))
		} finally {
			setIsSigning(false)
		}
	}

	if (wallet === null) return <Navigate to="/" replace />
	if (!actionId) return <Navigate to="/proposals" replace />

	const flowTitle = flow === 'initiate' ? 'Initiate cancel' : 'Sign cancel'
	const flowSubtitle =
		flow === 'initiate'
			? 'Sign on your hardware wallet to create the cancel proposal and add the first signature.'
			: 'Sign on your hardware wallet to add your signature to the existing cancel proposal.'

	const buttonLabel = isSigning
		? 'Signing…'
		: flow === 'initiate'
			? 'Sign to initiate cancel'
			: 'Sign to approve cancel'

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
					{flowTitle}
				</h1>
				<p className="m-0 mt-1 text-body-sm text-[#6b7280]">{flowSubtitle}</p>

				{isLoading && (
					<div className="mt-5 rounded-xl border border-[#e5e7eb] bg-white px-4 py-3 text-body text-[#6b7280]">
						Loading…
					</div>
				)}

				{loadError && (
					<div className="mt-5 rounded-xl border border-danger-border bg-danger-surface px-4 py-3">
						<p className="m-0 text-body text-danger-deep">{loadError}</p>
						<button
							type="button"
							className="mt-3 inline-flex items-center rounded-md border border-danger-deep bg-white px-3 py-1.5 text-label font-medium text-danger-deep transition hover:bg-danger-surface"
							onClick={() => navigate(`/proposals/${actionId}/cancel`)}
						>
							Back to cancel
						</button>
					</div>
				)}

				{!isLoading && alreadySigned && (
					<div className="mt-5 rounded-xl border border-accent-border bg-highlight-surface px-4 py-3">
						<p className="m-0 text-body font-medium text-emphasis">You have already signed this cancel proposal.</p>
					</div>
				)}

				{!isLoading && parentProposal !== null && !loadError && (
					<div className="mt-5 space-y-4">
						{/* Target proposal context */}
						<div className="rounded-xl border border-[#e5e7eb] bg-white px-6 py-5 shadow-sm">
							<p className="m-0 text-mono-sm font-semibold uppercase tracking-wider text-[#9ca3af]">Target proposal</p>
							<p className="m-0 mt-1 text-body-sm font-medium text-[#111827]">
								Proposal #{parentProposal.seqNo} · {parentProposal.authority}
							</p>
							<p className="m-0 mt-0.5 text-label text-[#6b7280]">
								This cancel action will remove the queued update from the activation queue.
							</p>
						</div>

						{/* What the device will render for this signature. Shown only while there is something
						    to sign, and next to the button, because the comparison happens with the device
						    already prompting. The hint carries its own card, so it needs no wrapper. */}
						{!alreadySigned && !signResult && <DeviceSigningHint display={deviceDisplay} />}

						{/* Sign error */}
						{signError && (
							<div className="rounded-xl border border-danger-border bg-danger-surface px-4 py-3">
								<p className="m-0 text-body text-danger-deep">{signError}</p>
							</div>
						)}

						{/* Sign success */}
						{signResult && (
							<div className="rounded-xl border border-[#d1fae5] bg-[#f0fdf4] px-4 py-3">
								<p className="m-0 text-body font-medium text-[#065f46]">Signed successfully.</p>
							</div>
						)}

						{/* Sign button */}
						{!alreadySigned && !signResult && (
							<button
								type="button"
								disabled={isSigning || !sighashHex}
								className="w-full rounded-xl border border-[#111827] bg-[#111827] px-4 py-2.5 text-body font-medium text-white transition hover:bg-black disabled:cursor-not-allowed disabled:opacity-50"
								onClick={() => void handleSign()}
							>
								{buttonLabel}
							</button>
						)}
					</div>
				)}
			</div>
		</ScreenShell>
	)
}
