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
use strata_crypto::keys::compressed::CompressedPublicKey;
use strata_crypto::threshold_signature::{IndexedSignature, SignatureSet};
use strata_l1_envelope_fmt::builder::EnvelopeScriptBuilder;
use strata_l1_txfmt::{MagicBytes, ParseConfig};

use crate::domain::proposal::ProposalSignature;
use crate::error::AppError;

/// Build the SSZ-encoded `SignedPayload` bytes to embed in the reveal envelope.
///
/// Converts stored hex signatures to `IndexedSignature`s using the canonical
/// ordered pubkey set from ASM. Tries all four recovery IDs for 64-byte compact
/// signatures and rearranges 65-byte mnemonic-format signatures (r||s||recid → recid||r||s).
pub(crate) fn build_signed_payload_bytes(
    seq_no: u64,
    action_hex: &str,
    signatures: &[ProposalSignature],
    canonical_pubkeys_hex: &[String],
    sighash: &[u8; 32],
) -> Result<Vec<u8>, AppError> {
    // 1. Decode the MultisigAction from SSZ bytes.
    let action_bytes = hex::decode(action_hex)
        .map_err(|e| AppError::BadRequest(format!("invalid action hex: {e}")))?;
    let action = MultisigAction::from_ssz_bytes(&action_bytes)
        .map_err(|e| AppError::BadRequest(format!("invalid SSZ action: {e:?}")))?;

    // 2. Build IndexedSignature for each stored signature.
    let secp = Secp256k1::new();
    let msg = Message::from_digest_slice(sighash)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("invalid sighash: {e}")))?;

    let indexed: Vec<IndexedSignature> = signatures
        .iter()
        .map(|sig| {
            // Determine signer index in canonical key set.
            let index = canonical_pubkeys_hex
                .iter()
                .position(|k| k.eq_ignore_ascii_case(&sig.signer_pubkey))
                .ok_or_else(|| {
                    AppError::BadRequest(format!(
                        "signer {} not found in canonical key set",
                        &sig.signer_pubkey
                    ))
                })? as u8;

            // Decode the signature bytes.
            let sig_bytes = hex::decode(&sig.signature_hex)
                .map_err(|e| AppError::BadRequest(format!("invalid signature hex: {e}")))?;

            // Build the 65-byte recoverable signature: recid || r || s.
            let recoverable_65: [u8; 65] = match sig_bytes.len() {
                64 => {
                    // Compact (r||s), no recovery ID — try all four.
                    let mut found: Option<[u8; 65]> = None;
                    for recid_byte in 0u8..4 {
                        let rec_id =
                            bitcoin::secp256k1::ecdsa::RecoveryId::from_i32(recid_byte as i32)
                                .map_err(|e| {
                                    AppError::Internal(anyhow::anyhow!("invalid recovery id: {e}"))
                                })?;
                        if let Ok(rec_sig) =
                            bitcoin::secp256k1::ecdsa::RecoverableSignature::from_compact(
                                &sig_bytes, rec_id,
                            )
                        {
                            if let Ok(recovered) = secp.recover_ecdsa(&msg, &rec_sig) {
                                let candidate = CompressedPublicKey::from(recovered);
                                let expected = {
                                    let bytes = hex::decode(&sig.signer_pubkey).map_err(|e| {
                                        AppError::BadRequest(format!("bad pubkey hex: {e}"))
                                    })?;
                                    let pk = bitcoin::secp256k1::PublicKey::from_slice(&bytes)
                                        .map_err(|e| {
                                            AppError::BadRequest(format!("bad pubkey: {e}"))
                                        })?;
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
                        AppError::BadRequest(format!(
                            "could not recover signature for signer {}",
                            &sig.signer_pubkey
                        ))
                    })?
                }
                65 => {
                    // May be r||s||recid (mnemonic format) → convert to recid||r||s.
                    let mut buf = [0u8; 65];
                    let recid_byte = sig_bytes[64];
                    buf[0] = recid_byte;
                    buf[1..65].copy_from_slice(&sig_bytes[..64]);
                    buf
                }
                n => {
                    return Err(AppError::BadRequest(format!(
                        "unexpected signature length {n} for signer {}",
                        &sig.signer_pubkey
                    )))
                }
            };

            Ok(IndexedSignature::new(index, recoverable_65))
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    let sig_set = SignatureSet::new(indexed)
        .map_err(|e| AppError::BadRequest(format!("invalid signature set: {e}")))?;

    let signed_payload = SignedPayload::new(seq_no, action, sig_set);
    Ok(signed_payload.as_ssz_bytes())
}

/// Derive the P2TR commit address for the given operator keypair and envelope payload.
///
/// The commit address is a taproot output whose only spend path is the reveal script.
/// Funding this address creates the commit UTXO that the reveal tx will spend.
pub(crate) fn derive_commit_address(
    operator_keypair: &UntweakedKeypair,
    payload: &[u8],
    network: Network,
) -> Result<(Address, ScriptBuf, TaprootSpendInfo), AppError> {
    let secp = Secp256k1::new();
    let (internal_key, _) = XOnlyPublicKey::from_keypair(operator_keypair);

    let reveal_script = EnvelopeScriptBuilder::with_pubkey(&internal_key.serialize())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("envelope builder error: {e:?}")))?
        .add_envelope(payload)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("add_envelope error: {e:?}")))?
        .build()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("build envelope script error: {e:?}")))?;

    let taproot_spend_info = TaprootBuilder::new()
        .add_leaf(0, reveal_script.clone())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("taproot add_leaf: {e}")))?
        .finalize(&secp, internal_key)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("taproot finalize failed")))?;

    let address = Address::p2tr(
        &secp,
        internal_key,
        taproot_spend_info.merkle_root(),
        network,
    );

    Ok((address, reveal_script, taproot_spend_info))
}

