import { useState, type ReactNode } from 'react'
import { Paginator } from '@/components/paginator'
import { PendingExpiryCountdown } from '@/components/pending-expiry-countdown'
import type { Proposal, ProposalStatus } from '@/api/proposals'
import {
	CheckCircleEmeraldIcon,
	ChevronRightMutedIcon,
	ClockAmberIcon,
	FileTextMutedIcon,
	SignaturePenMutedIcon,
	UndoIcon,
} from '@/assets/icons'
import { deriveProposalActions } from '@/domain/proposal-detail/model/derive-proposal-actions'

const CANCELABLE_AUTHORITIES = ['alpen_admin', 'strata_admin']
const PAGE_SIZE = 10

type Tab = 'pending' | 'past'

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
	onRefresh: () => void
	onCreateProposal: () => void
	onSignProposal: (actionId: string) => void
	onBroadcastProposal: (actionId: string) => void
	onViewProposal: (actionId: string) => void
	onCancelProposal: (actionId: string) => void
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
	onRefresh,
	onCreateProposal,
	onSignProposal,
	onBroadcastProposal,
	onViewProposal,
	onCancelProposal,
}: Props) {
	const [activeTab, setActiveTab] = useState<Tab>('pending')
	const [pastPage, setPastPage] = useState(1)

	const activeProposals = [...quorumReached, ...pending]
	const pastProposals = [...executedOrCanceled, ...expiredOrSkipped]
	const totalPastPages = Math.ceil(pastProposals.length / PAGE_SIZE)
	const pagedPastProposals = pastProposals.slice((pastPage - 1) * PAGE_SIZE, pastPage * PAGE_SIZE)

	const isEmpty = !isLoading && !error && activeProposals.length === 0 && pastProposals.length === 0

	const handleTabChange = (tab: Tab) => {
		setActiveTab(tab)
		setPastPage(1)
	}

	return (
		<section className="mx-auto w-full max-w-200">
			<div className="mb-6 flex items-end justify-between gap-4">
				<div>
					<h1 className="m-0 font-['BIZ_UDPMincho'] text-[28px] font-normal leading-[1.2] tracking-[-0.005em] text-[#0a0a0a]">
						Proposals
					</h1>
					<p className="m-0 mt-1 text-[13px] leading-normal text-[#6b7280]">
						Proposals you can sign, broadcast, or review under {authorityLabel}.
					</p>
				</div>
				<div className="flex shrink-0 items-center gap-2">
					<button
						type="button"
						aria-label="Refresh proposals"
						disabled={isLoading}
						className="inline-flex items-center justify-center rounded-lg border border-[#e5e7eb] bg-white p-2 text-[#6b7280] transition hover:border-[#d1d5db] hover:bg-[#f9fafb] active:scale-[0.97] disabled:opacity-50"
						onClick={onRefresh}
					>
						<UndoIcon width={16} height={16} className={isLoading ? 'animate-spin' : ''} />
					</button>
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
				<>
					{/* Tabs */}
					<div className="mb-5 flex gap-0.5 rounded-xl border border-[#e5e7eb] bg-[#f8f8fb] p-1">
						{(['pending', 'past'] as Tab[]).map((tab) => (
							<button
								key={tab}
								type="button"
								className="flex-1 rounded-lg py-2 text-[13px] font-medium transition"
								style={
									activeTab === tab
										? { background: '#fff', color: '#0a0a0a', boxShadow: '0 1px 3px rgba(0,0,0,0.08)' }
										: { background: 'transparent', color: '#6b7280' }
								}
								onClick={() => handleTabChange(tab)}
							>
								{tab === 'pending' ? 'Pending' : 'Past'}
								{tab === 'pending' && activeProposals.length > 0 && (
									<span
										className="ml-1.5 rounded-full px-1.5 py-0.5 text-[10px] font-semibold"
										style={
											activeTab === 'pending'
												? { background: '#0a0a0a', color: '#fff' }
												: { background: '#e5e7eb', color: '#6b7280' }
										}
									>
										{activeProposals.length}
									</span>
								)}
							</button>
						))}
					</div>

					{/* Tab content */}
					{activeTab === 'pending' ? (
						<PendingTab
							quorumReached={quorumReached}
							pending={pending}
							signerPubkey={signerPubkey}
							onSignProposal={onSignProposal}
							onBroadcastProposal={onBroadcastProposal}
							onViewProposal={onViewProposal}
							onCancelProposal={onCancelProposal}
						/>
					) : (
						<PastTab
							proposals={pagedPastProposals}
							totalProposals={pastProposals.length}
							page={pastPage}
							totalPages={totalPastPages}
							signerPubkey={signerPubkey}
							onPageChange={setPastPage}
							onSignProposal={onSignProposal}
							onBroadcastProposal={onBroadcastProposal}
							onViewProposal={onViewProposal}
							onCancelProposal={onCancelProposal}
						/>
					)}
				</>
			)}
		</section>
	)
}

