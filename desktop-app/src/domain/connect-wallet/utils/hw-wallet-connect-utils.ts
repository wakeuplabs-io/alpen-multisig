export function truncateAddr(addr: string): string {
	if (addr.length <= 16) return addr
	return `${addr.slice(0, 8)}…${addr.slice(-6)}`
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
