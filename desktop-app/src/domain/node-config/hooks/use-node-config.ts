import { useContext } from 'react'

import { NodeConfigContext } from '@/contexts/node-config-context'

export function useNodeConfig() {
	const ctx = useContext(NodeConfigContext)
	if (ctx === null) {
		throw new Error('useNodeConfig must be used within NodeConfigProvider')
	}
	return ctx
}
