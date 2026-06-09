/// Integration tests for admin wallet regtest balance/UTXOs.
///
/// Regtest tests: require a running bitcoind (funding/mining harness) AND a running electrs
/// indexer (wallet sync path, R2.2) — both provided by `scripts/local-stack.sh`. Gated with
/// `#[ignore]`. Run explicitly:
///   BITCOIN_RPC_URL=http://127.0.0.1:18443 \
///   BITCOIN_RPC_USER=user BITCOIN_RPC_PASS=pass \
///   ELECTRUM_URL=tcp://127.0.0.1:60401 \
///   cargo test -p desktop-app --test admin_wallet_integration -- --ignored
use desktop_app::application::wallet_service::WalletService;
use desktop_app::infrastructure::node_config_store::{ConnectionMode, NodeConfig};
use std::sync::{Arc, RwLock};
use std::time::Duration;

fn test_node_config() -> Arc<RwLock<NodeConfig>> {
    // R2.3: wallet sync reads the Electrum URL from NodeConfig. The harness override comes in
    // through Custom mode; without ELECTRUM_URL the Custom fallback is the local electrs.
    Arc::new(RwLock::new(NodeConfig {
        mode: ConnectionMode::Custom,
        custom_electrum_url: std::env::var("ELECTRUM_URL").ok(),
        ..Default::default()
    }))
}

// ─ Regtest tests (require running bitcoind + electrs) ───────────────────────

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

/// Syncs until `done` returns true or the deadline passes. electrs indexes new blocks on a short
/// polling interval, so the first sync after mining can land before the indexer caught up.
async fn sync_until<F>(svc: &WalletService, mut done: F) -> bool
where
    F: FnMut(&WalletService) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + '_>>,
{
    for _ in 0..30 {
        let _ = svc.sync().await;
        if done(svc).await {
            return true;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    false
}

fn fund_admin_wallet_address(address_str: &str) {
    use bdk_bitcoind_rpc::bitcoincore_rpc::RpcApi;

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
}

/// Fund admin wallet + mine 1 block → get_balance returns confirmed_sats > 0 (Electrum sync).
#[tokio::test]
#[ignore = "requires running bitcoind + electrs (scripts/local-stack.sh) — set BITCOIN_RPC_* and ELECTRUM_URL"]
async fn admin_wallet_get_balance_returns_confirmed_sats_after_funding_and_mine() {
    use bdk_wallet::bitcoin::Network;
    use desktop_app::infrastructure::admin_wallet::{get_external_address, load_admin_wallet};

    let wallet =
        load_admin_wallet(REGTEST_MNEMONIC, Network::Regtest).expect("load_admin_wallet ok");
    let address_str = get_external_address(&wallet).to_string();
    fund_admin_wallet_address(&address_str);

    let svc = WalletService::new(wallet, test_node_config());
    let funded = sync_until(&svc, |svc| {
        Box::pin(async move {
            svc.get_balance()
                .await
                .is_ok_and(|balance| balance.confirmed_sats > 0)
        })
    })
    .await;

    assert!(
        funded,
        "confirmed_sats must be > 0 after funding, mining 1 block, and Electrum sync"
    );
}

/// Fund admin wallet + mine 1 block → list_utxos includes the fresh UTXO with confirmations == 1.
/// A reused regtest chain may hold older UTXOs for the same address (the Electrum full scan
/// discovers all history), so the assertion targets the freshly funded one rather than a count.
#[tokio::test]
#[ignore = "requires running bitcoind + electrs (scripts/local-stack.sh) — set BITCOIN_RPC_* and ELECTRUM_URL"]
async fn admin_wallet_list_utxos_includes_fresh_utxo_with_one_confirmation_after_funding() {
    use bdk_wallet::bitcoin::Network;
    use desktop_app::infrastructure::admin_wallet::{get_external_address, load_admin_wallet};

    let wallet =
        load_admin_wallet(REGTEST_MNEMONIC, Network::Regtest).expect("load_admin_wallet ok");
    let address_str = get_external_address(&wallet).to_string();
    fund_admin_wallet_address(&address_str);

    let svc = WalletService::new(wallet, test_node_config());
    let synced = sync_until(&svc, |svc| {
        Box::pin(async move {
            svc.list_utxos()
                .await
                .is_ok_and(|utxos| utxos.iter().any(|u| u.confirmations == 1))
        })
    })
    .await;
    assert!(
        synced,
        "expected the freshly funded UTXO (1 confirmation) after mining 1 block"
    );
}
