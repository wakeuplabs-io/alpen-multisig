/** Maps backend capability reason codes to signer-facing copy. */
export function formatCanSignReason(reason: string | undefined): string | undefined {
	if (reason === undefined) {
		return undefined
	}
	switch (reason) {
		case 'watch-only-no-signer':
			return 'Hardware wallet required to sign'
		case 'no-session':
			return 'Connect your admin wallet session to send'
		case 'not-allowed-on-network':
			return 'Signing is not allowed on this network'
		default:
			return reason
	}
}
