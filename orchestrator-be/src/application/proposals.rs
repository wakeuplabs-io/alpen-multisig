//! Proposal business logic — CRUD operations over the repository trait.
//!
//! Functions receive a `ProposalRepository` as a parameter (dependency injection).
//! No authentication or quorum detection — those are added in future slices.

use bitcoin::key::UntweakedKeypair;
use bitcoin::Network;
use strata_l1_txfmt::MagicBytes;

use crate::application::traits::ProposalRepository;
use crate::domain::authority::Authority;
use crate::domain::proposal::{
    compute_action_id, ActionId, BroadcastBundle, BroadcastStatus, Proposal, ProposalSignature,
    ProposalStatus, SeqNo,
};
use crate::error::AppError;
use crate::infrastructure::asm_role_membership::ordered_keys_for_authority;
use crate::infrastructure::bitcoin_rpc::BitcoinRpcClient;
use crate::infrastructure::broadcast_tx;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionContext<'a> {
    pub authority: Authority,
    pub signer_pubkey: &'a str,
}

/// Create a new proposal with first signature. Rejects duplicate ActionId.
///
/// Mirrors PRD: `create_update_action(action, seq, sig)`.
pub(crate) async fn create_update_action(
    repo: &dyn ProposalRepository,
    session: SessionContext<'_>,
    seq_no: SeqNo,
    action_hex: &str,
    sig: &ProposalSignature,
    required_signatures: u16,
) -> Result<Proposal, AppError> {
    if !sig
        .signer_pubkey
        .eq_ignore_ascii_case(session.signer_pubkey)
    {
        return Err(AppError::Unauthorized);
    }

    let action_id = compute_action_id(seq_no, action_hex)?;

    let proposal = Proposal {
        action_id,
        seq_no,
        authority: session.authority,
        status: ProposalStatus::Pending,
        required_signatures,
        action_hex: action_hex.to_string(),
        signatures: vec![sig.clone()],
        broadcast_status: BroadcastStatus::default(),
        commit_txid: None,
        reveal_txid: None,
        broadcast_error: None,
    };

    repo.save_proposal(proposal.clone()).await?;

    Ok(proposal)
}

/// Add a signature to an existing proposal. Rejects duplicate signer.
///
/// Mirrors PRD: `approve_action(id, sig)`.
pub(crate) async fn approve_action(
    repo: &dyn ProposalRepository,
    session: SessionContext<'_>,
    action_id: &ActionId,
    sig: &ProposalSignature,
) -> Result<Proposal, AppError> {
    if !sig
        .signer_pubkey
        .eq_ignore_ascii_case(session.signer_pubkey)
    {
        return Err(AppError::Unauthorized);
    }
    let existing = repo.find_by_action_id(action_id).await?;
    let proposal = existing.ok_or(AppError::NotFound)?;
    if proposal.authority != session.authority {
        return Err(AppError::Unauthorized);
    }

    let already_signed = proposal
        .signatures
        .iter()
        .any(|s| s.signer_pubkey == sig.signer_pubkey);

    if already_signed {
        return Err(AppError::Conflict("signer already signed".to_string()));
    }

    let updated = repo
        .add_signature(action_id, &sig.signer_pubkey, &sig.signature_hex)
        .await?;

    let proposal = updated.ok_or(AppError::NotFound)?;

    if proposal.status == ProposalStatus::Pending
        && proposal.signatures.len() >= proposal.required_signatures as usize
    {
        let approved = repo
            .update_broadcast_status(
                action_id,
                proposal.broadcast_status,
                Some(ProposalStatus::Approved),
                None,
                None,
                None,
            )
            .await?;
        return approved.ok_or(AppError::NotFound);
    }

    Ok(proposal)
}

/// Get proposal by ActionId.
pub(crate) async fn get_update_action(
    repo: &dyn ProposalRepository,
    action_id: &ActionId,
) -> Result<Proposal, AppError> {
    repo.find_by_action_id(action_id)
        .await?
        .ok_or(AppError::NotFound)
}

