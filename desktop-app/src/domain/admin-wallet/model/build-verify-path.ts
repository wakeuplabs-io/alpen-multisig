// Verify-on-device derivation paths (Phase 8, PRD §4.2 / §4.3.4.2).
//
// Mirrors the backend custody conventions: BIP-86 / P2TR for the Admin Wallet
// receive keychain and BIP-84 / P2WPKH for the Admin ID. The coin type follows
// the active network (0' on mainnet, 1' on every test network), matching
// `admin_wallet_account_origin_path` / the Ledger coin-type mapping in Rust.

/** BIP-44 coin type for the network: 0' on mainnet, 1' on testnet/regtest/signet. */
export function coinType(network: string): number {
	return network === 'bitcoin' || network === 'mainnet' ? 0 : 1
}

/** Full receive (external) derivation path for a P2TR Admin Wallet address. */
export function buildReceiveVerifyPath(network: string, index: number): string {
	return `m/86'/${coinType(network)}'/73'/0/${index}`
}

/** Full Admin ID derivation path (BIP-84 / P2WPKH). */
export function buildAdminIdVerifyPath(network: string): string {
	return `m/84'/${coinType(network)}'/73'/0/0`
}
