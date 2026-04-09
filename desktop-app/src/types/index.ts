// ─── Authority ───────────────────────────────────────────────────────────────

export type Authority =
  | 'alpen_admin'
  | 'strata_admin'
  | 'sequencer_manager'
  | 'security_council'
  | 'payout_admin'

export const AUTHORITY_LABELS: Record<Authority, string> = {
  alpen_admin: 'Alpen Administrator',
  strata_admin: 'Strata Administrator',
  sequencer_manager: 'Sequencer Manager',
  security_council: 'Security Council',
  payout_admin: 'Payout Administrator',
}

// ─── Proposal ────────────────────────────────────────────────────────────────

export type ProposalStatus =
  | 'pending'
  | 'approved'
  | 'enacted'
  | 'canceled'
  | 'expired'

export type QuorumStatus = {
  collected: number
  required: number
  isReached: boolean
}

export type Proposal = {
  id: string
  actionId: string
  seqNo: number
  authority: Authority
  status: ProposalStatus
  actionPayload: unknown
  quorum: QuorumStatus
  createdAt: string
  expiresAt: string
  updatedAt: string
}

export type ProposalSignature = {
  id: string
  proposalId: string
  signerPubkey: string
  signature: string
  submittedAt: string
}

// ─── Session ─────────────────────────────────────────────────────────────────

export type Session = {
  sessionId: string
  signerPubkey: string
  authority: Authority
  expiresAt: string
}

// Returned by the Tauri create_session command — sessionId stays in Rust, never in JS.
export type SessionInfo = Omit<Session, 'sessionId'>

// ─── Auth ─────────────────────────────────────────────────────────────────────

export type AuthChallenge = {
  nonce: string
  expiresAt: string
}

// ─── Wallet ──────────────────────────────────────────────────────────────────

export type WalletAddress = {
  address: string
  derivationPath: string
  index: number
}

export type ConnectedWallet = {
  deviceName: string
  addresses: WalletAddress[]
}

// ─── API Response shapes ──────────────────────────────────────────────────────

export type ApiResult<T> =
  | { ok: true; data: T }
  | { ok: false; error: string }
