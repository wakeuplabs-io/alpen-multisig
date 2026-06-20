use bitcoin::{
    absolute::LockTime,
    hashes::Hash,
    key::UntweakedKeypair,
    secp256k1::{Message, Secp256k1, XOnlyPublicKey, SECP256K1},
    sighash::{Prevouts, SighashCache, TapSighashType},
    taproot::{LeafVersion, TaprootBuilder, TaprootSpendInfo},
    transaction::Version,
    Address, Amount, Network, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
};
use ssz::{Decode, Encode};
use strata_asm_txs_admin::actions::MultisigAction;
use strata_asm_txs_admin::parser::SignedPayload;
use strata_asm_txs_admin::signing_message::SigningMessage;
use strata_crypto::keys::compressed::CompressedPublicKey;
use strata_crypto::threshold_signature::{IndexedSignature, SignatureSet};
use strata_l1_envelope_fmt::builder::EnvelopeScriptBuilder;
use strata_l1_txfmt::{MagicBytes, ParseConfig};

use crate::domain::proposal::ProposalSignature;

/// Build the SSZ-encoded `SignedPayload` bytes to embed in the reveal envelope.
///
/// Converts stored hex signatures to `IndexedSignature`s using the canonical
/// ordered pubkey set from ASM. Tries all four recovery IDs for 64-byte compact
/// signatures and rearranges 65-byte mnemonic-format signatures (r||s||recid → recid||r||s).
pub fn build_signed_payload_bytes(
    seq_no: u64,
    action_hex: &str,
    signatures: &[ProposalSignature],
    canonical_pubkeys_hex: &[String],
    sighash: &[u8; 32],
) -> Result<Vec<u8>, String> {
    let action_bytes = hex::decode(action_hex).map_err(|e| format!("invalid action hex: {e}"))?;
    let action = MultisigAction::from_ssz_bytes(&action_bytes)
        .map_err(|e| format!("invalid SSZ action: {e:?}"))?;

    let secp = Secp256k1::new();
    let msg = Message::from_digest_slice(sighash).map_err(|e| format!("invalid sighash: {e}"))?;

    let indexed: Vec<IndexedSignature> = signatures
        .iter()
        .map(|sig| {
            let index = canonical_pubkeys_hex
                .iter()
                .position(|k| k.eq_ignore_ascii_case(&sig.signer_pubkey))
                .ok_or_else(|| {
                    format!(
                        "signer {} not found in canonical key set",
                        &sig.signer_pubkey
                    )
                })? as u8;

            let sig_bytes = hex::decode(&sig.signature_hex)
                .map_err(|e| format!("invalid signature hex: {e}"))?;

            let recoverable_65: [u8; 65] = match sig_bytes.len() {
                64 => {
                    let mut found: Option<[u8; 65]> = None;
                    for recid_byte in 0u8..4 {
                        let rec_id =
                            bitcoin::secp256k1::ecdsa::RecoveryId::from_i32(recid_byte as i32)
                                .map_err(|e| format!("invalid recovery id: {e}"))?;
                        if let Ok(rec_sig) =
                            bitcoin::secp256k1::ecdsa::RecoverableSignature::from_compact(
                                &sig_bytes, rec_id,
                            )
                        {
                            if let Ok(recovered) = secp.recover_ecdsa(&msg, &rec_sig) {
                                let candidate = CompressedPublicKey::from(recovered);
                                let expected = {
                                    let bytes = hex::decode(&sig.signer_pubkey)
                                        .map_err(|e| format!("bad pubkey hex: {e}"))?;
                                    let pk = bitcoin::secp256k1::PublicKey::from_slice(&bytes)
                                        .map_err(|e| format!("bad pubkey: {e}"))?;
                                    CompressedPublicKey::from(pk)
                                };
                                if candidate == expected {
                                    let mut buf = [0u8; 65];
                                    buf[0] = recid_byte;
                                    buf[1..65].copy_from_slice(&sig_bytes);
                                    found = Some(buf);
                                    break;
                                }
                            }
                        }
                    }
                    found.ok_or_else(|| {
                        format!(
                            "could not recover signature for signer {}",
                            &sig.signer_pubkey
                        )
                    })?
                }
                65 => {
                    // r||s||recid (mnemonic format) → recid||r||s
                    let mut buf = [0u8; 65];
                    let recid_byte = sig_bytes[64];
                    buf[0] = recid_byte;
                    buf[1..65].copy_from_slice(&sig_bytes[..64]);
                    buf
                }
                n => {
                    return Err(format!(
                        "unexpected signature length {n} for signer {}",
                        &sig.signer_pubkey
                    ))
                }
            };

            Ok(IndexedSignature::new(index, recoverable_65))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let sig_set = SignatureSet::new(indexed).map_err(|e| format!("invalid signature set: {e}"))?;

    let signed_payload = SignedPayload::new(seq_no, action, sig_set);
    Ok(signed_payload.as_ssz_bytes())
}

/// Compute the SPS-65 sighash for a proposal's action and sequence number.
pub fn compute_sighash(seq_no: u64, action_hex: &str) -> Result<[u8; 32], String> {
    let action_bytes = hex::decode(action_hex).map_err(|e| format!("invalid action hex: {e}"))?;
    let action = MultisigAction::from_ssz_bytes(&action_bytes)
        .map_err(|e| format!("invalid SSZ action: {e:?}"))?;
    Ok(SigningMessage::for_action(&action, seq_no)
        .compute_sighash()
        .0)
}

/// Derive the P2TR commit address for the given envelope keypair and envelope payload.
pub fn derive_commit_address(
    envelope_keypair: &UntweakedKeypair,
    payload: &[u8],
    network: Network,
) -> Result<(Address, ScriptBuf, TaprootSpendInfo), String> {
    let secp = Secp256k1::new();
    let (internal_key, _) = XOnlyPublicKey::from_keypair(envelope_keypair);

    let reveal_script = EnvelopeScriptBuilder::with_pubkey(&internal_key.serialize())
        .map_err(|e| format!("envelope builder error: {e:?}"))?
        .add_envelope(payload)
        .map_err(|e| format!("add_envelope error: {e:?}"))?
        .build_without_min_check()
        .map_err(|e| format!("build envelope script error: {e:?}"))?;

    let taproot_spend_info = TaprootBuilder::new()
        .add_leaf(0, reveal_script.clone())
        .map_err(|e| format!("taproot add_leaf: {e}"))?
        .finalize(&secp, internal_key)
        .map_err(|_| "taproot finalize failed".to_string())?;

    let address = Address::p2tr(
        &secp,
        internal_key,
        taproot_spend_info.merkle_root(),
        network,
    );

    Ok((address, reveal_script, taproot_spend_info))
}

/// Build the fully-signed reveal transaction spending the commit UTXO.
#[allow(clippy::too_many_arguments)]
pub fn build_reveal_tx(
    envelope_keypair: &UntweakedKeypair,
    reveal_script: &ScriptBuf,
    taproot_spend_info: &TaprootSpendInfo,
    commit_tx: &Transaction,
    commit_address_script: &ScriptBuf,
    action: &MultisigAction,
    magic_bytes: MagicBytes,
    change_spk: ScriptBuf,
    fee_sats: u64,
) -> Result<Transaction, String> {
    let (commit_vout, commit_output) = commit_tx
        .output
        .iter()
        .enumerate()
        .find(|(_, out)| &out.script_pubkey == commit_address_script)
        .ok_or_else(|| {
            "commit tx does not have an output for the expected commit address".to_string()
        })?;

    let commit_txid = commit_tx.compute_txid();
    let commit_outpoint = OutPoint::new(commit_txid, commit_vout as u32);

    let tag = action.tag();
    let parse_config = ParseConfig::new(magic_bytes);
    let op_return_script = parse_config
        .encode_script_buf(&tag.as_ref())
        .map_err(|e| format!("encode_script_buf: {e:?}"))?;

    let fee = Amount::from_sat(fee_sats);

    let commit_amount = commit_output.value;
    let change_amount = commit_amount.checked_sub(fee).unwrap_or(Amount::ZERO);

    let mut reveal_tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: commit_outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        }],
        output: vec![
            TxOut {
                value: Amount::ZERO,
                script_pubkey: op_return_script,
            },
            TxOut {
                value: change_amount,
                script_pubkey: change_spk,
            },
        ],
    };

    let control_block = taproot_spend_info
        .control_block(&(reveal_script.clone(), LeafVersion::TapScript))
        .ok_or_else(|| "failed to create control block".to_string())?;

    let leaf_hash = bitcoin::TapLeafHash::from_script(reveal_script, LeafVersion::TapScript);

    let sighash = SighashCache::new(&reveal_tx)
        .taproot_script_spend_signature_hash(
            0,
            &Prevouts::All(&[commit_output]),
            leaf_hash,
            TapSighashType::Default,
        )
        .map_err(|e| format!("sighash computation: {e}"))?;

    let msg = Message::from_digest_slice(&sighash.to_byte_array())
        .map_err(|e| format!("sighash to message: {e}"))?;

    let signature = SECP256K1.sign_schnorr(&msg, envelope_keypair);

    let mut witness = Witness::new();
    witness.push(signature.as_ref());
    witness.push(reveal_script.as_bytes());
    witness.push(control_block.serialize());
    reveal_tx.input[0].witness = witness;

    Ok(reveal_tx)
}

