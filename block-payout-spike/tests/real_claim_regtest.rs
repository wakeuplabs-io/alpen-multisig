//! AdminBurn against a *real* bridge Claim, rebuilt from published data only.
//!
//! "Bridge side" (stands in for the operators): builds the production `GameGraph`,
//! N/N-presigns it, and broadcasts the real `ClaimTx`. The only things it hands to
//! the "app side" are the JSON of `RpcGraphData.setup` (what `stratabridge_graphData`
//! returns) and the confirmed Claim transaction.
//!
//! "App side": rebuilds the connector from that JSON, cross-checks it against the
//! pinned admin set and the on-chain Claim output, then signs K-of-M and broadcasts.
//!
//! ```text
//! cargo test --test real_claim_regtest -- --ignored --nocapture
//! ```

use std::num::NonZero;

use bitcoin::{
    absolute,
    hashes::{sha256, Hash},
    relative,
    transaction::Version,
    Amount, Network, OutPoint, Transaction, TxOut, Txid, XOnlyPublicKey,
};
use bitcoin_bosd::Descriptor;
use block_payout_spike::{
    build::{build_admin_burn, signing_info, FEE as BURN_FEE},
    collect_threshold,
    from_bridge::{
        check_admin_set, connector_from_setup, parse_setup, verify_against_claim, SourceError,
        TrustedAdminSet,
    },
    SoftwareAdminSigner,
};
use secp256k1::{rand::random, rand::rngs::OsRng, Keypair, SECP256K1};
use strata_bridge_connectors::{prelude::*, test_utils::BitcoinNode, Connector};
use strata_bridge_primitives::scripts::prelude::{create_tx, create_tx_ins};
use strata_bridge_tx_graph::game_graph::{
    AdminMultisig, DepositParams, GameData, GameGraph, KeyData, ProtocolParams, SetupParams,
};
use strata_bridge_tx_graph::transactions::prelude::ClaimTx;
use strata_l1_txfmt::MagicBytes;

const N_WATCHTOWERS: usize = 4;
const N_ADMINS: usize = 4;
const ADMIN_THRESHOLD: usize = 2;
const CONTEST_TIMELOCK: relative::Height = relative::Height::from_height(10);
const DEPOSIT_AMOUNT: Amount = Amount::from_sat(100_000_000);
const STAKE_AMOUNT: Amount = Amount::from_sat(100_000_000);
const FUNDING_FEE: Amount = Amount::from_sat(1_000);

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
    let dir = std::path::Path::new(&exe)
        .parent()
        .expect("bitcoind lives in a bin directory");
    let path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{}:{path}", dir.display()));
}

fn keypair() -> Keypair {
    Keypair::new(SECP256K1, &mut OsRng)
}

fn xonly(k: &Keypair) -> XOnlyPublicKey {
    k.x_only_public_key().0
}

fn protocol() -> ProtocolParams {
    ProtocolParams {
        network: Network::Regtest,
        magic_bytes: MagicBytes::from(*b"ALPN"),
        contest_timelock: CONTEST_TIMELOCK,
        proof_timelock: relative::Height::from_height(5),
        ack_timelock: relative::Height::from_height(10),
        nack_timelock: relative::Height::from_height(5),
        contested_payout_timelock: relative::Height::from_height(15),
        counterproof_n_data: NonZero::new(128).unwrap(),
        deposit_amount: DEPOSIT_AMOUNT,
        stake_amount: STAKE_AMOUNT,
    }
}

struct Bridge {
    node: BitcoinNode,
    admins: Vec<Keypair>,
    /// What `stratabridge_graphData(..).setup` would return, as JSON.
    published_setup_json: String,
    game_data: GameData,
    claim_txid: Txid,
}

