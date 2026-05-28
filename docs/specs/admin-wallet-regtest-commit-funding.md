# Spec: Admin Wallet Regtest Commit Funding (US-H7)

> **⚠️ Superseded in part by Phase 3.6** ([`admin-wallet-commit-funding-only.md`](./admin-wallet-commit-funding-only.md)).
> The dual-path funding model described below — the `COMMIT_FUNDING` env-var toggle, the
> `BitcoindSendToAddress` legacy path, and the node-wallet `sendtoaddress` fallback — was **removed**.
> From Phase 3.6 onward the Admin Wallet (BDK) is the **sole** commit funder, enabled by
> `BITCOIN_NETWORK=regtest` + `ALLOW_DEV_MNEMONIC_SIGNING=1` (no `COMMIT_FUNDING`). The BIP-86
> derivation, UTXO selection, and commit/reveal protocol described here remain accurate.

## Objective

Enable governance **commit** funding from the Admin Wallet on regtest using BDK and a Bitcoin Core–compatible chain RPC endpoint, while keeping the existing commit/reveal protocol, orchestrator coordination, and reveal signing unchanged.

This spec is Phase 1 of the Admin Wallet program. It validates BIP-86 Taproot derivation at account `73'`, UTXO selection, and on-chain spend before the full PRD §4 wallet UI.

**Related:** [US-H7](../3-stories/story-map.md) · [Admin Wallet implementation plan](./admin-wallet-implementation-plan.md) · [Proposal broadcast commit + reveal](./proposal-broadcast-commit-reveal.md)

## Scope

### Included

- `approved` proposals only, within the existing desktop commit/reveal flow (US-H6).
- Commit transaction built and signed by BDK from Admin Wallet descriptors; broadcast via chain RPC (`BITCOIN_RPC_URL`).
- Regtest coin type `0'`, BIP-86 account `73'`:
  - External (receive/funding): `m/86'/0'/73'/0/*` — minimum `m/86'/0'/73'/0/0`.
  - Internal (change): first unused `m/86'/0'/73'/1/*`.
- Commit **destination**: operator-derived Taproot commit address from `broadcast_tx::derive_commit_address` (protocol unchanged).
- Reveal: built and signed with operator key in Tauri; orchestrator `claim` + `PATCH` unchanged.
- Feature flag `COMMIT_FUNDING=admin_wallet` (regtest only); default legacy `sendtoaddress` funding for CI/E2E.
- Phase 1 signing: regtest dev mnemonic in Tauri behind `ALLOW_DEV_*` guards; no HWI; no Ledger/Trezor for commit.
- Minimal broadcast UI: funding mode label, Admin Wallet address, available balance before confirm; existing phase progress and txids on success.
- Errors: insufficient Admin Wallet balance, chain RPC failure, misconfiguration.

### Not included

- Payout Administrator, P2TR Admin ID for payout, or any US-I* stories.
- Full WalletPanel / PRD §4.3.5 Send UX.
- US-H4 manual fee-rate UI for commit (use chain RPC fee estimate as today).
- US-H2 cancellation broadcast.
- HWI, hardware-signed commit, or mainnet/testnet enablement in US-H7.
- BDK Electrum (`bdk_electrum`) or Esplora backends.
- Changing SPS-50/SPS-51/SPS-65 validation or reveal construction rules.

## Requirements Alignment

- **Authorities:** Strata Administrator and Alpen Administrator only (current product scope for this program).
- **Two-key model:**
  - **Admin ID** `m/84'/0'/73'/0/0` (BIP-84 P2WPKH): auth + SPS-65 message signing; must not sign Bitcoin transactions.
  - **Admin Wallet** `m/86'/0'/73'/n/n` (BIP-86 P2TR): BTC custody and commit funding in US-H7.
- **Backend remains coordination-only** per `proposal-broadcast-commit-reveal.md` — no commit/reveal execution on orchestrator.
- **Signer safety:** commit funding only after quorum (`approved`); show funding source, destination context, and balance before confirm.
- **Manual fallback:** hex bundle export/copy from existing broadcast flow remains available.

## State Model

Unchanged from [proposal-broadcast-commit-reveal.md](./proposal-broadcast-commit-reveal.md):

