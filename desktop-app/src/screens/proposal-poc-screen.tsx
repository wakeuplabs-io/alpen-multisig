import { useMemo, useState, type CSSProperties } from 'react'
import { Navigate } from 'react-router-dom'
import { buildAdminMultisigUpdateHex } from '@/api/action-builder'
import { createProposal, listProposals, type Proposal } from '@/api/proposals'
import { computeSighash, signSighashMock, type SignatureResult } from '@/api/signing'
import { tauriCall } from '@/api/tauri-bridge'
import { ScreenShell } from '@/screens/screen-shell'
import { useWalletSession } from '@/hooks/use-wallet-session'

type RawOrchestratorAuthChallenge = {
	challenge_id: string
	challenge_hex: string
}

type RawOrchestratorAuthSession = {
	authority: string
	signer_pubkey: string
	expires_at_unix_ms: number
}

export function ProposalPocScreen() {
	const { wallet, adapter } = useWalletSession()
	const [baseUrl, setBaseUrl] = useState('http://127.0.0.1:3000/api/v1')
	const trimmedBaseUrl = useMemo(() => baseUrl.trim(), [baseUrl])
	const [seqNo, setSeqNo] = useState('1')
	const [actionHex, setActionHex] = useState('')
	const [builderAddKeys, setBuilderAddKeys] = useState('')
	const [builderRemoveKeys, setBuilderRemoveKeys] = useState('')
	const [builderThreshold, setBuilderThreshold] = useState('2')
	const [statusFilter, setStatusFilter] = useState('')
	const [useMockSigner, setUseMockSigner] = useState(false)
	const [mockPrivateKeyHex, setMockPrivateKeyHex] = useState('')
	const [isAuthenticating, setIsAuthenticating] = useState(false)
	const [isBuildingHex, setIsBuildingHex] = useState(false)
	const [isCreating, setIsCreating] = useState(false)
	const [isListing, setIsListing] = useState(false)
	const [authError, setAuthError] = useState<string | null>(null)
	const [builderError, setBuilderError] = useState<string | null>(null)
	const [createError, setCreateError] = useState<string | null>(null)
	const [listError, setListError] = useState<string | null>(null)
	const [authSession, setAuthSession] = useState<RawOrchestratorAuthSession | null>(null)
	const [lastCreated, setLastCreated] = useState<Proposal | null>(null)
	const [proposals, setProposals] = useState<Proposal[]>([])

	if (wallet === null) {
		return <Navigate to="/" replace />
	}

	function parseMultilineKeys(value: string): string[] {
		return value
			.split('\n')
			.map((line) => line.trim())
			.filter((line) => line.length > 0)
	}

	async function handleOrchestratorAuth() {
		setAuthError(null)
		setIsAuthenticating(true)
		try {
			const challengeRes = await tauriCall<RawOrchestratorAuthChallenge>('orchestrator_auth_start', {
				input: {
					baseUrl: trimmedBaseUrl,
					authority: 'strata_admin',
				},
			})
			if (!challengeRes.ok) {
				throw new Error(challengeRes.error)
			}

			const challengeSig = await adapter.signSighash(challengeRes.data.challenge_hex)
			const sessionRes = await tauriCall<RawOrchestratorAuthSession>('orchestrator_auth_complete', {
				input: {
					baseUrl: trimmedBaseUrl,
					challengeId: challengeRes.data.challenge_id,
					signerPubkey: challengeSig.publicKeyHex,
					signatureHex: challengeSig.signatureHex,
					signatureFormat: challengeSig.signatureFormat,
				},
			})
			if (!sessionRes.ok) {
				throw new Error(sessionRes.error)
			}
			setAuthSession(sessionRes.data)
		} catch (error) {
			setAuthSession(null)
			setAuthError(String(error))
		} finally {
			setIsAuthenticating(false)
		}
	}

	async function handleBuildActionHex() {
		setBuilderError(null)
		setIsBuildingHex(true)
		try {
			const newThreshold = Number(builderThreshold)
			if (!Number.isInteger(newThreshold) || newThreshold <= 0 || newThreshold > 255) {
				throw new Error('Threshold must be an integer between 1 and 255')
			}
			const result = await buildAdminMultisigUpdateHex({
				role: 'strata_admin',
				addKeys: parseMultilineKeys(builderAddKeys),
				removeKeys: parseMultilineKeys(builderRemoveKeys),
				newThreshold,
			})
			if (!result.ok) {
				throw new Error(result.error)
			}
			setActionHex(result.data.actionHex)
		} catch (error) {
			setBuilderError(String(error))
		} finally {
			setIsBuildingHex(false)
		}
	}

	async function handleCreateProposal() {
		setCreateError(null)
		setIsCreating(true)
		try {
			const numericSeqNo = Number(seqNo)
			if (!Number.isInteger(numericSeqNo) || numericSeqNo < 0) {
				throw new Error('seqNo must be a valid u64 integer')
			}
			const sighashRes = await computeSighash(numericSeqNo, actionHex.trim())
			if (!sighashRes.ok) {
				throw new Error(sighashRes.error)
			}
			let signature: SignatureResult
			if (useMockSigner) {
				if (mockPrivateKeyHex.trim().length === 0) {
					throw new Error('mock private key is required when mock signer mode is enabled')
				}
				const mockSignRes = await signSighashMock(mockPrivateKeyHex.trim(), sighashRes.data.sighashHex)
				if (!mockSignRes.ok) {
					throw new Error(mockSignRes.error)
				}
				signature = mockSignRes.data
			} else {
				const hwSignature = await adapter.signSighash(sighashRes.data.sighashHex)
				signature = {
					publicKeyHex: hwSignature.publicKeyHex,
					signatureHex: hwSignature.signatureHex,
				}
			}
			const createRes = await createProposal({
				baseUrl: trimmedBaseUrl,
				seqNo: numericSeqNo,
				actionHex: actionHex.trim(),
				signerPubkey: signature.publicKeyHex,
				signatureHex: signature.signatureHex,
			})
			if (!createRes.ok) {
				throw new Error(createRes.error)
			}
			setLastCreated(createRes.data)
		} catch (error) {
			setCreateError(String(error))
		} finally {
			setIsCreating(false)
		}
	}

	async function handleListProposals() {
		setListError(null)
		setIsListing(true)
		try {
			const listRes = await listProposals({
				baseUrl: trimmedBaseUrl,
				status: statusFilter.trim() === '' ? undefined : statusFilter.trim(),
			})
			if (!listRes.ok) {
				throw new Error(listRes.error)
			}
			setProposals(listRes.data)
		} catch (error) {
			setListError(String(error))
		} finally {
			setIsListing(false)
		}
	}

	return (
		<ScreenShell>
			<section style={styles.panel}>
				<h3 style={styles.title}>Proposal UI PoC (create + list)</h3>
				<label style={styles.label}>
					Orchestrator Base URL
					<input
						style={styles.input}
						type="text"
						value={baseUrl}
						onChange={(e) => setBaseUrl(e.target.value)}
						spellCheck={false}
					/>
				</label>
				<button type="button" style={styles.button} onClick={() => void handleOrchestratorAuth()} disabled={isAuthenticating}>
					{isAuthenticating ? 'Authenticating…' : 'Authenticate orchestrator session'}
				</button>
				{authSession && (
					<p style={styles.success}>
						Authenticated signer: <code>{authSession.signer_pubkey}</code>
					</p>
				)}
				{authError && <p style={styles.error}>{authError}</p>}
			</section>

			<section style={styles.panel}>
				<h3 style={styles.title}>Create Proposal</h3>
				<div style={styles.builderBox}>
					<p style={styles.resultLabel}>Admin Action Builder</p>
					<p style={styles.resultLabel}>Type: MultisigUpdate (role fixed: strata_admin)</p>
					<label style={styles.label}>
						Add signer keys (one compressed pubkey hex per line)
						<textarea
							style={styles.textarea}
							value={builderAddKeys}
							onChange={(e) => setBuilderAddKeys(e.target.value)}
							spellCheck={false}
						/>
					</label>
					<label style={styles.label}>
						Remove signer keys (one compressed pubkey hex per line)
						<textarea
							style={styles.textarea}
							value={builderRemoveKeys}
							onChange={(e) => setBuilderRemoveKeys(e.target.value)}
							spellCheck={false}
						/>
					</label>
					<label style={styles.label}>
						New threshold
						<input
							style={styles.input}
							type="number"
							min={1}
							max={255}
							value={builderThreshold}
							onChange={(e) => setBuilderThreshold(e.target.value)}
						/>
					</label>
					<button type="button" style={styles.button} onClick={() => void handleBuildActionHex()} disabled={isBuildingHex}>
						{isBuildingHex ? 'Building actionHex…' : 'Build actionHex'}
					</button>
					{builderError && <p style={styles.error}>{builderError}</p>}
				</div>
				<label style={styles.label}>
					SeqNo
					<input style={styles.input} type="number" min={0} value={seqNo} onChange={(e) => setSeqNo(e.target.value)} />
				</label>
				<label style={styles.label}>
					Action Hex
					<textarea style={styles.textarea} value={actionHex} onChange={(e) => setActionHex(e.target.value)} spellCheck={false} />
				</label>
				<label style={styles.checkboxLabel}>
					<input type="checkbox" checked={useMockSigner} onChange={(e) => setUseMockSigner(e.target.checked)} />
					Use mock signer (dev only)
				</label>
				{useMockSigner && (
					<label style={styles.label}>
						Mock private key hex
						<input
							style={styles.input}
							type="password"
							value={mockPrivateKeyHex}
							onChange={(e) => setMockPrivateKeyHex(e.target.value)}
							spellCheck={false}
						/>
					</label>
				)}
				<button
					type="button"
					style={styles.button}
					onClick={() => void handleCreateProposal()}
					disabled={isCreating || actionHex.trim().length === 0}
				>
					{isCreating ? 'Creating…' : 'Create proposal'}
				</button>
				{createError && <p style={styles.error}>{createError}</p>}
				{lastCreated && (
					<div style={styles.resultBox}>
						<p style={styles.resultLabel}>Action ID</p>
						<code style={styles.mono}>{lastCreated.actionId}</code>
						<p style={styles.resultLabel}>Status: {lastCreated.status}</p>
					</div>
				)}
			</section>

			<section style={styles.panel}>
				<h3 style={styles.title}>List Proposals</h3>
				<label style={styles.label}>
					Status filter (optional)
					<input
						style={styles.input}
						type="text"
						value={statusFilter}
						onChange={(e) => setStatusFilter(e.target.value)}
						placeholder='pending'
					/>
				</label>
				<button type="button" style={styles.button} onClick={() => void handleListProposals()} disabled={isListing}>
					{isListing ? 'Loading…' : 'Refresh proposals'}
				</button>
				{listError && <p style={styles.error}>{listError}</p>}
				<div style={styles.list}>
					{proposals.map((proposal) => (
						<div key={proposal.actionId} style={styles.listItem}>
							<code style={styles.mono}>{proposal.actionId}</code>
							<p style={styles.resultLabel}>seqNo: {proposal.seqNo}</p>
							<p style={styles.resultLabel}>status: {proposal.status}</p>
							<p style={styles.resultLabel}>signatures: {proposal.signatures.length}</p>
						</div>
					))}
					{proposals.length === 0 && <p style={styles.resultLabel}>No proposals loaded yet.</p>}
				</div>
			</section>
		</ScreenShell>
	)
}

