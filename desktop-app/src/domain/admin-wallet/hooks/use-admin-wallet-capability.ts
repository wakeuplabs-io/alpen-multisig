import { useEffect, useRef, useState } from 'react'
import { getAdminWalletCanSign } from '@/api/admin-wallet'
import { adminWalletCapabilitySchema } from '@/api/ipc-schemas'
import { formatCanSignReason } from '@/domain/admin-wallet/model/format-can-sign-reason'

const CAPABILITY_POLL_INTERVAL_MS = 2_000

export function useAdminWalletCapability() {
	const [canSign, setCanSign] = useState(false)
	const [signerKind, setSignerKind] = useState<'hardware' | 'mnemonic' | 'none'>('none')
	const [canSignReason, setCanSignReason] = useState<string | undefined>(undefined)
	const [deviceType, setDeviceType] = useState<'trezor' | 'ledger' | null>(null)
	const [network, setNetwork] = useState<string>('regtest')
	const [adminWalletAccountPath, setAdminWalletAccountPath] = useState<string | null>(null)
	const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)

	const fetchCapability = () => {
		void getAdminWalletCanSign().then((result) => {
			if (!result.ok) {
				setCanSign(false)
				setSignerKind('none')
				setCanSignReason(undefined)
				setDeviceType(null)
				setAdminWalletAccountPath(null)
				return
			}
			const parsed = adminWalletCapabilitySchema.safeParse(result.data)
			if (!parsed.success) {
				console.warn('[admin-wallet] invalid capability DTO:', parsed.error.message)
				setCanSign(false)
				setSignerKind('none')
				setCanSignReason(undefined)
				setDeviceType(null)
				setAdminWalletAccountPath(null)
				return
			}
			setCanSign(parsed.data.canSign)
			setSignerKind(parsed.data.signerKind)
			setCanSignReason(formatCanSignReason(parsed.data.reason))
			setDeviceType(parsed.data.deviceType)
			setNetwork(parsed.data.network)
			setAdminWalletAccountPath(parsed.data.adminWalletAccountPath)
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

	return { canSign, signerKind, canSignReason, deviceType, network, adminWalletAccountPath }
}
