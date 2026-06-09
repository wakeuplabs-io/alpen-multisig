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

/// Regression (change-invisibility bug): spending from the Admin Wallet routes the change to the
/// Internal keychain. After sync, `list_utxos` must report the Internal UTXO and
/// `list_addresses(Internal)` must include the change address at that derivation index — the
/// exact pair the panel joins to render "Addresses with balance". Before the fix the panel
/// dropped Internal UTXOs, showing "No addresses with balance" while the header balance was
/// non-zero.
#[tokio::test]
#[ignore = "requires running bitcoind + electrs (scripts/local-stack.sh) — set BITCOIN_RPC_* and ELECTRUM_URL"]
async fn admin_wallet_change_utxo_lands_on_internal_keychain_after_spend() {
    use bdk_bitcoind_rpc::bitcoincore_rpc::RpcApi;
    use bdk_wallet::bitcoin::{Amount, FeeRate, Network};
    use bdk_wallet::KeychainKind;
    use desktop_app::application::wallet_service::KeychainDto;
    use desktop_app::infrastructure::admin_wallet::{get_external_address, load_admin_wallet};

    let wallet =
        load_admin_wallet(REGTEST_MNEMONIC, Network::Regtest).expect("load_admin_wallet ok");
    let address_str = get_external_address(&wallet).to_string();
    fund_admin_wallet_address(&address_str);

    // Node-wallet address used both as spend recipient and mining target. Round-trips through
    // a string so no types cross between the bitcoincore-rpc and bdk bitcoin crates.
    let rpc = rpc_client();
    let node_addr = rpc
        .get_new_address(None, None)
        .expect("node address")
        .assume_checked();
    let recipient_str = node_addr.to_string();

    let svc = WalletService::new(wallet, test_node_config());
    let funded = sync_until(&svc, |svc| {
        Box::pin(async move {
            svc.get_balance()
                .await
                .is_ok_and(|b| b.confirmed_sats > 50_000)
        })
    })
    .await;
    assert!(funded, "wallet must hold confirmed funds before spending");

    // Spend to a node-side address; BDK sends the change to the Internal keychain.
    let recipient = recipient_str
        .parse::<bdk_wallet::bitcoin::Address<bdk_wallet::bitcoin::address::NetworkUnchecked>>()
        .expect("parse recipient")
        .require_network(Network::Regtest)
        .expect("regtest recipient");
    let tx = {
        let mut wallet = svc.wallet.lock().await;
        let mut builder = wallet.build_tx();
        builder.add_recipient(recipient.script_pubkey(), Amount::from_sat(10_000));
        builder.fee_rate(FeeRate::from_sat_per_vb(2).expect("fee rate"));
        let mut psbt = builder.finish().expect("build spend psbt");
        let finalized = wallet
            .sign(&mut psbt, bdk_wallet::SignOptions::default())
            .expect("sign spend");
        assert!(finalized, "spend PSBT must finalize with the mnemonic keys");
        psbt.extract_tx().expect("extract tx")
    };

    let raw_hex = bdk_wallet::bitcoin::consensus::encode::serialize_hex(&tx);
    rpc.send_raw_transaction(raw_hex.as_str())
        .expect("broadcast spend");
    rpc.generate_to_address(1, &node_addr)
        .expect("mine spend confirmation");

    let change_seen = sync_until(&svc, |svc| {
        Box::pin(async move {
            svc.list_utxos().await.is_ok_and(|utxos| {
                utxos
                    .iter()
                    .any(|u| matches!(u.keychain, KeychainDto::Internal) && u.confirmations >= 1)
            })
        })
    })
    .await;
    assert!(
        change_seen,
        "a confirmed Internal (change) UTXO must appear after spending and syncing"
    );

    // The change derivation index must fall inside the internal address window the panel
    // lists — that join is what makes the change row visible in "Addresses with balance".
    let utxos = svc.list_utxos().await.expect("list_utxos ok");
    let change = utxos
        .iter()
        .find(|u| matches!(u.keychain, KeychainDto::Internal))
        .expect("internal utxo present");
    let internal_addrs = svc
        .list_addresses(KeychainKind::Internal, 0, 20)
        .await
        .expect("list internal addresses ok");
    assert!(
        internal_addrs
            .iter()
            .any(|a| a.index == change.derivation_index),
        "change derivation index {} must be within the internal address window",
        change.derivation_index
    );
}
