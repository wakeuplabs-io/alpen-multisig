import { useEffect, useRef, useState } from 'react'
import { getMultisigConfig } from '@/api/asm-state'
import {
	broadcastManualProposal,
	prepareBroadcastManual,
	type BroadcastManualInput,
	type PrepareBroadcastResult,
} from '@/api/proposals'
import { computeSighash, decodeActionHex } from '@/api/signing'
import { actionTypeFromDecoded } from '@/domain/manual-proposal/model/action-type-from-decoded'
import { deviceSigningDisplay, type DeviceSigningDisplay } from '@/lib/device-signing-display'
import { useDeviceSigningMessage } from '@/hooks/use-device-signing-message'
import type { DecodedProposalData } from '@/domain/proposal-detail/hooks/use-decoded-proposal'
import type {
	ManualBundleJson,
	ManualImportData,
	ManualImportErrors,
	ManualImportForm,
	ManualSignature,
	ManualStep,
} from '@/domain/manual-proposal/model/manual-proposal.types'
import type { PastedSignature } from '@/domain/proposal-detail/model/pasted-signature'
import { useSession } from '@/hooks/use-session'

const AUTHORITIES = ['alpen_admin', 'strata_admin', 'sequencer_manager', 'security_council', 'payout_admin'] as const

function isValidHex(s: string): boolean {
	const clean = s.trim().replace(/^0x/i, '')
	return clean.length > 0 && clean.length % 2 === 0 && /^[0-9a-fA-F]+$/.test(clean)
}

function normalizeHex(s: string): string {
	return s.trim().replace(/^0x/i, '').toLowerCase()
}

export type BroadcastPhase = 'idle' | 'preparing' | 'confirming' | 'broadcasting' | 'done' | 'error'

