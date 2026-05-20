//! Proposal management — application layer entry point for the desktop app.
//!
//! Public API mirrors the PRD's `MultisigBackend` trait semantics:
//! - `create_update_action(action_hex, seq_no, signature)` — propose + first signature
//! - `approve_action(action_id, signature)` — add approval signature
//! - `get_update_action(action_id)` — fetch proposal detail
//!
//! Authority is implicit — bound to the authenticated session, not passed per call.
//! Signing and action encoding happen before reaching this layer.

use bitcoin::{key::UntweakedKeypair, Network};
use ssz::Decode;
use strata_asm_txs_admin::actions::MultisigAction;
use strata_l1_txfmt::MagicBytes;

use crate::application::orchestrator_client::{
    ApproveActionRequest, CreateProposalRequest, OrchestratorClient, OrchestratorError,
    ReportBroadcastProgressRequest, TransitionProposalRequest,
};
use crate::domain::proposal::{Proposal, Signature};
use crate::infrastructure::asm_role_membership;
use crate::infrastructure::bitcoin_rpc::BitcoinRpcClient;
use crate::infrastructure::broadcast_tx;

/// Errors that can occur during proposal operations.
#[derive(Debug, thiserror::Error)]
pub enum ProposalError {
    #[error("Orchestrator error: {0}")]
    Orchestrator(#[from] OrchestratorError),
}

/// Errors that can occur during direct broadcast from Tauri.
#[derive(Debug, thiserror::Error)]
pub enum BroadcastError {
    #[error("failed to fetch proposal: {0}")]
    ProposalFetch(#[from] OrchestratorError),
    #[error("broadcast setup error: {0}")]
    Setup(String),
    #[error("bitcoin RPC error: {0}")]
    BitcoinRpc(String),
    #[error("confirmation timeout for txid {txid}")]
    Timeout { txid: String },
}

/// Dust + protocol minimum sats locked in the commit output.
const COMMIT_DUST_SATS: u64 = 1500;
/// Estimated vbytes for an SPS-51 reveal transaction (conservative upper bound).
const REVEAL_TX_VBYTES: u64 = 350;

/// Assemble commit/reveal artifacts for an approved proposal without submitting to the network.
///
/// Returns `(commit_address, commit_amount_sats, estimated_fee_sats)`.
pub async fn prepare_broadcast_bundle(
    client: &dyn OrchestratorClient,
    btc_rpc: &dyn BitcoinRpcClient,
    asm_rpc_url: &str,
    operator_keypair: &UntweakedKeypair,
    network: Network,
    action_id: &str,
) -> Result<(String, u64, u64), BroadcastError> {
    let proposal = client.get_proposal(action_id).await?;

    if proposal.status != "approved" {
        return Err(BroadcastError::Setup(format!(
            "proposal must be in 'approved' state to broadcast (current: {})",
            proposal.status
        )));
    }

    let canonical_keys =
        asm_role_membership::ordered_keys_for_authority(asm_rpc_url, proposal.authority)
            .await
            .map_err(BroadcastError::Setup)?;

    let sighash = broadcast_tx::compute_sighash(proposal.seq_no, &proposal.action_hex)
        .map_err(BroadcastError::Setup)?;

    let payload = broadcast_tx::build_signed_payload_bytes(
        proposal.seq_no,
        &proposal.action_hex,
        &proposal.signatures,
        &canonical_keys,
        &sighash,
    )
    .map_err(BroadcastError::Setup)?;

    let (commit_address, _, _) =
        broadcast_tx::derive_commit_address(operator_keypair, &payload, network)
            .map_err(BroadcastError::Setup)?;

    let fee_rate = btc_rpc
        .estimate_fee_rate_sats_per_vb(6)
        .await
        .map_err(BroadcastError::BitcoinRpc)?;
    let estimated_fee_sats = fee_rate * REVEAL_TX_VBYTES;
    let commit_amount_sats = COMMIT_DUST_SATS + estimated_fee_sats;

    Ok((
        commit_address.to_string(),
        commit_amount_sats,
        estimated_fee_sats,
    ))
}

async fn report_broadcast(
    client: &dyn OrchestratorClient,
    action_id: &str,
    broadcast_status: &str,
    proposal_status: Option<&str>,
    commit_txid: Option<&str>,
    reveal_txid: Option<&str>,
    broadcast_error: Option<&str>,
) -> Result<(), BroadcastError> {
    client
        .report_broadcast_progress(
            action_id,
            ReportBroadcastProgressRequest {
                broadcast_status: broadcast_status.to_string(),
                proposal_status: proposal_status.map(str::to_string),
                commit_txid: commit_txid.map(str::to_string),
                reveal_txid: reveal_txid.map(str::to_string),
                broadcast_error: broadcast_error.map(str::to_string),
            },
        )
        .await?;
    Ok(())
}

/// Execute commit → confirm → reveal on the desktop; orchestrator records coordination state only.
///
/// Returns `(commit_txid, reveal_txid)` on success.
#[allow(clippy::too_many_arguments)]
pub async fn broadcast_commit_then_reveal(
    client: &dyn OrchestratorClient,
    btc_rpc: &dyn BitcoinRpcClient,
    asm_rpc_url: &str,
    operator_keypair: &UntweakedKeypair,
    magic_bytes: MagicBytes,
    network: Network,
    action_id: &str,
    confirm_poll_interval_ms: u64,
    confirm_timeout_ms: u64,
) -> Result<(String, String), BroadcastError> {
    let proposal = client.claim_broadcast(action_id).await.map_err(|e| {
        if let OrchestratorError::Backend {
            status: 409,
            message,
        } = &e
        {
            BroadcastError::Setup(format!("broadcast already in progress: {message}"))
        } else {
            BroadcastError::ProposalFetch(e)
        }
    })?;

    if proposal.status != "approved" {
        return Err(BroadcastError::Setup(format!(
            "proposal must be in 'approved' state to broadcast (current: {})",
            proposal.status
        )));
    }

    let canonical_keys =
        asm_role_membership::ordered_keys_for_authority(asm_rpc_url, proposal.authority)
            .await
            .map_err(BroadcastError::Setup)?;

    let sighash = broadcast_tx::compute_sighash(proposal.seq_no, &proposal.action_hex)
        .map_err(BroadcastError::Setup)?;

    let payload = broadcast_tx::build_signed_payload_bytes(
        proposal.seq_no,
        &proposal.action_hex,
        &proposal.signatures,
        &canonical_keys,
        &sighash,
    )
    .map_err(BroadcastError::Setup)?;

    let (commit_address, reveal_script, taproot_spend_info) =
        broadcast_tx::derive_commit_address(operator_keypair, &payload, network)
            .map_err(BroadcastError::Setup)?;

    let fee_rate = btc_rpc
        .estimate_fee_rate_sats_per_vb(6)
        .await
        .map_err(BroadcastError::BitcoinRpc)?;
    let reveal_fee_sats = fee_rate * REVEAL_TX_VBYTES;
    let commit_amount_sats = COMMIT_DUST_SATS + reveal_fee_sats;

    let broadcast_result: Result<(String, String), BroadcastError> = async {
        // Step 1: Broadcast commit.
        let commit_txid = btc_rpc
            .send_to_address(&commit_address.to_string(), commit_amount_sats, fee_rate)
            .await
            .map_err(BroadcastError::BitcoinRpc)?;

        report_broadcast(
            client,
            action_id,
            "commit_broadcasted",
            None,
            Some(&commit_txid),
            None,
            None,
        )
        .await?;

        if network == Network::Regtest {
            btc_rpc
                .mine_blocks(1)
                .await
                .map_err(BroadcastError::BitcoinRpc)?;
        }

        wait_for_confirmation(
            btc_rpc,
            &commit_txid,
            confirm_poll_interval_ms,
            confirm_timeout_ms,
        )
        .await?;

        report_broadcast(
            client,
            action_id,
            "commit_confirmed",
            None,
            Some(&commit_txid),
            None,
            None,
        )
        .await?;

        // Step 3: Build and broadcast reveal.
        let commit_tx = btc_rpc
            .get_raw_transaction(&commit_txid)
            .await
            .map_err(BroadcastError::BitcoinRpc)?;

        let commit_address_script = commit_address.script_pubkey();

        let action_bytes = hex::decode(&proposal.action_hex)
            .map_err(|e| BroadcastError::Setup(format!("invalid action hex: {e}")))?;
        let action = MultisigAction::from_ssz_bytes(&action_bytes)
            .map_err(|e| BroadcastError::Setup(format!("invalid SSZ action: {e:?}")))?;

        let reveal_tx = broadcast_tx::build_reveal_tx(
            operator_keypair,
            &reveal_script,
            &taproot_spend_info,
            &commit_tx,
            &commit_address_script,
            &action,
            magic_bytes,
            network,
            reveal_fee_sats,
        )
        .map_err(BroadcastError::Setup)?;

        let reveal_tx_hex = broadcast_tx::tx_to_hex(&reveal_tx);
        let reveal_txid = btc_rpc
            .send_raw_transaction(&reveal_tx_hex)
            .await
            .map_err(BroadcastError::BitcoinRpc)?;

        report_broadcast(
            client,
            action_id,
            "reveal_broadcasted",
            None,
            Some(&commit_txid),
            Some(&reveal_txid),
            None,
        )
        .await?;

        if network == Network::Regtest {
            btc_rpc
                .mine_blocks(1)
                .await
                .map_err(BroadcastError::BitcoinRpc)?;
        }

        wait_for_confirmation(
            btc_rpc,
            &reveal_txid,
            confirm_poll_interval_ms,
            confirm_timeout_ms,
        )
        .await?;

        report_broadcast(
            client,
            action_id,
            "reveal_confirmed",
            None,
            Some(&commit_txid),
            Some(&reveal_txid),
            None,
        )
        .await?;

        Ok((commit_txid, reveal_txid))
    }
    .await;

    if let Err(ref e) = broadcast_result {
        let _ = report_broadcast(
            client,
            action_id,
            "failed",
            None,
            None,
            None,
            Some(&e.to_string()),
        )
        .await;
    }

    broadcast_result
}

async fn wait_for_confirmation(
    btc_rpc: &dyn BitcoinRpcClient,
    txid: &str,
    poll_interval_ms: u64,
    timeout_ms: u64,
) -> Result<(), BroadcastError> {
    let start = std::time::Instant::now();
    loop {
        let confs = btc_rpc
            .get_transaction_confirmations(txid)
            .await
            .map_err(BroadcastError::BitcoinRpc)?;
        if confs >= 1 {
            return Ok(());
        }
        if start.elapsed().as_millis() as u64 >= timeout_ms {
            return Err(BroadcastError::Timeout {
                txid: txid.to_string(),
            });
        }
        tokio::time::sleep(std::time::Duration::from_millis(poll_interval_ms)).await;
    }
}

/// Create a new action and store the creator's signature.
///
/// Mirrors PRD: `create_update_action(action, seq, sig)`.
///
/// Callers are responsible for encoding the action to SSZ hex before calling this
/// function (`infrastructure::action_codec::encode_hex`).
pub async fn create_update_action(
    client: &dyn OrchestratorClient,
    action_hex: &str,
    seq_no: u64,
    signature: &Signature,
) -> Result<Proposal, ProposalError> {
    let request = CreateProposalRequest {
        seq_no,
        action_hex: action_hex.to_string(),
        signer_pubkey: signature.signer_pubkey.clone(),
        signature_hex: signature.signature_hex.clone(),
    };

    let proposal = client.create_proposal(request).await?;
    Ok(proposal)
}

fn orchestrator_quorum_reached(proposal: &Proposal) -> bool {
    proposal.signatures.len() >= proposal.required_signatures as usize
}

/// Explicit pending → approved after quorum (P-012 / ADR-006).
pub async fn transition_to_approved(
    client: &dyn OrchestratorClient,
    action_id: &str,
) -> Result<Proposal, ProposalError> {
    let proposal = client
        .transition_to_approved(
            action_id,
            TransitionProposalRequest {
                proposal_status: "approved".to_string(),
            },
        )
        .await?;
    Ok(proposal)
}

/// Append an approval signature; when quorum is reached, persist `approved` on the orchestrator.
pub async fn approve_action(
    client: &dyn OrchestratorClient,
    action_id: &str,
    signature: &Signature,
) -> Result<Proposal, ProposalError> {
    let request = ApproveActionRequest {
        signer_pubkey: signature.signer_pubkey.clone(),
        signature_hex: signature.signature_hex.clone(),
    };

    let proposal = client.approve_action(action_id, request).await?;
    if proposal.status == "pending" && orchestrator_quorum_reached(&proposal) {
        return transition_to_approved(client, action_id).await;
    }
    Ok(proposal)
}

/// Fetch the action payload and details.
///
/// Mirrors PRD: `get_update_action(id)`.
pub async fn get_update_action(
    client: &dyn OrchestratorClient,
    action_id: &str,
) -> Result<Proposal, ProposalError> {
    let proposal = client.get_proposal(action_id).await?;
    Ok(proposal)
}

/// List proposals, optionally filtered by status.
pub async fn list_proposals(
    client: &dyn OrchestratorClient,
    status: Option<&str>,
) -> Result<Vec<Proposal>, ProposalError> {
    let proposals = client.list_proposals(status).await?;
    Ok(proposals)
}

/// Prepare commit/reveal fee estimate locally (desktop-owned Bitcoin RPC).
pub async fn prepare_broadcast_local(
    client: &dyn OrchestratorClient,
    btc_rpc: &dyn BitcoinRpcClient,
    asm_rpc_url: &str,
    operator_keypair: &UntweakedKeypair,
    network: Network,
    action_id: &str,
) -> Result<(String, u64, u64), BroadcastError> {
    prepare_broadcast_bundle(
        client,
        btc_rpc,
        asm_rpc_url,
        operator_keypair,
        network,
        action_id,
    )
    .await
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::orchestrator_client::OrchestratorError;
    use crate::application::orchestrator_client::{
        CompleteOrchestratorAuthRequest, OrchestratorAuthChallenge, OrchestratorAuthSession,
        StartOrchestratorAuthRequest,
    };
    use crate::domain::action::{Action, CompressedPubKey, MultisigUpdate};
    use crate::domain::authority::Authority;
    use crate::domain::proposal::{Proposal as OrcProposal, ProposalSignature};
    use crate::infrastructure::action_codec;
    use crate::infrastructure::signing;
    use bitcoin::secp256k1::{PublicKey, SecretKey, SECP256K1};
    use rand::rngs::OsRng;
    use std::num::NonZeroU8;
    use std::sync::Mutex;

    // ─── Test helpers ───────────────────────────────────────────────────────

    fn generate_test_keypair() -> (String, String) {
        let sk = SecretKey::new(&mut OsRng);
        let pk = PublicKey::from_secret_key(SECP256K1, &sk);
        (hex::encode(sk.secret_bytes()), hex::encode(pk.serialize()))
    }

    /// Builds a sample `Action::MultisigUpdate` via domain types only.
    fn demo_action() -> Action {
        let demo_bytes = [0x42u8; 32];
        let demo_sk = SecretKey::from_slice(&demo_bytes).expect("valid fixed key");
        let new_signer_pk = PublicKey::from_secret_key(SECP256K1, &demo_sk);
        let new_signer = CompressedPubKey::new(new_signer_pk.serialize());
        Action::MultisigUpdate(MultisigUpdate {
            role: Authority::StrataAdmin,
            add_keys: vec![new_signer],
            remove_keys: vec![],
            new_threshold: NonZeroU8::new(2).expect("non-zero"),
        })
    }

    fn demo_action_hex() -> String {
        action_codec::encode_hex(&demo_action()).expect("encode ok")
    }

    fn sign_action(secret_key_hex: &str, seq_no: u64, action_hex: &str) -> Signature {
        let sighash = signing::compute_sighash(seq_no, action_hex).expect("sighash ok");
        let sig = signing::sign_sighash(secret_key_hex, &sighash.sighash_hex).expect("sign ok");
        Signature {
            signer_pubkey: sig.public_key_hex,
            signature_hex: sig.signature_hex,
        }
    }

    struct MockOrchestratorClient {
        last_create_request: Mutex<Option<CreateProposalRequest>>,
        last_approve_request: Mutex<Option<(String, ApproveActionRequest)>>,
        transition_called: Mutex<bool>,
        approve_signature_count: Mutex<usize>,
        claim_broadcast_called: Mutex<bool>,
        report_broadcast_called: Mutex<bool>,
        last_report_request:
            Mutex<Option<crate::application::orchestrator_client::ReportBroadcastProgressRequest>>,
        should_fail: bool,
    }

    impl MockOrchestratorClient {
        fn new() -> Self {
            Self {
                last_create_request: Mutex::new(None),
                last_approve_request: Mutex::new(None),
                transition_called: Mutex::new(false),
                approve_signature_count: Mutex::new(0),
                claim_broadcast_called: Mutex::new(false),
                report_broadcast_called: Mutex::new(false),
                last_report_request: Mutex::new(None),
                should_fail: false,
            }
        }

        fn failing() -> Self {
            Self {
                last_create_request: Mutex::new(None),
                last_approve_request: Mutex::new(None),
                transition_called: Mutex::new(false),
                approve_signature_count: Mutex::new(0),
                claim_broadcast_called: Mutex::new(false),
                report_broadcast_called: Mutex::new(false),
                last_report_request: Mutex::new(None),
                should_fail: true,
            }
        }

        fn claim_broadcast_called(&self) -> bool {
            *self.claim_broadcast_called.lock().unwrap()
        }

        fn last_create_request(&self) -> Option<CreateProposalRequest> {
            self.last_create_request.lock().unwrap().take()
        }

        fn last_approve_request(&self) -> Option<(String, ApproveActionRequest)> {
            self.last_approve_request.lock().unwrap().take()
        }
    }

    #[async_trait::async_trait]
    impl OrchestratorClient for MockOrchestratorClient {
        async fn auth_challenge(
            &self,
            _request: StartOrchestratorAuthRequest,
        ) -> Result<OrchestratorAuthChallenge, OrchestratorError> {
            Err(OrchestratorError::Request("not used in tests".to_string()))
        }

        async fn auth_verify(
            &self,
            _request: CompleteOrchestratorAuthRequest,
        ) -> Result<OrchestratorAuthSession, OrchestratorError> {
            Err(OrchestratorError::Request("not used in tests".to_string()))
        }

        async fn auth_logout(&self) -> Result<(), OrchestratorError> {
            Err(OrchestratorError::Request("not used in tests".to_string()))
        }

        async fn create_proposal(
            &self,
            request: CreateProposalRequest,
        ) -> Result<OrcProposal, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            let response = OrcProposal {
                action_id: format!("action_{}", request.seq_no),
                authority: Authority::StrataAdmin,
                seq_no: request.seq_no,
                action_hex: request.action_hex.clone(),
                status: "pending".to_string(),
                required_signatures: 2,
                signatures: vec![ProposalSignature {
                    signer_pubkey: request.signer_pubkey.clone(),
                    signature_hex: request.signature_hex.clone(),
                }],
                broadcast_status: "idle".to_string(),
                commit_txid: None,
                reveal_txid: None,
                broadcast_error: None,
            };
            *self.last_create_request.lock().unwrap() = Some(request);
            Ok(response)
        }

        async fn get_proposal(&self, action_id: &str) -> Result<OrcProposal, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            Ok(OrcProposal {
                action_id: action_id.to_string(),
                authority: Authority::StrataAdmin,
                seq_no: 1,
                action_hex: demo_action_hex(),
                status: "pending".to_string(),
                required_signatures: 2,
                signatures: vec![],
                broadcast_status: "idle".to_string(),
                commit_txid: None,
                reveal_txid: None,
                broadcast_error: None,
            })
        }

