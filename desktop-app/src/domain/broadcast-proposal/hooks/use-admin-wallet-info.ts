import { useEffect, useState } from 'react'
import { getAdminWalletInfo } from '@/api/admin-wallet'

type AdminWalletInfoView = {
	address: string
	balanceSats: number
}

type UseAdminWalletInfoReturn = {
	adminWalletInfo: AdminWalletInfoView | null
}

export function useAdminWalletInfo(): UseAdminWalletInfoReturn {
	const [adminWalletInfo, setAdminWalletInfo] = useState<AdminWalletInfoView | null>(null)

	useEffect(() => {
		getAdminWalletInfo().then((result) => {
			if (result.ok) {
				setAdminWalletInfo({ address: result.data.address, balanceSats: result.data.balance_sats })
			}
		})
	}, [])

	return { adminWalletInfo }
}
