//! Build an unsigned `AdminBurnTx` from a claim outpoint, connector, and fee/change.
//!
//! Fee inputs and change are malleable and must be fixed before anyone signs:
//! `signing_info_partial` commits to every input and output.

use bitcoin::{
    hashes::Hash, Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid, Witness,
};
use strata_bridge_connectors::{prelude::ClaimPayoutConnector, SigningInfo};
use strata_bridge_tx_graph::transactions::prelude::{AdminBurnData, AdminBurnTx};

/// Mining fee used by the upstream `admin_burn_payout` scenario.
pub const FEE: Amount = Amount::from_sat(1_000);

/// Dummy P2TR script pubkey for unit tests that never hit the chain.
fn dummy_p2tr() -> ScriptBuf {
    // OP_1 + 32-byte x-only key of zeros. Consensus-valid as a scriptPubKey shape; unspendable.
    ScriptBuf::from_bytes([&[0x51, 0x20][..], &[0u8; 32]].concat())
}

/// Placeholder fee input/prevout for tests that only inspect the AdminBurn witness.
pub fn dummy_fee_input() -> (TxIn, TxOut) {
    let input = TxIn {
        previous_output: OutPoint {
            txid: Txid::all_zeros(),
            vout: 0,
        },
        script_sig: ScriptBuf::new(),
        sequence: Sequence::MAX,
        witness: Witness::new(),
    };
    let prevout = TxOut {
        value: Amount::from_sat(50_000),
        script_pubkey: dummy_p2tr(),
    };
    (input, prevout)
}

/// Construct the AdminBurn transaction: connector input (vout 1 of `claim_txid`) plus fee/change.
pub fn build_admin_burn(
    claim_txid: Txid,
    connector: ClaimPayoutConnector,
    fee_input: TxIn,
    fee_prevout: TxOut,
    change: TxOut,
) -> AdminBurnTx {
    let mut tx = AdminBurnTx::new(AdminBurnData { claim_txid }, connector);
    tx.push_input(fee_input, fee_prevout);
    tx.push_output(change);
    tx
}

/// Sighash for the AdminBurn (first) input. Call only after fee inputs/outputs are final.
pub fn signing_info(tx: &AdminBurnTx) -> SigningInfo {
    tx.signing_info_partial()
}

/// Dummy claim txid for off-chain witness tests.
pub fn dummy_claim_txid() -> Txid {
    Txid::from_byte_array([0xAB; 32])
}

/// Change output that simply returns `fee_prevout.value - FEE` to a dummy script.
pub fn dummy_change(fee_prevout: &TxOut) -> TxOut {
    TxOut {
        value: fee_prevout.value - FEE,
        script_pubkey: dummy_p2tr(),
    }
}

/// Unsigned tx reference for assertions that do not consume `AdminBurnTx`.
pub fn as_tx(tx: &AdminBurnTx) -> &Transaction {
    tx.as_ref()
}
