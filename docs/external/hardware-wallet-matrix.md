# Hardware Wallet Compatibility Matrix

**Satisfies: PRD §3.2** — Hardware wallet support requirements

## Overview

The Strata Multisig application supports hardware wallets that provide the following capabilities required by the Strata/Alpen protocol:

- **Taproot key derivation** — BIP-86 compliant derivation path `m/86'/0'/73'/0/n`
- **Raw ECDSA signing** — secp256k1 signatures over SPS-65 sighash (no Bitcoin message prefix)
- **On-device display** — Signer must review action details before signing
- **HID interface** — USB communication for desktop integration

## Supported Devices

### Trezor

| Model | Status | Notes |
|-------|--------|-------|
| Trezor Model T | **Supported** | Full Taproot support, on-device display, message signing |
| Trezor Safe 3 | **Supported** | Full Taproot support, on-device display, message signing |
| Trezor One | **Not supported** | Lacks Taproot support |

**Trezor Integration:**
- Connected via USB HID
- Firmware version 2.6.0 or later recommended for full Taproot support
- On-device verification of addresses and transaction details
- Raw ECDSA signing for SPS-65 protocol compliance

### Ledger

| Model | Status | Notes |
|-------|--------|-------|
| Ledger Nano X | **Supported** | Taproot support via Bitcoin app 2.1.0+ |
| Ledger Nano S Plus | **Supported** | Taproot support via Bitcoin app 2.1.0+ |
| Ledger Nano S | **Limited** | Memory constraints may limit complex operations |
| Ledger Blue | **Not tested** | Compatibility not verified |

**Ledger Integration:**
- Connected via USB HID
- Bitcoin app version 2.1.0 or later required for Taproot support
- On-device verification of addresses and transaction details
- Raw ECDSA signing for SPS-65 protocol compliance

## Signing Format

The application uses **raw ECDSA** over the SPS-65 sighash, which differs from standard Bitcoin message signing (BIP-137). This is required because the Strata/Alpen protocol expects bare ECDSA signatures without the Bitcoin-specific prefix.

**Sighash computation:**
```
sighash = SHA256(
    SHA256(tag)           ← 32 bytes, tag = "strata/admin/<type_name>"
    ‖ seqno_be            ← 8 bytes, big-endian u64
    ‖ sighash_payload     ← variable, encoded action-specific data
)
```

Both Trezor and Ledger devices support this signing mode through their message signing capabilities.

## Address Derivation

All hardware wallets use the same derivation path for the Alpen/Strata multisig:

```
m/86'/0'/73'/0/n
```

Where:
- `86'` — BIP-86 Taproot account (hardened)
- `0'` — Coin type for Bitcoin (hardened)
- `73'` — Alpen/Strata account (hardened)
- `0` — Change index
- `n` — Address index (0-19 for the first 20 addresses)

The application displays the first 20 addresses from this path, and users can verify each address on their hardware wallet screen.

## Device Verification

Before signing any transaction, users should verify:

1. **Address verification** — The address shown in the application matches the address displayed on the hardware wallet screen
2. **Transaction details** — The action details (type, parameters, sequence number) displayed on the hardware wallet match what is shown in the application
3. **Signing prompt** — The hardware wallet clearly indicates what is being signed

## Firmware Requirements

| Device | Minimum Firmware | Recommended Firmware |
|--------|------------------|---------------------|
| Trezor Model T | 2.6.0 | Latest stable |
| Trezor Safe 3 | 2.6.0 | Latest stable |
| Ledger Nano X | 2.1.0 (Bitcoin app) | Latest stable |
| Ledger Nano S Plus | 2.1.0 (Bitcoin app) | Latest stable |

## Troubleshooting

### Device Not Detected

- Ensure the device is connected via USB and unlocked
- Check that your user account has permission to access USB devices (Linux: add your user to the `plugdev` group)
- Try a different USB port or cable
- Restart the application after connecting the device

### Signing Fails

- Ensure firmware is up to date
- Verify that the device supports Taproot (BIP-86)
- Check that the Bitcoin app (Ledger) or firmware (Trezor) supports raw message signing
- Ensure the device is unlocked and ready to sign

### Address Mismatch

- If the address shown in the application does not match the device display, **do not sign**
- Disconnect and reconnect the device
- Verify the derivation path is correct: `m/86'/0'/73'/0/n`
- Contact support if the issue persists

## Related Documents

- [Setup Guide](./setup-guide.md) — Installation and first-run setup
- [Architecture Overview](./architecture-overview.md) — System design and signing flow
