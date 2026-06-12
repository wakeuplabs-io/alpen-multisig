import { useEffect, useMemo, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { LogOutMutedIcon, LogOutRedIcon, SettingsGearIcon } from '@/assets/icons'
import { AuthRole } from '@/types'
import { HwWalletConnect } from '@/domain/connect-wallet/components/hw-wallet-connect'
import type { AuthorityOption } from '@/domain/connect-wallet/components/authority-selection-phase'
import { useSession } from '@/hooks/use-session'
import { ScreenShell } from '@/screens/screen-shell'
import { NodeConfigModal } from '@/domain/node-config/components/node-config-modal'
import { useNodeConfig } from '@/domain/node-config/hooks/use-node-config'

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
		id: 'alpen-administrator',
		role: AuthRole.AlpenAdministrator,
		label: 'Alpen Administrator',
		description: 'Alpen protocol parameters (EE verification key, signers).',
		signerSetSource: 'Alpen ASM state',
		availabilityLabel: 'Available',
		enabled: true,
	},
]

export function WalletConnectScreen() {
	const navigate = useNavigate()
	const {
		wallet,
		setConnectedWallet,
		adapter,
		selectAdapter,
		isAuthenticated,
		isOrchestratorSessionActive,
		selectedRole,
		setSelectedRole,
		connectOrchestratorSession,
		connectOnChainSession,
		disconnectSession,
		signingStep,
	} = useSession()
	const disconnectRef = useRef<(() => void) | null>(null)
	const [showTopBarDisconnect, setShowTopBarDisconnect] = useState(false)
	const [isNodeConfigOpen, setIsNodeConfigOpen] = useState(false)
	const [authorityStep, setAuthorityStep] = useState<'select-authority' | 'authenticate-session'>('select-authority')
	const [authError, setAuthError] = useState<string | null>(null)
	const [authOkMessage, setAuthOkMessage] = useState<string | null>(null)
	const [isAuthenticating, setIsAuthenticating] = useState(false)
	const { config: nodeConfig, localNodeStatus, localNodeUnreachable, isSaving, saveConfig, recheck } = useNodeConfig()
	const autoOpenedRef = useRef(false)

	useEffect(() => {
		if (!autoOpenedRef.current && localNodeUnreachable) {
			autoOpenedRef.current = true
			setIsNodeConfigOpen(true)
		}
	}, [localNodeUnreachable])

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
			await connectOrchestratorSession()
			setAuthOkMessage('Success: authenticated.')
			navigate(selectedRole === AuthRole.PayoutAdministrator ? '/block-payouts' : '/proposals')
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

	async function handleManualProposalAuth() {
		setAuthError(null)
		setIsAuthenticating(true)
		try {
			await connectOnChainSession()
			navigate('/manual')
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

		if ((isAuthenticated || isOrchestratorSessionActive) && roleChanged) {
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
		if (selectedAuthority.role === AuthRole.PayoutAdministrator) {
			navigate('/block-payouts')
			return
		}
		setAuthorityStep('authenticate-session')
	}

	function handleBackToAuthoritySelection() {
		setAuthorityStep('select-authority')
	}

	async function handleHeaderDisconnect() {
		disconnectRef.current?.()
		await disconnectSession()
	}

	function handleSelectWalletMethod(method: 'trezor' | 'ledger' | 'mnemonic', mnemonic?: string) {
		if (method === 'trezor') {
			selectAdapter('trezor')
			return
		}
		if (method === 'ledger') {
			selectAdapter('ledger')
			return
		}
		if (!mnemonic?.trim()) {
			return
		}
		selectAdapter('mnemonic', { mnemonic: mnemonic.trim() })
	}

	return (
		<>
			<ScreenShell
				centerContent={!showTopBarDisconnect}
				headerContent={
					<>
						{showTopBarDisconnect ? (
							<button
								type="button"
								className="group/disconnect inline-flex items-center gap-1.5 rounded-lg border border-[#e5e7eb] bg-white px-2.5 py-1.25 text-[12px] font-medium text-[#6b7280] transition hover:border-[#fca5a5] hover:bg-[#fef2f2] hover:text-[#b91c1c]"
								onClick={() => void handleHeaderDisconnect()}
							>
								<span className="relative inline-flex h-3 w-3 shrink-0">
									<LogOutMutedIcon
										width={12}
										height={12}
										className="absolute left-0 top-0 transition-opacity group-hover/disconnect:opacity-0"
									/>
									<LogOutRedIcon
										width={12}
										height={12}
										className="absolute left-0 top-0 opacity-0 transition-opacity group-hover/disconnect:opacity-100"
									/>
								</span>
								Disconnect
							</button>
						) : null}
						<button
							type="button"
							aria-label="Node connection settings"
							onClick={() => setIsNodeConfigOpen(true)}
							className={`relative inline-flex items-center justify-center rounded-lg border p-1.5 text-[#6b7280] transition hover:bg-[#f3f4f6] hover:text-[#374151] ${
								localNodeUnreachable
									? 'border-amber-300 text-amber-500 hover:text-amber-600'
									: 'border-[#e5e7eb] bg-white'
							}`}
						>
							<SettingsGearIcon width={14} height={14} className="text-current" />
							{localNodeUnreachable && (
								<span className="absolute -right-1 -top-1 h-2 w-2 rounded-full bg-amber-400" aria-hidden />
							)}
						</button>
					</>
				}
			>
				<HwWalletConnect
					adapter={adapter}
					walletVendor={adapter.vendor}
					onSelectWalletMethod={handleSelectWalletMethod}
					onConnected={setConnectedWallet}
					disconnectRef={disconnectRef}
					onHardwareSessionChange={setShowTopBarDisconnect}
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
									signingStep: signingStep ?? null,
									onSelectAuthority: handleSelectAuthority,
									onContinueToAuthenticate: handleContinueToAuthenticate,
									onBackToAuthority: handleBackToAuthoritySelection,
									onAuthenticate: () => void handleAuthenticate(),
									onManualProposal: () => void handleManualProposalAuth(),
								}
							: null
					}
				/>
			</ScreenShell>
			<NodeConfigModal
				isOpen={isNodeConfigOpen}
				config={nodeConfig}
				localNodeStatus={localNodeStatus}
				isSaving={isSaving}
				onSave={async (draft) => {
					await saveConfig(draft)
					setIsNodeConfigOpen(false)
				}}
				onClose={() => setIsNodeConfigOpen(false)}
				onRecheck={recheck}
			/>
		</>
	)
}