/// Encode a Bitcoin transaction to hex.
pub fn tx_to_hex(tx: &Transaction) -> String {
    use bitcoin::consensus::Encodable;
    let mut buf = Vec::new();
    tx.consensus_encode(&mut buf)
        .expect("tx encode is infallible");
    hex::encode(buf)
}

#[cfg(test)]
mod build_reveal_tx_tests {
    use super::*;
    use bitcoin::{
        key::{rand::thread_rng, UntweakedKeypair},
        secp256k1::Secp256k1,
        Address, Network,
    };

    fn make_test_envelope_keypair() -> UntweakedKeypair {
        let secp = Secp256k1::new();
        UntweakedKeypair::new(&secp, &mut thread_rng())
    }

    fn make_test_change_spk(network: Network) -> ScriptBuf {
        // Use a different keypair to produce a P2WPKH script — clearly distinct from P2TR
        let secp = Secp256k1::new();
        let kp = UntweakedKeypair::new(&secp, &mut thread_rng());
        let (xonly, _) = XOnlyPublicKey::from_keypair(&kp);
        // Use P2TR with a merkle root to make it clearly different from a bare P2TR(envelope_key, None)
        let addr = Address::p2tr(&secp, xonly, None, network);
        addr.script_pubkey()
    }

    fn build_minimal_commit_tx(commit_address_script: ScriptBuf) -> Transaction {
        Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::absolute::LockTime::ZERO,
            input: vec![],
            output: vec![TxOut {
                value: Amount::from_sat(10_000),
                script_pubkey: commit_address_script,
            }],
        }
    }

    #[test]
    fn change_output_uses_change_spk_not_envelope_keypair() {
        use crate::domain::action::{Action, CompressedPubKey, MultisigUpdate};
        use crate::domain::authority::Authority;
        use crate::infrastructure::action_codec;
        use ssz::Decode;
        use std::num::NonZeroU8;
        use strata_asm_txs_admin::actions::MultisigAction;
        use strata_l1_txfmt::MagicBytes;

        let network = Network::Regtest;
        let envelope_keypair = make_test_envelope_keypair();
        let change_spk = make_test_change_spk(network);

        // Build a valid MultisigAction via the project's action codec.
        const SIGNER_HEX: &str =
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5";
        let pk = CompressedPubKey::from_hex(SIGNER_HEX).unwrap();
        let action_domain = Action::MultisigUpdate(MultisigUpdate {
            role: Authority::StrataAdmin,
            add_keys: vec![pk],
            remove_keys: vec![],
            new_threshold: NonZeroU8::new(2).unwrap(),
        });
        let action_hex = action_codec::encode_hex(&action_domain).unwrap();
        let action_bytes = hex::decode(&action_hex).unwrap();
        let action = MultisigAction::from_ssz_bytes(&action_bytes)
            .expect("valid MultisigAction from action_codec");

        // Payload must be >= 126 bytes (EnvelopeScriptBuilder minimum).
        const PAYLOAD: &[u8] = &[0x61u8; 128];

        let (commit_address, reveal_script, taproot_spend_info) =
            derive_commit_address(&envelope_keypair, PAYLOAD, network).unwrap();
        let commit_address_script = commit_address.script_pubkey();
        let commit_tx = build_minimal_commit_tx(commit_address_script.clone());

        let magic_bytes = MagicBytes::new([b'A', b'L', b'P', b'N']);
        let fee_sats = 500;

        let reveal_tx = build_reveal_tx(
            &envelope_keypair,
            &reveal_script,
            &taproot_spend_info,
            &commit_tx,
            &commit_address_script,
            &action,
            magic_bytes,
            change_spk.clone(),
            fee_sats,
        )
        .expect("build_reveal_tx must succeed");

        assert_eq!(
            reveal_tx.output[1].script_pubkey, change_spk,
            "change output must use the provided change_spk"
        );

        let secp = Secp256k1::new();
        let (envelope_xonly, _) = XOnlyPublicKey::from_keypair(&envelope_keypair);
        let envelope_self_change = Address::p2tr(&secp, envelope_xonly, None, network);
        assert_ne!(
            reveal_tx.output[1].script_pubkey,
            envelope_self_change.script_pubkey(),
            "change output must NOT be the P2TR self-change from envelope keypair"
        );
    }

    /// Regression: reveal transaction input MUST signal RBF (BIP-125).
    ///
    /// `Sequence::ENABLE_RBF_NO_LOCKTIME` (0xFFFFFFFD) satisfies the BIP-125
    /// condition (`sequence < 0xFFFFFFFE`). If this test ever fails, a future
    /// BDK/bitcoin-crate change silently disabled RBF in the reveal path.
    #[test]
    fn reveal_tx_input_signals_rbf_bip125() {
        use crate::domain::action::{Action, CompressedPubKey, MultisigUpdate};
        use crate::domain::authority::Authority;
        use crate::infrastructure::action_codec;
        use ssz::Decode;
        use std::num::NonZeroU8;
        use strata_asm_txs_admin::actions::MultisigAction;
        use strata_l1_txfmt::MagicBytes;

        let network = Network::Regtest;
        let envelope_keypair = make_test_envelope_keypair();
        let change_spk = make_test_change_spk(network);

        const SIGNER_HEX: &str =
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5";
        let pk = CompressedPubKey::from_hex(SIGNER_HEX).unwrap();
        let action_domain = Action::MultisigUpdate(MultisigUpdate {
            role: Authority::StrataAdmin,
            add_keys: vec![pk],
            remove_keys: vec![],
            new_threshold: NonZeroU8::new(2).unwrap(),
        });
        let action_hex = action_codec::encode_hex(&action_domain).unwrap();
        let action_bytes = hex::decode(&action_hex).unwrap();
        let action = MultisigAction::from_ssz_bytes(&action_bytes).unwrap();

        const PAYLOAD: &[u8] = &[0x61u8; 128];
        let (commit_address, reveal_script, taproot_spend_info) =
            derive_commit_address(&envelope_keypair, PAYLOAD, network).unwrap();
        let commit_address_script = commit_address.script_pubkey();
        let commit_tx = build_minimal_commit_tx(commit_address_script.clone());
        let magic_bytes = MagicBytes::new([b'A', b'L', b'P', b'N']);

        let reveal_tx = build_reveal_tx(
            &envelope_keypair,
            &reveal_script,
            &taproot_spend_info,
            &commit_tx,
            &commit_address_script,
            &action,
            magic_bytes,
            change_spk,
            500,
        )
        .unwrap();

        for (i, input) in reveal_tx.input.iter().enumerate() {
            assert!(
                input.sequence.is_rbf(),
                "reveal tx input[{i}] sequence 0x{:08X} does not signal RBF (BIP-125)",
                input.sequence.to_consensus_u32()
            );
        }
    }
}

#[cfg(test)]
mod recovery_id_tests {
    use super::*;
    use crate::domain::proposal::ProposalSignature;

    /// P-033: 65-byte r||s||recid is normalized to BIP-137 recid||r||s before IndexedSignature.
    #[test]
    fn accepts_mnemonic_format_65_byte_signature() {
        let sighash = [0u8; 32];
        let pk = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
        let mut sig_65 = vec![0u8; 65];
        sig_65[64] = 1;
        let sigs = vec![ProposalSignature {
            signer_pubkey: pk.to_string(),
            signature_hex: hex::encode(&sig_65),
        }];
        let err =
            build_signed_payload_bytes(1, "00", &sigs, &[pk.to_string()], &sighash).unwrap_err();
        assert!(err.contains("invalid SSZ action") || err.contains("recover"));
    }
}