        async fn approve_action(
            &self,
            action_id: &str,
            request: ApproveActionRequest,
        ) -> Result<OrcProposal, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            *self.last_approve_request.lock().unwrap() = Some((action_id.to_string(), request));
            let mut count = self.approve_signature_count.lock().unwrap();
            *count += 1;
            let signatures = (0..*count)
                .map(|i| ProposalSignature {
                    signer_pubkey: format!("signer_{i}"),
                    signature_hex: format!("sig_{i}"),
                })
                .collect();
            Ok(OrcProposal {
                action_id: action_id.to_string(),
                authority: Authority::StrataAdmin,
                seq_no: 1,
                action_hex: demo_action_hex(),
                status: "pending".to_string(),
                required_signatures: 2,
                signatures,
                broadcast_status: "idle".to_string(),
                commit_txid: None,
                reveal_txid: None,
                broadcast_error: None,
            })
        }

        async fn transition_to_approved(
            &self,
            action_id: &str,
            _request: TransitionProposalRequest,
        ) -> Result<OrcProposal, OrchestratorError> {
            *self.transition_called.lock().unwrap() = true;
            Ok(OrcProposal {
                action_id: action_id.to_string(),
                authority: Authority::StrataAdmin,
                seq_no: 1,
                action_hex: demo_action_hex(),
                status: "approved".to_string(),
                required_signatures: 2,
                signatures: vec![
                    ProposalSignature {
                        signer_pubkey: "signer_0".to_string(),
                        signature_hex: "sig_0".to_string(),
                    },
                    ProposalSignature {
                        signer_pubkey: "signer_1".to_string(),
                        signature_hex: "sig_1".to_string(),
                    },
                ],
                broadcast_status: "idle".to_string(),
                commit_txid: None,
                reveal_txid: None,
                broadcast_error: None,
            })
        }

