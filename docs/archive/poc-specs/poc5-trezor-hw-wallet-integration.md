# Spec: UC-1 — Hardware Wallet Connect & Address Selection
> **Status: Historical (walking skeleton / POC).** Superseded for product behavior by [`admin-wallet-implementation-plan.md`](./admin-wallet-implementation-plan.md), [`architecture/overview.md`](../../architecture/overview.md), and active `admin-wallet-*` specs. Kept for traceability.
>

## Objective

Implement the first user-facing flow of the production app: connect a hardware wallet (Trezor, via HID/Tauri) and present the first 20 addresses at the `m/86'/0'/73'/0/n` derivation path (BIP86 Taproot, account `73'` hardened) so the user can select the address whose private key will sign administrative transactions.

This spec replaces the POC-5 skeleton, which originally proved HID transport and PSBT signing in a BIP84
shape before the product path converged on `m/86'/0'/73'/0/n`. That context is preserved in
`docs/2-discovery/16-poc5-trezor-findings.md`.

**PRD requirements covered:** 6.2, 6.3, 6.4, 6.5.

## Scope

### Included

- New Rust function `trezor::list_addresses(count)` — fetches `count` P2TR addresses at `m/86'/0'/73'/0/{n}`
- New struct `HwAddressEntry` in `hw_wallet/mod.rs`
- New Tauri command `list_hw_addresses` in `commands/hw_wallet.rs`
- New Tauri command `verify_address_on_device` in `commands/hw_wallet.rs`
- New TypeScript type `HwAddressEntry` in `wallet/types.ts`
- New `listAddresses()` method on `WalletAdapter` (optional, Trezor only for now)
- New React component `HwWalletConnect` — connect → pick → confirm address flow
- Copy-to-clipboard and "verify on device" actions on the selected address view

### NOT included

- Ledger support (stub remains, pending Speculos validation)
- PIN entry or passphrase entry (returns descriptive error — unchanged from POC-5)
- Signing — `sign_with_trezor` / `sign_admin_sps65_binding` are untouched
- Backend session or nonce signing (UC-2)
- Multisig selection screen (UC-3)

### POC-5 artifacts preserved (do not delete)

- `trezor::connect()` / `get_trezor_info` — current single-path connect defaults to `m/86'/0'/73'/0/0`
- `trezor::sign_admin_sps65_binding()` — PSBT signing, wired to `sign_with_trezor`
- `trezor::sign_message_poc_bip137()` — BIP-137 helper, POC only
- `trezor::open_trezor()`, `trezor::resolve()` — shared infrastructure

## Design Decisions

### BIP86 Taproot path (`m/86'/0'/73'/0/n`)

The product derivation path uses account `73'` (hardened) as the Strata admin key namespace. Each index `n`
(0–19) represents a distinct signer address the user may have enrolled on a multisig. The path uses BIP86
semantics: x-only Schnorr key, P2TR (bech32m) address. This differs from earlier POC BIP84/P2WPKH assumptions.

### `SPENDTAPROOT` script type

Taproot address derivation on Trezor requires `InputScriptType::SPENDTAPROOT` in `get_public_key`. The device returns an xpub from which the x-only pubkey is extracted; the address is then `p2tr_tweaked` (key-path spend, no script tree). Note: `sign_message` with `SPENDTAPROOT` is rejected by current Trezor firmware — this is a known limitation documented in the findings doc and is out of scope here.

### Sequential HID calls (20 round-trips)

`list_addresses` opens one HID session and loops 20 `get_public_key` calls. All calls share the same `Trezor` handle. If any call fails, the function returns the error immediately and the user sees the device error message.

### `spawn_blocking` for all HID work

Tauri async commands must not block the executor. All HID calls are dispatched through `tokio::task::spawn_blocking`.

### Address display format

Full bech32m address is shown after selection. The picker truncates to `first_8…last_6` chars for readability. Derivation path is always shown in full alongside each row.

## Technical Design

### Layer diagram

```
React — HwWalletConnect.tsx
  ├── listAddresses()          → invoke('list_hw_addresses')
  └── verifyOnDevice()         → invoke('verify_address_on_device')
        └── commands/hw_wallet.rs   (Tauri IPC boundary)
              └── infrastructure/hw_wallet/trezor.rs
                    └── trezor-client crate (HID → device)
```

