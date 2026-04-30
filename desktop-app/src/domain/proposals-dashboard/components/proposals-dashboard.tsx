import type { Proposal } from '@/api/proposals'

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
	return (
		<section className="w-full max-w-[1080px]">
			<div className="mb-4 flex items-center justify-between">
				<div>
					<h1 className="m-0 font-['BIZ_UDPMincho'] text-[2.1rem] font-normal leading-[1.1] text-[#0a0a0a]">
						Proposals
					</h1>
					<p className="m-0 mt-1 text-sm text-[#6b7280]">
						Proposals you can sign, broadcast, or review under {authorityLabel}.
					</p>
				</div>
				<button
					type="button"
					className="rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-4 py-2 text-sm font-medium text-white"
					onClick={onCreateProposal}
				>
					+ Create proposal
				</button>
			</div>

			{error ? (
				<div className="rounded-xl border border-[#fecaca] bg-[#fef2f2] px-4 py-3 text-sm text-[#991b1b]">
					<p className="m-0">{error}</p>
					<button
						type="button"
						className="mt-2 rounded-md border border-[#991b1b] bg-white px-3 py-1 text-xs font-medium text-[#991b1b]"
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
			) : (
				<div className="space-y-5">
					<ProposalGroup
						title={`Quorum reached · ${quorumReached.length}`}
						titleColorClassName="text-[#0f766e]"
						emptyMessage="No proposal has reached quorum yet."
						proposals={quorumReached}
					/>
					<ProposalGroup
						title={`Pending · ${pending.length}`}
						titleColorClassName="text-[#6b7280]"
						emptyMessage="No proposals are currently collecting signatures."
						proposals={pending}
					/>
					<ProposalGroup
						title={`Executed & Canceled · ${executedOrCanceled.length}`}
						titleColorClassName="text-[#6b7280]"
						emptyMessage="No executed or canceled proposals yet."
						proposals={executedOrCanceled}
					/>
					<ProposalGroup
						title={`Expired / Skipped · ${expiredOrSkipped.length}`}
						titleColorClassName="text-[#6b7280]"
						emptyMessage="No expired proposals."
						proposals={expiredOrSkipped}
					/>
				</div>
			)}
		</section>
	)
}

function ProposalGroup({
	title,
	titleColorClassName,
	emptyMessage,
	proposals,
}: {
	title: string
	titleColorClassName: string
	emptyMessage: string
	proposals: Proposal[]
}) {
	return (
		<div>
			<p className={`m-0 text-xs font-semibold uppercase tracking-[0.12em] ${titleColorClassName}`}>{title}</p>
			{proposals.length === 0 ? (
				<div className="mt-2 rounded-xl border border-[#e5e7eb] bg-white px-4 py-3 text-sm text-[#9ca3af]">
					{emptyMessage}
				</div>
			) : (
				<div className="mt-2 space-y-2">
					{proposals.map((proposal) => (
						<div key={proposal.actionId} className="rounded-xl border border-[#e5e7eb] bg-white px-4 py-3">
							<div className="flex items-center justify-between gap-3">
								<p className="m-0 font-mono text-xs text-[#374151]">{proposal.actionId}</p>
								<span className="rounded-md border border-[#dbeafe] bg-[#eff6ff] px-2 py-1 text-xs font-medium text-[#2563eb]">
									{proposal.status}
								</span>
							</div>
							<p className="m-0 mt-2 text-xs text-[#6b7280]">
								seqNo #{proposal.seqNo} · signatures {proposal.signatures.length}
							</p>
						</div>
					))}
				</div>
			)}
		</div>
	)
}