        async fn list_proposals(
            &self,
            _status: Option<&str>,
        ) -> Result<Vec<OrcProposal>, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            Ok(vec![OrcProposal {
                action_id: "action_1".to_string(),
                authority: Authority::StrataAdmin,
                seq_no: 1,
                action_hex: demo_action_hex(),
                status: "pending".to_string(),
                required_signatures: 2,
                signatures: vec![],
                broadcast_status: "idle".to_string(),
                commit_txid: None,
                reveal_txid: None,
                broadcast_error: None,
            }])
        }

        async fn get_next_seq_no(&self) -> Result<u64, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            Ok(1)
        }

        async fn claim_broadcast(&self, action_id: &str) -> Result<OrcProposal, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            *self.claim_broadcast_called.lock().unwrap() = true;
            Ok(OrcProposal {
                action_id: action_id.to_string(),
                authority: Authority::StrataAdmin,
                seq_no: 1,
                action_hex: demo_action_hex(),
                status: "approved".to_string(),
                required_signatures: 2,
                signatures: vec![],
                broadcast_status: "commit_broadcasted".to_string(),
                commit_txid: None,
                reveal_txid: None,
                broadcast_error: None,
            })
        }

        async fn report_broadcast_progress(
            &self,
            action_id: &str,
            request: crate::application::orchestrator_client::ReportBroadcastProgressRequest,
        ) -> Result<OrcProposal, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            *self.report_broadcast_called.lock().unwrap() = true;
            *self.last_report_request.lock().unwrap() = Some(request.clone());
            Ok(OrcProposal {
                action_id: action_id.to_string(),
                authority: Authority::StrataAdmin,
                seq_no: 1,
                action_hex: demo_action_hex(),
                status: request
                    .proposal_status
                    .unwrap_or_else(|| "approved".to_string()),
                required_signatures: 2,
                signatures: vec![],
                broadcast_status: request.broadcast_status,
                commit_txid: request.commit_txid,
                reveal_txid: request.reveal_txid,
                broadcast_error: request.broadcast_error,
            })
        }
    }

    // ─── Tests ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_update_action() {
        let mock = MockOrchestratorClient::new();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let result = create_update_action(&mock, &action_hex, 1, &sig)
            .await
            .expect("should succeed");

        assert_eq!(result.action_id, "action_1");
        assert_eq!(result.status, "pending");
        assert_eq!(result.signatures.len(), 1);
        assert_eq!(result.signatures[0].signer_pubkey, sig.signer_pubkey);

        let req = mock.last_create_request().expect("request sent");
        assert_eq!(req.seq_no, 1);
        assert_eq!(req.action_hex, action_hex);
    }

    #[tokio::test]
    async fn test_approve_action() {
        let mock = MockOrchestratorClient::new();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let result = approve_action(&mock, "action_1", &sig)
            .await
            .expect("should succeed");

        assert_eq!(result.action_id, "action_1");
        assert_eq!(result.status, "pending");

        let (action_id, req) = mock.last_approve_request().expect("request sent");
        assert_eq!(action_id, "action_1");
        assert_eq!(req.signer_pubkey, sig.signer_pubkey);
        assert!(!*mock.transition_called.lock().unwrap());
    }

    #[tokio::test]
    async fn test_approve_at_quorum_calls_transition() {
        let mock = MockOrchestratorClient::new();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let _first = approve_action(&mock, "action_1", &sig)
            .await
            .expect("first approve");
        assert!(!*mock.transition_called.lock().unwrap());

        let result = approve_action(&mock, "action_1", &sig)
            .await
            .expect("quorum approve");
        assert_eq!(result.status, "approved");
        assert!(*mock.transition_called.lock().unwrap());
    }

    #[tokio::test]
    async fn test_get_update_action() {
        let mock = MockOrchestratorClient::new();

        let result = get_update_action(&mock, "action_1")
            .await
            .expect("should succeed");

        assert_eq!(result.action_id, "action_1");
        assert_eq!(result.authority, Authority::StrataAdmin);
    }

    #[tokio::test]
    async fn test_create_then_get_consistent() {
        let mock = MockOrchestratorClient::new();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let created = create_update_action(&mock, &action_hex, 1, &sig)
            .await
            .expect("should succeed");

        let detail = get_update_action(&mock, &created.action_id)
            .await
            .expect("should succeed");

        assert_eq!(created.authority, detail.authority);
        assert_eq!(created.seq_no, detail.seq_no);
    }

    #[tokio::test]
    async fn test_signature_is_verifiable() {
        let mock = MockOrchestratorClient::new();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let _result = create_update_action(&mock, &action_hex, 1, &sig)
            .await
            .expect("should succeed");

        let req = mock.last_create_request().expect("request sent");
        let sighash = signing::compute_sighash(1, &action_hex).expect("sighash ok");
        let verify = signing::verify_threshold(
            &[req.signer_pubkey],
            1,
            &[req.signature_hex],
            &sighash.sighash_hex,
        )
        .expect("verify ok");

        assert!(verify.valid);
    }

    #[tokio::test]
    async fn test_create_backend_error_propagates() {
        let mock = MockOrchestratorClient::failing();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let result = create_update_action(&mock, &action_hex, 1, &sig).await;

        assert!(matches!(
            result.unwrap_err(),
            ProposalError::Orchestrator(_)
        ));
    }

    #[tokio::test]
    async fn test_approve_backend_error_propagates() {
        let mock = MockOrchestratorClient::failing();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let result = approve_action(&mock, "action_1", &sig).await;

        assert!(matches!(
            result.unwrap_err(),
            ProposalError::Orchestrator(_)
        ));
    }

    #[tokio::test]
    async fn test_list_proposals() {
        let mock = MockOrchestratorClient::new();
        let proposals = list_proposals(&mock, Some("pending"))
            .await
            .expect("should succeed");
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].action_id, "action_1");
    }

    #[tokio::test]
    async fn test_claim_broadcast_coordination() {
        let mock = MockOrchestratorClient::new();
        let proposal = mock.claim_broadcast("action_42").await.expect("claim ok");
        assert!(mock.claim_broadcast_called());
        assert_eq!(proposal.action_id, "action_42");
        assert_eq!(proposal.status, "approved");
    }

    #[tokio::test]
    async fn reveal_confirmed_report_keeps_proposal_approved() {
        let mock = MockOrchestratorClient::new();
        super::report_broadcast(
            &mock,
            "action-1",
            "reveal_confirmed",
            None,
            Some("commit-txid"),
            Some("reveal-txid"),
            None,
        )
        .await
        .expect("report ok");

        let req = mock
            .last_report_request
            .lock()
            .unwrap()
            .clone()
            .expect("report captured");
        assert_eq!(req.broadcast_status, "reveal_confirmed");
        assert_eq!(req.proposal_status, None);
    }
}
