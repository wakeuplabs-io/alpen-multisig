import { useBlocker } from 'react-router-dom'

export function useNavigationGuard(shouldBlock: boolean) {
	return useBlocker(shouldBlock)
}
