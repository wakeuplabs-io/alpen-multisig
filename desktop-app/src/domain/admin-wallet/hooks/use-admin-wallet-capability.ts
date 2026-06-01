import { useEffect, useState } from 'react'
import { getAdminWalletCanSign } from '@/api/admin-wallet'
import { adminWalletCapabilitySchema } from '@/api/ipc-schemas'

export function useAdminWalletCapability() {
	const [canSign, setCanSign] = useState(false)
	const [signerKind, setSignerKind] = useState<'hardware' | 'mnemonic' | 'none'>('none')
	const [canSignReason, setCanSignReason] = useState<string | undefined>(undefined)

	useEffect(() => {
		getAdminWalletCanSign().then((result) => {
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
	}, [])

	return { canSign, signerKind, canSignReason }
}
