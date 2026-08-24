// UI-facing hardware-device unions (Phase 8). Kept in the domain model so
// presentational components can type their props without importing the API
// boundary (architecture rule: components must not import infrastructure).

/** Specific connected hardware device. */
export type HwDeviceType = 'trezor' | 'ledger'

/** Address script type to confirm on-device: P2TR receive, P2WPKH Admin ID. */
export type VerifyScriptType = 'p2tr' | 'p2wpkh'

/**
 * What a hardware session needs in order to render an address on its device screen.
 * Present only for HW sessions: a mnemonic session has no screen to compare against.
 */
export type AdminIdVerifyContext = {
	deviceType: HwDeviceType
	network: string
	/**
	 * Connect-returned Admin ID path (BIP-84). Verifying against this exact path keeps the
	 * device showing the same key/coin it derived at connect — Trezor uses coin type 0',
	 * Ledger 1' on test nets — so app and device match (and the Trezor emulator, which
	 * rejects m/84'/1'/73', stays happy).
	 */
	derivationPath: string
}