/// Bridge side: build the real graph, broadcast the real Claim, mine it.
fn bridge_with_confirmed_claim() -> Bridge {
    ensure_bitcoind_on_path();
    let mut node = BitcoinNode::new();
    let n_of_n = keypair();
    let operator = keypair();
    let watchtowers: Vec<_> = (0..N_WATCHTOWERS).map(|_| keypair()).collect();
    let fault: Vec<_> = (0..N_WATCHTOWERS).map(|_| keypair()).collect();
    let admins: Vec<_> = (0..N_ADMINS).map(|_| keypair()).collect();
    let unstaking_preimage: [u8; 32] = random();

    let protocol = protocol();
    let wallet = Descriptor::try_from(node.wallet_address().clone()).unwrap();
    let keys = KeyData {
        n_of_n_pubkey: xonly(&n_of_n),
        operator_pubkey: xonly(&operator),
        operator_adaptor_pubkeys: vec![xonly(&operator); N_WATCHTOWERS],
        watchtower_pubkeys: watchtowers.iter().map(xonly).collect(),
        admin: AdminMultisig {
            pubkeys: admins.iter().map(xonly).collect(),
            threshold: ADMIN_THRESHOLD,
        },
        unstaking_image: sha256::Hash::hash(&unstaking_preimage),
        wt_fault_pubkeys: fault.iter().map(xonly).collect(),
        operator_descriptor: wallet.clone(),
        slash_watchtower_descriptors: vec![wallet; N_WATCHTOWERS],
    };

    // Funding tx: claim funds (wallet), deposit (N/N), stake (N/N).
    let claim_contest = ClaimContestConnector::new(
        protocol.network,
        keys.n_of_n_pubkey,
        keys.watchtower_pubkeys.clone(),
        protocol.contest_timelock,
        strata_bridge_tx_graph::fee::claim_contest_surcharge(
            N_WATCHTOWERS as u32,
            protocol.counterproof_n_data,
        ),
    );
    let claim_payout = ClaimPayoutConnector::new(
        protocol.network,
        keys.n_of_n_pubkey,
        keys.admin.pubkeys.clone(),
        keys.admin.threshold,
        keys.unstaking_image,
    );
    let claim_funds = ClaimTx::claim_funds_required(&claim_contest, &claim_payout);
    let deposit_c = NOfNConnector::new(protocol.network, keys.n_of_n_pubkey, DEPOSIT_AMOUNT);
    let stake_c = NOfNConnector::new(protocol.network, keys.n_of_n_pubkey, STAKE_AMOUNT);
    let funding_tx = create_tx(
        create_tx_ins([node.next_coinbase_outpoint()]),
        vec![
            TxOut {
                value: claim_funds,
                script_pubkey: node.wallet_address().script_pubkey(),
            },
            deposit_c.tx_out(),
            stake_c.tx_out(),
            TxOut {
                value: node.coinbase_amount()
                    - claim_funds
                    - DEPOSIT_AMOUNT
                    - STAKE_AMOUNT
                    - FUNDING_FEE,
                script_pubkey: node.wallet_address().script_pubkey(),
            },
        ],
    );
    let funding_txid = node.sign_and_broadcast(&funding_tx);
    node.mine_blocks(1);

    let setup = SetupParams {
        operator_index: 0,
        stake_outpoint: OutPoint {
            txid: funding_txid,
            vout: 2,
        },
        keys,
    };
    let game_data = GameData {
        protocol,
        setup: setup.clone(),
        deposit: DepositParams {
            game_index: NonZero::new(1).unwrap(),
            claim_funds: OutPoint {
                txid: funding_txid,
                vout: 0,
            },
            deposit_outpoint: OutPoint {
                txid: funding_txid,
                vout: 1,
            },
            adaptor_pubkeys: setup.keys.operator_adaptor_pubkeys.clone(),
            fault_pubkeys: setup.keys.wt_fault_pubkeys.clone(),
        },
    };

    // Real graph, N/N-presigned; broadcast the real Claim with its CPFP child.
    let (game, _connectors) = GameGraph::new(game_data.clone());
    let claim_txid = game.claim.as_ref().compute_txid();
    let claim = node.sign(game.claim.as_ref());
    let child = node.create_keyed_cpfp_child(&game.claim, FUNDING_FEE * 2, &operator);
    node.submit_package(&[claim, child]);
    node.mine_blocks(1);

    Bridge {
        node,
        admins,
        published_setup_json: serde_json::to_string(&setup).unwrap(),
        game_data,
        claim_txid,
    }
}

fn fetch_claim(bridge: &Bridge) -> Transaction {
    bridge
        .node
        .client()
        .get_raw_transaction(bridge.claim_txid)
        .expect("claim is on chain")
        .transaction()
        .expect("decode claim")
}

fn trusted(bridge: &Bridge) -> TrustedAdminSet {
    TrustedAdminSet {
        pubkeys: bridge.admins.iter().map(xonly).collect(),
        threshold: ADMIN_THRESHOLD,
    }
}