function PendingTab({
	quorumReached,
	pending,
	signerPubkey,
	onSignProposal,
	onBroadcastProposal,
	onViewProposal,
	onCancelProposal,
}: {
	quorumReached: Proposal[]
	pending: Proposal[]
	signerPubkey: string | null
	onSignProposal: (actionId: string) => void
	onBroadcastProposal: (actionId: string) => void
	onViewProposal: (actionId: string) => void
	onCancelProposal: (actionId: string) => void
}) {
	if (quorumReached.length === 0 && pending.length === 0) {
		return (
			<div className="rounded-xl border border-[#e5e7eb] bg-white px-5 py-4 text-[13px] text-[#9ca3af]">
				No proposals are currently collecting signatures.
			</div>
		)
	}

	return (
		<div className="flex flex-col gap-7">
			{quorumReached.length > 0 && (
				<ProposalGroup
					title="Quorum reached"
					count={quorumReached.length}
					groupIcon={<CheckCircleEmeraldIcon width={14} height={14} className="block" />}
					proposals={quorumReached}
					signerPubkey={signerPubkey}
					onSignProposal={onSignProposal}
					onBroadcastProposal={onBroadcastProposal}
					onViewProposal={onViewProposal}
					onCancelProposal={onCancelProposal}
				/>
			)}
			{pending.length > 0 && (
				<ProposalGroup
					title="Pending"
					count={pending.length}
					groupIcon={<ClockAmberIcon width={14} height={14} className="block" />}
					proposals={pending}
					signerPubkey={signerPubkey}
					onSignProposal={onSignProposal}
					onBroadcastProposal={onBroadcastProposal}
					onViewProposal={onViewProposal}
					onCancelProposal={onCancelProposal}
				/>
			)}
		</div>
	)
}

function PastTab({
	proposals,
	totalProposals,
	page,
	totalPages,
	signerPubkey,
	onPageChange,
	onSignProposal,
	onBroadcastProposal,
	onViewProposal,
	onCancelProposal,
}: {
	proposals: Proposal[]
	totalProposals: number
	page: number
	totalPages: number
	signerPubkey: string | null
	onPageChange: (page: number) => void
	onSignProposal: (actionId: string) => void
	onBroadcastProposal: (actionId: string) => void
	onViewProposal: (actionId: string) => void
	onCancelProposal: (actionId: string) => void
}) {
	if (totalProposals === 0) {
		return (
			<div className="rounded-xl border border-[#e5e7eb] bg-white px-5 py-4 text-[13px] text-[#9ca3af]">
				No past proposals yet.
			</div>
		)
	}

	return (
		<div className="flex flex-col gap-2.5">
			{proposals.map((proposal) => (
				<ProposalCard
					key={proposal.actionId}
					proposal={proposal}
					signerPubkey={signerPubkey}
					onSignProposal={onSignProposal}
					onBroadcastProposal={onBroadcastProposal}
					onViewProposal={onViewProposal}
					onCancelProposal={onCancelProposal}
				/>
			))}
			<Paginator page={page} totalPages={totalPages} onPageChange={onPageChange} />
		</div>
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
	proposals,
	signerPubkey,
	onSignProposal,
	onBroadcastProposal,
	onViewProposal,
	onCancelProposal,
}: {
	title: string
	count: number
	groupIcon: ReactNode
	proposals: Proposal[]
	signerPubkey: string | null
	onSignProposal: (actionId: string) => void
	onBroadcastProposal: (actionId: string) => void
	onViewProposal: (actionId: string) => void
	onCancelProposal: (actionId: string) => void
}) {
	const [open, setOpen] = useState(true)

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
					className="m-0 text-[13px] font-semibold uppercase tracking-wider"
					style={{ color: '#6b7280', fontFamily: 'inherit' }}
				>
					{title}
					<span className="ml-1.5 font-normal">· {count}</span>
				</h2>
			</button>

			{open && (
				<div className="flex flex-col gap-2.5">
					{proposals.map((proposal) => (
						<ProposalCard
							key={proposal.actionId}
							proposal={proposal}
							signerPubkey={signerPubkey}
							onSignProposal={onSignProposal}
							onBroadcastProposal={onBroadcastProposal}
							onViewProposal={onViewProposal}
							onCancelProposal={onCancelProposal}
						/>
					))}
				</div>
			)}
		</section>
	)
}

