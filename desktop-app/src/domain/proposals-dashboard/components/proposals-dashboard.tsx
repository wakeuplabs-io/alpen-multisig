import { useState, type ReactNode } from 'react'
import type { Proposal, ProposalStatus } from '@/api/proposals'
import {
	CheckCircleEmeraldIcon,
	ChevronRightMutedIcon,
	ClockAmberIcon,
	FileTextMutedIcon,
	SignaturePenMutedIcon,
} from '@/assets/icons'

type Props = {
	authorityLabel: string
	signerPubkey: string | null
	quorumReached: Proposal[]
	pending: Proposal[]
	executedOrCanceled: Proposal[]
	expiredOrSkipped: Proposal[]
	isLoading: boolean
	error: string | null
	onRetry: () => void
	onCreateProposal: () => void
	onSignProposal: (actionId: string) => void
	onBroadcastProposal: (actionId: string) => void
	onViewProposal: (actionId: string) => void
}

export function ProposalsDashboard({
	authorityLabel,
	signerPubkey,
	quorumReached,
	pending,
	executedOrCanceled,
	expiredOrSkipped,
	isLoading,
	error,
	onRetry,
	onCreateProposal,
	onSignProposal,
	onBroadcastProposal,
	onViewProposal,
}: Props) {
	const isEmpty =
		!isLoading &&
		!error &&
		quorumReached.length + pending.length + executedOrCanceled.length + expiredOrSkipped.length === 0

	return (
		<section className="mx-auto w-full max-w-[800px]">
			<div className="mb-6 flex items-end justify-between gap-4">
				<div>
					<h1 className="m-0 font-['BIZ_UDPMincho'] text-[28px] font-normal leading-[1.2] tracking-[-0.005em] text-[#0a0a0a]">
						Proposals
					</h1>
					<p className="m-0 mt-1 text-[13px] leading-[1.5] text-[#6b7280]">
						Proposals you can sign, broadcast, or review under {authorityLabel}.
					</p>
				</div>
				<button
					type="button"
					data-testid="e2e-dashboard-create-proposal"
					className="inline-flex shrink-0 items-center gap-1.5 rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-4 py-2 text-sm font-medium text-white transition hover:bg-[#2a2a2a] active:scale-[0.98]"
					onClick={onCreateProposal}
				>
					<span aria-hidden="true">+</span>
					Create proposal
				</button>
			</div>

			{error ? (
				<div className="rounded-xl border border-[#fecaca] bg-[#fef2f2] px-4 py-3">
					<p className="m-0 text-sm text-[#991b1b]">{error}</p>
					<button
						type="button"
						className="mt-2 rounded-md border border-[#991b1b] bg-white px-3 py-1 text-xs font-medium text-[#991b1b] transition hover:bg-[#fef2f2]"
						onClick={onRetry}
					>
						Retry
					</button>
				</div>
			) : null}

			{isLoading ? (
				<div className="rounded-xl border border-[#e5e7eb] bg-white px-4 py-3 text-sm text-[#6b7280]">
					Loading proposals...
				</div>
			) : isEmpty ? (
				<EmptyState authorityLabel={authorityLabel} onCreateProposal={onCreateProposal} />
			) : (
				<div className="flex flex-col gap-7">
					<ProposalGroup
						title="Quorum reached"
						count={quorumReached.length}
						groupIcon={<CheckCircleEmeraldIcon width={14} height={14} className="block" />}
						initialOpen
						emptyMessage="No proposal has reached quorum yet."
						proposals={quorumReached}
						signerPubkey={signerPubkey}
						onSignProposal={onSignProposal}
						onBroadcastProposal={onBroadcastProposal}
						onViewProposal={onViewProposal}
					/>
					<ProposalGroup
						title="Pending"
						count={pending.length}
						groupIcon={<ClockAmberIcon width={14} height={14} className="block" />}
						initialOpen
						emptyMessage="No proposals are currently collecting signatures."
						proposals={pending}
						signerPubkey={signerPubkey}
						onSignProposal={onSignProposal}
						onBroadcastProposal={onBroadcastProposal}
						onViewProposal={onViewProposal}
					/>
					<ProposalGroup
						title="Executed & Canceled"
						count={executedOrCanceled.length}
						groupIcon={null}
						initialOpen={false}
						emptyMessage="No executed or canceled proposals yet."
						proposals={executedOrCanceled}
						signerPubkey={signerPubkey}
						onSignProposal={onSignProposal}
						onBroadcastProposal={onBroadcastProposal}
						onViewProposal={onViewProposal}
					/>
					<ProposalGroup
						title="Expired / Skipped"
						count={expiredOrSkipped.length}
						groupIcon={null}
						initialOpen={false}
						emptyMessage="No expired proposals."
						proposals={expiredOrSkipped}
						signerPubkey={signerPubkey}
						onSignProposal={onSignProposal}
						onBroadcastProposal={onBroadcastProposal}
						onViewProposal={onViewProposal}
					/>
				</div>
			)}
		</section>
	)
}

