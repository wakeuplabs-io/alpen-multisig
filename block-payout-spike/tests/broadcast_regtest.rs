//! Broadcast an AdminBurnTx on a local bitcoind (regtest).
//!
//! Ignored by default because it downloads/spawns Bitcoin Core. Run with:
//!
//! ```text
//! cargo test --test broadcast_regtest -- --ignored --nocapture
//! ```
//!
//! Prefers `bitcoind` on PATH (same as strata-bridge connector tests). If it is
//! missing, prepends the Bitcoin Core 30.2 binary fetched by `corepc-node`'s
//! `download` feature.

use bitcoin::{absolute, transaction::Version, OutPoint, Transaction, TxOut};
use block_payout_spike::{
    build::{build_admin_burn, signing_info, FEE},
    collect_threshold, SpikeFixture, ADMIN_THRESHOLD,
};
use strata_bridge_connectors::{test_utils::BitcoinNode, Connector};

fn ensure_bitcoind_on_path() {
    if std::process::Command::new("bitcoind")
        .arg("-version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
    {
        return;
    }

    let exe = corepc_node::downloaded_exe_path()
        .expect("enable corepc-node feature `download` to fetch bitcoind");
    assert!(
        std::path::Path::new(&exe).exists(),
        "bitcoind was not downloaded to {exe}"
    );
    let dir = std::path::Path::new(&exe)
        .parent()
        .expect("bitcoind lives in a bin directory");
    let path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{}:{path}", dir.display()));
}

/// Fund a fake Claim whose payout connector sits at vout 1 (`ClaimTx::PAYOUT_VOUT`).
fn fund_claim_payout_connector(
    node: &mut BitcoinNode,
    connector: &impl Connector,
) -> bitcoin::Txid {
    let dummy_to_wallet = TxOut {
        value: bitcoin::Amount::from_sat(330),
        script_pubkey: node.wallet_address().script_pubkey(),
    };
    let change = TxOut {
        value: node.coinbase_amount() - connector.value() - dummy_to_wallet.value - FEE,
        script_pubkey: node.wallet_address().script_pubkey(),
    };
    let funding_tx = Transaction {
        version: Version(2),
        lock_time: absolute::LockTime::ZERO,
        input: vec![node.next_coinbase_txin()],
        output: vec![dummy_to_wallet, connector.tx_out(), change],
    };
    node.sign_and_broadcast(&funding_tx)
}

#[test]
#[ignore = "spawns bitcoind (PATH or corepc-node download of Core 30.2)"]
fn software_k_of_m_broadcasts_admin_burn() {
    ensure_bitcoind_on_path();
    let fixture = SpikeFixture::new();
    let mut node = BitcoinNode::new();

    let claim_txid = fund_claim_payout_connector(&mut node, &fixture.connector);
    node.mine_blocks(1);

    let burn = build_admin_burn(
        claim_txid,
        fixture.connector.clone(),
        node.next_coinbase_txin(),
        node.coinbase_tx_out(),
        TxOut {
            value: node.coinbase_amount() - FEE,
            script_pubkey: node.wallet_address().script_pubkey(),
        },
    );

    let info = signing_info(&burn);
    let sig0 = fixture.signer(0).sign(info);
    let sig1 = fixture.signer(1).sign(info);
    let admin_burn = burn.finalize_partial(collect_threshold([sig0, sig1], ADMIN_THRESHOLD));

    assert_eq!(admin_burn.version, Version(3));
    assert_eq!(
        admin_burn.input[0].previous_output,
        OutPoint {
            txid: claim_txid,
            vout: 1
        }
    );

    let burn_txid = node.sign_and_broadcast(&admin_burn);
    node.mine_blocks(1);

    let confirmed = node
        .client()
        .get_raw_transaction(burn_txid)
        .expect("burn tx is in the chain")
        .transaction()
        .expect("decode burn tx");
    assert_eq!(confirmed.compute_txid(), burn_txid);
}
