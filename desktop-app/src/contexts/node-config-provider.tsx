import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react'

import { checkLocalNode, getNodeConfig, saveNodeConfig } from '@/api/node-config'
import { orchestratorOverrideFromConfig, setOrchestratorBaseUrlOverride } from '@/api/orchestrator-auth'
import { NodeConfigContext } from '@/contexts/node-config-context'
import type { LocalNodeStatus, NodeConfig } from '@/domain/node-config/model/node-config.types'

/**
 * Loads the node config and checks the node once per app load, so the connect screen (editable) and
 * the signed-in header (read-only) show the same mode without re-checking on every navigation.
 */
export function NodeConfigProvider({ children }: { children: ReactNode }) {
	const [config, setConfig] = useState<NodeConfig | null>(null)
	const [localNodeStatus, setLocalNodeStatus] = useState<LocalNodeStatus | null>(null)
	const [isLoading, setIsLoading] = useState(true)
	const [isSaving, setIsSaving] = useState(false)

	const fetchAll = useCallback(async () => {
		setIsLoading(true)
		const configResult = await getNodeConfig()
		if (configResult.ok) {
			setConfig(configResult.data)
			setOrchestratorBaseUrlOverride(orchestratorOverrideFromConfig(configResult.data))
		}
		const statusResult = await checkLocalNode(configResult.ok ? configResult.data : { mode: 'local' })
		if (statusResult.ok) setLocalNodeStatus(statusResult.data)
		setIsLoading(false)
	}, [])

	useEffect(() => {
		void fetchAll()
	}, [fetchAll])

	const recheck = useCallback((config: NodeConfig) => {
		void (async () => {
			const statusResult = await checkLocalNode(config)
			if (statusResult.ok) setLocalNodeStatus(statusResult.data)
		})()
	}, [])

	const saveConfig = useCallback(async (draft: NodeConfig) => {
		setIsSaving(true)
		const result = await saveNodeConfig(draft)
		if (result.ok) {
			setConfig(draft)
			setOrchestratorBaseUrlOverride(orchestratorOverrideFromConfig(draft))
			const statusResult = await checkLocalNode(draft)
			if (statusResult.ok) setLocalNodeStatus(statusResult.data)
		}
		setIsSaving(false)
	}, [])

	const localNodeUnreachable =
		config?.mode === 'local' &&
		localNodeStatus !== null &&
		!localNodeStatus.strataReachable &&
		!localNodeStatus.btcReachable

	const value = useMemo(
		() => ({ config, localNodeStatus, isLoading, isSaving, localNodeUnreachable, saveConfig, recheck }),
		[config, localNodeStatus, isLoading, isSaving, localNodeUnreachable, saveConfig, recheck],
	)

	return <NodeConfigContext.Provider value={value}>{children}</NodeConfigContext.Provider>
}
