import type { BlockPayoutsHook, Tab } from '../hooks/use-block-payouts'
import { PendingTransactionsList } from './pending-transactions-list'
import { PastTransactionsList } from './past-transactions-list'
import { SignBlockPayoutModal } from './sign-block-payout-modal'
import { PasteSignaturesModal } from './paste-signatures-modal'
import { CreateBlockPayoutModal } from './create-block-payout-modal'

type Props = Omit<BlockPayoutsHook, 'setActiveTab'> & {
	onTabChange: (tab: Tab) => void
}

const TAB_LABELS: { id: Tab; label: string }[] = [
	{ id: 'pending', label: 'Pending' },
	{ id: 'past', label: 'Past' },
]

export function BlockPayoutsDashboard({
	activeTab,
	pendingTxs,
	pastTxs,
	hasConflicts,
	activeModal,
	toast,
	onTabChange,
	openSignModal,
	openPasteModal,
	openCreateModal,
	closeModal,
	handleSign,
	handlePasteSignatures,
	handleExport,
	handleCopySignatures,
	handleRebroadcast,
	handleCopyRawTx,
	handleCreateTx,
}: Props) {
	const signTx = activeModal?.kind === 'sign' ? (pendingTxs.find((t) => t.id === activeModal.txId) ?? null) : null
	const pasteTxId = activeModal?.kind === 'paste' ? activeModal.txId : null

	return (
		<section className="mx-auto w-full max-w-200">
			{/* Page header */}
			<div className="mb-6 flex items-end justify-between gap-4">
				<div>
					<h1 className="m-0 font-display text-display-md font-normal leading-[1.2] tracking-[-0.005em] text-[#0a0a0a]">
						Block Payouts
					</h1>
					<p className="m-0 mt-1 text-body-sm leading-normal text-[#6b7280]">
						Manage block_payouts transactions for the Payout Administrator multisig.
					</p>
				</div>
				<button
					type="button"
					className="inline-flex shrink-0 items-center gap-1.5 rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-4 py-2 text-body font-medium text-white transition hover:bg-[#2a2a2a] active:scale-[0.98]"
					onClick={openCreateModal}
				>
					<span aria-hidden="true">+</span>
					Block payouts
				</button>
			</div>

			{/* Tabs */}
			<div className="mb-5 flex gap-0.5 rounded-xl border border-[#e5e7eb] bg-bg-base p-1">
				{TAB_LABELS.map(({ id, label }) => (
					<button
						key={id}
						type="button"
						className="flex-1 rounded-lg py-2 text-body-sm font-medium transition"
						style={
							activeTab === id
								? { background: '#fff', color: '#0a0a0a', boxShadow: '0 1px 3px rgba(0,0,0,0.08)' }
								: { background: 'transparent', color: '#6b7280' }
						}
						onClick={() => onTabChange(id)}
					>
						{label}
						{id === 'pending' && pendingTxs.length > 0 && (
							<span
								className="ml-1.5 rounded-full px-1.5 py-0.5 text-[10px] font-semibold"
								style={
									activeTab === 'pending'
										? { background: '#0a0a0a', color: '#fff' }
										: { background: '#e5e7eb', color: '#6b7280' }
								}
							>
								{pendingTxs.length}
							</span>
						)}
					</button>
				))}
			</div>

			{/* Tab content */}
			{activeTab === 'pending' ? (
				<PendingTransactionsList
					txs={pendingTxs}
					hasConflicts={hasConflicts}
					onSign={openSignModal}
					onPasteSignatures={openPasteModal}
					onExport={handleExport}
					onCopySignatures={handleCopySignatures}
				/>
			) : (
				<PastTransactionsList txs={pastTxs} onRebroadcast={handleRebroadcast} onCopyRawTx={handleCopyRawTx} />
			)}

			{/* Modals */}
			{activeModal?.kind === 'sign' && signTx && (
				<SignBlockPayoutModal tx={signTx} onSign={() => handleSign(signTx.id)} onClose={closeModal} />
			)}

			{activeModal?.kind === 'paste' && pasteTxId && (
				<PasteSignaturesModal txId={pasteTxId} onSubmit={handlePasteSignatures} onClose={closeModal} />
			)}

			{activeModal?.kind === 'create' && (
				<CreateBlockPayoutModal walletBalanceSats={500_000} onConfirm={handleCreateTx} onClose={closeModal} />
			)}

			{/* Toast */}
			{toast && (
				<div
					className="fixed bottom-6 left-1/2 z-50 -translate-x-1/2 rounded-xl border border-[#a7f3d0] bg-[#ecfdf5] px-5 py-3 text-body-sm font-medium text-[#065f46] shadow-lg"
					role="status"
					aria-live="polite"
				>
					{toast}
				</div>
			)}
		</section>
	)
}