const styles = {
	panel: {
		width: '100%',
		background: '#fff',
		borderRadius: 12,
		padding: '1rem',
		border: '1px solid #e5e7eb',
		marginBottom: '0.75rem',
	} as CSSProperties,
	title: {
		margin: '0 0 0.5rem',
		fontSize: '1rem',
		color: '#0f172a',
	} as CSSProperties,
	label: {
		display: 'block',
		marginTop: '0.5rem',
		fontSize: '0.85rem',
		color: '#334155',
	} as CSSProperties,
	checkboxLabel: {
		display: 'flex',
		alignItems: 'center',
		gap: '0.45rem',
		marginTop: '0.7rem',
		fontSize: '0.85rem',
		color: '#334155',
	} as CSSProperties,
	input: {
		width: '100%',
		marginTop: '0.35rem',
		padding: '0.45rem 0.5rem',
		border: '1px solid #cbd5e1',
		borderRadius: 8,
		fontFamily: 'ui-monospace, monospace',
	} as CSSProperties,
	textarea: {
		width: '100%',
		minHeight: 110,
		marginTop: '0.35rem',
		padding: '0.45rem 0.5rem',
		border: '1px solid #cbd5e1',
		borderRadius: 8,
		fontFamily: 'ui-monospace, monospace',
	} as CSSProperties,
	button: {
		display: 'block',
		marginTop: '0.7rem',
		padding: '0.55rem 1rem',
		background: '#1d4ed8',
		color: '#fff',
		border: 'none',
		borderRadius: 8,
		cursor: 'pointer',
		fontSize: '0.9rem',
	} as CSSProperties,
	success: {
		marginTop: '0.5rem',
		fontSize: '0.85rem',
		color: '#166534',
	} as CSSProperties,
	error: {
		marginTop: '0.5rem',
		fontSize: '0.85rem',
		color: '#b91c1c',
		whiteSpace: 'pre-wrap',
	} as CSSProperties,
	resultBox: {
		marginTop: '0.85rem',
		display: 'flex',
		flexDirection: 'column',
		gap: '0.4rem',
	} as CSSProperties,
	builderBox: {
		marginTop: '0.3rem',
		marginBottom: '0.7rem',
		padding: '0.65rem',
		borderRadius: 8,
		border: '1px solid #e2e8f0',
		background: '#f8fafc',
	} as CSSProperties,
	resultLabel: {
		margin: 0,
		fontSize: '0.8rem',
		color: '#475569',
	} as CSSProperties,
	mono: {
		fontSize: '0.72rem',
		wordBreak: 'break-all',
		fontFamily: 'ui-monospace, monospace',
		background: '#f8fafc',
		padding: '0.4rem',
		borderRadius: 6,
	} as CSSProperties,
	list: {
		display: 'flex',
		flexDirection: 'column',
		gap: '0.5rem',
		marginTop: '0.8rem',
	} as CSSProperties,
	listItem: {
		padding: '0.55rem',
		borderRadius: 8,
		border: '1px solid #e2e8f0',
		background: '#f8fafc',
	} as CSSProperties,
}