### New types

**Rust** (`hw_wallet/mod.rs`):

```rust
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HwAddressEntry {
    pub index: u32,
    pub derivation_path: String,  // "m/86'/0'/73'/0/{index}"
    pub address: String,          // bech32m P2TR address
    pub public_key_hex: String,   // compressed 33-byte pubkey as hex
}
```

**TypeScript** (`wallet/types.ts`):

```ts
export type HwAddressEntry = {
  index: number;
  derivationPath: string;
  address: string;
  publicKeyHex: string;
};
```

### New Rust functions (`trezor.rs`)

```
pub fn list_addresses(count: usize) -> Result<Vec<HwAddressEntry>, String>
```

- Base path: `m/86'/0'/73'/0`
- For `n` in `0..count`:
  - Build path string `"m/86'/0'/73'/0/{n}"`, parse with `DerivationPath::from_str`.
  - Call `trezor.get_public_key(&path, InputScriptType::SPENDTAPROOT, Network::Bitcoin, /*display=*/false)` → `resolve()`.
  - Extract x-only pubkey from `xpub.public_key` (drop even/odd byte), construct `XOnlyPublicKey`, call `TweakedPublicKey::dangerous_assume_tweaked`, then `Address::p2tr_tweaked(..., KnownHrp::Mainnet)`.
  - Push `HwAddressEntry { index: n as u32, derivation_path, address, public_key_hex }`.
- Returns the full vec or the first error encountered.

```
pub fn verify_address_on_device(derivation_path: String) -> Result<(), String>
```

- Parses the path, calls `get_public_key` with `display = true` so the device shows the address on screen.
- Returns `Ok(())` on success; the UI overlays a "Check your device" message while this call is in-flight.

### New Tauri commands (`commands/hw_wallet.rs`)

```rust
#[tauri::command]
pub async fn list_hw_addresses(
    _state: State<'_, AppState>,
    count: Option<u32>,
) -> Result<Vec<HwAddressEntry>, String> {
    tokio::task::spawn_blocking(move || trezor::list_addresses(count.unwrap_or(20) as usize))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn verify_address_on_device(
    _state: State<'_, AppState>,
    derivation_path: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || trezor::verify_address_on_device(derivation_path))
        .await
        .map_err(|e| e.to_string())?
}
```

Both commands must be added to `generate_handler![]` in `main.rs`.

### TypeScript adapter (`wallet/trezor-poc-adapter.ts`)

Add `listAddresses(count = 20)` to the returned adapter object:

```ts
async listAddresses(count = 20): Promise<HwAddressEntry[]> {
    const result = await tauriCall<HwAddressEntry[]>('list_hw_addresses', { count })
    if (!result.ok) throw new Error(result.error)
    return result.data
},
```

Update `WalletAdapter` in `wallet/types.ts` with an optional method:

```ts
listAddresses?(count?: number): Promise<HwAddressEntry[]>
```

### React component (`components/HwWalletConnect.tsx`)

Three logical phases, one component:

**Phase 1 — Connect**

- "Connect hardware wallet" button (or auto-trigger).
- On click: calls `adapter.listAddresses(20)`.
- Loading state: spinner + "Reading addresses from device…".
- Error state: error string from Rust, retry button.

**Phase 2 — Address picker**

- Scrollable list of 20 rows. Each row:
  - Index badge (`#0` … `#19`)
  - Full derivation path (`m/86'/0'/73'/0/0`)
  - Truncated address (`bc1pXXXXXXXX…XXXXXX`)
- Row is selectable (single selection).
- "Use this address" button — enabled when a row is selected.

**Phase 3 — Selected address**

- Full bech32m address in a read-only monospace field.
- "Copy" button — `navigator.clipboard.writeText(address)` with brief confirmation ("Copied!").
- "Verify on device" button:
  - Calls `tauriCall('verify_address_on_device', { derivationPath })`.
  - While in-flight: overlay or button spinner + "Check your Trezor screen and confirm the address matches."
  - On success: brief success note.
  - On error: show error string.
