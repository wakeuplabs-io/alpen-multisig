export function truncTxid(txid: string): string {
	if (txid.length <= 16) return txid
	return `${txid.slice(0, 8)}…${txid.slice(-6)}`
}
