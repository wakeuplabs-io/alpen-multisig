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
	quorumReached: Proposal[]
	pending: Proposal[]
	executedOrCanceled: Proposal[]
	expiredOrSkipped: Proposal[]
	isLoading: boolean
	error: string | null
	onRetry: () => void
	onCreateProposal: () => void
}

export function ProposalsDashboard({
	authorityLabel,
	quorumReached,
	pending,
	executedOrCanceled,
	expiredOrSkipped,
	isLoading,
	error,
	onRetry,
	onCreateProposal,
}: Props) {
	const isEmpty =
		!isLoading &&
		!error &&
		quorumReached.length + pending.length + executedOrCanceled.length + expiredOrSkipped.length === 0

	return (
		<section className="w-full max-w-[800px]">
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
					/>
					<ProposalGroup
						title="Pending"
						count={pending.length}
						groupIcon={<ClockAmberIcon width={14} height={14} className="block" />}
						initialOpen
						emptyMessage="No proposals are currently collecting signatures."
						proposals={pending}
					/>
					<ProposalGroup
						title="Executed & Canceled"
						count={executedOrCanceled.length}
						groupIcon={null}
						initialOpen={false}
						emptyMessage="No executed or canceled proposals yet."
						proposals={executedOrCanceled}
					/>
					<ProposalGroup
						title="Expired / Skipped"
						count={expiredOrSkipped.length}
						groupIcon={null}
						initialOpen={false}
						emptyMessage="No expired proposals."
						proposals={expiredOrSkipped}
					/>
				</div>
			)}
		</section>
	)
}

function EmptyState({
	authorityLabel,
	onCreateProposal,
}: {
	authorityLabel: string
	onCreateProposal: () => void
}) {
	return (
		<div className="rounded-xl border border-[#e5e7eb] bg-white px-6 py-16 text-center">
			<div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full border border-[#e5e7eb] bg-[#f8f8fb] text-[#9ca3af]">
				<FileTextMutedIcon width={22} height={22} className="block" />
			</div>
			<p className="m-0 font-['BIZ_UDPMincho'] text-[22px] font-normal text-[#0a0a0a]">
				No proposals for {authorityLabel}
			</p>
			<p className="m-0 mt-2 text-[13px] text-[#6b7280]">
				Create the first proposal to begin collecting signatures.
			</p>
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
}: {
	title: string
	count: number
	groupIcon: ReactNode
	initialOpen: boolean
	emptyMessage: string
	proposals: Proposal[]
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
							<ProposalCard key={proposal.actionId} proposal={proposal} />
						))}
					</div>
				))}
		</section>
	)
}

function ProposalCard({ proposal }: { proposal: Proposal }) {
	const [hovered, setHovered] = useState(false)

	return (
		<div
			style={{
				background: '#fff',
				border: `1px solid ${hovered ? 'var(--color-accent-border)' : 'var(--color-border)'}`,
				borderRadius: 12,
				padding: '18px 20px',
				cursor: 'pointer',
				transition: 'all 150ms ease',
				transform: hovered ? 'translateY(-1px)' : 'translateY(0)',
				boxShadow: hovered ? 'var(--shadow-card)' : 'none',
			}}
			onMouseEnter={() => setHovered(true)}
			onMouseLeave={() => setHovered(false)}
		>
			<div className="flex items-start justify-between gap-3">
				<div className="min-w-0 flex-1">
					<p className="m-0 font-mono text-[11px] text-[#9ca3af]">
						#{proposal.seqNo} · {proposal.authority}
					</p>
					<p className="m-0 mt-1 break-all font-mono text-xs text-[#374151]">{proposal.actionId}</p>
				</div>
				<StatusBadge status={proposal.status} />
			</div>
			<p className="m-0 mt-2.5 flex items-center gap-1 text-xs text-[#6b7280]">
				<SignaturePenMutedIcon width={12} height={12} className="block shrink-0" />
				{proposal.signatures.length} {proposal.signatures.length === 1 ? 'signature' : 'signatures'} collected
			</p>
		</div>
	)
}

const STATUS_CONFIG: Record<
	ProposalStatus,
	{ bg: string; text: string; border: string; dot: string; label: string }
> = {
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
