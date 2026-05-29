import { useState, useMemo, useCallback } from 'react'
import type {
	PendingBlockPayoutTx,
	PastBlockPayoutTx,
	ActiveModal,
	BlockPayoutInput,
} from '../model/block-payouts.types'
import { MOCK_PENDING_TXS, MOCK_PAST_TXS } from '../model/block-payouts.mock'

export type Tab = 'pending' | 'past'

export type BlockPayoutsHook = {
	activeTab: Tab
	pendingTxs: PendingBlockPayoutTx[]
	pastTxs: PastBlockPayoutTx[]
	hasConflicts: boolean
	activeModal: ActiveModal
	toast: string | null
	setActiveTab: (tab: Tab) => void
	openSignModal: (txId: string) => void
	openPasteModal: (txId: string) => void
	openCreateModal: () => void
	closeModal: () => void
	handleSign: (txId: string) => void
	handlePasteSignatures: (txId: string, raw: string) => PasteResult
	handleExport: (txId: string) => void
	handleCopySignatures: (txId: string) => void
	handleRebroadcast: (txId: string) => void
	handleCopyRawTx: (txId: string) => void
	handleCreateTx: (inputs: BlockPayoutInput[], feeRateSatPerVb: number) => void
}

export type PasteResult = { ok: true } | { ok: false; invalidSignatures: string[] }

function markConflicts(txs: PendingBlockPayoutTx[]): PendingBlockPayoutTx[] {
	const counts = new Map<string, number>()
	for (const tx of txs) {
		for (const input of tx.inputs) {
			counts.set(input.outpoint, (counts.get(input.outpoint) ?? 0) + 1)
		}
	}
	return txs.map((tx) => ({
		...tx,
		inputs: tx.inputs.map((input) => ({
			...input,
			isConflicting: (counts.get(input.outpoint) ?? 0) > 1,
		})),
	}))
}

function isValidSignature(sig: string): boolean {
	return sig.trim().length >= 64
}

function generateTxId(): string {
	return Array.from({ length: 64 }, () => Math.floor(Math.random() * 16).toString(16)).join('')
}

export function useBlockPayouts(): BlockPayoutsHook {
	const [activeTab, setActiveTab] = useState<Tab>('pending')
	const [rawPending, setRawPending] = useState<PendingBlockPayoutTx[]>(MOCK_PENDING_TXS)
	const [pastTxs, setPastTxs] = useState<PastBlockPayoutTx[]>(MOCK_PAST_TXS)
	const [activeModal, setActiveModal] = useState<ActiveModal>(null)
	const [toast, setToast] = useState<string | null>(null)

	const pendingTxs = useMemo(() => markConflicts(rawPending), [rawPending])
	const hasConflicts = useMemo(() => pendingTxs.some((tx) => tx.inputs.some((i) => i.isConflicting)), [pendingTxs])

	const showToast = useCallback((msg: string) => {
		setToast(msg)
		setTimeout(() => setToast(null), 3500)
	}, [])

	const openSignModal = useCallback((txId: string) => setActiveModal({ kind: 'sign', txId }), [])
	const openPasteModal = useCallback((txId: string) => setActiveModal({ kind: 'paste', txId }), [])
	const openCreateModal = useCallback(() => setActiveModal({ kind: 'create' }), [])
	const closeModal = useCallback(() => setActiveModal(null), [])

	const handleSign = useCallback(
		(txId: string) => {
			setRawPending((prev) => {
				const updated = prev.map((tx) => {
					if (tx.id !== txId) return tx
					const newCount = tx.signaturesReceived + 1
					return { ...tx, signaturesReceived: newCount, signedByCurrentUser: true }
				})

				const signed = updated.find((tx) => tx.id === txId)
				if (signed && signed.signaturesReceived >= signed.signaturesRequired) {
					setRawPending(updated.filter((tx) => tx.id !== txId))
					setPastTxs((prev) => [
						{
							id: txId,
							inputs: signed.inputs,
							status: 'unconfirmed',
							broadcastAt: new Date(),
							rawTx: signed.rawTx,
						},
						...prev,
					])
					showToast('Quorum reached — transaction broadcast to Bitcoin network.')
					setActiveModal(null)
					return updated.filter((tx) => tx.id !== txId)
				}

				setActiveModal(null)
				return updated
			})
		},
		[showToast],
	)

	const handlePasteSignatures = useCallback((txId: string, raw: string): PasteResult => {
		const lines = raw
			.split('\n')
			.map((l) => l.trim())
			.filter(Boolean)
		const invalid = lines.filter((l) => !isValidSignature(l))
		if (invalid.length > 0) {
			return { ok: false, invalidSignatures: invalid }
		}
		setRawPending((prev) =>
			prev.map((tx) => {
				if (tx.id !== txId) return tx
				const newSigs = [...tx.signatures, ...lines]
				return {
					...tx,
					signatures: newSigs,
					signaturesReceived: Math.min(tx.signaturesReceived + lines.length, tx.signaturesRequired),
				}
			}),
		)
		return { ok: true }
	}, [])

	const handleExport = useCallback(
		(txId: string) => {
			const tx = rawPending.find((t) => t.id === txId)
			if (!tx) return
			const blob = new Blob([tx.rawTx], { type: 'text/plain' })
			const url = URL.createObjectURL(blob)
			const a = document.createElement('a')
			a.href = url
			a.download = `block-payout-${txId.slice(0, 8)}.txt`
			a.click()
			URL.revokeObjectURL(url)
		},
		[rawPending],
	)

	const handleCopySignatures = useCallback(
		(txId: string) => {
			const tx = rawPending.find((t) => t.id === txId)
			if (!tx) return
			void navigator.clipboard.writeText(tx.signatures.join('\n'))
			showToast('Signatures copied to clipboard.')
		},
		[rawPending, showToast],
	)

	const handleRebroadcast = useCallback(
		(_txId: string) => {
			showToast('Transaction rebroadcast successfully.')
		},
		[showToast],
	)

	const handleCopyRawTx = useCallback(
		(txId: string) => {
			const tx = pastTxs.find((t) => t.id === txId)
			if (!tx) return
			void navigator.clipboard.writeText(tx.rawTx)
			showToast('Transaction copied to clipboard.')
		},
		[pastTxs, showToast],
	)

	const handleCreateTx = useCallback(
		(inputs: BlockPayoutInput[], feeRateSatPerVb: number) => {
			const newId = `bp-pending-${generateTxId().slice(0, 8)}`
			const now = new Date()
			const newTx: PendingBlockPayoutTx = {
				id: newId,
				inputs,
				signaturesReceived: 1,
				signaturesRequired: 3,
				signedByCurrentUser: true,
				expiresAt: new Date(now.getTime() + 4 * 24 * 60 * 60 * 1000),
				createdAt: now,
				rawTx: `02000000${feeRateSatPerVb.toString(16).padStart(8, '0')}${newId}`,
				signatures: [`304402200102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f200220${newId}`],
			}
			setRawPending((prev) => [newTx, ...prev])
			setActiveModal(null)
			setActiveTab('pending')
			showToast('Transaction created and signed successfully.')
		},
		[showToast],
	)

	return {
		activeTab,
		pendingTxs,
		pastTxs,
		hasConflicts,
		activeModal,
		toast,
		setActiveTab,
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
	}
}
