import { useState } from 'react'
import type { Authority, Session } from '@/types'

type AuthState =
  | { status: 'unauthenticated' }
  | { status: 'authenticating' }
  | { status: 'authenticated'; session: Session }
  | { status: 'error'; message: string }

export function useAuth() {
  const [state, setState] = useState<AuthState>({ status: 'unauthenticated' })

  async function handleAuthenticate(signerPubkey: string, authority: Authority) {
    setState({ status: 'authenticating' })
    // TODO:
    // 1. getChallenge(signerPubkey, authority)
    // 2. Sign nonce with hardware wallet via Tauri
    // 3. createSession(ephemeralPubkey, nonce, attestationSignature, signerPubkey, authority)
  }

  async function handleSignOut() {
    if (state.status !== 'authenticated') return
    // TODO: deleteSession(state.session.sessionToken)
    setState({ status: 'unauthenticated' })
  }

  const sessionToken =
    state.status === 'authenticated' ? state.session.sessionId : null

  return { state, sessionToken, handleAuthenticate, handleSignOut }
}
