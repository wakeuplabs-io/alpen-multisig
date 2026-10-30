//! Off-chain proof that a 2-of-3 Payout Administrator set can software-sign AdminBurn.
//!
//! Each admin signs independently. No hardware wallet, no Bitcoin node.

use bitcoin::transaction::Version;
use block_payout_spike::{
    build::{
        as_tx, build_admin_burn, dummy_change, dummy_claim_txid, dummy_fee_input, signing_info,
    },
    collect_threshold, SpikeFixture, ADMIN_THRESHOLD,
};

fn unsigned_burn(
    fixture: &SpikeFixture,
) -> strata_bridge_tx_graph::transactions::prelude::AdminBurnTx {
    let (fee_input, fee_prevout) = dummy_fee_input();
    let change = dummy_change(&fee_prevout);
    build_admin_burn(
        dummy_claim_txid(),
        fixture.connector.clone(),
        fee_input,
        fee_prevout,
        change,
    )
}

#[test]
fn two_of_three_software_signers_finalize_admin_burn() {
    let fixture = SpikeFixture::new();
    let burn = unsigned_burn(&fixture);
    assert_eq!(as_tx(&burn).version, Version(3));
    assert_eq!(as_tx(&burn).input.len(), 2, "connector + fee input");
    assert_eq!(as_tx(&burn).output.len(), 1, "admin change");

    let info = signing_info(&burn);
    let sig0 = fixture.signer(0).sign(info);
    let sig1 = fixture.signer(1).sign(info);
    assert_ne!(sig0.pubkey_index, sig1.pubkey_index);
    assert_ne!(sig0.signature, sig1.signature);

    let signatures = collect_threshold([sig0, sig1], ADMIN_THRESHOLD);
    let tx = burn.finalize_partial(signatures);

    assert_eq!(tx.version, Version(3));
    let witness = &tx.input[0].witness;
    // descending pubkey index: admin 2 empty, admin 1 sig, admin 0 sig, leaf script, control block
    assert_eq!(witness.len(), 5);
    assert!(
        witness.nth(0).unwrap().is_empty(),
        "admin 2 did not sign, empty push"
    );
    assert_eq!(witness.nth(1).unwrap().len(), 64, "admin 1 schnorr sig");
    assert_eq!(witness.nth(2).unwrap().len(), 64, "admin 0 schnorr sig");
    assert!(witness.nth(3).unwrap().len() > 64, "AdminBurn leaf script");
    assert_eq!(
        witness.nth(4).unwrap().len(),
        65,
        "taproot control block (leaf version + internal key + sibling hash)"
    );
}

#[test]
fn two_of_three_non_consecutive_signers_leave_middle_empty() {
    let fixture = SpikeFixture::new();
    let burn = unsigned_burn(&fixture);
    let info = signing_info(&burn);
    // Signers 0 and 2, skipping 1 — still meets threshold.
    let sig0 = fixture.signer(0).sign(info);
    let sig2 = fixture.signer(2).sign(info);
    let tx = burn.finalize_partial(collect_threshold([sig0, sig2], ADMIN_THRESHOLD));

    let witness = &tx.input[0].witness;
    assert_eq!(witness.nth(0).unwrap().len(), 64, "admin 2 signed");
    assert!(witness.nth(1).unwrap().is_empty(), "admin 1 did not sign");
    assert_eq!(witness.nth(2).unwrap().len(), 64, "admin 0 signed");
}

#[test]
#[should_panic(
    expected = "admin burn requires at least threshold signatures for distinct known admin pubkeys"
)]
fn one_of_three_does_not_finalize() {
    let fixture = SpikeFixture::new();
    let burn = unsigned_burn(&fixture);
    let info = signing_info(&burn);
    let sig0 = fixture.signer(0).sign(info);
    let _ = burn.finalize_partial(vec![sig0]);
}