function ProposalCard({
	proposal,
	signerPubkey,
	onSignProposal,
	onBroadcastProposal,
	onViewProposal,
	onCancelProposal,
}: {
	proposal: Proposal
	signerPubkey: string | null
	onSignProposal: (actionId: string) => void
	onBroadcastProposal: (actionId: string) => void
	onViewProposal: (actionId: string) => void
	onCancelProposal: (actionId: string) => void
}) {
	const [hovered, setHovered] = useState(false)
	const requiredSignatures = proposal.requiredSignatures
	const collectedSignatures = proposal.signatures.length
	const signaturesProgress =
		requiredSignatures === 0 ? 0 : Math.min((collectedSignatures / requiredSignatures) * 100, 100)
	const proposalTitle = buildProposalTitle(proposal)
	const proposalTypeLabel = inferProposalType(proposal)
	const { hasQuorum, canSign, canBroadcast } = deriveProposalActions(proposal, signerPubkey)
	const broadcastInProgress = proposal.status === 'approved' && proposal.broadcastStatus !== 'idle'
	const awaitingEnactment = proposal.status === 'approved' && proposal.broadcastStatus === 'reveal_confirmed'

	const signButton = canSign ? (
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
	) : null

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
						{proposal.kind === 'cancel' && proposal.targetActionId !== null && (
							<>
								{' '}
								· Cancels <span className="font-mono">{proposal.targetActionId.slice(0, 8)}…</span>
							</>
						)}
					</p>
				</div>
				<StatusBadge status={awaitingEnactment ? 'awaiting_enactment' : proposal.status} />
			</div>

			<div className="mt-5">
				<div className="mb-1.5 flex items-center justify-between gap-3">
					<p className="m-0 text-[14px] font-medium text-[#121212]">Signatures</p>
					<p className="m-0 text-[30px] font-medium leading-none text-[#121212]">
						{collectedSignatures} / {requiredSignatures} <span className="text-[18px]">signed</span>
					</p>
				</div>

				<div className="h-1.75 rounded-full bg-[#ebedf0]">
					<div
						className="h-1.75 rounded-full transition-all"
						style={{
							width: `${signaturesProgress}%`,
							background: hasQuorum ? '#0f9d7a' : '#d97706',
						}}
					/>
				</div>

				{proposal.status === 'pending' && (
					<div className="mt-1.5">
						<PendingExpiryCountdown expiresAtMs={proposal.expiresAtMs} />
					</div>
				)}
			</div>

			{canBroadcast ? (
				<div className="mt-4 flex items-center justify-between gap-3 border-t border-[#eceff3] pt-3">
					<p className="m-0 inline-flex items-center gap-1.5 text-[14px] font-medium text-[#0f9d7a]">
						<CheckCircleEmeraldIcon width={15} height={15} className="block shrink-0" />
						Quorum reached - ready to broadcast
					</p>
					<div className="flex shrink-0 items-center gap-2">
						{signButton}
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
				</div>
			) : awaitingEnactment ? (
				<div className="mt-4 border-t border-[#eceff3] pt-3">
					<div className="flex items-start justify-between gap-3">
						<div>
							<p className="m-0 text-[14px] font-medium text-[#0f9d7a]">Reveal confirmed — awaiting ASM enactment</p>
							<p className="m-0 mt-1 text-[12px] text-[#6b7280]">
								Refresh the dashboard after the confirmation delay to see enacted status.
							</p>
						</div>
						{CANCELABLE_AUTHORITIES.includes(proposal.authority) && proposal.cancelProposal === null && (
							<button
								type="button"
								className="shrink-0 rounded-xl border border-[#dc2626] bg-white px-3 py-1.5 text-[13px] font-medium text-[#dc2626] transition hover:bg-[#fef2f2]"
								onClick={(e) => {
									e.stopPropagation()
									onCancelProposal(proposal.actionId)
								}}
							>
								Cancel
							</button>
						)}
					</div>
				</div>
			) : broadcastInProgress ? (
				<div className="mt-4 border-t border-[#eceff3] pt-3">
					<p className="m-0 text-[14px] font-medium text-[#6b7280]">Broadcast in progress</p>
				</div>
			) : hasQuorum ? (
				<div className="mt-4 flex items-center justify-between gap-3 border-t border-[#eceff3] pt-3">
					<p className="m-0 inline-flex items-center gap-1.5 text-[14px] font-medium text-[#0f9d7a]">
						<CheckCircleEmeraldIcon width={15} height={15} className="block shrink-0" />
						Quorum reached
					</p>
					{signButton}
				</div>
			) : (
				<div className="mt-4 flex items-center justify-between gap-3 border-t border-[#eceff3] pt-3">
					<p className="m-0 flex items-center gap-1 text-xs text-[#6b7280]">
						<SignaturePenMutedIcon width={12} height={12} className="block shrink-0" />
						{collectedSignatures} {collectedSignatures === 1 ? 'signature' : 'signatures'} collected
					</p>
					{signButton}
				</div>
			)}
		</div>
	)
}