/// List proposals, optionally filtered by status.
pub(crate) async fn list_proposals(
    repo: &dyn ProposalRepository,
    status: Option<ProposalStatus>,
) -> Result<Vec<Proposal>, AppError> {
    repo.list_by_status(status).await
}

// ---------------------------------------------------------------------------
// Broadcast services
// ---------------------------------------------------------------------------

/// Estimated vbytes for an SPS-51 reveal transaction (conservative upper bound).
const REVEAL_TX_VBYTES: u64 = 350;

/// Dust + protocol minimum sats locked in the commit output.
const COMMIT_DUST_SATS: u64 = 1500;

/// Ensure a proposal is in `Approved` state before broadcasting.
///
/// If the proposal is `Pending` but has already reached quorum (e.g. proposals created
/// before the auto-approve logic was introduced), transitions it to `Approved` in place.
/// Returns an error for any other non-`Approved` state.
async fn require_approved(
    repo: &dyn ProposalRepository,
    proposal: Proposal,
    action_id: &ActionId,
) -> Result<Proposal, AppError> {
    if proposal.status == ProposalStatus::Approved {
        return Ok(proposal);
    }

    if proposal.status == ProposalStatus::Pending
        && proposal.signatures.len() >= proposal.required_signatures as usize
    {
        let recovered = repo
            .update_broadcast_status(
                action_id,
                proposal.broadcast_status,
                Some(ProposalStatus::Approved),
                None,
                None,
                None,
            )
            .await?;
        return recovered.ok_or(AppError::NotFound);
    }

    Err(AppError::Conflict(format!(
        "proposal must be in 'approved' state to broadcast (current: {:?})",
        proposal.status
    )))
}

