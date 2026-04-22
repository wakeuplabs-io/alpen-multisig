// ─── Wallet ──────────────────────────────────────────────────────────────────

export type WalletAddress = {
	address: string
	derivationPath: string
	index: number
}

export type ConnectedWallet = {
	deviceName: string
	addresses: WalletAddress[]
}

// ─── API Response shapes ──────────────────────────────────────────────────────

export type ApiResult<T> = { ok: true; data: T } | { ok: false; error: string }
