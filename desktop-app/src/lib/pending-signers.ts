import { normalizeAdminId } from '@/lib/admin-id'

/**
 * The signers of a role that have not signed yet. The role's set comes from the ASM as Admin IDs; who signed comes
 * from the backend as keys, resolved to Admin IDs in Rust. `undefined` marks a signature whose Admin ID is still
 * resolving: until every one is known the answer is empty, so a signer who already signed never flashes as pending.
 */
export function pendingSigners(
	allSigners: readonly string[],
	signedAdminIds: readonly (string | undefined)[],
): string[] {
	if (signedAdminIds.some((id) => id === undefined)) return []
	const signed = new Set(signedAdminIds.map((id) => normalizeAdminId(id ?? '')))
	return allSigners.filter((signer) => !signed.has(normalizeAdminId(signer)))
}
