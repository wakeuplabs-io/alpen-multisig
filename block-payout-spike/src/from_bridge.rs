//! App-side data path: rebuild the claim payout connector from what the bridge
//! publishes and refuse to sign anything the chain does not confirm.
//!
//! Inputs, and where each one comes from in production:
//!
//! | Field | Source |
//! |---|---|
//! | N/N internal key, admin keys + threshold, `unstaking_image` | `stratabridge_graphData(graph_idx).setup.keys` |
//! | network | app/backend configuration |
//! | `claim_txid` | the confirmed Claim on L1 (or recomputed from the graph) |
//! | Claim output at `PAYOUT_VOUT` | any Bitcoin node (`getrawtransaction`) |
//! | trusted admin set | bridge `params.toml` `[keys.admin]`, pinned out-of-band |
//!
//! The data published by operators is *not trusted*: the rebuilt connector must
//! match the output that is really on chain before anyone signs.

use bitcoin::{Network, Transaction, XOnlyPublicKey};
use strata_bridge_connectors::{prelude::ClaimPayoutConnector, Connector};
use strata_bridge_tx_graph::{game_graph::SetupParams, transactions::prelude::ClaimTx};

/// Admin set the app pins out-of-band (from the bridge `params.toml`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedAdminSet {
    /// Admin x-only keys in `params.toml` order. Order is consensus-critical.
    pub pubkeys: Vec<XOnlyPublicKey>,
    /// Signatures required by the AdminBurn leaf.
    pub threshold: usize,
}

/// Reasons the app must refuse to build or sign an AdminBurn.
#[derive(Debug, PartialEq, Eq)]
pub enum SourceError {
    /// `graphData` carries an admin set different from the pinned one (keys, order or threshold).
    AdminSetMismatch,
    /// The Claim transaction has no output at `PAYOUT_VOUT`.
    ClaimOutputMissing,
    /// The rebuilt connector does not produce the output that is on chain.
    ConnectorMismatch,
}

/// Parse the `setup` half of an `RpcGraphData` JSON payload.
pub fn parse_setup(json: &str) -> serde_json::Result<SetupParams> {
    serde_json::from_str(json)
}

/// Rebuild the claim payout connector from published setup data.
pub fn connector_from_setup(network: Network, setup: &SetupParams) -> ClaimPayoutConnector {
    ClaimPayoutConnector::new(
        network,
        setup.keys.n_of_n_pubkey,
        setup.keys.admin.pubkeys.clone(),
        setup.keys.admin.threshold,
        setup.keys.unstaking_image,
    )
}

/// The published admin set must equal the pinned one, key for key and in order.
pub fn check_admin_set(setup: &SetupParams, trusted: &TrustedAdminSet) -> Result<(), SourceError> {
    let published = &setup.keys.admin;
    if published.pubkeys == trusted.pubkeys && published.threshold == trusted.threshold {
        Ok(())
    } else {
        Err(SourceError::AdminSetMismatch)
    }
}

/// The connector rebuilt off-chain must equal the output the Claim really created.
pub fn verify_against_claim(
    connector: &ClaimPayoutConnector,
    claim: &Transaction,
) -> Result<(), SourceError> {
    let on_chain = claim
        .output
        .get(ClaimTx::PAYOUT_VOUT as usize)
        .ok_or(SourceError::ClaimOutputMissing)?;
    if *on_chain == connector.tx_out() {
        Ok(())
    } else {
        Err(SourceError::ConnectorMismatch)
    }
}
