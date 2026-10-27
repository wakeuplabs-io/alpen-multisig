import { useAdminIds } from '@/hooks/use-admin-ids'
import { adminIdDisplay, isDisplayableAdminId, type AdminIdVariant } from '@/lib/admin-id'
import { normalizePubkey } from '@/lib/pubkey'

type Props = {
	/** The signer as the app holds it: a compressed public key. */
	pubkey: string
	variant?: AdminIdVariant
	className?: string
}

/** A signer, shown as its Admin ID. */
export function SignerAdminId({ pubkey, variant = 'truncated', className }: Props) {
	const adminId = useAdminIds([pubkey]).get(normalizePubkey(pubkey))
	return (
		<span className={className} title={isDisplayableAdminId(adminId) ? adminId : undefined}>
			{adminIdDisplay(adminId, variant)}
		</span>
	)
}
