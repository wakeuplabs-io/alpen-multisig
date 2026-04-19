import { createWalletAdapter } from './create-poc-wallet-adapter'

/** Single Trezor PoC adapter instance shared by wallet connect and signing flows. */
export const pocWalletAdapter = createWalletAdapter('trezor')
