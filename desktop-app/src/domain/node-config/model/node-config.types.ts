export type ConnectionMode = 'local' | 'trusted' | 'custom'

export type NodeConfig = {
	mode: ConnectionMode
	customStrataRpcUrl?: string
	customBtcRpcUrl?: string
	customBtcRpcUser?: string
	customBtcRpcPass?: string
	// Electrum indexer URL for Admin Wallet sync (R2.3)
	customElectrumUrl?: string
	// Orchestrator backend URL override. Empty/unset falls back to VITE_ORCHESTRATOR_BASE_URL.
	customOrchestratorUrl?: string
}

export type LocalNodeStatus = {
	strataReachable: boolean
	btcReachable: boolean
	electrumReachable: boolean
	orchestratorReachable: boolean
	strataUrl: string
	btcUrl: string
	electrumUrl: string
	orchestratorUrl: string
}
