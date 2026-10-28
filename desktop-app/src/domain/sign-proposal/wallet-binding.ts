// P-039: guard that the wallet signed with the expected session pubkey.
// Throws a descriptive error on mismatch so the caller surfaces it to the user.
export const WALLET_BINDING_MISMATCH =
	"The wallet signed with a different key than this session's Admin ID. Connect the wallet you signed in with."

export function assertWalletPubkeyBinding(sessionPubkeyHex: string, signaturePubkeyHex: string): void {
	if (sessionPubkeyHex.toLowerCase() !== signaturePubkeyHex.toLowerCase()) {
		// Named by Admin ID, not by key: the signer never sees a raw public key (feedback item 60).
		throw new Error(WALLET_BINDING_MISMATCH)
	}
}
