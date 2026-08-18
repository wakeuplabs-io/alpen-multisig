/** Compressed pubkey hex: 33 bytes, `02`/`03` prefix, optional `0x`. */
export const compressedPubKeyHexPattern = /^(?:0x)?(?:02|03)[0-9a-fA-F]{64}$/
