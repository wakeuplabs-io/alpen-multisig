import { useState } from 'react'
import type { BroadcastStatus, ProposalStatus } from '@/api/proposals'
import { resolveBroadcastStatus } from '@/api/proposals'
import type { PastedSignature } from '@/domain/proposal-detail/model/pasted-signature'

export type ImportBroadcastState = {
	broadcastStatus: BroadcastStatus | null
	commitTxid: string | null
	revealTxid: string | null
	proposalStatus: ProposalStatus | null
}

type Props = {
	existingSignatures: Array<{ signerPubkey: string; signatureHex: string }>
	existingBroadcastStatus: BroadcastStatus
	existingCommitTxid: string | null
	existingRevealTxid: string | null
	onImport: (newSigs: PastedSignature[], broadcastState: ImportBroadcastState) => void
	onClose: () => void
}

type ParseResult =
	| {
			ok: true
			newSigs: PastedSignature[]
			skipped: number
			broadcastState: ImportBroadcastState
	  }
	| { ok: false; error: string }

type VerifyState = { status: 'idle' } | { status: 'loading' } | { status: 'error'; message: string }

const BROADCAST_ORDER: BroadcastStatus[] = [
	'idle',
	'commit_broadcasted',
	'commit_confirmed',
	'reveal_broadcasted',
	'reveal_confirmed',
]

const PROPOSAL_STATUSES: ProposalStatus[] = ['pending', 'approved', 'enacted', 'canceled', 'expired']

function parseBundleJson(
	raw: string,
	existingSignatures: Array<{ signerPubkey: string; signatureHex: string }>,
	existingBroadcastStatus: BroadcastStatus,
	existingCommitTxid: string | null,
	existingRevealTxid: string | null,
): ParseResult {
	let parsed: unknown
	try {
		parsed = JSON.parse(raw)
	} catch {
		return { ok: false, error: 'Invalid JSON — paste a proposal bundle or signatures array.' }
	}

	let candidates: unknown[] = []
	let broadcastStatus: BroadcastStatus | null = null
	let commitTxid: string | null = null
	let revealTxid: string | null = null
	let proposalStatus: ProposalStatus | null = null

	if (Array.isArray(parsed)) {
		candidates = parsed
	} else if (typeof parsed === 'object' && parsed !== null) {
		const obj = parsed as Record<string, unknown>
		if (!Array.isArray(obj['signatures'])) {
			return { ok: false, error: 'Expected a signatures array or an object with a "signatures" field.' }
		}
		candidates = obj['signatures'] as unknown[]

		// Parse broadcastStatus — only advance if bundle is ahead of DB
		const rawBs = obj['broadcastStatus'] ?? obj['broadcast_status']
		if (typeof rawBs === 'string' && (BROADCAST_ORDER as string[]).includes(rawBs)) {
			const bundleOrder = BROADCAST_ORDER.indexOf(rawBs as BroadcastStatus)
			const existingOrder = BROADCAST_ORDER.indexOf(existingBroadcastStatus)
			if (bundleOrder > existingOrder) {
				broadcastStatus = rawBs as BroadcastStatus
			}
		}

		// Parse txids — sync if present and absent/different in DB
		const rawCommit = obj['commitTxid'] ?? obj['commit_txid']
		if (typeof rawCommit === 'string' && rawCommit !== existingCommitTxid) {
			commitTxid = rawCommit
		}
		const rawReveal = obj['revealTxid'] ?? obj['reveal_txid']
		if (typeof rawReveal === 'string' && rawReveal !== existingRevealTxid) {
			revealTxid = rawReveal
		}

		// Parse proposal status
		const rawStatus = obj['status']
		if (typeof rawStatus === 'string' && (PROPOSAL_STATUSES as string[]).includes(rawStatus)) {
			proposalStatus = rawStatus as ProposalStatus
		}
	} else {
		return { ok: false, error: 'Unrecognized format — paste a proposal bundle or signatures array.' }
	}

	const validSigs: PastedSignature[] = []
	for (const item of candidates) {
		const entry = item as Record<string, unknown>
		if (
			typeof item === 'object' &&
			item !== null &&
			typeof entry['signerPubkey'] === 'string' &&
			typeof entry['signatureHex'] === 'string'
		) {
			validSigs.push({
				signerPubkey: entry['signerPubkey'] as string,
				signatureHex: entry['signatureHex'] as string,
			})
		}
	}

	if (validSigs.length === 0 && broadcastStatus === null && commitTxid === null && revealTxid === null) {
		return {
			ok: false,
			error: 'No valid signatures found and no new broadcast state detected. Nothing to sync.',
		}
	}

	const existingKeys = new Set(existingSignatures.map((s) => s.signerPubkey.toLowerCase()))
	const newSigs = validSigs.filter((s) => !existingKeys.has(s.signerPubkey.toLowerCase()))
	const skipped = validSigs.length - newSigs.length

	return { ok: true, newSigs, skipped, broadcastState: { broadcastStatus, commitTxid, revealTxid, proposalStatus } }
}

