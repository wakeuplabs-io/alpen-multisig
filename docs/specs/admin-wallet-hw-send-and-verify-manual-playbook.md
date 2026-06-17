# Manual Test Playbook — HW Send + Verify-on-Device (Phase 8)

> Required deliverable for [`admin-wallet-hw-send-and-verify.md`](./admin-wallet-hw-send-and-verify.md).
> Hardware-device behavior is **not** in CI (see that spec's Testing strategy). These flows are
> validated by hand against an emulator or a real device, the same way Ledger broadcast signing is
> exercised today. CI covers only the no-device seams (signer/verify dispatch, error→no-broadcast).

## Scope of this playbook

| # | Flow | Device | Spec ref |
|---|------|--------|----------|
| 1 | Send happy path | Trezor | §4.3.5 |
| 2 | Send happy path (regression) | Ledger | §4.3.5 |
| 3 | Verify receive address (P2TR) | Trezor / Ledger | §4.3.4.2 |
| 4 | Verify Admin ID (P2WPKH) | Trezor / Ledger | §4.2 |
| 5 | On-device rejection / timeout | Trezor / Ledger | §4.3.5.5.1 |

## Prerequisites (local stack)

- Local regtest stack up (Bitcoin Core + Electrum indexer), backend running:
  `cargo run -p orchestrator-be`.
- Desktop app: `cd desktop-app && npm run tauri dev`.
- Funded Admin Wallet on regtest (mine a few blocks to a receive address, then Refresh the panel).

### Trezor emulator

```bash
./scripts/trezor-up.sh        # starts the Trezor emulator (UDP 21324)
# … run the flows …
./scripts/trezor-down.sh      # tears it down
```

Point the app at the emulator UDP port (21324), not Trezor Bridge — the connect error hint says so
if the transport is wrong.

### Ledger emulator (Speculos)

```bash
./scripts/ledger-up.sh <path-to-bitcoin_testnet_*.elf>
# In desktop-app/.env: LEDGER_SPECULOS_URL=http://localhost:5001
# To observe the real on-device prompt (no auto-approve): LEDGER_SPECULOS_AUTO_APPROVE=0
```

## Flow 1 — Trezor Send happy path (§4.3.5)

1. Connect the Trezor signer (BIP-86 Admin Wallet). Confirm the panel shows a balance and is **not**
   watch-only (Send button enabled).
2. Open **Send**, enter a regtest destination (e.g. a `bcrt1q…` from `bitcoin-cli getnewaddress`),
   an amount below balance, and a fee rate. Confirm the estimate summary renders.
3. Press **Confirm**. Expected:
   - The form shows **“Confirm on your Trezor”** with the pending caption and the button reads
     **“Confirm on your device…”**.
   - The device displays the **outputs** (recipient address + amount), **fee**, and a final
     **Sign transaction** screen. Amounts must match the form.
4. Approve on the device.
   - **PASS:** the Send result card shows a txid; the tx is broadcast and confirms after a block.
     The tx matches the Phase 6 contract: recipient gets the exact amount, change returns to the
     first unused internal index, every input signals RBF.
   - **FAIL:** any amount/destination on the device differs from the form, or finalize/broadcast fails.

## Flow 2 — Ledger Send happy path (regression, §4.3.5)

Repeat Flow 1 with a Ledger session over Speculos. With `LEDGER_SPECULOS_AUTO_APPROVE=0`, approve the
**Review transaction** screens manually. **PASS** when the send completes end-to-end exactly as before
this phase (confirms the generalized dispatch did not break the existing Ledger path).

## Flow 3 — Verify receive address (P2TR, §4.3.4.2)

1. In the wallet panel, find the **Receive address** card. For an HW session a **“Verify on device”**
   button is shown.
2. Press it. Expected: the button reads **“Confirm on your <Device>…”** and the device displays a
   **taproot** address (`bcrt1p…` on regtest).
3. Compare the on-screen address to the panel address character-by-character.
   - **PASS (match):** approve → the row shows **“Confirmed the receive address on your <Device>.”**
   - **PASS (tamper/mismatch):** if they differ, do **not** approve — reject; the row shows the failure.
   - **PASS (rejection):** rejecting on the device surfaces the failure caption.

## Flow 4 — Verify Admin ID (P2WPKH, §4.2)

Same as Flow 3, but on the **Admin ID** card. The device must display a **native segwit** address
(`bcrt1q…`, BIP-84 `m/84'/1'/73'/0/0` on regtest). The Admin ID is identity only — never fund it.

## Flow 5 — On-device rejection / timeout (§4.3.5.5.1)

1. Start a Send (Flow 1) and, at the device approval screen, **reject** the transaction.
   - **PASS:** the form returns to an error state with the on-device rejection message and the hint to
     adjust/try again; **nothing is broadcast** (verify mempool is unchanged).
2. Start a Send and let the device prompt sit untouched.
   - **PASS:** after ~3 minutes the app surfaces a timeout error (“… did not respond within 180
     seconds …”); nothing is broadcast. Retrying after re-approving succeeds.

## Notes

- The CI seams that back these flows (signer dispatch, verify dispatch, error→no-broadcast,
  network/script-type mapping) are covered by unit tests in `wallet_send.rs`, `hw_wallet.rs`,
  `commands/hw_wallet.rs`, and `commands/admin_wallet.rs` — run with `cargo test --workspace`.
- trezor-client 0.1.5's built-in `sign_tx` flow classifies P2TR inputs as `EXTERNAL` (won't sign), so
  the Trezor adapter drives its own `TxAck` loop with `SPENDTAPROOT` / `PAYTOTAPROOT`. If a future
  bump changes the flow, re-run Flow 1.
