export function truncateAddr(addr: string, opts?: { headChars?: number; tailChars?: number }): string {
	const headChars = opts?.headChars ?? 8
	const tailChars = opts?.tailChars ?? 6
	if (addr.length <= headChars + tailChars) return addr
	return `${addr.slice(0, headChars)}…${addr.slice(-tailChars)}`
}
