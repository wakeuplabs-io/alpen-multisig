import { useEffect, useState } from 'react'
import { getAdminWalletInfo } from '@/api/admin-wallet'

type AdminWalletInfoView = {
	address: string
	balanceSats: number
}

type UseAdminWalletInfoReturn = {
	/** null = loading, undefined = unavailable/error, AdminWalletInfoView = loaded */
	adminWalletInfo: AdminWalletInfoView | null | undefined
}

export function useAdminWalletInfo(sessionReady: boolean): UseAdminWalletInfoReturn {
	const [adminWalletInfo, setAdminWalletInfo] = useState<AdminWalletInfoView | null | undefined>(null)

	useEffect(() => {
		if (!sessionReady) return
		getAdminWalletInfo().then((result) => {
			if (result.ok) {
				setAdminWalletInfo({ address: result.data.address, balanceSats: result.data.balance_sats })
			} else {
				console.warn('[admin-wallet-info] getAdminWalletInfo failed:', result.error)
				setAdminWalletInfo(undefined)
			}
		})
	}, [sessionReady])

	return { adminWalletInfo }
}