/// Assemble commit/reveal artifacts for an approved proposal without submitting to the network.
///
/// Returns a [`BroadcastBundle`] containing the P2TR commit address to fund and the
/// pre-built reveal tx hex (not yet valid until the commit UTXO exists).
pub(crate) async fn prepare_broadcast_bundle(
    repo: &dyn ProposalRepository,
    btc_client: &dyn BitcoinRpcClient,
    asm_rpc_url: &str,
    operator_keypair: &UntweakedKeypair,
    network: Network,
    action_id: &ActionId,
) -> Result<BroadcastBundle, AppError> {
    let raw = repo
        .find_by_action_id(action_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let proposal = require_approved(repo, raw, action_id).await?;

    let canonical_keys = ordered_keys_for_authority(asm_rpc_url, proposal.authority).await?;

    let sighash = compute_sighash_for_proposal(&proposal)?;
    let payload = broadcast_tx::build_signed_payload_bytes(
        proposal.seq_no,
        &proposal.action_hex,
        &proposal.signatures,
        &canonical_keys,
        &sighash,
    )?;

    let (commit_address, _reveal_script, _taproot_info) =
        broadcast_tx::derive_commit_address(operator_keypair, &payload, network)?;

    let fee_rate = btc_client.estimate_fee_rate_sats_per_vb(6).await?;
    let estimated_fee_sats = fee_rate * REVEAL_TX_VBYTES;
    let commit_amount_sats = COMMIT_DUST_SATS + estimated_fee_sats;

    Ok(BroadcastBundle {
        commit_address: commit_address.to_string(),
        commit_amount_sats,
        reveal_tx_hex: String::new(), // filled in at broadcast time once commit UTXO is known
        estimated_fee_sats,
    })
}

/// Execute the full commit → confirm → reveal sequence for an approved proposal.
///
/// Returns `(commit_txid, reveal_txid)` on success and transitions the proposal to `Enacted`.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn broadcast_commit_then_reveal(
    repo: &dyn ProposalRepository,
    btc_client: &dyn BitcoinRpcClient,
    asm_rpc_url: &str,
    operator_keypair: &UntweakedKeypair,
    action_id: &ActionId,
    magic_bytes: MagicBytes,
    network: Network,
    confirm_poll_interval_ms: u64,
    confirm_timeout_ms: u64,
) -> Result<(String, String), AppError> {
    let raw = repo
        .find_by_action_id(action_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let _proposal = require_approved(repo, raw, action_id).await?;

    // Atomically claim broadcast rights: transitions idle → commit_broadcasted.
    // Returns Conflict if another caller already claimed (race-safe).
    let proposal = repo.claim_broadcast(action_id).await?;

    // --- Derive broadcast artifacts ---
    let canonical_keys = ordered_keys_for_authority(asm_rpc_url, proposal.authority).await?;
    let sighash = compute_sighash_for_proposal(&proposal)?;
    let payload = broadcast_tx::build_signed_payload_bytes(
        proposal.seq_no,
        &proposal.action_hex,
        &proposal.signatures,
        &canonical_keys,
        &sighash,
    )?;
    let (commit_address, reveal_script, taproot_spend_info) =
        broadcast_tx::derive_commit_address(operator_keypair, &payload, network)?;

    let fee_rate = btc_client.estimate_fee_rate_sats_per_vb(6).await?;
    let commit_amount_sats = COMMIT_DUST_SATS + fee_rate * REVEAL_TX_VBYTES;

    let result = do_broadcast(
        repo,
        btc_client,
        operator_keypair,
        action_id,
        &commit_address,
        &reveal_script,
        &taproot_spend_info,
        &proposal,
        &payload,
        commit_amount_sats,
        fee_rate,
        magic_bytes,
        network,
        confirm_poll_interval_ms,
        confirm_timeout_ms,
    )
    .await;

    if let Err(ref e) = result {
        let _ = repo
            .update_broadcast_status(
                action_id,
                BroadcastStatus::Failed,
                None,
                None,
                None,
                Some(&e.to_string()),
            )
            .await;
    }

    result
}

#[allow(clippy::too_many_arguments)]
async fn do_broadcast(
    repo: &dyn ProposalRepository,
    btc_client: &dyn BitcoinRpcClient,
    operator_keypair: &UntweakedKeypair,
    action_id: &ActionId,
    commit_address: &bitcoin::Address,
    reveal_script: &bitcoin::ScriptBuf,
    taproot_spend_info: &bitcoin::taproot::TaprootSpendInfo,
    proposal: &Proposal,
    _payload: &[u8],
    commit_amount_sats: u64,
    fee_rate_sats_per_vb: u64,
    magic_bytes: MagicBytes,
    network: Network,
    confirm_poll_interval_ms: u64,
    confirm_timeout_ms: u64,
) -> Result<(String, String), AppError> {
    // Step 1: Broadcast commit.
    let commit_txid = btc_client
        .send_to_address(
            &commit_address.to_string(),
            commit_amount_sats,
            fee_rate_sats_per_vb,
        )
        .await?;

    repo.update_broadcast_status(
        action_id,
        BroadcastStatus::CommitBroadcasted,
        None,
        Some(&commit_txid),
        None,
        None,
    )
    .await?;

    // Step 2: Wait for commit confirmation.
    wait_for_confirmation(
        btc_client,
        &commit_txid,
        confirm_poll_interval_ms,
        confirm_timeout_ms,
    )
    .await?;

    repo.update_broadcast_status(
        action_id,
        BroadcastStatus::CommitConfirmed,
        None,
        Some(&commit_txid),
        None,
        None,
    )
    .await?;

    // Step 3: Build and broadcast reveal.
    let commit_tx = btc_client.get_raw_transaction(&commit_txid).await?;
    let commit_address_script = commit_address.script_pubkey();

    use ssz::Decode;
    let action_bytes = hex::decode(&proposal.action_hex)
        .map_err(|e| AppError::BadRequest(format!("invalid action hex: {e}")))?;
    let action = strata_asm_txs_admin::actions::MultisigAction::from_ssz_bytes(&action_bytes)
        .map_err(|e| AppError::BadRequest(format!("invalid SSZ action: {e:?}")))?;

    let reveal_fee_sats = fee_rate_sats_per_vb * REVEAL_TX_VBYTES;
    let reveal_tx = broadcast_tx::build_reveal_tx(
        operator_keypair,
        reveal_script,
        taproot_spend_info,
        &commit_tx,
        &commit_address_script,
        &action,
        magic_bytes,
        network,
        reveal_fee_sats,
    )?;

    let reveal_tx_hex = broadcast_tx::tx_to_hex(&reveal_tx);
    let reveal_txid = btc_client.send_raw_transaction(&reveal_tx_hex).await?;

    repo.update_broadcast_status(
        action_id,
        BroadcastStatus::RevealBroadcasted,
        None,
        Some(&commit_txid),
        Some(&reveal_txid),
        None,
    )
    .await?;

    // Step 4: Wait for reveal confirmation.
    wait_for_confirmation(
        btc_client,
        &reveal_txid,
        confirm_poll_interval_ms,
        confirm_timeout_ms,
    )
    .await?;

    // Transition to enacted.
    repo.update_broadcast_status(
        action_id,
        BroadcastStatus::RevealConfirmed,
        Some(ProposalStatus::Enacted),
        Some(&commit_txid),
        Some(&reveal_txid),
        None,
    )
    .await?;

    Ok((commit_txid, reveal_txid))
}

/// Compute the SPS-65 sighash for a proposal's action and sequence number.
fn compute_sighash_for_proposal(proposal: &Proposal) -> Result<[u8; 32], AppError> {
    use ssz::Decode;
    use strata_asm_txs_admin::actions::{MultisigAction, Sighash};
    let action_bytes = hex::decode(&proposal.action_hex)
        .map_err(|e| AppError::BadRequest(format!("invalid action hex: {e}")))?;
    let action = MultisigAction::from_ssz_bytes(&action_bytes)
        .map_err(|e| AppError::BadRequest(format!("invalid SSZ action: {e:?}")))?;
    Ok(action.compute_sighash(proposal.seq_no).0)
}

/// Poll until a transaction has at least one confirmation or the timeout is exceeded.
async fn wait_for_confirmation(
    btc_client: &dyn BitcoinRpcClient,
    txid: &str,
    poll_interval_ms: u64,
    timeout_ms: u64,
) -> Result<(), AppError> {
    let start = std::time::Instant::now();
    loop {
        let confs = btc_client.get_transaction_confirmations(txid).await?;
        if confs >= 1 {
            return Ok(());
        }
        if start.elapsed().as_millis() as u64 >= timeout_ms {
            return Err(AppError::Internal(anyhow::anyhow!(
                "timeout waiting for confirmation of txid {txid}"
            )));
        }
        tokio::time::sleep(std::time::Duration::from_millis(poll_interval_ms)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::memory_repo::InMemoryProposalRepository;

    fn new_repo() -> InMemoryProposalRepository {
        InMemoryProposalRepository::new()
    }

    const ACTION_HEX: &str = "deadbeef";

    fn sig_a() -> ProposalSignature {
        ProposalSignature {
            signer_pubkey: "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"
                .to_string(),
            signature_hex: "sig_a".to_string(),
        }
    }

    fn sig_b() -> ProposalSignature {
        ProposalSignature {
            signer_pubkey: "02c6047f9441ed7d6d3045406e95c07cd85a1a3f1f3ff2b4f6f3f5b4f0c709ee5"
                .to_string(),
            signature_hex: "sig_b".to_string(),
        }
    }

    #[tokio::test]
    async fn test_create_update_action() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let proposal = create_update_action(&repo, session, 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();

        assert_eq!(proposal.seq_no, 1);
        assert_eq!(
            proposal.authority,
            crate::domain::authority::Authority::StrataAdmin
        );
        assert_eq!(proposal.action_hex, ACTION_HEX);
        assert_eq!(proposal.status, ProposalStatus::Pending);
        assert_eq!(proposal.required_signatures, 2);
        assert_eq!(proposal.signatures.len(), 1);
        assert_eq!(proposal.signatures[0].signer_pubkey, sig.signer_pubkey);

        let expected_id = compute_action_id(1, ACTION_HEX).unwrap();
        assert_eq!(proposal.action_id, expected_id);
    }

    #[tokio::test]
    async fn test_create_duplicate_action_rejected() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();

        let result = create_update_action(&repo, session, 1, ACTION_HEX, &sig, 2).await;

        assert!(matches!(result.unwrap_err(), AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_approve_action() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();

        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        let updated = approve_action(&repo, session_b, &created.action_id, &sig_b())
            .await
            .unwrap();

        assert_eq!(updated.signatures.len(), 2);
        assert_eq!(
            updated.signatures[1].signer_pubkey,
            "02c6047f9441ed7d6d3045406e95c07cd85a1a3f1f3ff2b4f6f3f5b4f0c709ee5"
        );
    }

    #[tokio::test]
    async fn test_approve_action_transitions_to_approved_at_threshold() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        // required_signatures = 2
        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();
        assert_eq!(created.status, ProposalStatus::Pending);

        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        let updated = approve_action(&repo, session_b, &created.action_id, &sig_b())
            .await
            .unwrap();

        assert_eq!(updated.status, ProposalStatus::Approved);
    }

    #[tokio::test]
    async fn test_approve_action_stays_pending_below_threshold() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        // required_signatures = 3, only 2 collected
        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 3)
            .await
            .unwrap();

        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        let updated = approve_action(&repo, session_b, &created.action_id, &sig_b())
            .await
            .unwrap();

        assert_eq!(updated.status, ProposalStatus::Pending);
        assert_eq!(updated.signatures.len(), 2);
    }

    #[tokio::test]
    async fn test_approve_duplicate_signer_rejected() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();

        let dup_sig = ProposalSignature {
            signer_pubkey: "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"
                .to_string(),
            signature_hex: "different_sig".to_string(),
        };
        let result = approve_action(&repo, session, &created.action_id, &dup_sig).await;

        assert!(matches!(result.unwrap_err(), AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_approve_nonexistent_proposal() {
        let repo = new_repo();
        let fake_id = ActionId("nonexistent".to_string());

        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_a().signer_pubkey,
        };
        let result = approve_action(&repo, session, &fake_id, &sig_a()).await;

        assert!(matches!(result.unwrap_err(), AppError::NotFound));
    }

    #[tokio::test]
    async fn test_get_update_action() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session, 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();

        let fetched = get_update_action(&repo, &created.action_id).await.unwrap();

        assert_eq!(fetched.action_id, created.action_id);
        assert_eq!(fetched.action_hex, created.action_hex);
        assert_eq!(fetched.seq_no, created.seq_no);
    }

    #[tokio::test]
    async fn test_get_nonexistent_proposal() {
        let repo = new_repo();
        let fake_id = ActionId("nonexistent".to_string());

        let result = get_update_action(&repo, &fake_id).await;

        assert!(matches!(result.unwrap_err(), AppError::NotFound));
    }

    #[tokio::test]
    async fn test_list_proposals() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        create_update_action(&repo, session.clone(), 1, "aa", &sig, 2)
            .await
            .unwrap();
        create_update_action(&repo, session, 2, "bb", &sig, 2)
            .await
            .unwrap();

        let all = list_proposals(&repo, None).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_list_proposals_with_status_filter() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        create_update_action(&repo, session.clone(), 1, "aa", &sig, 2)
            .await
            .unwrap();
        create_update_action(&repo, session, 2, "bb", &sig, 2)
            .await
            .unwrap();

        let pending = list_proposals(&repo, Some(ProposalStatus::Pending))
            .await
            .unwrap();
        assert_eq!(pending.len(), 2);

        let approved = list_proposals(&repo, Some(ProposalStatus::Approved))
            .await
            .unwrap();
        assert_eq!(approved.len(), 0);
    }

    #[tokio::test]
    async fn test_create_rejects_signer_mismatch_against_session() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: "03aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        };

        let err = create_update_action(&repo, session, 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }
}
