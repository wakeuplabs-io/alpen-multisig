// UI-facing hardware-device unions (Phase 8). Kept in the domain model so
// presentational components can type their props without importing the API
// boundary (architecture rule: components must not import infrastructure).

/** Specific connected hardware device. */
export type HwDeviceType = 'trezor' | 'ledger'

/** Address script type to confirm on-device: P2TR receive, P2WPKH Admin ID. */
export type VerifyScriptType = 'p2tr' | 'p2wpkh'
