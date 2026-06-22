// Infers the verify-on-device network token from a derivation path's coin type,
// so the coin name sent to the device matches the path the device actually derived.
//
// The Admin ID coin type is device-specific (Trezor uses 0', Ledger uses 1' on test
// networks), so we must derive the network from the connect-returned path rather than
// from the session/env network — otherwise the device is asked for a key/coin that does
// not match what it showed at connect (and the Trezor emulator rejects m/84'/1'/73').

/** Coin type 0' (`/0'/73'` or `/0h/73h`) → mainnet; anything else → regtest. */
export function networkFromPath(derivationPath: string): string {
	return /\/0'\/73'/.test(derivationPath) || /\/0h\/73h/.test(derivationPath) ? 'bitcoin' : 'regtest'
}
