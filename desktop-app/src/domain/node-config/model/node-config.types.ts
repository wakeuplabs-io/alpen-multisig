export type ConnectionMode = 'local' | 'trusted' | 'custom'

export type NodeConfig = {
	mode: ConnectionMode
	customStrataRpcUrl?: string
	customBtcRpcUrl?: string
	customBtcRpcUser?: string
	customBtcRpcPass?: string
}

export type LocalNodeStatus = {
	strataReachable: boolean
	btcReachable: boolean
}