function buildProposalTitle(proposal: Proposal): string {
	if (proposal.kind === 'cancel') return `Cancel #${proposal.seqNo}`
	return `Proposal #${proposal.seqNo} - ${inferProposalType(proposal)}`
}

function inferProposalType(proposal: Proposal): string {
	if (proposal.kind === 'cancel') return 'Cancel'
	if (proposal.actionType === 'vk_update') return 'Verification key update'
	if (proposal.actionType === 'multisig_update') {
		return proposal.authority.toLowerCase().includes('sequencer') ? 'Sequencer update' : 'Signer update'
	}
	return 'Unknown'
}

type DisplayStatus = ProposalStatus | 'awaiting_enactment'

const STATUS_CONFIG: Record<DisplayStatus, { bg: string; text: string; border: string; dot: string; label: string }> = {
	pending: { bg: '#fffbeb', text: '#d97706', border: '#fde68a', dot: '#d97706', label: 'Pending' },
	approved: { bg: '#eff6ff', text: '#2563eb', border: '#bfdbfe', dot: '#2563eb', label: 'Approved' },
	awaiting_enactment: {
		bg: '#f0fdf9',
		text: '#0f766e',
		border: '#99f6e4',
		dot: '#0f9d7a',
		label: 'Awaiting enactment',
	},
	enacted: { bg: '#ecfdf5', text: '#059669', border: '#a7f3d0', dot: '#059669', label: 'Enacted' },
	canceled: { bg: '#fef2f2', text: '#dc2626', border: '#fecaca', dot: '#dc2626', label: 'Canceled' },
	expired: { bg: '#f9fafb', text: '#6b7280', border: '#e5e7eb', dot: '#6b7280', label: 'Expired' },
}

function StatusBadge({ status }: { status: DisplayStatus }) {
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