/** `feeRateSatPerKvb === null` while fee presets load — the broadcast step stays blocked until ready. */
export function useManualProposal(initialBundle: ManualBundleJson | null, feeRateSatPerKvb: number | null) {
	const { adapter, wallet } = useSession()

	const [step, setStep] = useState<ManualStep>('import')

	// Step 1
	const [importForm, setImportForm] = useState<ManualImportForm>({ actionHex: '', seqNo: '', authority: '' })
	const [importErrors, setImportErrors] = useState<ManualImportErrors>({})
	const [isValidating, setIsValidating] = useState(false)
	const [importData, setImportData] = useState<ManualImportData | null>(null)

	// Step 2
	const [decodedData, setDecodedData] = useState<DecodedProposalData>({
		signerSetChange: null,
		allSigners: [],
		isLoading: false,
	})
	const [localSignatures, setLocalSignatures] = useState<ManualSignature[]>([])
	const [requiredSignatures, setRequiredSignatures] = useState<number | null>(null)
	const [isSigning, setIsSigning] = useState(false)
	const [signError, setSignError] = useState<string | null>(null)

	// Step 3
	const [broadcastPhase, setBroadcastPhase] = useState<BroadcastPhase>('idle')
	const [broadcastBundle, setBroadcastBundle] = useState<PrepareBroadcastResult | null>(null)
	const [broadcastError, setBroadcastError] = useState<string | null>(null)
	const [commitTxid, setCommitTxid] = useState<string | null>(null)
	const [revealTxid, setRevealTxid] = useState<string | null>(null)

	const hasQuorum = requiredSignatures !== null && localSignatures.length >= requiredSignatures

	// No device renders the SPS-65 sighash, so the offline flow also resolves what the device does
	// render (canonical message / its SHA-256) for the imported action — the signer has nothing else
	// to compare the device screen against on this path (#402). Resolved here rather than in the
	// component because this hook owns both `importData` and the session adapter.
	const { message: deviceMessage, messageHash: deviceMessageHash } = useDeviceSigningMessage(
		importData?.seqNo ?? null,
		importData?.actionHex ?? null,
	)
	const deviceDisplay: DeviceSigningDisplay = deviceSigningDisplay(adapter.vendor, {
		message: deviceMessage,
		messageHash: deviceMessageHash,
	})

	// When importData is set (step advances to sign-collect), fetch decoded data and config.
	useEffect(() => {
		if (importData === null) return

		let cancelled = false
		setDecodedData((prev) => ({ ...prev, isLoading: true }))

		void Promise.all([decodeActionHex(importData.actionHex), getMultisigConfig(importData.authority)]).then(
			([actionRes, configRes]) => {
				if (cancelled) return

				setDecodedData((prev) => ({
					...prev,
					isLoading: false,
					allSigners: configRes.ok ? configRes.data.signers : [],
					signerSetChange: (() => {
						if (!actionRes.ok || actionRes.data.kind !== 'multisig_update') return null
						const decoded = actionRes.data
						const signers = configRes.ok ? configRes.data.signers : []
						const removeSet = new Set(decoded.removeKeys.map((k) => k.toLowerCase()))
						const beforeSigners = signers
						const afterSigners = [...signers.filter((k) => !removeSet.has(k.toLowerCase())), ...decoded.addKeys]

						const rowMap = new Map<
							string,
							{ pubkey: string; inBefore: boolean; inAfter: boolean; isAdded: boolean; isRemoved: boolean }
						>()
						for (const k of beforeSigners) {
							rowMap.set(k.toLowerCase(), {
								pubkey: k,
								inBefore: true,
								inAfter: false,
								isAdded: false,
								isRemoved: false,
							})
						}
						for (const k of afterSigners) {
							const lower = k.toLowerCase()
							const existing = rowMap.get(lower)
							if (existing) {
								existing.inAfter = true
							} else {
								rowMap.set(lower, { pubkey: k, inBefore: false, inAfter: true, isAdded: true, isRemoved: false })
							}
						}
						for (const row of rowMap.values()) {
							if (row.inBefore && !row.inAfter) row.isRemoved = true
						}

						return {
							rows: Array.from(rowMap.values()),
							thresholdBefore: configRes.ok ? configRes.data.threshold : null,
							thresholdAfter: decoded.newThreshold,
						}
					})(),
				}))

				if (configRes.ok) {
					setRequiredSignatures(configRes.data.threshold)
				}
			},
		)

		return () => {
			cancelled = true
		}
	}, [importData])

	function handleImportChange(field: keyof ManualImportForm, value: string) {
		setImportForm((prev) => ({ ...prev, [field]: value }))
		setImportErrors((prev) => ({ ...prev, [field]: undefined }))
	}

	async function handleImportSubmit() {
		const errors: ManualImportErrors = {}
		const rawHex = normalizeHex(importForm.actionHex)

		if (!rawHex) {
			errors.actionHex = 'Action hex is required'
		} else if (!isValidHex(importForm.actionHex)) {
			errors.actionHex = 'Must be valid hex (even-length, hex chars only)'
		}

		const seqNoNum = parseInt(importForm.seqNo, 10)
		if (importForm.seqNo.trim() === '' || isNaN(seqNoNum) || seqNoNum < 0) {
			errors.seqNo = 'Must be a non-negative integer'
		}

		if (!importForm.authority || !(AUTHORITIES as readonly string[]).includes(importForm.authority)) {
			errors.authority = 'Select an authority'
		}

		if (Object.keys(errors).some((k) => errors[k as keyof ManualImportErrors] !== undefined)) {
			setImportErrors(errors)
			return
		}

		setIsValidating(true)
		setImportErrors({})

		try {
			const [decodeRes, sighashRes, configRes] = await Promise.all([
				decodeActionHex(rawHex),
				computeSighash(seqNoNum, rawHex),
				getMultisigConfig(importForm.authority),
			])

			if (!decodeRes.ok) {
				setImportErrors({ actionHex: `Decode failed: ${decodeRes.error}` })
				return
			}
			if (decodeRes.data.kind === 'unknown') {
				setImportErrors({ actionHex: 'Unknown action kind — cannot decode this hex' })
				return
			}
			if (!sighashRes.ok) {
				setImportErrors({ actionHex: `Sighash computation failed: ${sighashRes.error}` })
				return
			}

			const pubkey = wallet?.publicKeyHex?.toLowerCase()
			if (pubkey && configRes.ok && !configRes.data.signers.some((s) => s.toLowerCase() === pubkey)) {
				setImportErrors({ authority: `Your key is not a signer for ${importForm.authority}` })
				return
			}

			setImportData({
				actionHex: rawHex,
				seqNo: seqNoNum,
				authority: importForm.authority,
				sighashHex: sighashRes.data.sighashHex,
				actionType: actionTypeFromDecoded(decodeRes.data),
			})
			setStep('sign-collect')
		} finally {
			setIsValidating(false)
		}
	}

	async function handleSign() {
		if (!importData) return
		setIsSigning(true)
		setSignError(null)
		try {
			const signed = await adapter.signSighash(importData.sighashHex, {
				seqno: importData.seqNo,
				actionHex: importData.actionHex,
			})
			setLocalSignatures((prev) => {
				const exists = prev.some((s) => s.signerPubkey.toLowerCase() === signed.publicKeyHex.toLowerCase())
				if (exists) return prev
				return [...prev, { signerPubkey: signed.publicKeyHex, signatureHex: signed.signatureHex, source: 'local' }]
			})
		} catch (e) {
			setSignError(String(e))
		} finally {
			setIsSigning(false)
		}
	}

	async function processBundle(bundle: ManualBundleJson) {
		const rawHex = normalizeHex(bundle.actionHex)

		const errors: ManualImportErrors = {}
		if (!rawHex || !isValidHex(rawHex)) errors.actionHex = 'Bundle: missing or invalid actionHex'
		if (bundle.seqNo < 0 || !Number.isInteger(bundle.seqNo)) errors.seqNo = 'Bundle: invalid seqNo'
		if (!bundle.authority || !(AUTHORITIES as readonly string[]).includes(bundle.authority))
			errors.authority = 'Bundle: missing or unknown authority'

		if (Object.values(errors).some(Boolean)) {
			setImportErrors(errors)
			return
		}

		setImportForm({ actionHex: rawHex, seqNo: String(bundle.seqNo), authority: bundle.authority })
		setImportErrors({})
		setIsValidating(true)

		try {
			const [decodeRes, sighashRes, configRes] = await Promise.all([
				decodeActionHex(rawHex),
				computeSighash(bundle.seqNo, rawHex),
				getMultisigConfig(bundle.authority),
			])

			if (!decodeRes.ok) {
				setImportErrors({ actionHex: `Decode failed: ${decodeRes.error}` })
				return
			}
			if (decodeRes.data.kind === 'unknown') {
				setImportErrors({ actionHex: 'Unknown action kind — cannot decode this hex' })
				return
			}
			if (!sighashRes.ok) {
				setImportErrors({ actionHex: `Sighash failed: ${sighashRes.error}` })
				return
			}

			const pubkey = wallet?.publicKeyHex?.toLowerCase()
			if (pubkey && configRes.ok && !configRes.data.signers.some((s) => s.toLowerCase() === pubkey)) {
				setImportErrors({ authority: `Your key is not a signer for ${bundle.authority}` })
				return
			}

			const bundleSigs = bundle.signatures
				.filter(
					(s): s is { signerPubkey: string; signatureHex: string } =>
						typeof s === 'object' &&
						s !== null &&
						typeof (s as Record<string, unknown>).signerPubkey === 'string' &&
						typeof (s as Record<string, unknown>).signatureHex === 'string',
				)
				.map((s): ManualSignature => ({ signerPubkey: s.signerPubkey, signatureHex: s.signatureHex, source: 'pasted' }))

			setImportData({
				actionHex: rawHex,
				seqNo: bundle.seqNo,
				authority: bundle.authority,
				sighashHex: sighashRes.data.sighashHex,
				actionType: actionTypeFromDecoded(decodeRes.data),
			})
			if (bundleSigs.length > 0) setLocalSignatures(bundleSigs)
			setStep('sign-collect')
		} finally {
			setIsValidating(false)
		}
	}

	async function handleLoadFromJson(file: File) {
		let parsed: unknown
		try {
			parsed = JSON.parse(await file.text())
		} catch {
			setImportErrors({ actionHex: 'Invalid JSON file' })
			return
		}

		if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
			setImportErrors({ actionHex: 'Invalid bundle format' })
			return
		}

		const obj = parsed as Record<string, unknown>
		const rawHex = typeof obj.actionHex === 'string' ? normalizeHex(obj.actionHex) : ''
		const seqNoRaw = typeof obj.seqNo === 'number' && Number.isInteger(obj.seqNo) ? (obj.seqNo as number) : -1
		const authority = typeof obj.authority === 'string' ? obj.authority : ''
		const signatures = Array.isArray(obj.signatures) ? (obj.signatures as ManualBundleJson['signatures']) : []

		await processBundle({ actionHex: rawHex, seqNo: seqNoRaw, authority, signatures })
	}

	const initialBundleRef = useRef(initialBundle)
	useEffect(() => {
		if (initialBundleRef.current) void processBundle(initialBundleRef.current)
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, []) // runs once on mount — initialBundleRef is stable

	function handlePasteSignatures(sigs: PastedSignature[], _broadcastState?: unknown) {
		setLocalSignatures((prev) => {
			const existingKeys = new Set(prev.map((s) => s.signerPubkey.toLowerCase()))
			const newSigs = sigs
				.filter((s) => !existingKeys.has(s.signerPubkey.toLowerCase()))
				.map((s): ManualSignature => ({ ...s, source: 'pasted' }))
			return [...prev, ...newSigs]
		})
	}

	async function handleAdvanceToBroadcast() {
		if (!hasQuorum || !importData || feeRateSatPerKvb === null) return
		setBroadcastPhase('preparing')
		setBroadcastError(null)
		setStep('broadcast')

		const prepInput: BroadcastManualInput = {
			actionHex: importData.actionHex,
			seqNo: importData.seqNo,
			authority: importData.authority,
			signatures: localSignatures.map(({ signerPubkey, signatureHex }) => ({ signerPubkey, signatureHex })),
			feeRateSatPerKvb,
		}

		const res = await prepareBroadcastManual(prepInput)
		if (res.ok) {
			setBroadcastBundle(res.data)
			setBroadcastPhase('confirming')
		} else {
			setBroadcastError(res.error)
			setBroadcastPhase('error')
		}
	}

	async function handleConfirmBroadcast() {
		if (!importData || feeRateSatPerKvb === null) return
		setBroadcastPhase('broadcasting')
		setBroadcastError(null)

		const input: BroadcastManualInput = {
			actionHex: importData.actionHex,
			seqNo: importData.seqNo,
			authority: importData.authority,
			signatures: localSignatures.map(({ signerPubkey, signatureHex }) => ({ signerPubkey, signatureHex })),
			feeRateSatPerKvb,
		}

		const res = await broadcastManualProposal(input)
		if (res.ok) {
			setCommitTxid(res.data.commitTxid ?? null)
			setRevealTxid(res.data.revealTxid ?? null)
			setBroadcastPhase('done')
		} else {
			setBroadcastError(res.error)
			setBroadcastPhase('error')
		}
	}

	function handleBackToSignCollect() {
		setStep('sign-collect')
		setBroadcastPhase('idle')
		setBroadcastError(null)
		setBroadcastBundle(null)
	}

	function handleReset() {
		setStep('import')
		setImportForm({ actionHex: '', seqNo: '', authority: '' })
		setImportErrors({})
		setIsValidating(false)
		setImportData(null)
		setDecodedData({ signerSetChange: null, allSigners: [], isLoading: false })
		setLocalSignatures([])
		setRequiredSignatures(null)
		setIsSigning(false)
		setSignError(null)
		setBroadcastPhase('idle')
		setBroadcastBundle(null)
		setBroadcastError(null)
		setCommitTxid(null)
		setRevealTxid(null)
	}

	return {
		step,
		// step 1
		importForm,
		importErrors,
		isValidating,
		handleImportChange,
		handleImportSubmit,
		handleLoadFromJson,
		// step 2
		importData,
		decodedData,
		deviceDisplay,
		localSignatures,
		requiredSignatures,
		hasQuorum,
		isSigning,
		signError,
		handleSign,
		handlePasteSignatures,
		handleAdvanceToBroadcast,
		// step 3
		broadcastPhase,
		broadcastBundle,
		broadcastError,
		commitTxid,
		revealTxid,
		handleConfirmBroadcast,
		handleBackToSignCollect,
		handleReset,
	}
}
