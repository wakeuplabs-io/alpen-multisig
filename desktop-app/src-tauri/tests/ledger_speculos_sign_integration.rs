//! End-to-end Ledger commit PSBT signing against Speculos + regtest bitcoind.
//!
//! Run when Speculos and bitcoind are up:
//!   LEDGER_SPECULOS_URL=http://localhost:5001 \
//!   BITCOIN_RPC_URL=http://127.0.0.1:18443 BITCOIN_RPC_USER=user BITCOIN_RPC_PASS=password \
//!   BITCOIN_NETWORK=regtest \
//!   cargo test -p desktop-app --test ledger_speculos_sign_integration -- --nocapture

use bdk_bitcoind_rpc::bitcoincore_rpc::RpcApi;
use desktop_app::application::wallet_session::WalletSession;
use desktop_app::infrastructure::admin_wallet::get_external_address;
use desktop_app::infrastructure::hw_wallet::hw_psbt_signer::HwDeviceType;
use desktop_app::infrastructure::hw_wallet::ledger;

#[tokio::test]
async fn ledger_signs_admin_commit_psbt_without_register_wallet() {
    if std::env::var("LEDGER_SPECULOS_URL").is_err() {
        eprintln!("skip: LEDGER_SPECULOS_URL not set");
        return;
    }
    std::env::set_var(
        "BITCOIN_RPC_URL",
        std::env::var("BITCOIN_RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:18443".into()),
    );
    std::env::set_var(
        "BITCOIN_RPC_USER",
        std::env::var("BITCOIN_RPC_USER").unwrap_or_else(|_| "user".into()),
    );
    std::env::set_var(
        "BITCOIN_RPC_PASS",
        std::env::var("BITCOIN_RPC_PASS").unwrap_or_else(|_| "password".into()),
    );
    std::env::set_var("BITCOIN_NETWORK", "regtest");

    let account_xpub = tokio::task::spawn_blocking(|| ledger::get_account_xpub("m/86'/1'/73'"))
        .await
        .expect("join")
        .expect("ledger xpub");
    let master_fingerprint = tokio::task::spawn_blocking(ledger::get_master_fingerprint)
        .await
        .expect("join")
        .expect("ledger fingerprint");

    let session = WalletSession::empty();
    session
        .init_from_xpub_with_hw(
            &account_xpub,
            master_fingerprint,
            HwDeviceType::Ledger,
            Some("regtest"),
        )
        .await
        .expect("hw session");
    let svc = session.current().expect("session wallet");

    let fund_addr = {
        let wallet = svc.wallet.lock().await;
        get_external_address(&wallet).to_string()
    };

    let url = std::env::var("BITCOIN_RPC_URL").unwrap();
    let user = std::env::var("BITCOIN_RPC_USER").unwrap();
    let pass = std::env::var("BITCOIN_RPC_PASS").unwrap();
    let rpc = bdk_bitcoind_rpc::bitcoincore_rpc::Client::new(
        &url,
        bdk_bitcoind_rpc::bitcoincore_rpc::Auth::UserPass(user, pass),
    )
    .expect("rpc client");
    if rpc.get_blockchain_info().is_err() {
        eprintln!("skip: bitcoind not reachable at {url}");
        return;
    }

    let recv: bdk_bitcoind_rpc::bitcoincore_rpc::bitcoin::Address<_> = fund_addr
        .parse::<bdk_bitcoind_rpc::bitcoincore_rpc::bitcoin::Address<
            bdk_bitcoind_rpc::bitcoincore_rpc::bitcoin::address::NetworkUnchecked,
        >>()
        .expect("parse fund address")
        .require_network(bdk_bitcoind_rpc::bitcoincore_rpc::bitcoin::Network::Regtest)
        .expect("regtest network");
    eprintln!("funding Ledger admin wallet at {fund_addr}");
    let amount = bdk_bitcoind_rpc::bitcoincore_rpc::bitcoin::Amount::from_sat(200_000);
    rpc.send_to_address(&recv, amount, None, None, None, None, None, None)
        .expect("fund");
    rpc.generate_to_address(1, &recv).expect("mine");

    let _ = svc.sync().await.expect("sync");

    let commit_dest = fund_addr.clone();
    let tx = svc
        .build_signed_commit(&commit_dest, 50_000, 2)
        .await
        .expect("build_signed_commit with Ledger PSBT sign");

    assert!(!tx.input.is_empty(), "signed commit tx must have inputs");
    eprintln!("ledger signed commit txid: {}", tx.compute_txid());
}
