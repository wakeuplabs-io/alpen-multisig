import { useEffect, useRef, useState } from 'react'
import { getAdminWalletCanSign } from '@/api/admin-wallet'
import { adminWalletCapabilitySchema } from '@/api/ipc-schemas'

const CAPABILITY_POLL_INTERVAL_MS = 2_000

export function useAdminWalletCapability() {
	const [canSign, setCanSign] = useState(false)
	const [signerKind, setSignerKind] = useState<'hardware' | 'mnemonic' | 'none'>('none')
	const [canSignReason, setCanSignReason] = useState<string | undefined>(undefined)
	const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)

	const fetchCapability = () => {
		void getAdminWalletCanSign().then((result) => {
			if (result.ok) {
				const parsed = adminWalletCapabilitySchema.parse(result.data)
				setCanSign(parsed.canSign)
				setSignerKind(parsed.signerKind)
				setCanSignReason(parsed.reason)
			} else {
				setCanSign(false)
				setSignerKind('none')
				setCanSignReason(undefined)
			}
		})
	}

	useEffect(() => {
		fetchCapability() // Initial fetch
		intervalRef.current = setInterval(fetchCapability, CAPABILITY_POLL_INTERVAL_MS)
		return () => {
			if (intervalRef.current) {
				clearInterval(intervalRef.current)
			}
		}
	}, [])

	return { canSign, signerKind, canSignReason }
}
