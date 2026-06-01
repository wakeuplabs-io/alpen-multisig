import { useEffect, useState } from 'react'
import { getAdminWalletCanSign } from '@/api/admin-wallet'

export function useAdminWalletCapability() {
	const [canSign, setCanSign] = useState(false)

	useEffect(() => {
		getAdminWalletCanSign().then((result) => {
			if (result.ok) setCanSign(result.data)
			else setCanSign(false)
		})
	}, [])

	return { canSign }
}
