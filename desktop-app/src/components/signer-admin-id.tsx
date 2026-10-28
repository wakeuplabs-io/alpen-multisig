import { useAdminIds } from '@/hooks/use-admin-ids'
import { adminIdDisplay, isDisplayableAdminId, type AdminIdVariant } from '@/lib/admin-id'
import { normalizePubkey } from '@/lib/pubkey'

type Props = {
	variant?: AdminIdVariant
	className?: string
} & (
	| {
			/** A signer the app holds as a compressed public key (signatures, the session); resolved in Rust. */
			pubkey: string
			adminId?: never
	  }
	| {
			/** A signer the ASM lists, already an Admin ID on the chain's network. */
			adminId: string
			pubkey?: never
	  }
)

/** A signer, shown as its Admin ID. */
export function SignerAdminId(props: Props) {
	return props.adminId !== undefined ? (
		<AdminIdText adminId={props.adminId} variant={props.variant} className={props.className} />
	) : (
		<ResolvedAdminId pubkey={props.pubkey} variant={props.variant} className={props.className} />
	)
}

function ResolvedAdminId({
	pubkey,
	variant,
	className,
}: {
	pubkey: string
	variant?: AdminIdVariant
	className?: string
}) {
	const adminId = useAdminIds([pubkey]).get(normalizePubkey(pubkey))
	return <AdminIdText adminId={adminId} variant={variant} className={className} />
}

function AdminIdText({
	adminId,
	variant = 'truncated',
	className,
}: {
	adminId: string | undefined
	variant?: AdminIdVariant
	className?: string
}) {
	return (
		<span className={className} title={isDisplayableAdminId(adminId) ? adminId : undefined}>
			{adminIdDisplay(adminId, variant)}
		</span>
	)
}
