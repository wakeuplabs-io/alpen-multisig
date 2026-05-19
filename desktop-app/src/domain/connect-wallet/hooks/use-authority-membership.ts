import { useEffect, useState } from 'react'
import { checkAuthorityMemberships } from '@/api/asm-state'
import type { AuthorityOption } from '@/domain/connect-wallet/components/authority-selection-phase'

type Result = {
	resolvedOptions: AuthorityOption[]
	isChecking: boolean
}

export function useAuthorityMembership(signerPubkeyHex: string | null, options: AuthorityOption[]): Result {
	const [resolvedOptions, setResolvedOptions] = useState<AuthorityOption[]>(options)
	const [isChecking, setIsChecking] = useState(false)

	useEffect(() => {
		if (signerPubkeyHex === null) {
			setResolvedOptions(options)
			setIsChecking(false)
			return
		}

		let cancelled = false
		setIsChecking(true)

		checkAuthorityMemberships(signerPubkeyHex).then((result) => {
			if (cancelled) return
			setIsChecking(false)

			const memberships = result.ok ? result.data : {}

			setResolvedOptions(
				options.map((option) => {
					if (option.role === null) return option

					const isMember = memberships[option.role] === true
					return {
						...option,
						enabled: isMember,
						availabilityLabel: isMember ? 'Available' : 'Not a signer',
					}
				}),
			)
		})

		return () => {
			cancelled = true
		}
		// options reference is stable (defined as module-level constant in wallet-connect-screen)
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, [signerPubkeyHex])

	return { resolvedOptions, isChecking }
}
