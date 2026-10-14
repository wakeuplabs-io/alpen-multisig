import type { SignerSetChange } from '@/domain/signer-set-change/model/build-signer-set-change'

/**
 * The signers who could approve an enacted proposal: the members when it was signed, not the
 * live post-rotation set. A signer the proposal removed stays listed; one it added does not.
 *
 * Reads the Before column the signer-set table already reconstructs, so it shares that table's
 * assumption: no later rotation of the same authority was enacted after this one. Without a
 * signer-set change, the current config is the roster.
 */
export function approvalRoster(currentSigners: string[], change: SignerSetChange | null): string[] {
	if (change === null) return currentSigners
	return change.rows.filter((row) => row.inBefore).map((row) => row.pubkey)
}
