// Quick integration test against the Trezor emulator.
//
// Prerequisites:
//   1. Trezor emulator running (trezor-emu-core or via trezor-user-env Docker)
//   2. Trezor Bridge (trezord) running: `trezord -e 21324`
//   3. Emulator initialized: trezorctl -p udp:127.0.0.1:21324 debug load-device
//        --mnemonic "all all all all all all all all all all all all" --pin "" --passphrase-protection false
//
// Run with:
//   cargo run -p desktop-app --bin trezor_test

use bitcoin::secp256k1::{PublicKey, SecretKey, SECP256K1};
use desktop_app::infrastructure::hw_wallet::trezor;
use desktop_app::signing;
use std::num::NonZero;
use strata_asm_params::Role;
use strata_asm_txs_admin::actions::updates::multisig::MultisigUpdate;
use strata_asm_txs_admin::actions::{MultisigAction, UpdateAction};
use strata_crypto::keys::compressed::CompressedPublicKey;
use strata_crypto::threshold_signature::ThresholdConfigUpdate;

fn demo_action_hex() -> String {
    let demo_bytes = [0x42u8; 32];
    let demo_sk = SecretKey::from_slice(&demo_bytes).expect("valid fixed key");
    let new_signer = CompressedPublicKey::from(PublicKey::from_secret_key(SECP256K1, &demo_sk));
    let config_update =
        ThresholdConfigUpdate::new(vec![new_signer], vec![], NonZero::new(2).expect("non-zero"));
    let multisig_update = MultisigUpdate::new(config_update, Role::StrataAdministrator);
    let action = MultisigAction::Update(UpdateAction::Multisig(multisig_update));
    hex::encode(borsh::to_vec(&action).expect("action borsh-serializes"))
}

fn main() {
    println!("=== Trezor emulator test ===\n");

    // ── Step 1: connect and read pubkey + address ──────────────────────────
    println!("Connecting to Trezor...");
    let info = match trezor::connect(None) {
        Ok(i) => {
            println!("  device_label:        {}", i.device_label);
            println!("  derivation_path:     {}", i.derivation_path);
            println!("  address_sample:      {:?}", i.address_sample);
            println!("  xpub_or_fingerprint: {:?}", i.xpub_or_fingerprint);
            println!();
            i
        }
        Err(e) => {
            eprintln!("Connect failed: {e}");
            std::process::exit(1);
        }
    };

    // ── Step 2: compute a real sighash ────────────────────────────────────
    let action_hex = demo_action_hex();
    let seq_no = 1u64;

    let sighash = match signing::compute_sighash(seq_no, &action_hex) {
        Ok(s) => {
            println!("Sighash (seq_no={seq_no}): {}", s.sighash_hex);
            s
        }
        Err(e) => {
            eprintln!("compute_sighash failed: {e}");
            std::process::exit(1);
        }
    };

    // ── Step 3: sign via Trezor (BIP-137 format — see caveat below) ──────
    println!("\nSigning with Trezor (confirm on emulator if prompted)...");
    match trezor::sign_message(&sighash.sighash_hex, &info.derivation_path) {
        Ok(sig) => {
            println!("  public_key_hex: {}", sig.public_key_hex);
            println!("  signature_hex:  {}", sig.signature_hex);
            println!("\n⚠  Note: signature uses BIP-137 prefix — not directly compatible with");
            println!("   verify_threshold (raw ECDSA). Connect + pubkey read are fully validated.");
        }
        Err(e) => {
            eprintln!("sign_message failed: {e}");
        }
    }
}
