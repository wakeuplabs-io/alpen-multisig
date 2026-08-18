import { useCallback, useEffect, useState } from 'react'
import { adminIdCertificateMessage, buildAdminIdCertificate, type AdminIdCertificate } from '@/api/admin-wallet'
import { useWalletSession } from '@/hooks/use-wallet-session'

export type AdminIdCertificateState =
	| { status: 'idle' }
	/** The signer is confirming on their device (or the mnemonic signer is deriving). */
	| { status: 'waiting' }
	| { status: 'signed'; certificate: AdminIdCertificate }
	| { status: 'error'; message: string }

type UseAdminIdCertificateReturn = {
	/** The exact string that will be signed, as Rust renders it. Null until loaded. */
	message: string | null
	state: AdminIdCertificateState
	/** Signs the message through the session's signer and stores the verified certificate. */
	sign(): Promise<void>
	reset(): void
}

/**
 * Admin ID Verification Certificate state machine (PRD 06 §3.c.i).
 *
 * The signature comes from the wallet adapter port that already signs the session
 * challenge, so Trezor, Ledger and the mnemonic signer take the same path here and no
 * device dispatch is duplicated. Rust renders the message and rebuilds the certificate
 * from the signature, refusing any whose recovered key is not this Admin ID's — so a
 * `signed` state is always a certificate the app has verified, never merely received.
 */
export function useAdminIdCertificate(adminId: string | undefined, isOpen: boolean): UseAdminIdCertificateReturn {
	const { adapter } = useWalletSession()
	const [message, setMessage] = useState<string | null>(null)
	const [state, setState] = useState<AdminIdCertificateState>({ status: 'idle' })

	// The message is shown before anything is signed (wireframe 1), so it is loaded when
	// the modal opens. Rust owns the literal; deriving it here would put the `Admin ID: `
	// prefix — which is part of the signed bytes — in two places.
	useEffect(() => {
		if (!isOpen || !adminId) return
		let cancelled = false
		void adminIdCertificateMessage(adminId).then((result) => {
			if (cancelled) return
			if (result.ok) {
				setMessage(result.data)
			} else {
				setState({ status: 'error', message: result.error })
			}
		})
		return () => {
			cancelled = true
		}
	}, [isOpen, adminId])

	const reset = useCallback(() => setState({ status: 'idle' }), [])

	const sign = useCallback(async () => {
		if (!adminId || !message) {
			setState({ status: 'error', message: 'No Admin ID to sign yet.' })
			return
		}

		setState({ status: 'waiting' })
		let signatureHex: string
		try {
			signatureHex = (await adapter.signSighash(message)).signatureHex
		} catch (reason) {
			setState({ status: 'error', message: `Could not sign your Admin ID — ${String(reason)}` })
			return
		}

		const built = await buildAdminIdCertificate(adminId, signatureHex)
		if (!built.ok) {
			setState({ status: 'error', message: built.error })
			return
		}
		setState({ status: 'signed', certificate: built.data })
	}, [adapter, adminId, message])

	return { message, state, sign, reset }
}
