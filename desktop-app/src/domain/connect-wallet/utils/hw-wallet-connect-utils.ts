export function truncateAddr(addr: string, opts?: { headChars?: number; tailChars?: number }): string {
	const headChars = opts?.headChars ?? 8
	const tailChars = opts?.tailChars ?? 6
	if (addr.length <= headChars + tailChars) return addr
	return `${addr.slice(0, headChars)}…${addr.slice(-tailChars)}`
}

export function buildAsmConfigSnippet(publicKeyHex: string): string {
	return `{
  "authorities": {
    "strata_administrator": {
      "config": {
        "keys": ["${publicKeyHex}"],
        "threshold": 1
      }
    }
  }
}`
}
