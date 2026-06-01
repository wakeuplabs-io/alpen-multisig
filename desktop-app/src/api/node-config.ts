import type { ApiResult } from '@/types'
import { tauriCall } from '@/api/tauri-bridge'
import { localNodeStatusSchema, nodeConfigSchema } from '@/api/ipc-schemas'
import type { LocalNodeStatus, NodeConfig } from '@/domain/node-config/model/node-config.types'

export function getNodeConfig(): Promise<ApiResult<NodeConfig>> {
	return tauriCall('get_node_config', {}, nodeConfigSchema)
}

export function saveNodeConfig(config: NodeConfig): Promise<ApiResult<void>> {
	return tauriCall('save_node_config', { config })
}

export function checkLocalNode(): Promise<ApiResult<LocalNodeStatus>> {
	return tauriCall('check_local_node', {}, localNodeStatusSchema)
}
