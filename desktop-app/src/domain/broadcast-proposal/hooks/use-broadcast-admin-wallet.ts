import { useEffect, useMemo } from 'react'
import type { AdminWalletError } from '@/api/admin-wallet'
import { useAdminWalletCapability } from '@/domain/admin-wallet/hooks/use-admin-wallet-capability'
import { useAdminWalletSync } from '@/domain/admin-wallet/hooks'
import { useEnsureAdminWalletSession } from '@/domain/admin-wallet/hooks/use-ensure-admin-wallet-session'
import type { WalletAdapter } from '@/wallet/types'
import { useAdminWalletInfo } from './use-admin-wallet-info'
import type { AdminWalletInfoView } from './use-admin-wallet-info'
import type { SignerKind } from './use-broadcast-proposal'

/** Admin-wallet props of `BroadcastDetailsCard` — spread as `{...cardProps}`. */
export type BroadcastAdminWalletCardProps = {
	canSign: boolean
	canSignReason: string | undefined
	/** `null` = loading, `undefined` = unavailable, object = loaded. */
	adminWalletInfo: AdminWalletInfoView | null | undefined
	lastSyncedAt: string | null | undefined
	syncError: AdminWalletError | null | undefined
}

type UseBroadcastAdminWalletReturn = {
	/** Normalised for `useBroadcastProposal` — `'hardware'` only when the backend says so. */
	signerKind: SignerKind
	/** Raw backend value including `'none'`, for `<BroadcastFundingSignerBanner />`. */
	backendSignerKind: 'hardware' | 'mnemonic' | 'none'
	canSign: boolean
	canSignReason: string | undefined
	isAdminWalletMode: boolean
	cardProps: BroadcastAdminWalletCardProps
}

/**
 * Everything a broadcast screen needs from the Admin Wallet: an initialised session, the funding
 * snapshot, the signing capability and the Electrum sync state.
 *
 * Both send screens (proposal and cancel) depend on this identically — the commit is always paid
 * by the Admin Wallet — so it lives in one place. Leaving it to each screen is what left the cancel
 * route with a permanently disabled send button (issue #484).
 */
export function useBroadcastAdminWallet(adapter: WalletAdapter): UseBroadcastAdminWalletReturn {
	const { sessionReady } = useEnsureAdminWalletSession(adapter)

	const { adminWalletInfo, refresh: refreshAdminWalletInfo } = useAdminWalletInfo(sessionReady)
	const { canSign, signerKind: backendSignerKind, canSignReason } = useAdminWalletCapability()
	const isAdminWalletMode = adminWalletInfo != null
	const signerKind: SignerKind = backendSignerKind === 'hardware' ? 'hardware' : 'mnemonic'

	const { syncStatus, triggerSync } = useAdminWalletSync()

	// Trigger an Electrum sync on mount when in admin_wallet mode; re-read the funding info once
	// the sync resolves so the card shows the post-sync balance and receive address (the initial
	// fetch races the sync and would otherwise pin a stale 0-sats snapshot).
	//
	// `triggerSync` and `refreshAdminWalletInfo` are stable callbacks and `isAdminWalletMode` is
	// derived from a reference-stable snapshot, so this fires once, on the false → true flip.
	// Depending on `adminWalletInfo` itself would re-sync on every poll tick.
	useEffect(() => {
		if (isAdminWalletMode) {
			void triggerSync().then(() => refreshAdminWalletInfo())
		}
	}, [isAdminWalletMode, triggerSync, refreshAdminWalletInfo])

	const cardProps = useMemo<BroadcastAdminWalletCardProps>(
		() => ({
			canSign,
			canSignReason,
			adminWalletInfo,
			lastSyncedAt: isAdminWalletMode ? (syncStatus?.lastSyncedAt ?? null) : undefined,
			syncError: isAdminWalletMode
				? syncStatus?.lastError != null
					? { type: 'SyncIncomplete' as const, message: syncStatus.lastError.message }
					: null
				: undefined,
		}),
		[canSign, canSignReason, adminWalletInfo, isAdminWalletMode, syncStatus],
	)

	return { signerKind, backendSignerKind, canSign, canSignReason, isAdminWalletMode, cardProps }
}
