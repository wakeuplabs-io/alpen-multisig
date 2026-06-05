import { useState } from 'react'
import type { ActionType, Proposal } from '@/api/proposals'
import { CheckCircleEmeraldIcon, CopyClipboardIcon, DownloadIcon } from '@/assets/icons'
import { ProposalDetail } from '@/domain/proposal-detail/components/proposal-detail'
import type { DecodedProposalData } from '@/domain/proposal-detail/hooks/use-decoded-proposal'
import type { ManualImportData, ManualSignature } from '@/domain/manual-proposal/model/manual-proposal.types'
import type { ImportBroadcastState } from '@/domain/proposal-detail/components/import-bundle-modal'
import type { PastedSignature } from '@/domain/proposal-detail/model/pasted-signature'

type Props = {
	importData: ManualImportData
	decodedData: DecodedProposalData
	localSignatures: ManualSignature[]
	requiredSignatures: number | null
	hasQuorum: boolean
	isSigning: boolean
	signError: string | null
	signerPubkey: string | null
	onSign: () => void
	onBroadcast: () => void
	onPasteSignatures?: (sigs: PastedSignature[], broadcastState: ImportBroadcastState) => void
}

function derivedActionType(actionHex: string): ActionType {
	const h = actionHex.toLowerCase()
	if (h.startsWith('01')) return 'vk_update'
	return 'multisig_update'
}

export function ManualSignCollect({
	importData,
	decodedData,
	localSignatures,
	requiredSignatures,
	hasQuorum,
	isSigning,
	signError,
	signerPubkey,
	onSign,
	onBroadcast,
	onPasteSignatures,
}: Props) {
	const [bundleCopied, setBundleCopied] = useState(false)

	const syntheticProposal: Proposal = {
		actionId: `manual-${importData.sighashHex.slice(0, 16)}`,
		seqNo: importData.seqNo,
		authority: importData.authority,
		status: 'pending',
		requiredSignatures: requiredSignatures ?? 0,
		actionHex: importData.actionHex,
		actionType: derivedActionType(importData.actionHex),
		signatures: localSignatures.map(({ signerPubkey, signatureHex }) => ({ signerPubkey, signatureHex })),
		broadcastStatus: 'idle',
		kind: 'update',
		targetActionId: null,
		activationHeight: null,
		updateIdInQueue: null,
		cancelProposal: null,
		createdAtMs: 0,
		expiresAtMs: 0,
	}

	const bundle = {
		actionHex: importData.actionHex,
		seqNo: importData.seqNo,
		authority: importData.authority,
		status: hasQuorum ? 'approved' : 'pending',
		broadcastStatus: 'idle',
		activationHeight: null,
		signatures: localSignatures.map(({ signerPubkey, signatureHex }) => ({ signerPubkey, signatureHex })),
	}

	function handleCopyBundle() {
		void navigator.clipboard.writeText(JSON.stringify(bundle, null, 2)).then(() => {
			setBundleCopied(true)
			setTimeout(() => setBundleCopied(false), 2000)
		})
	}

	function handleDownloadBundle() {
		const blob = new Blob([JSON.stringify(bundle, null, 2)], { type: 'application/json' })
		const url = URL.createObjectURL(blob)
		const a = document.createElement('a')
		a.href = url
		a.download = `proposal-manual-${importData.seqNo}.json`
		document.body.appendChild(a)
		a.click()
		document.body.removeChild(a)
		URL.revokeObjectURL(url)
	}

	return (
		<div className="space-y-4">
			{isSigning && (
				<div className="rounded-xl border border-[#bfdbfe] bg-[#eff6ff] px-4 py-3">
					<p className="m-0 text-[13px] text-[#1d4ed8]">Waiting for hardware wallet approval…</p>
				</div>
			)}

			{signError && (
				<div className="rounded-xl border border-[#fecaca] bg-[#fef2f2] px-4 py-3">
					<p className="m-0 text-[13px] font-medium text-[#dc2626]">Signing failed</p>
					<p className="m-0 mt-0.5 text-[12px] text-[#991b1b]">{signError}</p>
				</div>
			)}

			<ProposalDetail
				proposal={syntheticProposal}
				signerPubkey={signerPubkey}
				decodedData={decodedData}
				onSign={isSigning ? () => {} : onSign}
				onBroadcast={onBroadcast}
				onPasteSignatures={onPasteSignatures}
			/>

			{hasQuorum && (
				<div className="flex items-center justify-between gap-3 rounded-xl border border-[#a7f3d0] bg-[#ecfdf5] px-4 py-3">
					<p className="m-0 inline-flex items-center gap-1.5 text-[13px] font-medium text-[#065f46]">
						<CheckCircleEmeraldIcon width={14} height={14} className="block shrink-0" />
						Quorum reached — ready to broadcast
					</p>
					<div className="flex items-center gap-1.5">
						<button
							type="button"
							title={bundleCopied ? 'Copied!' : 'Copy bundle'}
							onClick={handleCopyBundle}
							className="inline-flex items-center rounded-md border border-[#a7f3d0] bg-white px-2.5 py-1.5 text-xs font-medium text-[#065f46] transition hover:bg-[#f0fdf4]"
						>
							<CopyClipboardIcon width={12} height={12} className="text-current" />
						</button>
						<button
							type="button"
							title="Download bundle"
							onClick={handleDownloadBundle}
							className="inline-flex items-center rounded-md border border-[#a7f3d0] bg-white px-2.5 py-1.5 text-xs font-medium text-[#065f46] transition hover:bg-[#f0fdf4]"
						>
							<DownloadIcon width={12} height={12} className="text-current" />
						</button>
					</div>
				</div>
			)}
		</div>
	)
}