- Canonical proposal states: `pending`, `approved`, `enacted`, `canceled`, `expired`.
- Broadcast sub-status: `idle` → `commit_broadcasted` → `commit_confirmed` → `reveal_broadcasted` → `reveal_confirmed` / `failed`.

US-H7 only changes **how the commit tx is funded and signed**, not orchestrator state transitions.

## Product Flow

### Entry

Same as US-H6: user opens an `approved` proposal on the broadcast screen (`/proposals/:actionId/broadcast`).

### Prepare (desktop)

`proposals_prepare_broadcast` unchanged for reveal fee preview and commit address display. When `COMMIT_FUNDING=admin_wallet`, also load Admin Wallet external address `m/86'/0'/73'/0/0` and spendable balance from BDK sync.

### Confirm + broadcast

On user confirmation:

1. `POST …/broadcast/claim` (orchestrator) — unchanged.
2. **Commit funding** (new path when enabled):
   - `CommitFunding::BdkAdminWalletMnemonic` builds commit tx: inputs from Admin Wallet, output to operator commit address, change to first unused `m/86'/0'/73'/1/*`.
   - Broadcast commit via BDK + chain RPC.
3. Wait for commit confirmation (regtest: optional `generatetoaddress` / mine helper) — unchanged.
4. Build and broadcast reveal with operator key in Tauri — unchanged.
5. `PATCH` progress after each phase — unchanged.

### UI additions (minimal)

- Display active commit funding mode: `bitcoind wallet (legacy)` vs `Admin Wallet (BDK)`.
- Show Admin Wallet funding address and available balance before confirm when mode is `admin_wallet`.
- Reuse existing broadcast phase indicators and txid display on success.

## Technical Design

### BDK stack

Add workspace dependencies (Phase 1 implementation):

| Crate | Role |
|---|---|
| `bdk_wallet` | Wallet, descriptors, UTXO selection, tx building |
| `bdk_bitcoind_rpc` | Bitcoin Core–compatible JSON-RPC sync and broadcast |

Chain access uses existing env `BITCOIN_RPC_URL` (+ `BITCOIN_RPC_USER` / `BITCOIN_RPC_PASS`). This is **chain RPC** (transport/protocol), not a product requirement to bundle Bitcoin Core for end users.

### Descriptor template (regtest, account `73'`)

Taproot (BIP-86) descriptors for multisig app Admin Wallet — regtest `coin_type = 0'`:

```
# External (chain index 0)
tr(m/86'/0'/73'/0/*)

# Internal / change (chain index 1)
tr(m/86'/0'/73'/1/*)
```

Phase 1 uses a single regtest mnemonic loaded in Tauri (dev only) to derive the descriptor secret. Production paths add hardware wallet signing in later phases; US-H7 does not.

### Hook: `broadcast_commit_then_reveal`

Location: `desktop-app/src-tauri/src/application/proposals.rs`.

Introduce a `CommitFunding` trait (or enum) selected at runtime:

| Variant | Behavior | Default |
|---|---|---|
| `BitcoindSendToAddress` | Current path: `BitcoinRpcClient::send_to_address` to commit address | Yes (CI/E2E) |
| `BdkAdminWalletMnemonic` | BDK wallet sync, build commit PSBT/tx, sign with dev mnemonic | Regtest + `COMMIT_FUNDING=admin_wallet` |

Reveal steps remain after commit confirmation; they continue to use `operator_keypair` and `BitcoinRpcClient::send_raw_transaction` as today.

Suggested module layout:

- `infrastructure/admin_wallet/` — BDK wallet load, sync, balance, commit tx build
- `application/commit_funding.rs` — `CommitFunding` trait + implementations
- `application/proposals.rs` — inject funding backend into `broadcast_commit_then_reveal`

### Admin ID vs Admin Wallet

| Key | Path | Script | Signs |
|---|---|---|---|
| Admin ID | `m/84'/0'/73'/0/0` | P2WPKH (BIP-84) | SPS-65 / backend auth only |
| Admin Wallet | `m/86'/0'/73'/0/0` (fund), `…/1/*` (change) | P2TR (BIP-86) | Bitcoin txs (commit in US-H7) |

