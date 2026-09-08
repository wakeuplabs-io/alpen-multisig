import { useEffect, useRef, useState } from 'react'
import type { ApiResult } from '@/types'
import { getMultisigConfig, type MultisigConfig } from '@/api/asm-state'
import {
	broadcastManualProposal,
	prepareBroadcastManual,
	type BroadcastManualInput,
	type PrepareBroadcastResult,
} from '@/api/proposals'
import { computeSighash, decodeActionHex, type DecodedAction } from '@/api/signing'
import { deriveBroadcastError } from '@/domain/broadcast-proposal/model/broadcast-proposal'
import { actionTypeFromDecoded } from '@/domain/manual-proposal/model/action-type-from-decoded'
import { decodedActionAuthorizingAuthority } from '@/domain/manual-proposal/model/authorizing-authority'
import { multisigUpdateTargetAuthority } from '@/lib/multisig-update-target'
import { deviceSigningDisplay, type DeviceSigningDisplay } from '@/lib/device-signing-display'
import { useDeviceSigningMessage } from '@/hooks/use-device-signing-message'
import type { DecodedProposalData } from '@/domain/proposal-detail/hooks/use-decoded-proposal'
import { buildSignerSetChange, type SignerSetChange } from '@/domain/signer-set-change/model/build-signer-set-change'
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

/**
 * The manual/imported bundle path has no `Proposal` and no `enacted` status to reconstruct
 * around — every bundle reviewed here is unsigned, pre-broadcast — so `buildSignerSetChange` is
 * always called with `isEnacted: false`. A failed config read for the target falls back to
 * suppression, matching `useDecodedProposal` (§4.9): never render against some other config.
 */
function buildManualTableOrNull(
	action: Extract<DecodedAction, { kind: 'multisig_update' }>,
	configRes: ApiResult<MultisigConfig>,
): SignerSetChange | null {
	if (!configRes.ok) return null

	return buildSignerSetChange({
		signers: configRes.data.signers,
		threshold: configRes.data.threshold,
		addKeys: action.addKeys,
		removeKeys: action.removeKeys,
		newThreshold: action.newThreshold,
		isEnacted: false,
	})
}

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

	// The Tauri layer answers a failed send with a structured error — `code`, a readable
	// `message` and, when no broadcaster could be reached, the signed commit and reveal
	// hex. This route used to print that JSON at the signer and drop the transactions
	// with it, which on the one path built for "the orchestrator is gone" is the worst
	// place to lose them (AC 15b).
	const broadcastErrorDetail = broadcastError === null ? null : deriveBroadcastError(broadcastError)

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

		// `allSigners` and `requiredSignatures` must always read the declared authority — never the
		// target of the decoded action — mirroring `useDecodedProposal` (§4.9). This first read is
		// unchanged by the retarget below: same request, same timing.
		void Promise.all([decodeActionHex(importData.actionHex), getMultisigConfig(importData.authority)]).then(
			([actionRes, ownConfigRes]) => {
				if (cancelled) return

				const allSigners = ownConfigRes.ok ? ownConfigRes.data.signers : []
				if (ownConfigRes.ok) {
					setRequiredSignatures(ownConfigRes.data.threshold)
				}

				// The kind check narrows `actionRes.data` below; the target check is the actual guard —
				// a council rotation's target authority (`role`) differs from the declared one.
				if (!actionRes.ok || actionRes.data.kind !== 'multisig_update') {
					setDecodedData((prev) => ({ ...prev, isLoading: false, allSigners, signerSetChange: null }))
					return
				}
				const action = actionRes.data
				const target = multisigUpdateTargetAuthority(action)
				if (target === null) {
					// Unreachable: `kind` is already narrowed to 'multisig_update' above, so the helper
					// can only return `action.role` here. Kept as a real check, not a cast, so the
					// compiler — not this comment — is what stays honest if that ever changes.
					setDecodedData((prev) => ({ ...prev, isLoading: false, allSigners, signerSetChange: null }))
					return
				}

				if (target === importData.authority) {
					// No retarget: the config already read above for `allSigners` is also the target's.
					setDecodedData((prev) => ({
						...prev,
						isLoading: false,
						allSigners,
						signerSetChange: buildManualTableOrNull(action, ownConfigRes),
					}))
					return
				}

				// Retarget (§4.9): the decoded action modifies an authority other than the one declared
				// on import — a council rotation. The table renders against the *target's* config, read
				// here, conditionally, only when it differs.
				void getMultisigConfig(target).then((targetConfigRes) => {
					if (cancelled) return
					setDecodedData((prev) => ({
						...prev,
						isLoading: false,
						allSigners,
						signerSetChange: buildManualTableOrNull(action, targetConfigRes),
					}))
				})
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
			const authorizingAuthority = decodedActionAuthorizingAuthority(decodeRes.data)
			if (authorizingAuthority !== null && authorizingAuthority !== importForm.authority) {
				setImportErrors({
					authority: `This action must be authorized by ${authorizingAuthority}, not ${importForm.authority}`,
				})
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
			const authorizingAuthority = decodedActionAuthorizingAuthority(decodeRes.data)
			if (authorizingAuthority !== null && authorizingAuthority !== bundle.authority) {
				setImportErrors({
					authority: `This action must be authorized by ${authorizingAuthority}, not ${bundle.authority}`,
				})
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
		broadcastErrorDetail,
		commitTxid,
		revealTxid,
		handleConfirmBroadcast,
		handleBackToSignCollect,
		handleReset,
	}
}
