import { createWalletAdapter } from './create-wallet-adapter'

/** Single Trezor adapter instance shared by wallet connect and signing flows. */
export const defaultWalletAdapter = createWalletAdapter('trezor')
