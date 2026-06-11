export type ConnectionMode = 'local' | 'trusted' | 'custom'

export type NodeConfig = {
	mode: ConnectionMode
	customStrataRpcUrl?: string
	customBtcRpcUrl?: string
	customBtcRpcUser?: string
	customBtcRpcPass?: string
	// Electrum indexer URL for Admin Wallet sync (R2.3)
	customElectrumUrl?: string
}

export type LocalNodeStatus = {
	strataReachable: boolean
	btcReachable: boolean
	electrumReachable: boolean
}
