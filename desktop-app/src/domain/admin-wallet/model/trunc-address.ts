export function truncAddress(address: string): string {
	if (address.length <= 14) return address
	return `${address.slice(0, 5)}…${address.slice(-4)}`
}
