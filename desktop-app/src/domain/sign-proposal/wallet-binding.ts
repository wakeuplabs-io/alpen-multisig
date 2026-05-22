// P-039: guard that the wallet signed with the expected session pubkey.
// Throws a descriptive error on mismatch so the caller surfaces it to the user.
export function assertWalletPubkeyBinding(sessionPubkeyHex: string, signaturePubkeyHex: string): void {
	if (sessionPubkeyHex.toLowerCase() !== signaturePubkeyHex.toLowerCase()) {
		throw new Error(
			`Wallet pubkey mismatch: wallet signed with ${signaturePubkeyHex}, session expects ${sessionPubkeyHex}`,
		)
	}
}
