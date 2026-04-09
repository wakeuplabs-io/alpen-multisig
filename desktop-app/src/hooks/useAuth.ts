import { useState } from 'react'
import { getChallenge, createSession, deleteSession } from '@/api/auth'
import type { Authority, SessionInfo } from '@/types'

// Session token is NOT stored here — it lives in Rust's AppState.
// This hook only holds the session metadata needed to render the UI.

type AuthState =
	| { status: 'unauthenticated' }
	| { status: 'authenticating' }
	| { status: 'authenticated'; session: SessionInfo }
	| { status: 'error'; message: string }

export function useAuth() {
	const [state, setState] = useState<AuthState>({ status: 'unauthenticated' })

	async function handleAuthenticate(signerPubkey: string, authority: Authority) {
		setState({ status: 'authenticating' })

		// Step 1: get nonce challenge from backend (via Rust)
		const challengeResult = await getChallenge(signerPubkey, authority)
		if (!challengeResult.ok) {
			setState({ status: 'error', message: challengeResult.error })
			return
		}

		// Step 2: sign the nonce with the hardware wallet (Trezor/Ledger JS SDK)
		// Hardware wallet SDKs run in the WebView, so signing stays in JS.
		// TODO: call the wallet adapter here
		const { nonce } = challengeResult.data
		const ephemeralPubkey = 'TODO'
		const attestationSignature = `TODO: sign ${nonce} with hardware wallet`

		// Step 3: exchange signed nonce for a session.
		// Rust stores the bearer token internally — we only receive SessionInfo back.
		const sessionResult = await createSession({
			ephemeralPubkey,
			nonce,
			attestationSignature,
			signerPubkey,
			authority,
		})

		if (!sessionResult.ok) {
			setState({ status: 'error', message: sessionResult.error })
			return
		}

		setState({ status: 'authenticated', session: sessionResult.data })
	}

	async function handleSignOut() {
		if (state.status !== 'authenticated') return
		await deleteSession()
		setState({ status: 'unauthenticated' })
	}

	return { state, handleAuthenticate, handleSignOut }
}