#[test]
#[ignore = "spawns bitcoind"]
fn admin_burn_from_published_graph_data_spends_real_claim() {
    let mut bridge = bridge_with_confirmed_claim();

    // ---- app side: only JSON + pinned admins + network + chain ----
    let setup = parse_setup(&bridge.published_setup_json).expect("graphData.setup parses");
    check_admin_set(&setup, &trusted(&bridge)).expect("published admins match pinned");
    let connector = connector_from_setup(Network::Regtest, &setup);
    let claim = fetch_claim(&bridge);
    verify_against_claim(&connector, &claim).expect("rebuilt connector == on-chain output");

    // claim_txid recomputed from graph data + protocol params equals the chain's.
    let (recomputed, _) = GameGraph::new(bridge.game_data.clone());
    assert_eq!(recomputed.claim.as_ref().compute_txid(), bridge.claim_txid);
    assert_eq!(claim.compute_txid(), bridge.claim_txid);

    let burn = build_admin_burn(
        bridge.claim_txid,
        connector,
        bridge.node.next_coinbase_txin(),
        bridge.node.coinbase_tx_out(),
        TxOut {
            value: bridge.node.coinbase_amount() - BURN_FEE,
            script_pubkey: bridge.node.wallet_address().script_pubkey(),
        },
    );
    let info = signing_info(&burn);
    // Admins 0 and 2 sign (skipping index 1) to exercise the index-keyed witness.
    let sigs = [0usize, 2].map(|i| SoftwareAdminSigner::new(i, bridge.admins[i]).sign(info));
    let tx = burn.finalize_partial(collect_threshold(sigs, ADMIN_THRESHOLD));
    assert_eq!(tx.version, Version(3));
    assert_eq!(tx.lock_time, absolute::LockTime::ZERO);
    assert_eq!(
        tx.input[0].previous_output,
        OutPoint {
            txid: bridge.claim_txid,
            vout: ClaimTx::PAYOUT_VOUT
        }
    );

    let burn_txid = bridge.node.sign_and_broadcast(&tx);
    bridge.node.mine_blocks(1);
    let confirmed = bridge
        .node
        .client()
        .get_raw_transaction(burn_txid)
        .expect("burn confirmed")
        .transaction()
        .unwrap();
    assert_eq!(confirmed.compute_txid(), burn_txid);
}

#[test]
#[ignore = "spawns bitcoind"]
fn reordered_admin_keys_are_caught_before_signing() {
    let bridge = bridge_with_confirmed_claim();
    let mut setup = parse_setup(&bridge.published_setup_json).unwrap();
    setup.keys.admin.pubkeys.swap(0, 1);

    assert_eq!(
        check_admin_set(&setup, &trusted(&bridge)),
        Err(SourceError::AdminSetMismatch)
    );
    // Even if the pinned-set check were skipped, the chain check still stops it.
    let connector = connector_from_setup(Network::Regtest, &setup);
    assert_eq!(
        verify_against_claim(&connector, &fetch_claim(&bridge)),
        Err(SourceError::ConnectorMismatch)
    );
}

#[test]
#[ignore = "spawns bitcoind"]
fn wrong_unstaking_image_is_caught_by_chain_check() {
    let bridge = bridge_with_confirmed_claim();
    let mut setup = parse_setup(&bridge.published_setup_json).unwrap();
    setup.keys.unstaking_image = sha256::Hash::hash(b"not the real image");

    // The admin set alone cannot see this; only the on-chain comparison does.
    assert_eq!(check_admin_set(&setup, &trusted(&bridge)), Ok(()));
    let connector = connector_from_setup(Network::Regtest, &setup);
    assert_eq!(
        verify_against_claim(&connector, &fetch_claim(&bridge)),
        Err(SourceError::ConnectorMismatch)
    );
}

#[test]
#[ignore = "spawns bitcoind"]
fn wrong_protocol_params_change_the_recomputed_claim_txid() {
    let bridge = bridge_with_confirmed_claim();
    let mut data = bridge.game_data.clone();
    data.protocol.contest_timelock = relative::Height::from_height(11);
    let (other, _) = GameGraph::new(data);
    assert_ne!(other.claim.as_ref().compute_txid(), bridge.claim_txid);
}