Do not use Admin ID keys for commit funding.

### Environment and guards

| Variable | Purpose |
|---|---|
| `BITCOIN_RPC_URL` | Chain RPC endpoint (local regtest in dev; remote in later phases) |
| `BITCOIN_RPC_USER` / `BITCOIN_RPC_PASS` | RPC auth |
| `BITCOIN_NETWORK` | Must be `regtest` for US-H7 admin_wallet path |
| `COMMIT_FUNDING` | `bitcoind` (default) or `admin_wallet` |
| `ADMIN_WALLET_REGTEST_MNEMONIC` | BIP-39 mnemonic for Admin Wallet descriptors (dev/regtest only) |
| `ALLOW_DEV_MNEMONIC_SIGNING` | Existing pattern — must be set (or debug build) to load dev mnemonic |
| `ALLOW_DEV_MNEMONIC_SIGNING` | Guards both Admin Wallet funding and commit/reveal key derivation (Phase 3.5+); `ALLOW_DEV_OPERATOR_KEY` retired |

**Regtest-only guards:** If `COMMIT_FUNDING=admin_wallet` and network ≠ regtest, fail fast with a clear configuration error.

### Regtest playbook

Canonical recipe (avoids coinbase-maturity confusion):

1. Start local node: `./scripts/bitcoind-asm-runner.sh start`.
2. Mature coinbase for the CI wallet: `bitcoin-cli -rpcwallet=<CI> generatetoaddress 101 <CI-wallet-addr>`.
3. Derive Admin Wallet external address from `ADMIN_WALLET_REGTEST_MNEMONIC` at `m/86'/0'/73'/0/0`.
4. Fund it: `bitcoin-cli -rpcwallet=<CI> sendtoaddress <admin-wallet-addr> 1.0`.
5. Confirm: `bitcoin-cli -rpcwallet=<CI> generatetoaddress 1 <CI-wallet-addr>`.
6. Set desktop `.env`:
   - `COMMIT_FUNDING=admin_wallet`
   - `ADMIN_WALLET_REGTEST_MNEMONIC=…`
   - `ALLOW_DEV_MNEMONIC_SIGNING=1`
   - `BITCOIN_RPC_*`, `BITCOIN_NETWORK=regtest`
7. Run orchestrator + `npm run tauri dev`; broadcast an `approved` proposal.

## API Contract (orchestrator)

Unchanged from [proposal-broadcast-commit-reveal.md](./proposal-broadcast-commit-reveal.md). US-H7 does not add endpoints.

## Test Plan

| Layer | What to verify |
|---|---|
| Unit | Descriptor derivation for `m/86'/0'/73'/0/0` and change `…/1/0`; address matches expected test vectors for regtest mnemonic |
| Unit | `CommitFunding` selection: default `bitcoind`; `admin_wallet` only when env + regtest |
| CI / E2E | Default `COMMIT_FUNDING` unset or `bitcoind` — existing commit/reveal tests pass without BDK mnemonic |
| CI / E2E | Existing commit/reveal smoke suite remains unchanged when `COMMIT_FUNDING` is unset (regression guard for the legacy path) |
| Manual regtest | `COMMIT_FUNDING=admin_wallet` — commit tx inputs trace to Admin Wallet; reveal succeeds; orchestrator shows txids |
| Negative | Insufficient balance, RPC down, `admin_wallet` on non-regtest → high-signal errors |
| Negative | Retry after a partial commit-funding failure (signed but broadcast failed) does not produce a double-spend: orchestrator `claim` prevents duplicate execution; second attempt either rebroadcasts the same tx or surfaces a clear "already claimed" error |

## Manual Fallback

Unchanged: signers may export commit/reveal hex and broadcast externally per PRD §2.

## Links

- Story: [US-H7](../3-stories/story-map.md)
- Program phases: [admin-wallet-implementation-plan.md](./admin-wallet-implementation-plan.md)
- Protocol broadcast: [proposal-broadcast-commit-reveal.md](./proposal-broadcast-commit-reveal.md)
- PRD: `docs/0-prd/03-prd-update.md` §3.2, §5.3.2.2–5.3.2.3
