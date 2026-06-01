/// Integration tests for admin wallet regtest balance/UTXOs.
///
/// Regtest tests: require a running bitcoind and are gated with `#[ignore]`.
/// Run explicitly:
///   BITCOIN_RPC_URL=http://127.0.0.1:18443 \
///   BITCOIN_RPC_USER=user BITCOIN_RPC_PASS=pass \
///   BITCOIN_NETWORK=regtest \
///   cargo test -p desktop-app --test admin_wallet_integration -- --ignored
use desktop_app::application::wallet_service::WalletService;
use std::sync::Mutex;

// Serialize env-var tests to avoid cross-test pollution.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn clear_guard_env_vars() {
    std::env::remove_var("BITCOIN_NETWORK");
}

fn set_all_guard_env_vars() {
    std::env::set_var("BITCOIN_NETWORK", "regtest");
}

// ── Regtest tests (require running bitcoind) ──────────────────────────────────

const REGTEST_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn rpc_client() -> bdk_bitcoind_rpc::bitcoincore_rpc::Client {
    use bdk_bitcoind_rpc::bitcoincore_rpc::Auth;
    let url = std::env::var("BITCOIN_RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:18443".into());
    let user = std::env::var("BITCOIN_RPC_USER").unwrap_or_else(|_| "user".into());
    let pass = std::env::var("BITCOIN_RPC_PASS").unwrap_or_else(|_| "pass".into());
    bdk_bitcoind_rpc::bitcoincore_rpc::Client::new(&url, Auth::UserPass(user, pass))
        .expect("failed to create RPC client")
}

/// Fund admin wallet + mine 1 block → get_balance returns confirmed_sats > 0.
#[tokio::test]
#[ignore = "requires running bitcoind (Phase 1 regtest harness) — set BITCOIN_RPC_URL/USER/PASS"]
async fn admin_wallet_get_balance_returns_confirmed_sats_after_funding_and_mine() {
    use bdk_bitcoind_rpc::bitcoincore_rpc::RpcApi;
    use bdk_wallet::bitcoin::Network;
    use desktop_app::infrastructure::admin_wallet::{get_external_address, load_admin_wallet};

    let wallet =
        load_admin_wallet(REGTEST_MNEMONIC, Network::Regtest).expect("load_admin_wallet ok");
    let address = get_external_address(&wallet);
    let address_str = address.to_string();

    let rpc = rpc_client();
    let amount = bdk_bitcoind_rpc::bitcoincore_rpc::bitcoin::Amount::from_sat(100_000);
    let recv_addr: bdk_bitcoind_rpc::bitcoincore_rpc::bitcoin::Address<
        bdk_bitcoind_rpc::bitcoincore_rpc::bitcoin::address::NetworkChecked,
    > = address_str
        .parse::<bdk_bitcoind_rpc::bitcoincore_rpc::bitcoin::Address<
            bdk_bitcoind_rpc::bitcoincore_rpc::bitcoin::address::NetworkUnchecked,
        >>()
        .expect("parse address")
        .require_network(bdk_bitcoind_rpc::bitcoincore_rpc::bitcoin::Network::Regtest)
        .expect("regtest network check");

    rpc.send_to_address(&recv_addr, amount, None, None, None, None, None, None)
        .expect("sendtoaddress");
    rpc.generate_to_address(1, &recv_addr)
        .expect("generatetoaddress");

    let svc = WalletService::new(wallet);
    let _ = svc.sync().await;
    let balance = svc
        .get_balance()
        .await
        .expect("get_balance should not fail");

    assert!(
        balance.confirmed_sats > 0,
        "confirmed_sats must be > 0 after funding and mining 1 block, got {}",
        balance.confirmed_sats
    );
}

/// Fund admin wallet + mine 1 block → list_utxos returns 1 UTXO with confirmations == 1.
#[tokio::test]
#[ignore = "requires running bitcoind (Phase 1 regtest harness) — set BITCOIN_RPC_URL/USER/PASS"]
async fn admin_wallet_list_utxos_returns_one_utxo_with_one_confirmation_after_funding() {
    use bdk_bitcoind_rpc::bitcoincore_rpc::RpcApi;
    use bdk_wallet::bitcoin::Network;
    use desktop_app::infrastructure::admin_wallet::{get_external_address, load_admin_wallet};

    let wallet =
        load_admin_wallet(REGTEST_MNEMONIC, Network::Regtest).expect("load_admin_wallet ok");
    let address = get_external_address(&wallet);
    let address_str = address.to_string();

    let rpc = rpc_client();
    let amount = bdk_bitcoind_rpc::bitcoincore_rpc::bitcoin::Amount::from_sat(100_000);
    let recv_addr: bdk_bitcoind_rpc::bitcoincore_rpc::bitcoin::Address<
        bdk_bitcoind_rpc::bitcoincore_rpc::bitcoin::address::NetworkChecked,
    > = address_str
        .parse::<bdk_bitcoind_rpc::bitcoincore_rpc::bitcoin::Address<
            bdk_bitcoind_rpc::bitcoincore_rpc::bitcoin::address::NetworkUnchecked,
        >>()
        .expect("parse address")
        .require_network(bdk_bitcoind_rpc::bitcoincore_rpc::bitcoin::Network::Regtest)
        .expect("regtest network check");

    rpc.send_to_address(&recv_addr, amount, None, None, None, None, None, None)
        .expect("sendtoaddress");
    rpc.generate_to_address(1, &recv_addr)
        .expect("generatetoaddress");

    let svc = WalletService::new(wallet);
    let _ = svc.sync().await;
    let utxos = svc.list_utxos().await.expect("list_utxos should not fail");

    assert_eq!(
        utxos.len(),
        1,
        "expected exactly 1 UTXO after funding, got {}",
        utxos.len()
    );
    assert_eq!(
        utxos[0].confirmations, 1,
        "expected 1 confirmation after mining 1 block, got {}",
        utxos[0].confirmations
    );
}