/// Build the fully-signed reveal transaction spending the commit UTXO.
///
/// Requires the commit tx to already be mined (or at least broadcast) so we can
/// look up the exact vout that matches the commit address.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_reveal_tx(
    operator_keypair: &UntweakedKeypair,
    reveal_script: &ScriptBuf,
    taproot_spend_info: &TaprootSpendInfo,
    commit_tx: &Transaction,
    commit_address_script: &ScriptBuf,
    action: &MultisigAction,
    magic_bytes: MagicBytes,
    network: Network,
) -> Result<Transaction, AppError> {
    // Locate the vout in the commit tx that pays to our P2TR address.
    let (commit_vout, commit_output) = commit_tx
        .output
        .iter()
        .enumerate()
        .find(|(_, out)| &out.script_pubkey == commit_address_script)
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "commit tx does not have an output for the expected commit address"
            ))
        })?;

    let commit_txid = commit_tx.compute_txid();
    let commit_outpoint = OutPoint::new(commit_txid, commit_vout as u32);

    // Build SPS-50 OP_RETURN tag.
    let tag = action.tag();
    let parse_config = ParseConfig::new(magic_bytes);
    let op_return_script = parse_config
        .encode_script_buf(&tag.as_ref())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("encode_script_buf: {e:?}")))?;

    // Fee and change: keep it simple — send change to a P2TR from the operator key.
    let fee = Amount::from_sat(2000);
    let secp = Secp256k1::new();
    let (internal_key, _) = XOnlyPublicKey::from_keypair(operator_keypair);
    let change_address = Address::p2tr(&secp, internal_key, None, network);

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
                script_pubkey: change_address.script_pubkey(),
            },
        ],
    };

    // Sign the tapscript spend.
    let control_block = taproot_spend_info
        .control_block(&(reveal_script.clone(), LeafVersion::TapScript))
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("failed to create control block")))?;

    let leaf_hash = bitcoin::TapLeafHash::from_script(reveal_script, LeafVersion::TapScript);

    let sighash = SighashCache::new(&reveal_tx)
        .taproot_script_spend_signature_hash(
            0,
            &Prevouts::All(&[commit_output]),
            leaf_hash,
            TapSighashType::Default,
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!("sighash computation: {e}")))?;

    let msg = Message::from_digest_slice(&sighash.to_byte_array())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("sighash to message: {e}")))?;

    let signature = SECP256K1.sign_schnorr(&msg, operator_keypair);

    let mut witness = Witness::new();
    witness.push(signature.as_ref());
    witness.push(reveal_script.as_bytes());
    witness.push(control_block.serialize());
    reveal_tx.input[0].witness = witness;

    Ok(reveal_tx)
}

/// Encode a Bitcoin transaction to hex.
pub(crate) fn tx_to_hex(tx: &Transaction) -> String {
    use bitcoin::consensus::Encodable;
    let mut buf = Vec::new();
    tx.consensus_encode(&mut buf)
        .expect("tx encode is infallible");
    hex::encode(buf)
}
