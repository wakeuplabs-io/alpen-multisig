import { useContext } from 'react'
import { SessionContext } from '@/contexts/session-context'

export function useSession() {
	const ctx = useContext(SessionContext)
	if (ctx === null) {
		throw new Error('useSession must be used within SessionProvider')
	}
	return ctx
}
