import { useEffect, useState } from 'react'
import { checkAuthorityMemberships } from '@/api/asm-state'
import type { AuthorityOption } from '@/domain/connect-wallet/components/authority-selection-phase'
import { AuthRole } from '@/types'

// Roles not backed by ASM state — skip membership check and preserve static config
const ASM_EXEMPT_ROLES = new Set<AuthRole>([AuthRole.PayoutAdministrator])

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
					if (option.role === null || ASM_EXEMPT_ROLES.has(option.role)) return option

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
