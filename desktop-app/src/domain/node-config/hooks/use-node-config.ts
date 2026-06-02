import { useCallback, useEffect, useState } from 'react'

import { checkLocalNode, getNodeConfig, saveNodeConfig } from '@/api/node-config'
import type { LocalNodeStatus, NodeConfig } from '@/domain/node-config/model/node-config.types'

type UseNodeConfigResult = {
	config: NodeConfig | null
	localNodeStatus: LocalNodeStatus | null
	isLoading: boolean
	isSaving: boolean
	localNodeUnreachable: boolean
	saveConfig: (draft: NodeConfig) => Promise<void>
	recheck: () => void
}

export function useNodeConfig(): UseNodeConfigResult {
	const [config, setConfig] = useState<NodeConfig | null>(null)
	const [localNodeStatus, setLocalNodeStatus] = useState<LocalNodeStatus | null>(null)
	const [isLoading, setIsLoading] = useState(true)
	const [isSaving, setIsSaving] = useState(false)

	const fetchAll = useCallback(async () => {
		setIsLoading(true)
		const [configResult, statusResult] = await Promise.all([getNodeConfig(), checkLocalNode()])
		if (configResult.ok) setConfig(configResult.data)
		if (statusResult.ok) setLocalNodeStatus(statusResult.data)
		setIsLoading(false)
	}, [])

	useEffect(() => {
		void fetchAll()
	}, [fetchAll])

	const recheck = useCallback(() => {
		void fetchAll()
	}, [fetchAll])

	const saveConfigFn = useCallback(async (draft: NodeConfig) => {
		setIsSaving(true)
		const result = await saveNodeConfig(draft)
		if (result.ok) {
			setConfig(draft)
			const statusResult = await checkLocalNode()
			if (statusResult.ok) setLocalNodeStatus(statusResult.data)
		}
		setIsSaving(false)
	}, [])

	const localNodeUnreachable =
		config?.mode === 'local' &&
		localNodeStatus !== null &&
		!localNodeStatus.strataReachable &&
		!localNodeStatus.btcReachable

	return {
		config,
		localNodeStatus,
		isLoading,
		isSaving,
		localNodeUnreachable,
		saveConfig: saveConfigFn,
		recheck,
	}
}