- "Change address" link — returns to Phase 2 with list already loaded (no re-fetch).
- "Disconnect" link — resets all state back to Phase 1.

### Key component table

| Component                    | File                             | Responsibility                                                      |
| ---------------------------- | -------------------------------- | ------------------------------------------------------------------- |
| `list_addresses()`           | `trezor.rs`                      | 20 SPENDTAPROOT `get_public_key` calls, builds `HwAddressEntry` vec |
| `verify_address_on_device()` | `trezor.rs`                      | `get_public_key` with `display=true`                                |
| `HwAddressEntry`             | `hw_wallet/mod.rs`               | Shared Rust type, camelCase serialization                           |
| `list_hw_addresses`          | `commands/hw_wallet.rs`          | Tauri command wrapping `list_addresses` in `spawn_blocking`         |
| `verify_address_on_device`   | `commands/hw_wallet.rs`          | Tauri command wrapping `verify_address_on_device`                   |
| `HwAddressEntry`             | `wallet/types.ts`                | TypeScript mirror type                                              |
| `listAddresses()`            | `trezor-poc-adapter.ts`          | `WalletAdapter` optional method                                     |
| `HwWalletConnect`            | `components/HwWalletConnect.tsx` | Full connect → pick → confirm UI flow                               |

## Constraints

- Do not modify `sign_admin_sps65_binding`, `sign_with_trezor`, `connect()`, or `get_trezor_info` — they remain for backward compat and future use.
- Do not add BIP84-only path guards to functions handling `m/86'` paths.
- All HID calls in async Tauri commands go through `tokio::task::spawn_blocking`.
- No `unwrap()` in new production code paths — propagate as `Result<_, String>`.
- `cargo clippy -- -D warnings` must pass. `npm run lint` and `npm run format:check` must pass.
- Use `InputScriptType::SPENDTAPROOT` for all `m/86'` paths.
- Frontend: tabs, single quotes, ~120 char lines.

## Acceptance Criteria (manual test — emulator)

1. App displays 20 rows, all with bech32m addresses (`bc1p…`) and paths `m/86'/0'/73'/0/0` through `m/86'/0'/73'/0/19`.
2. Selecting row index 5 stores path `m/86'/0'/73'/0/5` as the active derivation path.
3. Full address shown in Phase 3 matches the address in the selected row.
4. Copy button writes the full address to clipboard.
5. "Verify on device" triggers a ButtonRequest on the emulator (visible in emulator UI or trezord logs).
6. "Change address" returns to the picker without re-fetching from device.
7. "Disconnect" resets to Phase 1.
8. If device is disconnected mid-fetch, a clear error message is shown.

## Open Questions (carry-forward from POC-5)

These are not blockers for UC-1 but must be resolved before production signing:

| #   | Question                                                                                                          | Owner   |
| --- | ----------------------------------------------------------------------------------------------------------------- | ------- |
| Q1  | Must SPS-65 on-chain verification accept a Bitcoin sighash (PSBT binding) or exactly the tagged admin digest?     | Alpen   |
| Q2  | Is `m/86'/0'/73'/0/n` the final canonical path, or is account index subject to change?                            | Alpen   |
| Q3  | Must every cosigner use the same signing semantics (all HW, all software, or mixed)?                              | Alpen   |
| Q4  | Is PIN/passphrase support required before production, or is emulator/pinless device acceptable for alpha signers? | Product |

## Emulator Quick Reference

```bash
# Start bridge
trezord -e 21324

# Seed emulator
trezorctl -p udp:127.0.0.1:21324 debug load-device \
  --mnemonic "all all all all all all all all all all all all" \
  --pin "" \
  --passphrase-protection false

# Full Tauri app
cd desktop-app && npm run tauri dev
```

Expected: 20 rows with `bc1p…` addresses (bech32m, P2TR) at paths `m/86'/0'/73'/0/0` through `m/86'/0'/73'/0/19`.

## Related

- [ADR-001](../../architecture/adrs/001-alpen-crate-dependencies.md) — Alpen crate pins
- [ADR-005](../../architecture/adrs/005-layered-architecture.md) — Layered desktop architecture
- [`2-discovery/README.md`](../../2-discovery/README.md) — POC findings index

