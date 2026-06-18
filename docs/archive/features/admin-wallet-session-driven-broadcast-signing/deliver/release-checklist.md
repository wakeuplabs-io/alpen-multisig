# Release Checklist — Manual Real-Device Path Verification

**Feature:** admin-wallet-session-driven-broadcast-signing
**Scenario:** REG-01 — Manual real-device path
**Step:** 06-01

> **Note:** This checklist cannot be automated — it requires physical hardware devices (Trezor and Ledger) and real on-device confirmation. Run these steps on a developer machine with devices connected before any release.

---

## Prerequisites

- [ ] Trezor device (Model T or One) with firmware up to date
- [ ] Ledger device (Nano S Plus or X) with firmware up to date
- [ ] Desktop app built and running locally (`npm run tauri dev`)
- [ ] Backend running locally (`cargo run -p orchestrator-be`)
- [ ] Test wallet created on each device with known seed (testnet only)
- [ ] Test multisig wallet configured with the hardware wallet as a participant
- [ ] At least one proposal created and in "pending signatures" state

---

## Path A: Trezor — Happy Path

| # | Step | Expected Result | Pass |
|---|------|----------------|------|
| A1 | Connect Trezor via USB | Device detected, Trezor Suite does not claim exclusive access | [ ] |
| A2 | Open desktop app, navigate to wallet login | Trezor listed as available signer device | [ ] |
| A3 | Select Trezor and authenticate | On-device prompt appears, user confirms on device, app shows logged-in state with Trezor identity | [ ] |
| A4 | Navigate to a pending proposal with approval | Proposal details visible, "Broadcast" control enabled | [ ] |
| A5 | Click "Broadcast" | App shows "Confirm on device" prompt, Trezor displays transaction details for confirmation | [ ] |
| A6 | Confirm transaction on Trezor device | Trezor screen shows "Transaction confirmed", app proceeds to commit+reveal | [ ] |
| A7 | Verify commit+reveal completed | Backend shows proposal status = "broadcast", on-chain transaction hash visible in app | [ ] |

---

## Path B: Ledger — Happy Path

| # | Step | Expected Result | Pass |
|---|------|----------------|------|
| B1 | Connect Ledger via USB | Device detected, Ledger Live does not claim exclusive access | [ ] |
| B2 | Open desktop app, navigate to wallet login | Ledger listed as available signer device | [ ] |
| B3 | Select Ledger and authenticate | On-device prompt appears, user confirms on device, app shows logged-in state with Ledger identity | [ ] |
| B4 | Navigate to a pending proposal with approval | Proposal details visible, "Broadcast" control enabled | [ ] |
| B5 | Click "Broadcast" | App shows "Confirm on device" prompt, Ledger displays transaction details for confirmation | [ ] |
| B6 | Confirm transaction on Ledger device | Ledger screen shows "Transaction confirmed", app proceeds to commit+reveal | [ ] |
| B7 | Verify commit+reveal completed | Backend shows proposal status = "broadcast", on-chain transaction hash visible in app | [ ] |

---

## Path C: Trezor — Device Unplugged During Flow

| # | Step | Expected Result | Pass |
|---|------|----------------|------|
| C1 | Complete steps A1–A4 (Trezor connected and logged in) | Logged in, proposal visible | [ ] |
| C2 | Physically unplug Trezor USB cable | App detects device disconnection | [ ] |
| C3 | Attempt to click "Broadcast" | Error state shown: "Device disconnected", no resubmit control enabled, nothing sent to backend | [ ] |
| C4 | Verify no broadcast occurred | Backend proposal status unchanged (still "pending"), no on-chain transaction | [ ] |

---

## Path D: Ledger — User Refuses on Device

| # | Step | Expected Result | Pass |
|---|------|----------------|------|
| D1 | Complete steps B1–B5 (Ledger connected, broadcast initiated) | Device showing confirmation prompt | [ ] |
| D2 | Reject/cancel the transaction on the Ledger device | Ledger shows "Transaction rejected", app receives refusal signal | [ ] |
| D3 | Verify app state after refusal | Error state shown: "Transaction refused by device", no resubmit control enabled, nothing sent to backend | [ ] |
| D4 | Verify no broadcast occurred | Backend proposal status unchanged (still "pending"), no on-chain transaction | [ ] |

---

## Path E: Trezor — User Refuses on Device

| # | Step | Expected Result | Pass |
|---|------|----------------|------|
| E1 | Complete steps A1–A5 (Trezor connected, broadcast initiated) | Device showing confirmation prompt | [ ] |
| E2 | Reject/cancel the transaction on the Trezor device | Trezor shows "Transaction rejected", app receives refusal signal | [ ] |
| E3 | Verify app state after refusal | Error state shown: "Transaction refused by device", no resubmit control enabled, nothing sent to backend | [ ] |
| E4 | Verify no broadcast occurred | Backend proposal status unchanged (still "pending"), no on-chain transaction | [ ] |

---

## Path F: Ledger — Device Unplugged During Flow

| # | Step | Expected Result | Pass |
|---|------|----------------|------|
| F1 | Complete steps B1–B4 (Ledger connected and logged in) | Logged in, proposal visible | [ ] |
| F2 | Physically unplug Ledger USB cable | App detects device disconnection | [ ] |
| F3 | Attempt to click "Broadcast" | Error state shown: "Device disconnected", no resubmit control enabled, nothing sent to backend | [ ] |
| F4 | Verify no broadcast occurred | Backend proposal status unchanged (still "pending"), no on-chain transaction | [ ] |

---

## Sign-Off

| Role | Name | Date | All Paths Passed |
|------|------|------|-----------------|
| Tester | | | [ ] |
| Reviewer | | | [ ] |

---

**Result:** All 6 paths (A–F) must pass before release. Any failure blocks the release and must be filed as a P0 bug.