const BROADCAST_LABELS: Record<BroadcastStatus, string> = {
	idle: 'Idle',
	commit_broadcasted: 'Commit broadcasted',
	commit_confirmed: 'Commit confirmed',
	reveal_broadcasted: 'Reveal broadcasted',
	reveal_confirmed: 'Reveal confirmed',
	failed: 'Failed',
}

export function ImportBundleModal({
	existingSignatures,
	existingBroadcastStatus,
	existingCommitTxid,
	existingRevealTxid,
	onImport,
	onClose,
}: Props) {
	const [raw, setRaw] = useState('')
	const [parseResult, setParseResult] = useState<ParseResult | null>(null)
	const [verifyState, setVerifyState] = useState<VerifyState>({ status: 'idle' })
	const [verifiedStatus, setVerifiedStatus] = useState<{
		broadcastStatus: BroadcastStatus
		commitConfirmations: number | null
		revealConfirmations: number | null
	} | null>(null)

	function handleChange(value: string) {
		setRaw(value)
		setParseResult(null)
		setVerifyState({ status: 'idle' })
		setVerifiedStatus(null)
	}

	function handlePreview() {
		if (raw.trim().length === 0) return
		setVerifyState({ status: 'idle' })
		setVerifiedStatus(null)
		setParseResult(
			parseBundleJson(raw, existingSignatures, existingBroadcastStatus, existingCommitTxid, existingRevealTxid),
		)
	}

	async function handleVerifyOnBitcoin() {
		if (!preview) return
		const { commitTxid, revealTxid } = preview.broadcastState
		if (!commitTxid && !revealTxid) return

		setVerifyState({ status: 'loading' })
		const res = await resolveBroadcastStatus({
			commitTxid: commitTxid ?? undefined,
			revealTxid: revealTxid ?? undefined,
		})

		if (!res.ok) {
			setVerifyState({ status: 'error', message: res.error })
			return
		}

		setVerifiedStatus({
			broadcastStatus: res.data.broadcastStatus as BroadcastStatus,
			commitConfirmations: res.data.commitConfirmations,
			revealConfirmations: res.data.revealConfirmations,
		})
		setVerifyState({ status: 'idle' })

		// Promote broadcastStatus in the parse result if Bitcoin confirms a more advanced state
		const verifiedOrder = BROADCAST_ORDER.indexOf(res.data.broadcastStatus as BroadcastStatus)
		const bundleOrder = BROADCAST_ORDER.indexOf(preview.broadcastState.broadcastStatus ?? 'idle')
		if (verifiedOrder > bundleOrder) {
			setParseResult({
				...preview,
				broadcastState: {
					...preview.broadcastState,
					broadcastStatus: res.data.broadcastStatus as BroadcastStatus,
				},
			})
		}
	}

	function handleImport() {
		if (!parseResult?.ok) return
		onImport(parseResult.newSigs, parseResult.broadcastState)
		onClose()
	}

	const preview = parseResult?.ok ? parseResult : null
	const { broadcastState } = preview ?? { broadcastState: null }

	const hasTxids = Boolean(broadcastState?.commitTxid ?? broadcastState?.revealTxid)
	const hasSomethingToSync =
		preview !== null &&
		(preview.newSigs.length > 0 ||
			broadcastState?.broadcastStatus !== null ||
			broadcastState?.commitTxid !== null ||
			broadcastState?.revealTxid !== null)

	return (
		<Backdrop onClose={onClose}>
			<div className="flex flex-col gap-5">
				<div>
					<h2 className="m-0 font-display text-display-sm font-normal text-[#0a0a0a]">Import bundle</h2>
					<p className="m-0 mt-1 text-body-sm text-[#6b7280]">
						Paste a complete proposal bundle. New signatures, broadcast TXIDs, and execution state will be synced.
					</p>
				</div>

				<textarea
					className="h-40 w-full resize-none rounded-xl border border-[#e5e7eb] bg-bg-base px-4 py-3 font-mono text-label text-[#0a0a0a] outline-none transition focus:border-[#0a0a0a] focus:bg-white"
					placeholder={
						'Paste JSON bundle here…\n{ "signatures": [...], "broadcastStatus": "reveal_confirmed", "commitTxid": "…" }'
					}
					value={raw}
					onChange={(e) => handleChange(e.target.value)}
					spellCheck={false}
				/>

				{parseResult !== null && !parseResult.ok && (
					<div className="rounded-xl border border-[#fecaca] bg-[#fef2f2] px-4 py-3">
						<p className="m-0 text-label font-medium text-[#dc2626]">{parseResult.error}</p>
					</div>
				)}

				{verifyState.status === 'error' && (
					<div className="rounded-xl border border-[#fecaca] bg-[#fef2f2] px-4 py-3">
						<p className="m-0 text-label font-medium text-[#dc2626]">Bitcoin check failed: {verifyState.message}</p>
					</div>
				)}

				{preview !== null && (
					<div className="space-y-2 rounded-xl border border-[#a7f3d0] bg-[#ecfdf5] px-4 py-3">
						{preview.newSigs.length > 0 && (
							<div className="space-y-1">
								<p className="m-0 text-label font-semibold text-[#065f46]">
									{preview.newSigs.length} new signature{preview.newSigs.length !== 1 ? 's' : ''} to sync
									{preview.skipped > 0 && (
										<span className="ml-2 font-normal text-[#6b7280]">
											({preview.skipped} already present, skipped)
										</span>
									)}
								</p>
								{preview.newSigs.map((s, i) => (
									<p key={i} className="m-0 truncate font-mono text-mono-sm text-[#374151]">
										{s.signerPubkey.slice(0, 14)}…{s.signerPubkey.slice(-8)}
									</p>
								))}
							</div>
						)}

						{(broadcastState?.broadcastStatus !== null ||
							broadcastState?.commitTxid !== null ||
							broadcastState?.revealTxid !== null) && (
							<div className="space-y-0.5">
								{broadcastState?.broadcastStatus && (
									<p className="m-0 text-label font-semibold text-[#065f46]">
										Broadcast state →{' '}
										<span className="font-normal">{BROADCAST_LABELS[broadcastState.broadcastStatus]}</span>
									</p>
								)}
								{broadcastState?.commitTxid && (
									<div className="flex items-center gap-1.5">
										<p className="m-0 min-w-0 flex-1 truncate font-mono text-mono-sm text-[#374151]">
											commit: {broadcastState.commitTxid}
										</p>
										{verifiedStatus?.commitConfirmations !== null && verifiedStatus !== null && (
											<span className="shrink-0 text-mono-sm text-[#059669]">
												{verifiedStatus.commitConfirmations} conf
											</span>
										)}
									</div>
								)}
								{broadcastState?.revealTxid && (
									<div className="flex items-center gap-1.5">
										<p className="m-0 min-w-0 flex-1 truncate font-mono text-mono-sm text-[#374151]">
											reveal: {broadcastState.revealTxid}
										</p>
										{verifiedStatus?.revealConfirmations !== null && verifiedStatus !== null && (
											<span className="shrink-0 text-mono-sm text-[#059669]">
												{verifiedStatus.revealConfirmations} conf
											</span>
										)}
									</div>
								)}
							</div>
						)}

						{broadcastState?.proposalStatus === 'enacted' && (
							<p className="m-0 text-label font-semibold text-[#065f46]">
								Execution state → <span className="font-normal">Enacted (will verify on-chain)</span>
							</p>
						)}

						{!hasSomethingToSync && (
							<p className="m-0 text-label text-[#6b7280]">DB is already up to date — nothing to sync.</p>
						)}
					</div>
				)}

				<div className="flex items-center justify-between gap-2.5">
					{/* Verify on Bitcoin — shown when txids are detected */}
					<div className="flex-1">
						{hasTxids && (
							<button
								type="button"
								disabled={verifyState.status === 'loading'}
								onClick={() => void handleVerifyOnBitcoin()}
								className="inline-flex items-center gap-1.5 rounded-lg border border-[#e5e7eb] bg-white px-3 py-1.5 text-label font-medium text-[#6b7280] transition hover:border-[#d1d5db] hover:text-[#111827] disabled:cursor-not-allowed disabled:opacity-50"
							>
								{verifyState.status === 'loading' ? 'Checking…' : 'Verify on Bitcoin'}
							</button>
						)}
					</div>

					<div className="flex items-center gap-2.5">
						<button
							type="button"
							className="rounded-lg border border-[#e5e7eb] bg-white px-4 py-2 text-body-sm font-medium text-[#374151] transition hover:bg-[#f9fafb]"
							onClick={onClose}
						>
							Cancel
						</button>

						{preview === null && (
							<button
								type="button"
								disabled={raw.trim().length === 0}
								className="inline-flex items-center rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-4 py-2 text-body-sm font-medium text-white transition hover:bg-[#2a2a2a] active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-50"
								onClick={handlePreview}
							>
								Preview
							</button>
						)}

						{preview !== null && !hasSomethingToSync && (
							<button
								type="button"
								className="inline-flex items-center rounded-lg border border-[#e5e7eb] bg-white px-4 py-2 text-body-sm font-medium text-[#374151] transition hover:bg-[#f9fafb]"
								onClick={onClose}
							>
								Close
							</button>
						)}

						{hasSomethingToSync && (
							<button
								type="button"
								className="inline-flex items-center rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-4 py-2 text-body-sm font-medium text-white transition hover:bg-[#2a2a2a] active:scale-[0.98]"
								onClick={handleImport}
							>
								Sync
							</button>
						)}
					</div>
				</div>
			</div>
		</Backdrop>
	)
}

function Backdrop({ children, onClose }: { children: React.ReactNode; onClose: () => void }) {
	return (
		<div
			className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 px-4"
			onClick={(e) => {
				if (e.target === e.currentTarget) onClose()
			}}
		>
			<div className="w-full max-w-120 rounded-2xl border border-[#e5e7eb] bg-white p-6 shadow-xl">{children}</div>
		</div>
	)
}
