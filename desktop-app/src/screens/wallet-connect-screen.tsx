import { useEffect, useMemo, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import {
	authorityFromRole,
	orchestratorAuthComplete,
	orchestratorAuthStart,
	ORCHESTRATOR_BASE_URL,
} from '@/api/orchestrator-auth'
import { AuthRole } from '@/types'
import { HwWalletConnect } from '@/domain/connect-wallet/components/hw-wallet-connect'
import { useAuthSession } from '@/hooks/use-auth-session'
import { useWalletSession } from '@/hooks/use-wallet-session'
import { ScreenShell } from '@/screens/screen-shell'

type AuthorityOption = {
	id: string
	role: AuthRole | null
	label: string
	description: string
	signerSetSource: string
	availabilityLabel: string
	enabled: boolean
}

const AUTHORITY_OPTIONS: AuthorityOption[] = [
	{
		id: 'strata-administrator',
		role: AuthRole.StrataAdministrator,
		label: 'Strata Administrator',
		description: 'Strata protocol parameters (verification key, signers, operators, bridge, safe harbor).',
		signerSetSource: 'Strata ASM state',
		availabilityLabel: 'Available',
		enabled: true,
	},
	{
		id: 'strata-sequencer-manager',
		role: AuthRole.StrataSequencerManager,
		label: 'Strata Sequencer Manager',
		description: 'Sequencer configuration (signers, sequencer pubkey).',
		signerSetSource: 'Strata ASM state',
		availabilityLabel: 'Not in v0.1',
		enabled: false,
	},
	{
		id: 'security-council',
		role: null,
		label: 'Security Council',
		description: 'Emergency actions and recovery controls.',
		signerSetSource: 'Strata ASM state',
		availabilityLabel: 'Not in v0.1',
		enabled: false,
	},
	{
		id: 'payout-administrator',
		role: null,
		label: 'Payout Administrator',
		description: 'Bridge payout spending rules and payout control.',
		signerSetSource: 'Bridge multisig script',
		availabilityLabel: 'Not in v0.1',
		enabled: false,
	},
]

export function WalletConnectScreen() {
	const navigate = useNavigate()
	const { wallet, setConnectedWallet, adapter } = useWalletSession()
	const { isAuthenticated, authenticate, selectedRole, setSelectedRole } = useAuthSession()
	const [authorityStep, setAuthorityStep] = useState<'select-authority' | 'authenticate-session'>('select-authority')
	const [authError, setAuthError] = useState<string | null>(null)
	const [authOkMessage, setAuthOkMessage] = useState<string | null>(null)
	const [isAuthenticating, setIsAuthenticating] = useState(false)
	const defaultEnabledAuthority = useMemo(
		() => AUTHORITY_OPTIONS.find((option) => option.enabled && option.role !== null) ?? null,
		[],
	)
	const selectedAuthorityId = useMemo(
		() => AUTHORITY_OPTIONS.find((option) => option.role === selectedRole)?.id ?? null,
		[selectedRole],
	)
	const selectedAuthorityLabel = useMemo(
		() => AUTHORITY_OPTIONS.find((option) => option.id === selectedAuthorityId)?.label ?? null,
		[selectedAuthorityId],
	)

	useEffect(() => {
		if (selectedAuthorityId !== null) {
			return
		}
		if (defaultEnabledAuthority?.role) {
			setSelectedRole(defaultEnabledAuthority.role)
		}
	}, [defaultEnabledAuthority, selectedAuthorityId, setSelectedRole])

	async function handleAuthenticate() {
		setAuthError(null)
		setAuthOkMessage(null)
		setIsAuthenticating(true)
		try {
			await authenticate((challengeHex: string) => adapter.signSighash(challengeHex))
			const challengeResult = await orchestratorAuthStart({
				baseUrl: ORCHESTRATOR_BASE_URL,
				authority: authorityFromRole(selectedRole),
			})
			if (!challengeResult.ok) {
				throw new Error(challengeResult.error)
			}
			const signature = await adapter.signSighash(challengeResult.data.challengeHex)
			const completeResult = await orchestratorAuthComplete({
				baseUrl: ORCHESTRATOR_BASE_URL,
				challengeId: challengeResult.data.challengeId,
				signerPubkey: signature.publicKeyHex,
				signatureHex: signature.signatureHex,
				signatureFormat: signature.signatureFormat,
			})
			if (!completeResult.ok) {
				throw new Error(completeResult.error)
			}
			setAuthOkMessage('Success: authenticated.')
			navigate('/proposals')
		} catch (e) {
			const message = String(e)
			if (message.toLowerCase().includes('not a member')) {
				setAuthError('You do not have permissions for the selected role.')
			} else {
				setAuthError(message)
			}
		} finally {
			setIsAuthenticating(false)
		}
	}

	function handleSelectAuthority(nextAuthorityId: string) {
		const selectedAuthority = AUTHORITY_OPTIONS.find((option) => option.id === nextAuthorityId)
		if (!selectedAuthority || selectedAuthority.role === null || !selectedAuthority.enabled) {
			return
		}

		const roleChanged = selectedAuthority.role !== selectedRole
		setSelectedRole(selectedAuthority.role)

		if (isAuthenticated && roleChanged) {
			setAuthError('Authority changed. Re-authenticate to continue.')
		} else {
			setAuthError(null)
		}
		setAuthOkMessage(null)
	}

	function handleContinueToAuthenticate() {
		const selectedAuthority = AUTHORITY_OPTIONS.find((option) => option.id === selectedAuthorityId)
		if (!selectedAuthority || !selectedAuthority.enabled || selectedAuthority.role === null) {
			return
		}
		setAuthorityStep('authenticate-session')
	}

	function handleBackToAuthoritySelection() {
		setAuthorityStep('select-authority')
	}

	return (
		<ScreenShell centerContent>
			<HwWalletConnect
				adapter={adapter}
				onConnected={setConnectedWallet}
				authoritySelection={
					wallet !== null
						? {
								step: authorityStep,
								selectedAuthorityId,
								selectedAuthorityLabel,
								options: AUTHORITY_OPTIONS,
								isAuthenticating,
								isAuthenticated,
								authError,
								authOkMessage,
								onSelectAuthority: handleSelectAuthority,
								onContinueToAuthenticate: handleContinueToAuthenticate,
								onBackToAuthority: handleBackToAuthoritySelection,
								onAuthenticate: () => void handleAuthenticate(),
							}
						: null
				}
			/>
		</ScreenShell>
	)
}
