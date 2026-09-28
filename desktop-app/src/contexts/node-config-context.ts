import { createContext } from 'react'
import type { LocalNodeStatus, NodeConfig } from '@/domain/node-config/model/node-config.types'

export type NodeConfigContextValue = {
	config: NodeConfig | null
	localNodeStatus: LocalNodeStatus | null
	isLoading: boolean
	isSaving: boolean
	localNodeUnreachable: boolean
	saveConfig: (draft: NodeConfig) => Promise<void>
	recheck: (config: NodeConfig) => void
}

export const NodeConfigContext = createContext<NodeConfigContextValue | null>(null)