function EmptyState({ authorityLabel, onCreateProposal }: { authorityLabel: string; onCreateProposal: () => void }) {
	return (
		<div className="rounded-xl border border-[#e5e7eb] bg-white px-6 py-16 text-center">
			<div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full border border-[#e5e7eb] bg-[#f8f8fb] text-[#9ca3af]">
				<FileTextMutedIcon width={22} height={22} className="block" />
			</div>
			<p className="m-0 font-['BIZ_UDPMincho'] text-[22px] font-normal text-[#0a0a0a]">
				No proposals for {authorityLabel}
			</p>
			<p className="m-0 mt-2 text-[13px] text-[#6b7280]">Create the first proposal to begin collecting signatures.</p>
			<button
				type="button"
				className="mt-5 inline-flex items-center gap-1.5 rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-4 py-2 text-sm font-medium text-white transition hover:bg-[#2a2a2a]"
				onClick={onCreateProposal}
			>
				<span aria-hidden="true">+</span>
				Create proposal
			</button>
		</div>
	)
}

function ProposalGroup({
	title,
	count,
	groupIcon,
	initialOpen,
	emptyMessage,
	proposals,
	signerPubkey,
	onSignProposal,
	onBroadcastProposal,
	onViewProposal,
}: {
	title: string
	count: number
	groupIcon: ReactNode
	initialOpen: boolean
	emptyMessage: string
	proposals: Proposal[]
	signerPubkey: string | null
	onSignProposal: (actionId: string) => void
	onBroadcastProposal: (actionId: string) => void
	onViewProposal: (actionId: string) => void
}) {
	const [open, setOpen] = useState(initialOpen)

	return (
		<section>
			<button
				type="button"
				className="mb-1.5 flex w-full items-center gap-2 bg-transparent border-none p-0 py-2 cursor-pointer"
				onClick={() => setOpen((o) => !o)}
			>
				<ChevronRightMutedIcon
					width={14}
					height={14}
					className="shrink-0 transition-transform duration-150 ease-out"
					style={{ transform: open ? 'rotate(90deg)' : 'rotate(0deg)' }}
				/>
				{groupIcon}
				<h2
					className="m-0 text-[13px] font-semibold uppercase tracking-[0.05em]"
					style={{ color: '#6b7280', fontFamily: 'inherit' }}
				>
					{title}
					<span className="ml-1.5 font-normal">· {count}</span>
				</h2>
			</button>

			{open &&
				(proposals.length === 0 ? (
					<div className="rounded-xl border border-[#e5e7eb] bg-white px-5 py-4 text-[13px] text-[#9ca3af]">
						{emptyMessage}
					</div>
				) : (
					<div className="flex flex-col gap-2.5">
						{proposals.map((proposal) => (
							<ProposalCard
								key={proposal.actionId}
								proposal={proposal}
								signerPubkey={signerPubkey}
								onSignProposal={onSignProposal}
								onBroadcastProposal={onBroadcastProposal}
								onViewProposal={onViewProposal}
							/>
						))}
					</div>
				))}
		</section>
	)
}

function ProposalCard({
	proposal,
	signerPubkey,
	onSignProposal,
	onBroadcastProposal,
	onViewProposal,
}: {
	proposal: Proposal
	signerPubkey: string | null
	onSignProposal: (actionId: string) => void
	onBroadcastProposal: (actionId: string) => void
	onViewProposal: (actionId: string) => void
}) {
	const [hovered, setHovered] = useState(false)
	const requiredSignatures = proposal.requiredSignatures
	const collectedSignatures = proposal.signatures.length
	const signaturesProgress =
		requiredSignatures === 0 ? 0 : Math.min((collectedSignatures / requiredSignatures) * 100, 100)
	const proposalTitle = buildProposalTitle(proposal)
	const proposalTypeLabel = inferProposalType(proposal)
	const isTerminal = proposal.status === 'enacted' || proposal.status === 'canceled' || proposal.status === 'expired'
	const hasQuorum = !isTerminal && (proposal.status === 'approved' || collectedSignatures >= requiredSignatures)
	const broadcastInProgress = proposal.status === 'approved' && proposal.broadcastStatus !== 'idle'
	const awaitingEnactment =
		proposal.status === 'approved' && proposal.broadcastStatus === 'reveal_confirmed'
	const canBroadcast = hasQuorum && proposal.status === 'approved' && proposal.broadcastStatus === 'idle'
	const alreadySigned =
		signerPubkey !== null &&
		proposal.signatures.some((s) => s.signerPubkey.toLowerCase() === signerPubkey.toLowerCase())

	return (
		<div
			className="group"
			style={{
				background: '#fff',
				border: `1px solid ${hovered ? 'var(--color-accent-border)' : 'var(--color-border)'}`,
				borderRadius: 12,
				padding: '18px 18px 16px',
				cursor: 'pointer',
				transition: 'all 150ms ease',
				transform: hovered ? 'translateY(-1px)' : 'translateY(0)',
				boxShadow: hovered ? 'var(--shadow-card)' : 'none',
			}}
			onMouseEnter={() => setHovered(true)}
			onMouseLeave={() => setHovered(false)}
			onClick={() => onViewProposal(proposal.actionId)}
		>
			<div className="flex items-start justify-between gap-3">
				<div className="min-w-0 flex-1">
					<p className="m-0 font-['BIZ_UDPMincho'] text-[24px] leading-[1.2] text-[#121212]">{proposalTitle}</p>
					<p className="m-0 mt-1 text-[13px] text-[#6b7280]">
						#{proposal.seqNo} · {proposalTypeLabel} · {proposal.authority}
					</p>
				</div>
				<StatusBadge status={proposal.status} />
			</div>

			<div className="mt-5">
				<div className="mb-1.5 flex items-center justify-between gap-3">
					<p className="m-0 text-[14px] font-medium text-[#121212]">Signatures</p>
					<p className="m-0 text-[30px] font-medium leading-none text-[#121212]">
						{collectedSignatures} / {requiredSignatures} <span className="text-[18px]">signed</span>
					</p>
				</div>

				<div className="h-[7px] rounded-full bg-[#ebedf0]">
					<div
						className="h-[7px] rounded-full transition-all"
						style={{
							width: `${signaturesProgress}%`,
							background: hasQuorum ? '#0f9d7a' : '#d97706',
						}}
					/>
				</div>
			</div>

			{canBroadcast ? (
				<div className="mt-4 flex items-center justify-between gap-3 border-t border-[#eceff3] pt-3">
					<p className="m-0 inline-flex items-center gap-1.5 text-[14px] font-medium text-[#0f9d7a]">
						<CheckCircleEmeraldIcon width={15} height={15} className="block shrink-0" />
						Quorum reached - ready to broadcast
					</p>
					<button
						type="button"
						className="inline-flex items-center rounded-xl border border-[#111827] bg-[#111827] px-4 py-2 text-sm font-medium text-white transition hover:bg-black"
						onClick={(e) => {
							e.stopPropagation()
							onBroadcastProposal(proposal.actionId)
						}}
						data-testid="e2e-proposal-broadcast-button"
					>
						Broadcast
					</button>
				</div>
			) : awaitingEnactment ? (
				<div className="mt-4 border-t border-[#eceff3] pt-3">
					<p className="m-0 text-[14px] font-medium text-[#0f9d7a]">
						Reveal confirmed — awaiting ASM enactment
					</p>
					<p className="m-0 mt-1 text-[12px] text-[#6b7280]">
						Refresh the dashboard after the confirmation delay to see enacted status.
					</p>
				</div>
			) : broadcastInProgress ? (
				<div className="mt-4 border-t border-[#eceff3] pt-3">
					<p className="m-0 text-[14px] font-medium text-[#6b7280]">Broadcast in progress</p>
				</div>
			) : hasQuorum ? (
				<div className="mt-4 border-t border-[#eceff3] pt-3">
					<p className="m-0 inline-flex items-center gap-1.5 text-[14px] font-medium text-[#0f9d7a]">
						<CheckCircleEmeraldIcon width={15} height={15} className="block shrink-0" />
						Quorum reached
					</p>
				</div>
			) : (
				<div className="mt-4 flex items-center justify-between gap-3 border-t border-[#eceff3] pt-3">
					<p className="m-0 flex items-center gap-1 text-xs text-[#6b7280]">
						<SignaturePenMutedIcon width={12} height={12} className="block shrink-0" />
						{collectedSignatures} {collectedSignatures === 1 ? 'signature' : 'signatures'} collected
					</p>
					{!isTerminal && !alreadySigned && (
						<button
							type="button"
							data-testid="e2e-proposal-sign-button"
							className="inline-flex items-center rounded-xl border border-[#111827] bg-[#111827] px-4 py-2 text-sm font-medium text-white transition hover:bg-black"
							onClick={(e) => {
								e.stopPropagation()
								onSignProposal(proposal.actionId)
							}}
						>
							Sign
						</button>
					)}
				</div>
			)}
		</div>
	)
}

function buildProposalTitle(proposal: Proposal): string {
	return `Proposal #${proposal.seqNo} - ${inferProposalType(proposal)}`
}

function inferProposalType(proposal: Proposal): string {
	if (proposal.authority.toLowerCase().includes('sequencer')) {
		return 'Sequencer update'
	}
	if (proposal.actionHex.toLowerCase().startsWith('0x01')) {
		return 'Verification key update'
	}
	return 'Signer update'
}

const STATUS_CONFIG: Record<ProposalStatus, { bg: string; text: string; border: string; dot: string; label: string }> =
	{
		pending: { bg: '#fffbeb', text: '#d97706', border: '#fde68a', dot: '#d97706', label: 'Pending' },
		approved: { bg: '#eff6ff', text: '#2563eb', border: '#bfdbfe', dot: '#2563eb', label: 'Approved' },
		enacted: { bg: '#ecfdf5', text: '#059669', border: '#a7f3d0', dot: '#059669', label: 'Enacted' },
		canceled: { bg: '#fef2f2', text: '#dc2626', border: '#fecaca', dot: '#dc2626', label: 'Canceled' },
		expired: { bg: '#f9fafb', text: '#6b7280', border: '#e5e7eb', dot: '#6b7280', label: 'Expired' },
	}

function StatusBadge({ status }: { status: ProposalStatus }) {
	const s = STATUS_CONFIG[status]
	return (
		<span
			className="inline-flex shrink-0 items-center gap-1.5 rounded-md border px-[10px] py-[3px] text-[11px] font-medium whitespace-nowrap"
			style={{ background: s.bg, color: s.text, borderColor: s.border }}
		>
			<span className="h-1.5 w-1.5 flex-none rounded-full" style={{ background: s.dot }} aria-hidden="true" />
			{s.label}
		</span>
	)
}
