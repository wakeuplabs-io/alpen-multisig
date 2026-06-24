// Verify-on-device derivation paths (Phase 8, PRD §4.2 / §4.3.4.2).
//
// Mirrors the backend custody conventions: BIP-86 / P2TR for the Admin Wallet receive
// keychain. The coin type is device-specific (Trezor 0' on every network, Ledger 1' on
// test nets / 0' on mainnet), so the receive verify path is built from the connect-returned
// account path (see `admin_wallet_account_path` in Rust and `AdminWalletCapability`), NOT
// rebuilt from the session network — matching how the Admin ID verify path uses the exact
// connect-returned path (see `network-from-path.ts` and `admin-id-row.tsx`).

/** Full receive (external) verify path for a P2TR Admin Wallet address at `index`. */
export function receiveVerifyPathFromAccount(accountPath: string, index: number): string {
	return `${accountPath}/0/${index}`
}
