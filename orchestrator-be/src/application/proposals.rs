//! Proposal business logic — CRUD operations over the repository trait.
//!
//! Functions receive a `ProposalRepository` as a parameter (dependency injection).

use crate::application::traits::ProposalRepository;
use crate::domain::authority::Authority;
use crate::domain::proposal::{
    compute_action_id, ActionId, BroadcastAttempt, BroadcastStatus, Proposal, ProposalSignature,
    ProposalStatus, SeqNo,
};
use crate::error::AppError;
use crate::infrastructure::asm_role_membership::{
    last_seqno_for_authority, lock_period_for_action, threshold_for_authority,
    update_id_in_queue_for_action,
};
use crate::infrastructure::bitcoin_rpc::BitcoinRpcClient;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionContext<'a> {
    pub authority: Authority,
    pub signer_pubkey: &'a str,
}

/// Canonical lowercase hex for signer pubkeys at every ingress (P-063).
pub(crate) fn normalize_signer_pubkey_hex(hex: &str) -> String {
    hex.trim().to_lowercase()
}

/// Treat a blank or whitespace-only title as absent, so the UI has one case to handle
/// instead of two that look identical on screen.
fn normalize_title(title: Option<String>) -> Option<String> {
    title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
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
    title: Option<String>,
) -> Result<Proposal, AppError> {
    if !sig
        .signer_pubkey
        .eq_ignore_ascii_case(session.signer_pubkey)
    {
        return Err(AppError::Unauthorized);
    }

    let action_id = compute_action_id(seq_no, action_hex)?;

    // PRD 02 §3.4: a duplicate `(action, seq_no)` MUST be rejected and MUST NOT mutate the
    // existing proposal. The rejection names the proposal that already holds this `ActionId`, so
    // the second creator can reach the one they meant to sign — and signs it through
    // `approve_action`, because a creation call is not an approval.
    if repo.find_by_action_id(&action_id).await?.is_some() {
        return Err(AppError::Conflict(format!(
            "a proposal for this action and seq_no already exists: `{}` — approve it instead of \
             creating it again",
            action_id.0
        )));
    }

    let mut first_sig = sig.clone();
    first_sig.signer_pubkey = normalize_signer_pubkey_hex(&sig.signer_pubkey);

    let proposal = Proposal {
        action_id,
        seq_no,
        authority: session.authority,
        status: ProposalStatus::Pending,
        required_signatures,
        action_hex: action_hex.to_string(),
        title: normalize_title(title),
        signatures: vec![first_sig],
        broadcast_status: BroadcastStatus::default(),
        commit_txid: None,
        reveal_txid: None,
        broadcast_error: None,
        target_action_id: None,
        activation_height: None,
        update_id_in_queue: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        broadcast_claimed_at: None,
        broadcast_attempts: Vec::new(),
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
    let existing = repo.find_by_action_id(action_id).await?;
    let proposal = existing.ok_or(AppError::NotFound)?;
    if proposal.authority != session.authority {
        return Err(AppError::Unauthorized);
    }

    let signer_pubkey = normalize_signer_pubkey_hex(&sig.signer_pubkey);

    let already_signed = proposal
        .signatures
        .iter()
        .any(|s| s.signer_pubkey.eq_ignore_ascii_case(&signer_pubkey));

    if already_signed {
        return Err(AppError::Conflict("signer already signed".to_string()));
    }

    let updated = repo
        .add_signature(action_id, &signer_pubkey, &sig.signature_hex)
        .await?;

    let proposal = updated.ok_or(AppError::NotFound)?;

    Ok(proposal)
}

/// Whether off-chain quorum is satisfied (coordination only — not on-chain validity).
pub(crate) fn has_reached_quorum(proposal: &Proposal) -> bool {
    proposal.signatures.len() >= proposal.required_signatures as usize
}

/// Explicit pending → approved transition (P-012 / ADR-006). Desktop calls after quorum.
pub(crate) async fn transition_to_approved(
    repo: &dyn ProposalRepository,
    session: SessionContext<'_>,
    asm_rpc_url: &str,
    action_id: &ActionId,
) -> Result<Proposal, AppError> {
    let raw = repo
        .find_by_action_id(action_id)
        .await?
        .ok_or(AppError::NotFound)?;
    require_proposal_authority(&raw, session.authority)?;

    if raw.status == ProposalStatus::Approved {
        return Ok(raw);
    }

    if raw.status != ProposalStatus::Pending {
        return Err(AppError::Conflict(format!(
            "proposal must be pending to transition to approved (current: {:?})",
            raw.status
        )));
    }

    if !has_reached_quorum(&raw) {
        return Err(AppError::Conflict(format!(
            "quorum not reached: {} of {} signatures collected",
            raw.signatures.len(),
            raw.required_signatures
        )));
    }

    ensure_threshold_snapshot_current(&raw, asm_rpc_url).await?;

    repo.update_broadcast_status(
        action_id,
        raw.broadcast_status,
        Some(ProposalStatus::Approved),
        None,
        None,
        None,
    )
    .await?
    .ok_or(AppError::NotFound)
}

fn require_proposal_authority(proposal: &Proposal, authority: Authority) -> Result<(), AppError> {
    if proposal.authority != authority {
        return Err(AppError::Unauthorized);
    }
    Ok(())
}

/// Scope a read without revealing whether another authority owns the requested id.
fn require_visible_proposal_authority(
    proposal: &Proposal,
    authority: Authority,
) -> Result<(), AppError> {
    if proposal.authority != authority {
        return Err(AppError::NotFound);
    }
    Ok(())
}

/// Refuse broadcast when the proposal snapshot disagrees with live ASM threshold (P-035).
pub(crate) async fn ensure_threshold_snapshot_current(
    proposal: &Proposal,
    asm_rpc_url: &str,
) -> Result<(), AppError> {
    let live = threshold_for_authority(asm_rpc_url, proposal.authority).await?;
    if proposal.required_signatures != live {
        return Err(AppError::Conflict(format!(
            "proposal required_signatures ({}) does not match current ASM threshold ({live})",
            proposal.required_signatures
        )));
    }
    Ok(())
}

/// Get proposal by ActionId scoped to the session authority.
pub(crate) async fn get_update_action(
    repo: &dyn ProposalRepository,
    authority: Authority,
    action_id: &ActionId,
) -> Result<Proposal, AppError> {
    let proposal = repo
        .find_by_action_id(action_id)
        .await?
        .ok_or(AppError::NotFound)?;
    require_visible_proposal_authority(&proposal, authority)?;
    Ok(proposal)
}

/// Pre-broadcast guard for a Cancel proposal: is its target proposal still `Approved`?
///
/// If the target has already left `Approved` (e.g. it was `Enacted` via the normal flow,
/// or `Canceled`/`Expired`), broadcasting the cancel tx would be rejected by the ASM.
pub(crate) async fn get_cancel_target_status(
    repo: &dyn ProposalRepository,
    authority: Authority,
    action_id: &ActionId,
) -> Result<bool, AppError> {
    let proposal = get_update_action(repo, authority, action_id).await?;

    if !proposal.is_cancel() {
        return Err(AppError::BadRequest(
            "proposal is not a cancel proposal".to_string(),
        ));
    }

    let target_action_id = proposal.target_action_id.as_ref().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("cancel proposal missing target_action_id"))
    })?;

    let target = repo
        .find_by_action_id(target_action_id)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(target.status == ProposalStatus::Approved)
}

/// List proposals for one authority, optionally filtered by status.
pub(crate) async fn list_proposals(
    repo: &dyn ProposalRepository,
    authority: Authority,
    status: Option<ProposalStatus>,
) -> Result<Vec<Proposal>, AppError> {
    repo.list_by_status(authority, status).await
}

/// Next seq_no for a new proposal: one past the greater of on-chain ASM `last_seqno`
/// and the highest seq_no already stored locally for this authority.
pub(crate) async fn next_seq_no_for_authority(
    repo: &dyn ProposalRepository,
    asm_rpc_url: &str,
    authority: Authority,
) -> Result<u64, AppError> {
    let asm_last = last_seqno_for_authority(asm_rpc_url, authority).await?;
    let proposals = list_proposals(repo, authority, None).await?;
    let local_max = proposals.iter().map(|p| p.seq_no).max().unwrap_or(0);
    Ok(next_seq_no_from_state(asm_last, local_max))
}

pub(crate) fn next_seq_no_from_state(asm_last_seqno: u64, local_max_seq_no: u64) -> u64 {
    std::cmp::max(asm_last_seqno, local_max_seq_no) + 1
}

/// When a proposal stops being signable: `expiry_days` after it was proposed.
///
/// The one place the window is computed. `expire_if_overdue` decides with it and the API serves
/// it, so the countdown a client shows and the moment the proposal expires cannot drift (#551).
pub(crate) fn expires_at(proposal: &Proposal, expiry_days: u64) -> DateTime<Utc> {
    proposal.created_at + chrono::Duration::days(expiry_days as i64)
}

/// Lazily expire a pending proposal once its `expiry_days` window has passed.
///
/// No-op for any non-pending status. Persists the `Expired` transition so subsequent
/// reads also return the updated state.
pub(crate) async fn expire_if_overdue(
    repo: &dyn ProposalRepository,
    proposal: Proposal,
    expiry_days: u64,
) -> Result<Proposal, AppError> {
    if proposal.status != ProposalStatus::Pending {
        return Ok(proposal);
    }
    if expires_at(&proposal, expiry_days) > Utc::now() {
        return Ok(proposal);
    }
    repo.update_broadcast_status(
        &proposal.action_id,
        proposal.broadcast_status,
        Some(ProposalStatus::Expired),
        None,
        None,
        None,
    )
    .await?
    .ok_or(AppError::NotFound)
}

// ---------------------------------------------------------------------------
// Broadcast coordination (metadata only — on-chain execution is desktop-owned)
// ---------------------------------------------------------------------------

/// Broadcast requires explicit off-chain `approved` (P-012 — no auto-transition on claim).
fn require_approved(proposal: Proposal) -> Result<Proposal, AppError> {
    if proposal.status == ProposalStatus::Approved {
        return Ok(proposal);
    }

    Err(AppError::Conflict(format!(
        "proposal must be in 'approved' state to broadcast (current: {:?})",
        proposal.status
    )))
}

/// Request body for desktop-reported broadcast progress (coordination only — no Bitcoin RPC).
#[derive(Debug, serde::Deserialize)]
pub struct ReportBroadcastProgressRequest {
    pub broadcast_status: String,
    pub proposal_status: Option<String>,
    pub commit_txid: Option<String>,
    pub reveal_txid: Option<String>,
    pub broadcast_error: Option<String>,
}

/// How long a `commit_broadcasted` row with no txids stays taken.
///
/// The desktop claims first and then waits up to 180s for the device to sign,
/// so this sits well above that window. A claim older than this, with neither
/// txid stored, never reached the network and may be taken again.
pub(crate) const CLAIM_STALE_AFTER: chrono::Duration = chrono::Duration::seconds(600);

/// Stable conflict text for a refused claim. The desktop matches this sentence.
pub(crate) const AUTHORITY_IN_FLIGHT: &str = "a broadcast for this authority is already in flight";

/// The fields `claim_broadcast` decides on. `claimed_at` is not part of the API payload.
pub(crate) struct BroadcastClaimFacts {
    pub proposal_status: ProposalStatus,
    pub broadcast_status: BroadcastStatus,
    pub commit_txid: Option<String>,
    pub reveal_txid: Option<String>,
    pub claimed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<&Proposal> for BroadcastClaimFacts {
    fn from(proposal: &Proposal) -> Self {
        Self {
            proposal_status: proposal.status,
            broadcast_status: proposal.broadcast_status,
            commit_txid: proposal.commit_txid.clone(),
            reveal_txid: proposal.reveal_txid.clone(),
            claimed_at: proposal.broadcast_claimed_at,
        }
    }
}

fn txids_absent(facts: &BroadcastClaimFacts) -> bool {
    facts.commit_txid.is_none() && facts.reveal_txid.is_none()
}

/// `true` when `claimed_at` is at least [`CLAIM_STALE_AFTER`] before `now`.
/// A missing timestamp is not stale: the row cannot be proved old.
fn claim_is_stale(facts: &BroadcastClaimFacts, now: chrono::DateTime<chrono::Utc>) -> bool {
    match facts.claimed_at {
        Some(at) => now.signed_duration_since(at) >= CLAIM_STALE_AFTER,
        None => false,
    }
}

fn own_row_is_claimable(facts: &BroadcastClaimFacts, now: chrono::DateTime<chrono::Utc>) -> bool {
    match facts.broadcast_status {
        BroadcastStatus::Idle | BroadcastStatus::Failed => true,
        BroadcastStatus::CommitBroadcasted if txids_absent(facts) && claim_is_stale(facts, now) => {
            true
        }
        _ => false,
    }
}

/// Another proposal of this authority blocks a new claim.
///
/// Only an `approved` row can. Closing the proposal (`superseded`, `expired`,
/// `canceled`, `enacted`) leaves `broadcast_status` where it was, and reconcile
/// does not walk those rows, so counting them would wedge the authority after
/// the bundle can no longer be sent. `commit_confirmed` and `reveal_broadcasted`
/// are on the network. A `commit_broadcasted` row blocks when it has a txid, or
/// when its empty claim is still inside the window. An empty claim older than
/// the window does not block: nothing was published. `idle`, `failed` and
/// `reveal_confirmed` do not block.
fn other_bundle_in_flight(facts: &BroadcastClaimFacts, now: chrono::DateTime<chrono::Utc>) -> bool {
    if facts.proposal_status != ProposalStatus::Approved {
        return false;
    }
    match facts.broadcast_status {
        BroadcastStatus::CommitConfirmed | BroadcastStatus::RevealBroadcasted => true,
        BroadcastStatus::CommitBroadcasted => !txids_absent(facts) || !claim_is_stale(facts, now),
        _ => false,
    }
}

/// A `commit_broadcasted` row with no txid whose claim is past [`CLAIM_STALE_AFTER`].
///
/// Nothing reached the network and the row can be claimed again, so the desktop offers
/// Send. Inside the window the claimer may still be at the device, and the row reads as
/// in flight. Decided here, on the server clock, so every desktop agrees with the claim
/// gate.
pub(crate) fn empty_claim_is_stale(
    proposal: &Proposal,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    let facts = BroadcastClaimFacts::from(proposal);
    facts.broadcast_status == BroadcastStatus::CommitBroadcasted
        && txids_absent(&facts)
        && claim_is_stale(&facts, now)
}

/// Same predicate as the `claim_broadcast` SQL in `postgres_repo`.
pub(crate) fn broadcast_claim_allowed(
    own: &BroadcastClaimFacts,
    others: &[BroadcastClaimFacts],
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    own_row_is_claimable(own, now)
        && !others
            .iter()
            .any(|other| other_bundle_in_flight(other, now))
}

/// Atomically claim broadcast coordination rights (`idle` → `commit_broadcasted`).
pub(crate) async fn claim_broadcast_coordination(
    repo: &dyn ProposalRepository,
    session_authority: Authority,
    asm_rpc_url: &str,
    action_id: &ActionId,
) -> Result<Proposal, AppError> {
    let raw = repo
        .find_by_action_id(action_id)
        .await?
        .ok_or(AppError::NotFound)?;
    require_proposal_authority(&raw, session_authority)?;

    let proposal = require_approved(raw)?;
    ensure_threshold_snapshot_current(&proposal, asm_rpc_url).await?;
    ensure_seq_no_still_open(&proposal, asm_rpc_url).await?;

    repo.claim_broadcast(action_id).await
}

/// Conflict text when there is no pair in flight to send another one beside (#605).
pub(crate) const NO_PAIR_IN_FLIGHT: &str =
    "this proposal has no commit and reveal in flight to send another pair beside";

/// Conflict text when the caller already registered a pair for this proposal (#605).
pub(crate) const OWN_PAIR_EXISTS: &str = "you already sent a commit and reveal for this proposal";

/// Conflict text when the pair being registered is already listed under another signer (#605).
pub(crate) const PAIR_ALREADY_LISTED: &str =
    "this commit and reveal is already listed for this proposal";

/// Conflict text when the caller already listed [`MAX_PAIRS_PER_SIGNER`] pairs (#605).
pub(crate) const PAIR_LIMIT_REACHED: &str =
    "you already sent the most commit and reveal pairs allowed for this proposal";

/// How many pairs one signer may list for a proposal, abandoned ones included (#605). A pair whose
/// commit is nowhere lets its signer send another, and registration takes any txids, so without a
/// bound one signer could list pairs without end: each one takes the row over, and every reconcile
/// asks the node about every pair.
pub(crate) const MAX_PAIRS_PER_SIGNER: usize = 3;

/// Whether listing `attempt` would take its signer past [`MAX_PAIRS_PER_SIGNER`] (#605). A pair
/// already listed is not a new one. Checked on every path that lists a pair — a registration, and
/// a claimer's pre-registration — so a claim followed by `failed`, again and again, is bounded too.
/// Shared by both repositories, which evaluate it under their own lock.
pub(crate) fn exceeds_pair_limit(proposal: &Proposal, attempt: &BroadcastAttempt) -> bool {
    let listed =
        |kept: &&BroadcastAttempt| kept.reveal_txid.eq_ignore_ascii_case(&attempt.reveal_txid);
    let Some(signer) = attempt.broadcaster_pubkey.as_deref() else {
        return false;
    };
    if proposal.broadcast_attempts.iter().any(|kept| listed(&kept)) {
        return false;
    }
    proposal
        .broadcast_attempts
        .iter()
        .filter(|kept| {
            kept.broadcaster_pubkey
                .as_deref()
                .is_some_and(|kept| kept.eq_ignore_ascii_case(signer))
        })
        .count()
        >= MAX_PAIRS_PER_SIGNER
}

/// Whether a desktop's report may change the row (#605).
///
/// An untagged row (no reveal yet) accepts it: that is the claimer's pre-registration. A tagged
/// row accepts it only when the report names that same reveal. A report that names no reveal, or a
/// different one, must not move the row. Shared by both repositories, which evaluate it in the
/// write itself.
pub(crate) fn reports_tracked_pair(
    reported_reveal: Option<&str>,
    tracked_reveal: Option<&str>,
) -> bool {
    match (reported_reveal, tracked_reveal) {
        (_, None) => true,
        (Some(reported), Some(tracked)) => reported.eq_ignore_ascii_case(tracked),
        (None, Some(_)) => false,
    }
}

/// Whether the row still carries the reveal a reconcile observed (#605). Either side moving on
/// means the decision was made against a row that no longer exists, and the next read decides.
pub(crate) fn row_still_has_reveal(observed: Option<&str>, tracked: Option<&str>) -> bool {
    match (observed, tracked) {
        (None, None) => true,
        (Some(observed), Some(tracked)) => observed.eq_ignore_ascii_case(tracked),
        _ => false,
    }
}

/// What registering another commit + reveal pair does to a proposal (#605).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AdditionalPair {
    /// A new pair: keep it and make it the one the row tracks.
    Register,
    /// This signer already registered this very pair — a retry after a lost response. Nothing
    /// changes; the desktop goes on to broadcast it.
    AlreadyRegistered,
}

/// Whether another commit + reveal pair may be registered for `proposal`, or the conflict text
/// refusing it (#605).
///
/// Only beside a pair that is on its way: the row is approved and `commit_broadcasted` with a
/// txid, `commit_confirmed` or `reveal_broadcasted`. An empty claim is a signer still at the
/// device; `idle`, `failed` and a stale empty claim go through the claim instead. One pair per
/// signer: whoever already sent one bumps its fee, and the desktop holds one stored bundle per
/// proposal — except that the same signer registering the same pair again is the retry of a
/// registration whose response was lost, and succeeds. A pair of theirs listed in `abandoned`
/// (its commit is neither in the mempool nor in a block, as when the registering session died
/// before broadcasting) no longer counts as theirs on its way, up to [`MAX_PAIRS_PER_SIGNER`] pairs
/// in all. Shared by both repositories, which evaluate it under their own lock.
pub(crate) fn decide_additional_pair(
    proposal: &Proposal,
    attempt: &BroadcastAttempt,
    abandoned: &[String],
) -> Result<AdditionalPair, &'static str> {
    let in_flight = proposal.status == ProposalStatus::Approved
        && match proposal.broadcast_status {
            BroadcastStatus::CommitBroadcasted => proposal.reveal_txid.is_some(),
            BroadcastStatus::CommitConfirmed | BroadcastStatus::RevealBroadcasted => true,
            _ => false,
        };
    if !in_flight {
        return Err(NO_PAIR_IN_FLIGHT);
    }
    let same_signer = |kept: &BroadcastAttempt| {
        kept.broadcaster_pubkey
            .as_deref()
            .zip(attempt.broadcaster_pubkey.as_deref())
            .is_some_and(|(kept, new)| kept.eq_ignore_ascii_case(new))
    };
    let same_reveal =
        |kept: &BroadcastAttempt| kept.reveal_txid.eq_ignore_ascii_case(&attempt.reveal_txid);
    if proposal
        .broadcast_attempts
        .iter()
        .any(|kept| same_signer(kept) && same_reveal(kept))
    {
        return Ok(AdditionalPair::AlreadyRegistered);
    }
    if exceeds_pair_limit(proposal, attempt) {
        return Err(PAIR_LIMIT_REACHED);
    }
    let given_up = |kept: &BroadcastAttempt| {
        abandoned
            .iter()
            .any(|reveal| reveal.eq_ignore_ascii_case(&kept.reveal_txid))
    };
    if proposal.broadcast_attempts.iter().any(same_reveal) {
        return Err(PAIR_ALREADY_LISTED);
    }
    if proposal
        .broadcast_attempts
        .iter()
        .any(|kept| same_signer(kept) && !given_up(kept))
    {
        return Err(OWN_PAIR_EXISTS);
    }
    Ok(AdditionalPair::Register)
}

/// Whether the node answered that a transaction is nowhere: neither in its mempool nor in a block.
/// Only a node with `-txindex` can say so; without one the answer is not taken (#605).
fn tx_nowhere(result: &Result<u64, AppError>) -> bool {
    match result {
        Err(AppError::BadRequest(message)) => {
            let message = message.to_ascii_lowercase();
            message.contains("no such mempool or blockchain transaction")
                && !message.contains("being indexed")
        }
        _ => false,
    }
}

/// The reveal txids of `signer`'s own listed pairs whose commit the node finds nowhere (#605): a
/// registration whose session died before broadcasting, or a pair dropped from the mempool.
async fn abandoned_pairs_of(
    proposal: &Proposal,
    signer: &str,
    btc_client: &dyn BitcoinRpcClient,
) -> Vec<String> {
    let own = proposal.broadcast_attempts.iter().filter(|kept| {
        kept.broadcaster_pubkey
            .as_deref()
            .is_some_and(|pubkey| pubkey.eq_ignore_ascii_case(signer))
            && is_txid(&kept.commit_txid)
    });
    let lookups = own.map(|kept| async move {
        let lookup = btc_client
            .get_block_height_for_txid(&kept.commit_txid)
            .await;
        tx_nowhere(&lookup).then(|| kept.reveal_txid.clone())
    });
    futures::future::join_all(lookups)
        .await
        .into_iter()
        .flatten()
        .collect()
}

/// The reveal txids of the listed pairs that may still be on their way (#605): every pair except
/// one whose commit the node answers is nowhere — neither in its mempool nor in a block. A node
/// that could not answer (a timeout, an index still syncing) does not make a pair dead: marking the
/// row `failed` beside a live pair would stop its watchers and reopen the authority's claim gate.
async fn live_pairs(proposal: &Proposal, btc_client: &dyn BitcoinRpcClient) -> Vec<String> {
    let lookups = proposal
        .broadcast_attempts
        .iter()
        .filter(|kept| is_txid(&kept.commit_txid))
        .map(|kept| async move {
            let lookup = btc_client
                .get_block_height_for_txid(&kept.commit_txid)
                .await;
            (!tx_nowhere(&lookup)).then(|| kept.reveal_txid.clone())
        });
    futures::future::join_all(lookups)
        .await
        .into_iter()
        .flatten()
        .collect()
}

fn is_txid(txid: &str) -> bool {
    txid.len() == 64 && txid.bytes().all(|b| b.is_ascii_hexdigit())
}

fn require_txid(name: &str, txid: &str) -> Result<(), AppError> {
    if is_txid(txid) {
        return Ok(());
    }
    Err(AppError::BadRequest(format!(
        "{name} must be a 64-char hex txid"
    )))
}

/// Register a signer's own commit + reveal pair beside the one already in flight (#605).
///
/// A low-fee bundle can sit in the mempool while the other signers have no way to advance it.
/// Every pair carries the same signed payload, so whichever reveal is mined first is the one the
/// ASM acts on and the rest are rejected on the sequence number. The new pair becomes the one the
/// row tracks (`commit_broadcasted`); the earlier ones stay listed and the reconcile still promotes
/// the row if one of them is mined first. Registered before anything is broadcast, like the
/// claimer's pre-registration (#516).
pub(crate) async fn register_additional_pair(
    repo: &dyn ProposalRepository,
    session: SessionContext<'_>,
    asm_rpc_url: &str,
    btc_client: &dyn BitcoinRpcClient,
    action_id: &ActionId,
    commit_txid: &str,
    reveal_txid: &str,
) -> Result<Proposal, AppError> {
    require_txid("commit_txid", commit_txid)?;
    require_txid("reveal_txid", reveal_txid)?;
    let raw = repo
        .find_by_action_id(action_id)
        .await?
        .ok_or(AppError::NotFound)?;
    require_proposal_authority(&raw, session.authority)?;

    let proposal = require_approved(raw)?;
    ensure_threshold_snapshot_current(&proposal, asm_rpc_url).await?;
    ensure_seq_no_still_open(&proposal, asm_rpc_url).await?;

    let signer = normalize_signer_pubkey_hex(session.signer_pubkey);
    let abandoned = abandoned_pairs_of(&proposal, &signer, btc_client).await;
    repo.register_additional_pair(
        action_id,
        BroadcastAttempt {
            broadcaster_pubkey: Some(signer),
            commit_txid: commit_txid.to_ascii_lowercase(),
            reveal_txid: reveal_txid.to_ascii_lowercase(),
            created_at: Utc::now(),
        },
        &abandoned,
    )
    .await
}

/// Whether this signer may send their own commit + reveal pair now (#605): the checks
/// [`register_additional_pair`] makes that do not need the pair's txids, so the desktop asks before
/// the signer approves a commit on the device rather than after. The registration checks again;
/// the row can move in between.
pub(crate) async fn check_additional_pair(
    repo: &dyn ProposalRepository,
    session: SessionContext<'_>,
    asm_rpc_url: &str,
    btc_client: &dyn BitcoinRpcClient,
    action_id: &ActionId,
) -> Result<Proposal, AppError> {
    let raw = repo
        .find_by_action_id(action_id)
        .await?
        .ok_or(AppError::NotFound)?;
    require_proposal_authority(&raw, session.authority)?;

    let proposal = require_approved(raw)?;
    ensure_threshold_snapshot_current(&proposal, asm_rpc_url).await?;
    ensure_seq_no_still_open(&proposal, asm_rpc_url).await?;

    let signer = normalize_signer_pubkey_hex(session.signer_pubkey);
    let abandoned = abandoned_pairs_of(&proposal, &signer, btc_client).await;
    // No txids yet: a reveal that matches nothing, so only the row and the signer's own pairs
    // decide.
    let unsigned = BroadcastAttempt {
        broadcaster_pubkey: Some(signer),
        commit_txid: String::new(),
        reveal_txid: String::new(),
        created_at: Utc::now(),
    };
    decide_additional_pair(&proposal, &unsigned, &abandoned)
        .map_err(|refusal| AppError::Conflict(refusal.to_string()))?;
    Ok(proposal)
}

/// Refuse to broadcast a bundle the ASM will refuse.
///
/// Upstream accepts an action only when its seqno is strictly above the role's `last_seqno`, so a
/// proposal the role has already passed is a commit fee, a reveal fee and a destroyed ephemeral
/// key spent on a transaction that cannot execute. Not an ordering rule (PRD 02 §4.3): nothing
/// here stops a higher seqno from going first, and the proposal being refused is one no ordering
/// could rescue. Best-effort, like the threshold snapshot beside it — the chain can move between
/// this check and the block.
async fn ensure_seq_no_still_open(proposal: &Proposal, asm_rpc_url: &str) -> Result<(), AppError> {
    let last_seqno = last_seqno_for_authority(asm_rpc_url, proposal.authority).await?;
    if proposal.seq_no > last_seqno {
        return Ok(());
    }
    Err(AppError::Conflict(format!(
        "sequence number {} was already used on chain (the authority is at {}); \
         this proposal can no longer be broadcast and needs to be recreated",
        proposal.seq_no, last_seqno
    )))
}

/// What one reveal lookup told us. "Not in a block" is an answer; a node that could not answer is
/// not, and must not be read as the reveal losing the race (#605).
enum RevealHeight {
    Mined {
        height: u64,
        tx: Option<bitcoin::Transaction>,
    },
    Unmined,
    /// The node keeps no transaction index, so it cannot find a mined transaction at all.
    NotIndexed,
    Unknown,
}

/// `get_mined_reveal` reports both "not in a block" and a failed node call as `BadRequest`. Only
/// the first is a real answer.
fn classify_reveal_height(
    result: Result<crate::infrastructure::bitcoin_rpc::MinedReveal, AppError>,
) -> RevealHeight {
    match result {
        Ok(mined) => RevealHeight::Mined {
            height: mined.height,
            tx: mined.tx,
        },
        Err(AppError::BadRequest(message)) if reveal_not_in_a_block(&message) => {
            RevealHeight::Unmined
        }
        Err(AppError::BadRequest(message)) if node_has_no_txindex(&message) => {
            RevealHeight::NotIndexed
        }
        Err(_) => RevealHeight::Unknown,
    }
}

/// A node without `-txindex` answers -5 "No such mempool transaction. Use -txindex …" for any
/// transaction outside its mempool, mined or not.
fn node_has_no_txindex(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    message.contains("-txindex") && !message.contains("being indexed")
}

/// Whether a mined `reveal` is a commit + reveal of `proposal` (#605): it spends `commit_txid`,
/// and its envelope carries this proposal's sequence number and action, signed by a quorum of its
/// approvers. Read the way the ASM reads it (`strata_asm_txs_admin::parser::parse_tx`).
/// Registration takes any txid, so this is what keeps an unrelated mined transaction — or one
/// carrying this action without the signatures the ASM needs — from becoming the pair the
/// reconcile confirms.
fn reveal_carries_proposal(
    reveal: &bitcoin::Transaction,
    commit_txid: &str,
    proposal: &Proposal,
    network: bitcoin::Network,
) -> bool {
    use ssz::{Decode, Encode};
    let Some(input) = reveal.input.first() else {
        return false;
    };
    if !input
        .previous_output
        .txid
        .to_string()
        .eq_ignore_ascii_case(commit_txid)
    {
        return false;
    }
    let Some(leaf) = input.witness.taproot_leaf_script() else {
        return false;
    };
    let Ok(payload) = strata_l1_envelope_fmt::parser::parse_envelope_payload(&leaf.script.into())
    else {
        return false;
    };
    let Ok(signed) = strata_asm_txs_admin::parser::SignedPayload::from_ssz_bytes(&payload) else {
        return false;
    };
    let Ok(action) = hex::decode(&proposal.action_hex) else {
        return false;
    };
    signed.seqno == proposal.seq_no
        && signed.action.as_ssz_bytes() == action
        && signed_by_quorum(&signed, proposal, network)
}

/// Whether the payload's signatures include at least `required_signatures` distinct approvers of
/// `proposal`, each a valid signature over the payload's own sighash (#605). Every signature is
/// checked with the ASM's own verifier against one approver's key at a time, so this needs neither
/// the role's key order nor an ASM call; the approvers were checked against the role when they
/// signed.
///
/// `_network` is not read at this pin. From the v0.5 pin on it feeds
/// `SigningMessage::for_action`, whose message names the network.
fn signed_by_quorum(
    signed: &strata_asm_txs_admin::parser::SignedPayload,
    proposal: &Proposal,
    _network: bitcoin::Network,
) -> bool {
    use strata_crypto::keys::compressed::CompressedPublicKey;
    use strata_crypto::threshold_signature::{
        verify_threshold_signatures, IndexedSignature, ThresholdConfig,
    };
    let sighash = strata_asm_txs_admin::signing_message::SigningMessage::for_action(
        &signed.action,
        signed.seqno,
    )
    .compute_sighash()
    .0;
    let approvers: Vec<ThresholdConfig> = proposal
        .signatures
        .iter()
        .filter_map(|approval| hex::decode(&approval.signer_pubkey).ok())
        .filter_map(|key| CompressedPublicKey::from_slice(&key).ok())
        .filter_map(|key| ThresholdConfig::try_new(vec![key], std::num::NonZero::<u8>::MIN).ok())
        .collect();
    let mut signed_by = vec![false; approvers.len()];
    for signature in signed.signatures.signatures() {
        let mut bytes = [0u8; 65];
        bytes[0] = signature.recovery_id();
        bytes[1..33].copy_from_slice(signature.r());
        bytes[33..].copy_from_slice(signature.s());
        let alone = [IndexedSignature::new(0, bytes)];
        if let Some(approver) = approvers
            .iter()
            .position(|key| verify_threshold_signatures(key, &alone, &sighash).is_ok())
        {
            signed_by[approver] = true;
        }
    }
    signed_by.iter().filter(|signed| **signed).count() >= usize::from(proposal.required_signatures)
}

/// What a progress report writes. A `failed` report for the pair the row tracks puts the row back
/// on the newest other pair the node still has (#605): that pair may yet be mined, and marking the
/// proposal `failed` would offer a fresh claim beside it. `live` lists the reveals whose commit the
/// node had in its mempool or in a block just before the write; a pair not in it is dead or
/// unknown, and the row is marked `failed` as reported.
pub(crate) struct BroadcastWrite {
    pub status: BroadcastStatus,
    pub commit_txid: Option<String>,
    pub reveal_txid: Option<String>,
    pub error: Option<String>,
}

pub(crate) fn broadcast_write_for_report(
    proposal: &Proposal,
    status: BroadcastStatus,
    commit_txid: Option<&str>,
    reveal_txid: Option<&str>,
    error: Option<&str>,
    live: &[String],
) -> BroadcastWrite {
    if status == BroadcastStatus::Failed {
        if let Some(reported) = reveal_txid {
            if let Some(previous) = proposal
                .broadcast_attempts
                .iter()
                .filter(|attempt| !attempt.reveal_txid.eq_ignore_ascii_case(reported))
                .filter(|attempt| {
                    live.iter()
                        .any(|reveal| reveal.eq_ignore_ascii_case(&attempt.reveal_txid))
                })
                .max_by_key(|attempt| attempt.created_at)
            {
                return BroadcastWrite {
                    status: BroadcastStatus::CommitBroadcasted,
                    commit_txid: Some(previous.commit_txid.clone()),
                    reveal_txid: Some(previous.reveal_txid.clone()),
                    error: None,
                };
            }
        }
    }
    BroadcastWrite {
        status,
        commit_txid: commit_txid.map(str::to_string),
        reveal_txid: reveal_txid.map(str::to_string),
        error: error.map(str::to_string),
    }
}

/// Whether the node answered that the transaction is not in a block: in the mempool, or nowhere.
///
/// A node without `-txindex` answers -5 "No such mempool transaction. Use -txindex …" for a
/// transaction that *is* mined, and one still building its index says so too: neither is an
/// answer, and reading them as "not mined" would skip the reveal that won (#605).
fn reveal_not_in_a_block(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    if message.contains("-txindex") || message.contains("being indexed") {
        return false;
    }
    message.contains("not yet confirmed")
        || message.contains("no such mempool or blockchain transaction")
        || message.contains("no such transaction")
}

/// What the node says about every pair of a proposal (#605).
#[derive(Debug, PartialEq, Eq)]
enum MinedPair {
    /// The pair mined first: `(commit_txid, reveal_txid)`.
    First(String, String),
    /// No pair of this proposal is in a block.
    None,
    /// The node keeps no transaction index and cannot tell.
    NotIndexed,
    /// The node could not answer about one of them; the next read decides.
    Unknown,
}

/// The commit + reveal pair mined first, out of the row's own pair and every pair registered for
/// the proposal (#605): the lowest block height, and within one block the reveal the block lists
/// first — the one the ASM executed. A mined reveal that is not a commit + reveal of this proposal
/// does not count, however low its block. The lookups run side by side.
async fn first_mined_pair(
    proposal: &Proposal,
    btc_client: &dyn BitcoinRpcClient,
    network: bitcoin::Network,
) -> MinedPair {
    let tracked = proposal.reveal_txid.as_deref();
    // A malformed txid (stored before reports were validated) can never be mined, and its failing
    // lookup would hold back every other pair; this includes one on the row itself.
    let candidates: Vec<(Option<&str>, &str)> = tracked
        .map(|reveal| (proposal.commit_txid.as_deref(), reveal))
        .into_iter()
        .chain(
            proposal
                .broadcast_attempts
                .iter()
                .filter(|attempt| {
                    !tracked.is_some_and(|txid| txid.eq_ignore_ascii_case(&attempt.reveal_txid))
                })
                .map(|attempt| {
                    (
                        Some(attempt.commit_txid.as_str()),
                        attempt.reveal_txid.as_str(),
                    )
                }),
        )
        .filter(|(_, reveal)| is_txid(reveal))
        .collect();

    let lookups = candidates.iter().map(|(_, reveal)| async move {
        classify_reveal_height(btc_client.get_mined_reveal(reveal).await)
    });
    let answers = futures::future::join_all(lookups).await;

    let mut mined: Vec<(u64, String, &str)> = Vec::new();
    let mut not_indexed = false;
    for ((commit, reveal), answer) in candidates.iter().zip(answers) {
        match answer {
            RevealHeight::Mined { height, tx } => {
                // A row tagged with its reveal only (a partial import) takes the commit the reveal
                // spends. With neither, the pair cannot be checked, nor confirmed.
                let commit = match (commit, tx.as_ref().and_then(|tx| tx.input.first())) {
                    (Some(commit), _) => commit.to_string(),
                    (None, Some(input)) => input.previous_output.txid.to_string(),
                    (None, None) => continue,
                };
                // `None` only from a test double that reads no transaction.
                if tx
                    .as_ref()
                    .is_some_and(|tx| !reveal_carries_proposal(tx, &commit, proposal, network))
                {
                    tracing::warn!(
                        action_id = %proposal.action_id.0,
                        reveal,
                        "a listed reveal is mined but does not carry this proposal; ignored"
                    );
                    continue;
                }
                mined.push((height, commit, reveal));
            }
            RevealHeight::Unmined => {}
            RevealHeight::NotIndexed => not_indexed = true,
            RevealHeight::Unknown => return MinedPair::Unknown,
        }
    }
    let Some(lowest) = mined.iter().map(|(height, _, _)| *height).min() else {
        return if not_indexed {
            MinedPair::NotIndexed
        } else {
            MinedPair::None
        };
    };
    // A pair the node cannot look up may have been mined lower than the one it found.
    if not_indexed {
        return MinedPair::NotIndexed;
    }
    let mut first: Vec<(String, &str)> = mined
        .into_iter()
        .filter(|(height, _, _)| *height == lowest)
        .map(|(_, commit, reveal)| (commit, reveal))
        .collect();
    let (commit, reveal) = if first.len() == 1 {
        first.remove(0)
    } else {
        // Two reveals in one block: the ASM acts on the one the block lists first. A node that
        // cannot place them leaves the row for the next read rather than guess.
        let mut placed = Vec::with_capacity(first.len());
        for (commit, reveal) in first {
            let Ok(position) = btc_client.get_tx_position_in_block(reveal).await else {
                return MinedPair::Unknown;
            };
            placed.push((position, commit, reveal));
        }
        let Some((_, commit, reveal)) = placed.into_iter().min_by_key(|(position, _, _)| *position)
        else {
            return MinedPair::Unknown;
        };
        (commit, reveal)
    };
    MinedPair::First(commit, reveal.to_string())
}

/// Notice a reveal that confirmed while nothing was watching.
///
/// The desktop reports `reveal_confirmed` from a task that lives with its send screen: close the
/// screen and nobody asks again. A reveal mined afterwards left the proposal reading *Reveal sent*
/// forever — and invisible to both the enactment check and the supersession sweep, which is what
/// made a bundle that had executed indistinguishable from one that was dead.
///
/// Returns the proposal as it now stands. Any failure — an unmined reveal is one — leaves it alone.
async fn confirm_reveal_if_mined(
    repo: &dyn ProposalRepository,
    proposal: Proposal,
    btc_client: &dyn BitcoinRpcClient,
    network: bitcoin::Network,
    asm_rpc_url: &str,
) -> Proposal {
    // The desktop pre-registers both txids at `commit_broadcasted` before it broadcasts (#516),
    // so a row whose later reports never landed still carries the reveal txid to check. A `failed`
    // row keeps its txids too: if that reveal was mined, the `failed` was a mistake, and leaving it
    // would let the sweep below retire as Superseded a proposal whose own reveal used the seqno.
    if !matches!(
        proposal.broadcast_status,
        BroadcastStatus::CommitBroadcasted
            | BroadcastStatus::CommitConfirmed
            | BroadcastStatus::RevealBroadcasted
            | BroadcastStatus::Failed
    ) {
        return proposal;
    }
    let observed_reveal = proposal.reveal_txid.clone();
    // Name the winner's txids, so a pair registered after this read cannot be the one `COALESCE`
    // leaves in place. The write applies only while the row still tracks the reveal this read
    // observed: a pair registered in between is left for the next reconcile, which sees both
    // (#605).
    let MinedPair::First(commit_txid, reveal_txid) =
        first_mined_pair(&proposal, btc_client, network).await
    else {
        return proposal;
    };
    let updated = repo
        .confirm_tracked_reveal(
            &proposal.action_id,
            observed_reveal.as_deref(),
            &commit_txid,
            &reveal_txid,
        )
        .await;
    match updated {
        Ok(Some(updated)) => {
            if !updated.is_cancel() {
                record_reveal_confirmed_facts(repo, &updated, btc_client, asm_rpc_url).await;
            }
            updated
        }
        Ok(None) => proposal,
        Err(e) => {
            tracing::warn!(
                action_id = %proposal.action_id.0,
                "failed to persist reveal confirmation: {e}"
            );
            proposal
        }
    }
}

/// The role's live sequence number, or `None` when the ASM cannot answer.
///
/// Supersession is a cleanup, never a reason to fail a read: a node that is down must not turn a
/// proposal list into an error.
async fn live_last_seqno(asm_rpc_url: &str, authority: Authority) -> Option<u64> {
    match last_seqno_for_authority(asm_rpc_url, authority).await {
        Ok(seqno) => Some(seqno),
        Err(e) => {
            tracing::warn!("supersession: last_seqno lookup failed for {authority:?}: {e}");
            None
        }
    }
}

/// Retire a proposal the chain has moved past.
///
/// Upstream accepts an action only when its seqno is strictly above the role's `last_seqno`, and
/// the seqno is inside the signed message — so once the role stands past it, the proposal cannot
/// be sent, and cannot be relabelled without a fresh quorum. Only reached after the enactment
/// check, so a proposal that did enact is never swept.
/// See docs/specs/proposal-lifecycle-seqno-truth.md §4.
///
/// The comparison is `<=`, matching upstream's rejection rule exactly: it refuses
/// `payload.seqno <= last_seqno`, so a proposal whose seqno *equals* the role's is as dead as one
/// below it. Only reached when the proposal did not enact, and enactment is what claims equality
/// legitimately.
///
/// A queued update is the exception, and it is not a small one: upstream consumes the seqno when
/// it *accepts* an action, and an action with a confirmation depth waits in the admin queue for
/// its activation height. So a perfectly healthy update maturing over 2016 blocks has its seqno
/// passed by the next action of the same role, and would be swept while it is still on its way.
/// Presence in the live queue is proof the ASM accepted it; that outranks the seqno.
async fn supersede_if_seq_no_consumed(
    repo: &dyn ProposalRepository,
    asm_rpc_url: &str,
    proposal: &Proposal,
    last_seqno: Option<u64>,
) -> Result<(), AppError> {
    if last_seqno.is_none_or(|last| proposal.seq_no > last) {
        return Ok(());
    }
    match update_id_in_queue_for_action(asm_rpc_url, &proposal.action_hex).await {
        Ok(Some(_)) => return Ok(()),
        Ok(None) => {}
        Err(e) => {
            tracing::warn!(
                action_id = %proposal.action_id.0,
                "supersession: queue lookup failed, leaving the proposal alone: {e}"
            );
            return Ok(());
        }
    }
    tracing::info!(
        action_id = %proposal.action_id.0,
        seq_no = proposal.seq_no,
        "superseding proposal: the role's sequence number has passed it"
    );
    repo.update_broadcast_status(
        &proposal.action_id,
        proposal.broadcast_status,
        Some(ProposalStatus::Superseded),
        None,
        None,
        None,
    )
    .await?;
    Ok(())
}

async fn bitcoin_tip_for_enactment(
    action_hex: &str,
    btc_client: &dyn BitcoinRpcClient,
) -> Result<Option<u64>, AppError> {
    if !crate::infrastructure::asm_enactment::action_needs_chain_tip(action_hex) {
        return Ok(None);
    }
    btc_client.get_chain_tip().await.map(Some)
}

/// Whether a cancel of this proposal already reached the chain, so the sweep must not decide the
/// target's lifecycle this cycle.
///
/// A cancel that confirms takes its target out of the admin queue while the target's seqno is
/// already consumed — the exact shape `supersede_if_seq_no_consumed` reads as "dead". Left alone,
/// a cancelled Defcon 3 would land on `Superseded` instead of `Canceled`, and `enact_cancel` would
/// then refuse (it requires an Approved target) and retire the cancel as `Expired`. The other
/// ordering loses too: the cancel writes `Canceled`, and this sweep's copy — read before that
/// write — puts the target back. Only the cancel decides a cancelled target.
///
/// An `Expired` cancel is the case where the target enacted first, so it holds nothing up.
async fn cancel_reached_chain(
    repo: &dyn ProposalRepository,
    target_action_id: &ActionId,
) -> Result<bool, AppError> {
    let Some(cancel) = repo.find_cancel_for_target(target_action_id).await? else {
        return Ok(false);
    };
    Ok(cancel.broadcast_status == BroadcastStatus::RevealConfirmed
        && cancel.status != ProposalStatus::Expired)
}

/// Advance one approved proposal: notice a reveal that confirmed, decide enactment, or retire it.
///
/// `last_seqno` is the role's live sequence number, or `None` when the ASM could not answer —
/// supersession is a cleanup, never a reason to fail a read.
///
/// An ASM enactment check that fails — including a Defcon 3 whose height or tip is missing —
/// leaves the proposal exactly where it is: the next cycle asks again. `Ok(false)` is "not
/// enacted"; inconclusive must be `Err`.
async fn reconcile_one(
    repo: &dyn ProposalRepository,
    asm_rpc_url: &str,
    btc_client: &dyn BitcoinRpcClient,
    network: bitcoin::Network,
    proposal: Proposal,
    last_seqno: Option<u64>,
) -> Result<(), AppError> {
    // First: did its reveal confirm while nobody was watching? Both the enactment check and the
    // sweep below read the broadcast status, so a missed confirmation would make an executed
    // proposal look dead.
    let proposal = confirm_reveal_if_mined(repo, proposal, btc_client, network, asm_rpc_url).await;

    // A cancel that is already on chain owns this proposal's outcome — see `cancel_reached_chain`.
    if !proposal.is_cancel() && cancel_reached_chain(repo, &proposal.action_id).await? {
        return Ok(());
    }

    // Only a confirmed reveal can have enacted; everything else goes straight to the supersession
    // check, which is what retires a bundle that never made it on chain.
    let awaiting_enactment = proposal.broadcast_status == BroadcastStatus::RevealConfirmed
        && proposal.reveal_txid.is_some();
    let enacted = if awaiting_enactment {
        let bitcoin_tip = match bitcoin_tip_for_enactment(&proposal.action_hex, btc_client).await {
            Ok(tip) => tip,
            Err(e) => {
                tracing::warn!(
                    action_id = %proposal.action_id.0,
                    "reconcile: bitcoin tip lookup failed: {e}"
                );
                return Ok(());
            }
        };
        match crate::infrastructure::asm_enactment::is_proposal_enacted_on_asm(
            asm_rpc_url,
            proposal.authority,
            proposal.seq_no,
            &proposal.action_hex,
            proposal.activation_height,
            bitcoin_tip,
        )
        .await
        {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    action_id = %proposal.action_id.0,
                    "reconcile: ASM enactment check failed: {e}"
                );
                return Ok(());
            }
        }
    } else {
        false
    };

    if !enacted {
        return supersede_if_seq_no_consumed(repo, asm_rpc_url, &proposal, last_seqno).await;
    }

    if let Some(target_action_id) = &proposal.target_action_id {
        let applied = repo
            .enact_cancel(&proposal.action_id, target_action_id)
            .await?;
        if !applied {
            repo.update_broadcast_status(
                &proposal.action_id,
                BroadcastStatus::RevealConfirmed,
                Some(ProposalStatus::Expired),
                None,
                None,
                None,
            )
            .await?;
        }
        return Ok(());
    }

    repo.update_broadcast_status(
        &proposal.action_id,
        BroadcastStatus::RevealConfirmed,
        Some(ProposalStatus::Enacted),
        None,
        None,
        None,
    )
    .await?;
    cascade_enact_associated_cancel(repo, &proposal.action_id).await
}

/// Promote `approved` + `reveal_confirmed` proposals when ASM shows enactment post-conditions,
/// and retire the ones whose sequence number the chain has already consumed.
pub(crate) async fn reconcile_enacted_for_authority(
    repo: &dyn ProposalRepository,
    asm_rpc_url: &str,
    btc_client: &dyn BitcoinRpcClient,
    network: bitcoin::Network,
    authority: Authority,
) -> Result<(), AppError> {
    let approved = repo
        .list_by_status(authority, Some(ProposalStatus::Approved))
        .await?;

    if approved.is_empty() {
        return Ok(());
    }
    let last_seqno = live_last_seqno(asm_rpc_url, authority).await;

    for proposal in approved {
        reconcile_one(repo, asm_rpc_url, btc_client, network, proposal, last_seqno).await?;
    }
    Ok(())
}

/// Retry the facts [`record_reveal_confirmed_facts`] stores if they are still null after
/// RevealConfirmed. A transient Bitcoin or ASM failure at confirm time otherwise leaves the
/// countdown and queue id blank for the life of the row.
///
/// Non-fatal: logs and returns Ok(()) on any lookup failure so callers can proceed.
pub(crate) async fn reconcile_reveal_confirmed_facts(
    repo: &dyn ProposalRepository,
    asm_rpc_url: &str,
    btc_client: &dyn BitcoinRpcClient,
    action_id: &ActionId,
) -> Result<(), AppError> {
    let Some(proposal) = repo.find_by_action_id(action_id).await? else {
        return Ok(());
    };

    if proposal.is_cancel() || proposal.broadcast_status != BroadcastStatus::RevealConfirmed {
        return Ok(());
    }

    if proposal.activation_height.is_none() {
        if let Err(e) =
            compute_and_store_activation_height(&proposal, repo, btc_client, asm_rpc_url).await
        {
            tracing::warn!(
                action_id = %action_id.0,
                "reconcile_reveal_confirmed_facts: failed to compute height: {e}"
            );
        }
    }

    if proposal.update_id_in_queue.is_none() {
        match update_id_in_queue_for_action(asm_rpc_url, &proposal.action_hex).await {
            Ok(Some(id)) => {
                if let Err(e) = repo.update_update_id_in_queue(action_id, id).await {
                    tracing::warn!(
                        action_id = %action_id.0,
                        "reconcile_reveal_confirmed_facts: failed to persist id: {e}"
                    );
                }
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(
                    action_id = %action_id.0,
                    "reconcile_reveal_confirmed_facts: ASM lookup failed: {e}"
                );
            }
        }
    }

    Ok(())
}

/// Reconcile a single proposal before returning it to the client.
pub(crate) async fn reconcile_enacted_for_action(
    repo: &dyn ProposalRepository,
    asm_rpc_url: &str,
    btc_client: &dyn BitcoinRpcClient,
    network: bitcoin::Network,
    authority: Authority,
    action_id: &ActionId,
) -> Result<(), AppError> {
    let Some(proposal) = repo.find_by_action_id(action_id).await? else {
        return Ok(());
    };
    require_visible_proposal_authority(&proposal, authority)?;

    if proposal.status != ProposalStatus::Approved {
        return Ok(());
    }

    // Read up front rather than only on the supersession branch: it costs one extra RPC on the
    // cycle where the proposal enacts — once, since an enacted proposal returns above — and it is
    // what lets this share `reconcile_one` with the sweep.
    let last_seqno = live_last_seqno(asm_rpc_url, authority).await;
    reconcile_one(repo, asm_rpc_url, btc_client, network, proposal, last_seqno).await
}

/// When a target proposal becomes `Enacted` via the normal flow, its associated cancel
/// proposal (if any, and still pending/approved) is now moot — the ASM would reject its
/// broadcast, so mark it `Expired` so it stops appearing as actionable.
async fn cascade_enact_associated_cancel(
    repo: &dyn ProposalRepository,
    target_action_id: &ActionId,
) -> Result<(), AppError> {
    let Some(cancel) = repo.find_cancel_for_target(target_action_id).await? else {
        return Ok(());
    };
    if matches!(
        cancel.status,
        ProposalStatus::Enacted | ProposalStatus::Canceled | ProposalStatus::Expired
    ) {
        return Ok(());
    }
    repo.update_broadcast_status(
        &cancel.action_id,
        cancel.broadcast_status,
        Some(ProposalStatus::Expired),
        None,
        None,
        None,
    )
    .await?;
    Ok(())
}

/// Create a cancel proposal for an Approved target whose reveal confirmed, or return the existing
/// one (idempotent).
///
/// `action_hex` encodes `MultisigAction::Cancel` — built by the desktop before signing.
pub(crate) async fn create_cancel_proposal(
    repo: &dyn ProposalRepository,
    asm_rpc_url: &str,
    session: SessionContext<'_>,
    target_action_id: ActionId,
    seq_no: SeqNo,
    action_hex: &str,
    signature_hex: &str,
) -> Result<Proposal, AppError> {
    let target = repo
        .find_by_action_id(&target_action_id)
        .await?
        .ok_or(AppError::NotFound)?;

    // The cancel is stored under the *target's* authority, so without this a session of any
    // authority could file one — with its own signature as the first — into another authority's
    // queue. It could never be enacted (the signature is outside the target's key set), but a
    // session is bound to exactly one authority and this is the write path that forgot it. The
    // read path beside it, `get_cancel_target_status`, has always checked.
    require_proposal_authority(&target, session.authority)?;

    if target.status != ProposalStatus::Approved {
        return Err(AppError::BadRequest(format!(
            "target proposal must be approved to cancel (current: {})",
            target.status
        )));
    }

    // A cancel names the update's id in the ASM queue, and the update is queued only once its
    // reveal confirms — the same rule the desktop's `canCancelProposal` applies before offering it.
    if target.broadcast_status != BroadcastStatus::RevealConfirmed {
        return Err(AppError::BadRequest(format!(
            "cancel is not possible yet: the update is queued only once its reveal confirms \
             (broadcast: {})",
            target.broadcast_status
        )));
    }

    // Cancelability is the target action's confirmation depth, never its authority: the Security
    // Council signs both Defcon 1 (depth 0) and Defcon 3 (timelocked). A zero-depth action is never
    // enqueued, so an on-chain cancel would fail with `UnknownAction`.
    let target_depth = lock_period_for_action(asm_rpc_url, &target.action_hex).await?;
    if target_depth == 0 {
        return Err(AppError::BadRequest(format!(
            "cancel is not possible: this action has a confirmation depth of {target_depth}, so it \
             is never enqueued and an on-chain cancel would fail with UnknownAction"
        )));
    }

    if let Some(existing) = repo.find_cancel_for_target(&target_action_id).await? {
        return Ok(existing);
    }

    let required_signatures = threshold_for_authority(asm_rpc_url, target.authority).await?;
    let action_id = compute_action_id(seq_no, action_hex)?;
    let normalized_pubkey = normalize_signer_pubkey_hex(session.signer_pubkey);

    let proposal = Proposal {
        action_id,
        seq_no,
        authority: target.authority,
        status: ProposalStatus::Pending,
        required_signatures,
        action_hex: action_hex.to_string(),
        // Cancel proposals carry no user-written title; the UI labels them from the target.
        title: None,
        signatures: vec![ProposalSignature {
            signer_pubkey: normalized_pubkey,
            signature_hex: signature_hex.to_string(),
        }],
        broadcast_status: BroadcastStatus::default(),
        commit_txid: None,
        reveal_txid: None,
        broadcast_error: None,
        target_action_id: Some(target_action_id),
        activation_height: None,
        update_id_in_queue: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        broadcast_claimed_at: None,
        broadcast_attempts: Vec::new(),
    };

    repo.save_proposal(proposal.clone()).await?;
    Ok(proposal)
}

/// Compute and persist `activation_height` after a non-cancel proposal's reveal is confirmed.
///
/// Non-fatal: caller should log and continue on error rather than failing the progress report.
async fn compute_and_store_activation_height(
    proposal: &Proposal,
    repo: &dyn ProposalRepository,
    btc_client: &dyn BitcoinRpcClient,
    asm_rpc_url: &str,
) -> Result<(), AppError> {
    let Some(reveal_txid) = &proposal.reveal_txid else {
        return Ok(());
    };
    let reveal_confirm_block = btc_client.get_block_height_for_txid(reveal_txid).await?;
    let lock_period = lock_period_for_action(asm_rpc_url, &proposal.action_hex).await?;
    let activation_height = reveal_confirm_block + lock_period;
    repo.update_activation_height(&proposal.action_id, activation_height)
        .await
}

/// Persist broadcast sub-status reported by the desktop after local Bitcoin submission.
pub(crate) async fn report_broadcast_progress(
    repo: &dyn ProposalRepository,
    asm_rpc_url: &str,
    session: SessionContext<'_>,
    action_id: &ActionId,
    btc_client: &dyn BitcoinRpcClient,
    network: bitcoin::Network,
    body: ReportBroadcastProgressRequest,
) -> Result<Proposal, AppError> {
    let raw = repo
        .find_by_action_id(action_id)
        .await?
        .ok_or(AppError::NotFound)?;
    require_proposal_authority(&raw, session.authority)?;
    // A malformed txid would be listed and then fail every node lookup, which holds back the
    // reconcile of every pair of the proposal (#605).
    if let Some(txid) = body.commit_txid.as_deref() {
        require_txid("commit_txid", txid)?;
    }
    if let Some(txid) = body.reveal_txid.as_deref() {
        require_txid("reveal_txid", txid)?;
    }

    let commit_txid = body
        .commit_txid
        .as_deref()
        .map(|txid| txid.to_ascii_lowercase());
    let reveal_txid = body
        .reveal_txid
        .as_deref()
        .map(|txid| txid.to_ascii_lowercase());

    // A report about a pair the row does not track changes nothing (#605): its `failed` or
    // `reveal_broadcasted` would overwrite the pair now in flight, and its txids are not recorded
    // either — the import screen reports any bundle against the open proposal, so they may belong
    // to another action. Every pair of this proposal is already listed: it was reported while the
    // row tracked it, or registered. A report that names no reveal does not qualify either, once
    // the row is tagged. Checked here to skip the work, and again in the write: a pair that lands
    // after this read, including one that tags an empty claim, is not overwritten and not listed.
    // A report about the pair the row tracks cannot change its commit: the reconcile checks that
    // the mined reveal spends it, and a wrong one would leave a winning pair unconfirmed (#605).
    let commit_txid = match (&raw.commit_txid, &raw.reveal_txid, reveal_txid.as_deref()) {
        (Some(tracked_commit), Some(tracked), Some(reported))
            if tracked.eq_ignore_ascii_case(reported) =>
        {
            Some(tracked_commit.to_ascii_lowercase())
        }
        _ => commit_txid,
    };

    // Validated before the report can be skipped, so a malformed one is a 400 either way.
    let broadcast_status: BroadcastStatus = body.broadcast_status.parse().map_err(|_| {
        AppError::BadRequest(format!(
            "unknown broadcast_status: {}",
            body.broadcast_status
        ))
    })?;
    let requested_status = requested_proposal_status(body.proposal_status.as_deref())?;
    if !reports_tracked_pair(reveal_txid.as_deref(), raw.reveal_txid.as_deref()) {
        log_report_for_replaced_pair(action_id, reveal_txid.as_deref(), &raw);
        return Ok(raw);
    }

    // With other pairs listed, which reveal confirmed is the node's call, not the report's (#605):
    // an earlier pair may be mined in a lower block. The report records its pair as sent, and the
    // reconcile promotes the pair mined first. A node that does not see it yet leaves the row for
    // the next read. A lone pair has nothing to race, so its report is taken as before. So is any
    // report a node without `-txindex` cannot check: it can never find a mined reveal, and the
    // proposal would otherwise never confirm — the operator is told at startup.
    let competing_pairs = raw.broadcast_attempts.iter().any(|attempt| {
        !reveal_txid
            .as_deref()
            .is_some_and(|reported| attempt.reveal_txid.eq_ignore_ascii_case(reported))
    });
    let mut confirming = broadcast_status == BroadcastStatus::RevealConfirmed
        && raw.broadcast_status != BroadcastStatus::RevealConfirmed
        && competing_pairs;
    if let (true, Some(reveal)) = (confirming, reveal_txid.as_deref()) {
        if matches!(
            classify_reveal_height(btc_client.get_mined_reveal(reveal).await),
            RevealHeight::NotIndexed
        ) {
            tracing::warn!(
                action_id = %action_id.0,
                reveal,
                "the node keeps no transaction index; taking the desktop's reveal_confirmed for \
                 a proposal with several pairs without checking which one was mined first"
            );
            confirming = false;
        }
    }
    let broadcast_status = if confirming {
        BroadcastStatus::RevealBroadcasted
    } else {
        broadcast_status
    };
    let proposal_status = match requested_status {
        None => None,
        Some(RequestedProposalStatus::Pending) => Some(ProposalStatus::Pending),
        Some(RequestedProposalStatus::Enacted) => {
            if raw.status == ProposalStatus::Enacted {
                None
            } else {
                if raw.broadcast_status != BroadcastStatus::RevealConfirmed {
                    return Err(AppError::Conflict(format!(
                        "proposal must have broadcast_status reveal_confirmed before enacted (current: {})",
                        raw.broadcast_status
                    )));
                }
                if !crate::infrastructure::asm_enactment::is_proposal_enacted_on_asm(
                    asm_rpc_url,
                    raw.authority,
                    raw.seq_no,
                    &raw.action_hex,
                    raw.activation_height,
                    bitcoin_tip_for_enactment(&raw.action_hex, btc_client).await?,
                )
                .await?
                {
                    return Err(AppError::Conflict(
                        "ASM state does not yet show proposal enactment".to_string(),
                    ));
                }
                Some(ProposalStatus::Enacted)
            }
        }
        Some(RequestedProposalStatus::Canceled) => Some(ProposalStatus::Canceled),
        Some(RequestedProposalStatus::Expired) => Some(ProposalStatus::Expired),
    };

    // Listed in the same write as the status, and only when that write applies (#605). A `failed`
    // report lists nothing: a pair that reached the orchestrator is listed already, and one whose
    // pre-registration never landed was never broadcast.
    let attempt = if broadcast_status == BroadcastStatus::Failed {
        None
    } else {
        reported_attempt(session, commit_txid.as_deref(), reveal_txid.as_deref())
    };
    // Only a `failed` report can put the row back on an earlier pair, and only on one the node
    // still has (#605).
    let live = if broadcast_status == BroadcastStatus::Failed && competing_pairs {
        live_pairs(&raw, btc_client).await
    } else {
        Vec::new()
    };
    let updated = repo
        .update_reported_pair_status(
            action_id,
            broadcast_status,
            proposal_status,
            commit_txid.as_deref(),
            reveal_txid.as_deref(),
            body.broadcast_error.as_deref(),
            attempt,
            &live,
        )
        .await?
        .ok_or(AppError::NotFound)?;
    if !reports_tracked_pair(reveal_txid.as_deref(), updated.reveal_txid.as_deref()) {
        log_report_for_replaced_pair(action_id, reveal_txid.as_deref(), &updated);
        return Ok(updated);
    }
    if confirming {
        return Ok(confirm_reveal_if_mined(repo, updated, btc_client, network, asm_rpc_url).await);
    }

    if updated.broadcast_status == BroadcastStatus::RevealConfirmed && !updated.is_cancel() {
        record_reveal_confirmed_facts(repo, &updated, btc_client, asm_rpc_url).await;
    }

    Ok(updated)
}

/// A `proposal_status` a broadcast report may carry.
enum RequestedProposalStatus {
    Pending,
    Enacted,
    Canceled,
    Expired,
}

fn requested_proposal_status(
    raw: Option<&str>,
) -> Result<Option<RequestedProposalStatus>, AppError> {
    Ok(match raw {
        None => None,
        Some("pending") => Some(RequestedProposalStatus::Pending),
        Some("approved") => {
            return Err(AppError::BadRequest(
                "use PATCH /proposals/:action_id with proposal_status approved to transition; not via broadcast PATCH"
                    .to_string(),
            ));
        }
        Some("enacted") => Some(RequestedProposalStatus::Enacted),
        Some("canceled") => Some(RequestedProposalStatus::Canceled),
        Some("expired") => Some(RequestedProposalStatus::Expired),
        Some(other) => {
            return Err(AppError::BadRequest(format!(
                "unknown proposal_status: {other}"
            )));
        }
    })
}

/// The pair a report names, stored lowercase so the case-sensitive key matches once (#605).
/// `None` when either txid is missing: that report names no pair.
fn reported_attempt(
    session: SessionContext<'_>,
    commit_txid: Option<&str>,
    reveal_txid: Option<&str>,
) -> Option<BroadcastAttempt> {
    let (commit_txid, reveal_txid) = commit_txid.zip(reveal_txid)?;
    Some(BroadcastAttempt {
        broadcaster_pubkey: Some(normalize_signer_pubkey_hex(session.signer_pubkey)),
        commit_txid: commit_txid.to_ascii_lowercase(),
        reveal_txid: reveal_txid.to_ascii_lowercase(),
        created_at: Utc::now(),
    })
}

fn log_report_for_replaced_pair(action_id: &ActionId, reported: Option<&str>, row: &Proposal) {
    tracing::info!(
        action_id = %action_id.0,
        reported,
        tracked = row.reveal_txid.as_deref(),
        "broadcast report for a pair the row no longer tracks; status left unchanged"
    );
}

/// The two facts a confirmed reveal makes available: when the update activates, and the id the ASM
/// queued it under. Both non-fatal — a proposal is never held back because a lookup failed.
///
/// Shared by the desktop's progress report and by the reconciliation that notices a reveal the
/// desktop never got to report.
async fn record_reveal_confirmed_facts(
    repo: &dyn ProposalRepository,
    proposal: &Proposal,
    btc_client: &dyn BitcoinRpcClient,
    asm_rpc_url: &str,
) {
    if let Err(e) =
        compute_and_store_activation_height(proposal, repo, btc_client, asm_rpc_url).await
    {
        tracing::warn!(
            action_id = %proposal.action_id.0,
            "failed to compute activation_height: {e}"
        );
    }
    match update_id_in_queue_for_action(asm_rpc_url, &proposal.action_hex).await {
        Ok(Some(id)) => {
            if let Err(e) = repo
                .update_update_id_in_queue(&proposal.action_id, id)
                .await
            {
                tracing::warn!(
                    action_id = %proposal.action_id.0,
                    "failed to store update_id_in_queue: {e}"
                );
            }
        }
        Ok(None) => {
            tracing::warn!(
                action_id = %proposal.action_id.0,
                "update not found in ASM queue after RevealConfirmed"
            );
        }
        Err(e) => {
            tracing::warn!(
                action_id = %proposal.action_id.0,
                "failed to query ASM for update_id_in_queue: {e}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::action_codec::{
        test_fixture_action_hex, test_fixture_council_rotation_action_hex,
        test_fixture_defcon_1_action_hex, test_fixture_safe_harbor_address_action_hex,
    };
    use crate::infrastructure::bitcoin_rpc::{BitcoinRpcClient, MinedReveal, TxIndex};
    use crate::infrastructure::memory_repo::InMemoryProposalRepository;
    use serde_json::json;

    fn new_repo() -> InMemoryProposalRepository {
        InMemoryProposalRepository::new()
    }

    struct MockBitcoinRpcClient {
        block_height: u64,
        chain_tip: u64,
    }

    #[async_trait::async_trait]
    impl BitcoinRpcClient for MockBitcoinRpcClient {
        async fn estimate_fee_rate_sats_per_vb(
            &self,
            _target_blocks: u16,
        ) -> Result<u64, AppError> {
            Ok(1)
        }

        async fn get_block_height_for_txid(&self, _txid: &str) -> Result<u64, AppError> {
            Ok(self.block_height)
        }

        async fn get_chain_tip(&self) -> Result<u64, AppError> {
            Ok(self.chain_tip)
        }
    }

    fn mock_btc() -> MockBitcoinRpcClient {
        MockBitcoinRpcClient {
            block_height: 800_000,
            chain_tip: 800_000,
        }
    }

    const ACTION_HEX: &str = "deadbeef";

    /// 1·G, so a test can sign as this approver with the secret scalar 1.
    fn sig_a() -> ProposalSignature {
        ProposalSignature {
            signer_pubkey: "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"
                .to_string(),
            signature_hex: "sig_a".to_string(),
        }
    }

    /// 2·G, so a test can sign as this approver with the secret scalar 2.
    fn sig_b() -> ProposalSignature {
        ProposalSignature {
            signer_pubkey: "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5"
                .to_string(),
            signature_hex: "sig_b".to_string(),
        }
    }

    #[test]
    fn test_next_seq_no_from_state_uses_asm_when_no_local_proposals() {
        assert_eq!(next_seq_no_from_state(5, 0), 6);
    }

    #[test]
    fn test_next_seq_no_from_state_increments_past_local_pending() {
        assert_eq!(next_seq_no_from_state(2, 4), 5);
    }

    #[test]
    fn test_next_seq_no_from_state_asm_ahead_of_stale_local() {
        assert_eq!(next_seq_no_from_state(10, 3), 11);
    }

    /// PRD §5.3.3: a pending update expires once `expiry_days` have passed since it was proposed,
    /// and the transition is persisted so every later read agrees. One day either side of the
    /// window, and a status the rule must never touch.
    #[tokio::test]
    async fn expire_if_overdue_expires_only_a_pending_proposal_past_its_window() {
        const EXPIRY_DAYS: u64 = 7;
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };
        let created = create_update_action(&repo, session, 3, ACTION_HEX, &sig, 2, None)
            .await
            .unwrap();
        let aged = |days: i64| Proposal {
            created_at: Utc::now() - chrono::Duration::days(days),
            ..created.clone()
        };

        let inside = expire_if_overdue(&repo, aged(EXPIRY_DAYS as i64 - 1), EXPIRY_DAYS)
            .await
            .unwrap();
        assert_eq!(inside.status, ProposalStatus::Pending, "inside the window");

        let past = expire_if_overdue(&repo, aged(EXPIRY_DAYS as i64 + 1), EXPIRY_DAYS)
            .await
            .unwrap();
        assert_eq!(past.status, ProposalStatus::Expired, "past the window");
        let stored = repo
            .find_by_action_id(&created.action_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            stored.status,
            ProposalStatus::Expired,
            "the expiry is persisted"
        );

        let approved = save_approved_proposal(&repo, Authority::StrataAdmin, 4).await;
        let old_approved = Proposal {
            created_at: Utc::now() - chrono::Duration::days(EXPIRY_DAYS as i64 + 1),
            ..approved
        };
        let untouched = expire_if_overdue(&repo, old_approved, EXPIRY_DAYS)
            .await
            .unwrap();
        assert_eq!(
            untouched.status,
            ProposalStatus::Approved,
            "only a pending proposal expires"
        );
    }

    /// #551: the served expiry follows the configured window, not a constant.
    #[tokio::test]
    async fn expires_at_is_created_at_plus_the_configured_window() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };
        let created = create_update_action(&repo, session, 3, ACTION_HEX, &sig, 2, None)
            .await
            .unwrap();

        assert_eq!(
            expires_at(&created, 7),
            created.created_at + chrono::Duration::days(7)
        );
        assert_eq!(
            expires_at(&created, 1),
            created.created_at + chrono::Duration::days(1)
        );
    }

    #[tokio::test]
    async fn test_next_seq_no_for_authority_considers_local_proposals() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        create_update_action(&repo, session.clone(), 3, ACTION_HEX, &sig, 2, None)
            .await
            .unwrap();
        create_update_action(&repo, session.clone(), 5, "cafebabe", &sig, 2, None)
            .await
            .unwrap();

        let next =
            next_seq_no_for_authority(&repo, "mock://asm-membership", Authority::StrataAdmin)
                .await
                .unwrap();

        assert_eq!(next, 6);
    }

    // #491 — the title the author typed used to be dropped on the way in. These pin that it
    // survives the round trip, and that a blank one is stored as absent rather than as an empty
    // string the UI would then have to special-case.
    #[tokio::test]
    async fn test_create_update_action_persists_title() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let proposal = create_update_action(
            &repo,
            session,
            1,
            ACTION_HEX,
            &sig,
            2,
            Some("Rotate signing key Q3".to_string()),
        )
        .await
        .unwrap();

        assert_eq!(proposal.title.as_deref(), Some("Rotate signing key Q3"));

        let stored = repo
            .find_by_action_id(&proposal.action_id)
            .await
            .unwrap()
            .expect("proposal should be stored");
        assert_eq!(stored.title.as_deref(), Some("Rotate signing key Q3"));
    }

    #[tokio::test]
    async fn test_create_update_action_blank_title_is_absent() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let proposal = create_update_action(
            &repo,
            session,
            1,
            ACTION_HEX,
            &sig,
            2,
            Some("   ".to_string()),
        )
        .await
        .unwrap();

        assert_eq!(proposal.title, None);
    }

    #[tokio::test]
    async fn test_create_update_action_without_title_is_none() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let proposal = create_update_action(&repo, session, 1, ACTION_HEX, &sig, 2, None)
            .await
            .unwrap();

        assert_eq!(proposal.title, None);
    }

    #[tokio::test]
    async fn test_create_update_action() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let proposal = create_update_action(&repo, session, 1, ACTION_HEX, &sig, 2, None)
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
    /// PRD 02 §3.4: the duplicate is rejected and the existing proposal is left alone — including
    /// the signature the second creator arrived with. The rejection names the `ActionId` they
    /// should approve instead. A second *different* signer is what makes this a test of the rule
    /// rather than of equality.
    async fn test_create_duplicate_action_rejected_naming_the_existing_proposal() {
        let repo = new_repo();
        let first = sig_a();
        let second = sig_b();

        let created = create_update_action(
            &repo,
            SessionContext {
                authority: Authority::StrataAdmin,
                signer_pubkey: &first.signer_pubkey,
            },
            1,
            ACTION_HEX,
            &first,
            2,
            None,
        )
        .await
        .unwrap();

        let err = create_update_action(
            &repo,
            SessionContext {
                authority: Authority::StrataAdmin,
                signer_pubkey: &second.signer_pubkey,
            },
            1,
            ACTION_HEX,
            &second,
            2,
            None,
        )
        .await
        .expect_err("the duplicate is rejected");

        assert!(matches!(err, AppError::Conflict(_)));
        assert!(err.to_string().contains(&created.action_id.0));

        let stored = repo.find_by_action_id(&created.action_id).await.unwrap();
        let stored = stored.expect("the first proposal survives");
        assert_eq!(stored.signatures.len(), 1);
        assert_eq!(stored.signatures[0].signer_pubkey, first.signer_pubkey);
    }

    #[tokio::test]
    async fn test_approve_action() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2, None)
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
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5"
        );
    }

    #[tokio::test]
    async fn test_approve_at_quorum_stays_pending_until_explicit_transition() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2, None)
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
        assert_eq!(updated.status, ProposalStatus::Pending);
    }

    #[tokio::test]
    async fn test_transition_to_approved_at_quorum() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2, None)
            .await
            .unwrap();
        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        approve_action(&repo, session_b.clone(), &created.action_id, &sig_b())
            .await
            .unwrap();

        let approved = transition_to_approved(
            &repo,
            session_b,
            "mock://asm-membership",
            &created.action_id,
        )
        .await
        .unwrap();

        assert_eq!(approved.status, ProposalStatus::Approved);
    }

    #[tokio::test]
    async fn test_transition_to_approved_fails_below_quorum() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 3, None)
            .await
            .unwrap();

        let err =
            transition_to_approved(&repo, session, "mock://asm-membership", &created.action_id)
                .await
                .unwrap_err();

        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_transition_to_approved_idempotent() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2, None)
            .await
            .unwrap();
        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        approve_action(&repo, session_b.clone(), &created.action_id, &sig_b())
            .await
            .unwrap();

        transition_to_approved(
            &repo,
            session_b.clone(),
            "mock://asm-membership",
            &created.action_id,
        )
        .await
        .unwrap();
        let again = transition_to_approved(
            &repo,
            session_b,
            "mock://asm-membership",
            &created.action_id,
        )
        .await
        .unwrap();

        assert_eq!(again.status, ProposalStatus::Approved);
    }

    #[tokio::test]
    async fn test_transition_rejects_threshold_drift() {
        let repo = new_repo();
        let proposal = Proposal {
            action_id: ActionId("drift".to_string()),
            seq_no: 1,
            authority: Authority::StrataAdmin,
            status: ProposalStatus::Pending,
            required_signatures: 99,
            action_hex: ACTION_HEX.to_string(),
            title: None,
            signatures: vec![sig_a(), sig_b()],
            broadcast_status: BroadcastStatus::Idle,
            commit_txid: None,
            reveal_txid: None,
            broadcast_error: None,
            target_action_id: None,
            activation_height: None,
            update_id_in_queue: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            broadcast_claimed_at: None,
            broadcast_attempts: Vec::new(),
        };
        repo.save_proposal(proposal.clone()).await.unwrap();

        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_a().signer_pubkey,
        };
        let err =
            transition_to_approved(&repo, session, "mock://asm-membership", &proposal.action_id)
                .await
                .unwrap_err();

        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_claim_broadcast_rejects_pending_at_quorum() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2, None)
            .await
            .unwrap();
        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        approve_action(&repo, session_b, &created.action_id, &sig_b())
            .await
            .unwrap();

        let err = claim_broadcast_coordination(
            &repo,
            Authority::StrataAdmin,
            "mock://asm-membership",
            &created.action_id,
        )
        .await
        .unwrap_err();

        assert!(matches!(err, AppError::Conflict(_)));
    }

    /// The bundle would be refused on chain, so the fee is spent for nothing and the ephemeral
    /// envelope key is destroyed in the attempt.
    #[tokio::test]
    async fn claim_refuses_a_sequence_number_the_chain_already_used() {
        let repo = new_repo();
        let action_id = save_approved(&repo, 1).await;

        let err = claim_broadcast_coordination(
            &repo,
            Authority::StrataAdmin,
            crate::infrastructure::asm_enactment::MOCK_SEQNO_AHEAD_URL,
            &action_id,
        )
        .await
        .unwrap_err();

        let AppError::Conflict(message) = err else {
            panic!("expected a conflict naming the sequence number");
        };
        assert!(
            message.contains('1'),
            "message should name the sequence: {message}"
        );

        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        assert_eq!(proposal.broadcast_status, BroadcastStatus::Idle);
    }

    #[tokio::test]
    async fn test_claim_broadcast_409_when_not_idle() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2, None)
            .await
            .unwrap();
        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        approve_action(&repo, session_b, &created.action_id, &sig_b())
            .await
            .unwrap();
        transition_to_approved(
            &repo,
            SessionContext {
                authority: Authority::StrataAdmin,
                signer_pubkey: &sig_b().signer_pubkey,
            },
            "mock://asm-membership",
            &created.action_id,
        )
        .await
        .unwrap();

        claim_broadcast_coordination(
            &repo,
            Authority::StrataAdmin,
            "mock://asm-membership",
            &created.action_id,
        )
        .await
        .unwrap();

        let err = claim_broadcast_coordination(
            &repo,
            Authority::StrataAdmin,
            "mock://asm-membership",
            &created.action_id,
        )
        .await
        .unwrap_err();

        let AppError::Conflict(message) = err else {
            panic!("expected a conflict");
        };
        assert_eq!(message, AUTHORITY_IN_FLIGHT);
    }

    fn stale_claim_time() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now() - CLAIM_STALE_AFTER - chrono::Duration::seconds(1)
    }

    #[tokio::test]
    async fn stale_empty_claim_can_be_retaken() {
        let repo = new_repo();
        let action_id = save_approved(&repo, 1).await;
        claim_broadcast_coordination(
            &repo,
            Authority::StrataAdmin,
            "mock://asm-membership",
            &action_id,
        )
        .await
        .unwrap();
        let stale = stale_claim_time();
        repo.stage_broadcast_claim(
            &action_id,
            BroadcastStatus::CommitBroadcasted,
            None,
            None,
            Some(stale),
            Some("stuck"),
        )
        .unwrap();

        let reclaimed = claim_broadcast_coordination(
            &repo,
            Authority::StrataAdmin,
            "mock://asm-membership",
            &action_id,
        )
        .await
        .unwrap();

        assert_eq!(
            reclaimed.broadcast_status,
            BroadcastStatus::CommitBroadcasted
        );
        assert!(reclaimed.broadcast_error.is_none());
        let claimed_at = repo.broadcast_claimed_at(&action_id).unwrap();
        assert!(claimed_at > stale);
    }

    #[tokio::test]
    async fn recent_empty_claim_stays_taken() {
        let repo = new_repo();
        let action_id = save_approved(&repo, 1).await;
        claim_broadcast_coordination(
            &repo,
            Authority::StrataAdmin,
            "mock://asm-membership",
            &action_id,
        )
        .await
        .unwrap();
        let recent = chrono::Utc::now() - chrono::Duration::seconds(60);
        repo.stage_broadcast_claim(
            &action_id,
            BroadcastStatus::CommitBroadcasted,
            None,
            None,
            Some(recent),
            Some("signing"),
        )
        .unwrap();

        let err = claim_broadcast_coordination(
            &repo,
            Authority::StrataAdmin,
            "mock://asm-membership",
            &action_id,
        )
        .await
        .unwrap_err();

        let AppError::Conflict(message) = err else {
            panic!("expected a conflict");
        };
        assert_eq!(message, AUTHORITY_IN_FLIGHT);
        let row = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        assert_eq!(row.broadcast_status, BroadcastStatus::CommitBroadcasted);
        assert_eq!(row.broadcast_error.as_deref(), Some("signing"));
        assert_eq!(repo.broadcast_claimed_at(&action_id), Some(recent));
    }

    #[tokio::test]
    async fn claim_with_a_txid_is_never_retaken() {
        let repo = new_repo();
        let action_id = save_approved(&repo, 1).await;
        claim_broadcast_coordination(
            &repo,
            Authority::StrataAdmin,
            "mock://asm-membership",
            &action_id,
        )
        .await
        .unwrap();

        for (commit, reveal) in [(Some("commit"), None), (None, Some("reveal"))] {
            repo.stage_broadcast_claim(
                &action_id,
                BroadcastStatus::CommitBroadcasted,
                commit,
                reveal,
                Some(stale_claim_time()),
                None,
            )
            .unwrap();
            let err = claim_broadcast_coordination(
                &repo,
                Authority::StrataAdmin,
                "mock://asm-membership",
                &action_id,
            )
            .await
            .unwrap_err();
            assert!(matches!(err, AppError::Conflict(_)));
            let row = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
            assert_eq!(row.commit_txid.as_deref(), commit);
            assert_eq!(row.reveal_txid.as_deref(), reveal);
        }
    }

    /// A retry of a dropped bundle must not keep the old txids: the claim runs before the device
    /// signs, and the new txids are registered only after that. Leftover txids would make a crash
    /// in between look like a published bundle.
    #[tokio::test]
    async fn claiming_a_failed_row_drops_its_txids() {
        let repo = new_repo();
        let action_id = save_approved(&repo, 1).await;
        repo.stage_broadcast_claim(
            &action_id,
            BroadcastStatus::Failed,
            Some("old-commit"),
            Some("old-reveal"),
            None,
            Some("dropped"),
        )
        .unwrap();

        let claimed = claim_broadcast_coordination(
            &repo,
            Authority::StrataAdmin,
            "mock://asm-membership",
            &action_id,
        )
        .await
        .unwrap();

        assert_eq!(claimed.broadcast_status, BroadcastStatus::CommitBroadcasted);
        assert!(claimed.commit_txid.is_none());
        assert!(claimed.reveal_txid.is_none());
        assert!(claimed.broadcast_error.is_none());
    }

    #[tokio::test]
    async fn another_proposal_of_the_same_authority_is_refused_while_one_is_in_flight() {
        let repo = new_repo();
        let first = save_approved(&repo, 1).await;
        let second = save_approved(&repo, 2).await;
        claim_broadcast_coordination(
            &repo,
            Authority::StrataAdmin,
            "mock://asm-membership",
            &first,
        )
        .await
        .unwrap();

        let blocks = [
            (
                BroadcastStatus::RevealBroadcasted,
                Some("commit"),
                Some("reveal"),
                Some(stale_claim_time()),
            ),
            (
                BroadcastStatus::CommitBroadcasted,
                Some("commit"),
                None,
                Some(stale_claim_time()),
            ),
            (
                BroadcastStatus::CommitBroadcasted,
                None,
                None,
                Some(chrono::Utc::now()),
            ),
            (
                BroadcastStatus::CommitConfirmed,
                None,
                None,
                Some(stale_claim_time()),
            ),
        ];
        for (status, commit, reveal, claimed_at) in blocks {
            repo.stage_broadcast_claim(&first, status, commit, reveal, claimed_at, None)
                .unwrap();
            let err = claim_broadcast_coordination(
                &repo,
                Authority::StrataAdmin,
                "mock://asm-membership",
                &second,
            )
            .await
            .unwrap_err();
            assert!(matches!(err, AppError::Conflict(_)), "{status}");
            let row = repo.find_by_action_id(&second).await.unwrap().unwrap();
            assert_eq!(row.broadcast_status, BroadcastStatus::Idle);
        }
    }

    /// Closing a proposal leaves its broadcast status where it was. Those rows must not keep
    /// blocking the authority: reconcile never walks them, so nothing else would clear the gate.
    #[tokio::test]
    async fn a_closed_proposal_does_not_block_its_authority() {
        let repo = new_repo();
        let closed = save_approved(&repo, 1).await;
        let cases = [
            (
                ProposalStatus::Superseded,
                BroadcastStatus::RevealBroadcasted,
            ),
            (ProposalStatus::Expired, BroadcastStatus::CommitConfirmed),
        ];
        for (index, (proposal_status, broadcast_status)) in cases.into_iter().enumerate() {
            repo.update_broadcast_status(
                &closed,
                broadcast_status,
                Some(proposal_status),
                Some("commit"),
                Some("reveal"),
                None,
            )
            .await
            .unwrap();
            let next = save_approved(&repo, (index + 2) as u64).await;
            claim_broadcast_coordination(
                &repo,
                Authority::StrataAdmin,
                "mock://asm-membership",
                &next,
            )
            .await
            .unwrap();
            assert_eq!(
                repo.find_by_action_id(&next)
                    .await
                    .unwrap()
                    .unwrap()
                    .broadcast_status,
                BroadcastStatus::CommitBroadcasted,
                "{proposal_status} at {broadcast_status} must not block"
            );
            repo.update_broadcast_status(
                &next,
                BroadcastStatus::RevealConfirmed,
                None,
                Some("commit"),
                Some("reveal"),
                None,
            )
            .await
            .unwrap();
        }
    }

    /// Send reopens only when the claim gate would take the row again. Inside the window the
    /// claimer may still be at the device, and any txid means something may be on the network.
    #[tokio::test]
    async fn only_an_empty_claim_past_the_window_reads_as_stale() {
        let repo = new_repo();
        let action_id = save_approved(&repo, 1).await;
        let recent = Utc::now() - chrono::Duration::seconds(30);
        let cases = [
            (
                BroadcastStatus::CommitBroadcasted,
                None,
                Some(stale_claim_time()),
                true,
            ),
            (
                BroadcastStatus::CommitBroadcasted,
                None,
                Some(recent),
                false,
            ),
            (BroadcastStatus::CommitBroadcasted, None, None, false),
            (
                BroadcastStatus::CommitBroadcasted,
                Some("commit"),
                Some(stale_claim_time()),
                false,
            ),
            (
                BroadcastStatus::RevealBroadcasted,
                Some("commit"),
                Some(stale_claim_time()),
                false,
            ),
            (
                BroadcastStatus::Failed,
                None,
                Some(stale_claim_time()),
                false,
            ),
        ];
        for (status, commit, claimed_at, expected) in cases {
            repo.stage_broadcast_claim(&action_id, status, commit, None, claimed_at, None)
                .unwrap();
            let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
            assert_eq!(
                empty_claim_is_stale(&proposal, Utc::now()),
                expected,
                "{status} commit={commit:?} claimed_at={claimed_at:?}"
            );
        }
    }

    #[tokio::test]
    async fn a_finished_or_foreign_or_stale_empty_claim_does_not_block() {
        let repo = new_repo();
        let first = save_approved(&repo, 1).await;
        let second = save_approved(&repo, 2).await;
        claim_broadcast_coordination(
            &repo,
            Authority::StrataAdmin,
            "mock://asm-membership",
            &first,
        )
        .await
        .unwrap();

        repo.stage_broadcast_claim(
            &first,
            BroadcastStatus::CommitBroadcasted,
            None,
            None,
            Some(stale_claim_time()),
            None,
        )
        .unwrap();
        claim_broadcast_coordination(
            &repo,
            Authority::StrataAdmin,
            "mock://asm-membership",
            &second,
        )
        .await
        .unwrap();
        assert_eq!(
            repo.find_by_action_id(&second)
                .await
                .unwrap()
                .unwrap()
                .broadcast_status,
            BroadcastStatus::CommitBroadcasted
        );
        repo.update_broadcast_status(
            &second,
            BroadcastStatus::RevealConfirmed,
            None,
            Some("commit"),
            Some("reveal"),
            None,
        )
        .await
        .unwrap();

        let third = save_approved(&repo, 3).await;
        repo.update_broadcast_status(
            &first,
            BroadcastStatus::Failed,
            None,
            None,
            None,
            Some("dropped"),
        )
        .await
        .unwrap();
        claim_broadcast_coordination(
            &repo,
            Authority::StrataAdmin,
            "mock://asm-membership",
            &third,
        )
        .await
        .unwrap();
        repo.update_broadcast_status(
            &third,
            BroadcastStatus::RevealConfirmed,
            None,
            Some("commit-3"),
            Some("reveal-3"),
            None,
        )
        .await
        .unwrap();

        let fourth = save_approved(&repo, 4).await;
        repo.update_broadcast_status(
            &first,
            BroadcastStatus::RevealConfirmed,
            None,
            Some("commit"),
            Some("reveal"),
            None,
        )
        .await
        .unwrap();
        claim_broadcast_coordination(
            &repo,
            Authority::StrataAdmin,
            "mock://asm-membership",
            &fourth,
        )
        .await
        .unwrap();
        repo.update_broadcast_status(
            &fourth,
            BroadcastStatus::RevealConfirmed,
            None,
            Some("commit-4"),
            Some("reveal-4"),
            None,
        )
        .await
        .unwrap();

        let foreign = Proposal {
            action_id: ActionId("alpen-bundle".to_string()),
            seq_no: 1,
            authority: Authority::AlpenAdmin,
            status: ProposalStatus::Approved,
            required_signatures: 2,
            action_hex: ACTION_HEX.to_string(),
            title: None,
            signatures: Vec::new(),
            broadcast_status: BroadcastStatus::Idle,
            commit_txid: None,
            reveal_txid: None,
            broadcast_error: None,
            target_action_id: None,
            activation_height: None,
            update_id_in_queue: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            broadcast_claimed_at: None,
            broadcast_attempts: Vec::new(),
        };
        repo.save_proposal(foreign).await.unwrap();
        repo.stage_broadcast_claim(
            &first,
            BroadcastStatus::RevealBroadcasted,
            Some("commit"),
            Some("reveal"),
            Some(chrono::Utc::now()),
            None,
        )
        .unwrap();
        claim_broadcast_coordination(
            &repo,
            Authority::AlpenAdmin,
            "mock://asm-membership",
            &ActionId("alpen-bundle".to_string()),
        )
        .await
        .unwrap();
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
        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 3, None)
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

        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2, None)
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

        let created = create_update_action(&repo, session, 1, ACTION_HEX, &sig, 2, None)
            .await
            .unwrap();

        let fetched = get_update_action(&repo, Authority::StrataAdmin, &created.action_id)
            .await
            .unwrap();

        assert_eq!(fetched.action_id, created.action_id);
        assert_eq!(fetched.action_hex, created.action_hex);
        assert_eq!(fetched.seq_no, created.seq_no);
    }

    #[tokio::test]
    async fn test_get_nonexistent_proposal() {
        let repo = new_repo();
        let fake_id = ActionId("nonexistent".to_string());

        let result = get_update_action(&repo, Authority::StrataAdmin, &fake_id).await;

        assert!(matches!(result.unwrap_err(), AppError::NotFound));
    }

    #[tokio::test]
    async fn test_get_update_action_conceals_foreign_authority() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session, 1, ACTION_HEX, &sig, 2, None)
            .await
            .unwrap();

        let result = get_update_action(&repo, Authority::AlpenAdmin, &created.action_id).await;

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

        create_update_action(&repo, session.clone(), 1, "aa", &sig, 2, None)
            .await
            .unwrap();
        create_update_action(&repo, session, 2, "bb", &sig, 2, None)
            .await
            .unwrap();

        let all = list_proposals(&repo, Authority::StrataAdmin, None)
            .await
            .unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_list_proposals_scoped_to_authority() {
        let repo = new_repo();
        let sig = sig_a();
        let strata_session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };
        create_update_action(&repo, strata_session, 1, "aa", &sig, 2, None)
            .await
            .unwrap();

        let alpen_proposal = Proposal {
            action_id: ActionId("alpen_action".to_string()),
            seq_no: 9,
            authority: Authority::AlpenAdmin,
            status: ProposalStatus::Pending,
            required_signatures: 2,
            action_hex: "cc".to_string(),
            title: None,
            signatures: vec![sig.clone()],
            broadcast_status: BroadcastStatus::default(),
            commit_txid: None,
            reveal_txid: None,
            broadcast_error: None,
            target_action_id: None,
            activation_height: None,
            update_id_in_queue: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            broadcast_claimed_at: None,
            broadcast_attempts: Vec::new(),
        };
        repo.save_proposal(alpen_proposal).await.unwrap();

        let strata_only = list_proposals(&repo, Authority::StrataAdmin, None)
            .await
            .unwrap();
        assert_eq!(strata_only.len(), 1);
        assert_eq!(strata_only[0].authority, Authority::StrataAdmin);
    }

    #[tokio::test]
    async fn test_list_proposals_with_status_filter() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        create_update_action(&repo, session.clone(), 1, "aa", &sig, 2, None)
            .await
            .unwrap();
        create_update_action(&repo, session, 2, "bb", &sig, 2, None)
            .await
            .unwrap();

        let pending = list_proposals(&repo, Authority::StrataAdmin, Some(ProposalStatus::Pending))
            .await
            .unwrap();
        assert_eq!(pending.len(), 2);

        let approved = list_proposals(
            &repo,
            Authority::StrataAdmin,
            Some(ProposalStatus::Approved),
        )
        .await
        .unwrap();
        assert_eq!(approved.len(), 0);
    }

    /// AC 10: rotating the council is an administrator proposal even though the action modifies
    /// the council. Authority scoping must hold independently of lifecycle status.
    #[tokio::test]
    async fn test_council_rotation_visibility_is_strata_admin_scoped_for_contract_statuses() {
        let repo = new_repo();
        let action_hex = test_fixture_council_rotation_action_hex();
        let statuses = [
            ProposalStatus::Pending,
            ProposalStatus::Approved,
            ProposalStatus::Enacted,
        ];

        for (index, status) in statuses.into_iter().enumerate() {
            let seq_no = index as u64 + 1;
            let proposal = Proposal {
                action_id: compute_action_id(seq_no, &action_hex).unwrap(),
                seq_no,
                authority: Authority::StrataAdmin,
                status,
                required_signatures: 2,
                action_hex: action_hex.clone(),
                title: None,
                signatures: vec![sig_a()],
                broadcast_status: BroadcastStatus::default(),
                commit_txid: None,
                reveal_txid: None,
                broadcast_error: None,
                target_action_id: None,
                activation_height: None,
                update_id_in_queue: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                broadcast_claimed_at: None,
                broadcast_attempts: Vec::new(),
            };
            repo.save_proposal(proposal.clone()).await.unwrap();

            let council_visible = list_proposals(&repo, Authority::SecurityCouncil, Some(status))
                .await
                .unwrap();
            assert!(
                council_visible.is_empty(),
                "{status} council rotation must be absent from the council list"
            );
            let council_get =
                get_update_action(&repo, Authority::SecurityCouncil, &proposal.action_id).await;
            assert!(
                matches!(council_get.unwrap_err(), AppError::NotFound),
                "{status} council rotation detail must conceal its existence from the council"
            );

            let admin_visible = list_proposals(&repo, Authority::StrataAdmin, Some(status))
                .await
                .unwrap();
            assert_eq!(
                admin_visible.len(),
                1,
                "{status} council rotation must remain visible to Strata Admin"
            );
            assert_eq!(admin_visible[0].action_id, proposal.action_id);
            let admin_get = get_update_action(&repo, Authority::StrataAdmin, &proposal.action_id)
                .await
                .unwrap();
            assert_eq!(admin_get.action_id, proposal.action_id);
            assert_eq!(admin_get.action_hex, action_hex);
            assert_eq!(admin_get.authority, Authority::StrataAdmin);
            assert_eq!(admin_get.status, status);
        }
    }

    #[tokio::test]
    async fn test_broadcast_rejects_threshold_drift() {
        let proposal = Proposal {
            action_id: ActionId("a".to_string()),
            seq_no: 1,
            authority: Authority::StrataAdmin,
            status: ProposalStatus::Approved,
            required_signatures: 99,
            action_hex: ACTION_HEX.to_string(),
            title: None,
            signatures: vec![sig_a()],
            broadcast_status: BroadcastStatus::Idle,
            commit_txid: None,
            reveal_txid: None,
            broadcast_error: None,
            target_action_id: None,
            activation_height: None,
            update_id_in_queue: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            broadcast_claimed_at: None,
            broadcast_attempts: Vec::new(),
        };

        let err = ensure_threshold_snapshot_current(&proposal, "mock://asm-membership")
            .await
            .unwrap_err();

        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_approve_rejects_duplicate_signer_case_insensitive() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2, None)
            .await
            .unwrap();

        let upper_pubkey = sig.signer_pubkey.to_uppercase();
        let second = ProposalSignature {
            signer_pubkey: upper_pubkey,
            signature_hex: "other_sig".to_string(),
        };
        let result = approve_action(&repo, session, &created.action_id, &second).await;

        assert!(matches!(result.unwrap_err(), AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_create_rejects_signer_mismatch_against_session() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: "03aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        };

        let err = create_update_action(&repo, session, 1, ACTION_HEX, &sig, 2, None)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    async fn save_reveal_confirmed_approved(
        repo: &InMemoryProposalRepository,
        action_hex: &str,
    ) -> ActionId {
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };
        let created = create_update_action(repo, session.clone(), 1, action_hex, &sig, 2, None)
            .await
            .unwrap();
        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        approve_action(repo, session_b.clone(), &created.action_id, &sig_b())
            .await
            .unwrap();
        transition_to_approved(repo, session_b, "mock://asm-membership", &created.action_id)
            .await
            .unwrap();
        repo.update_broadcast_status(
            &created.action_id,
            BroadcastStatus::RevealConfirmed,
            None,
            Some(COMMIT_A),
            Some(REVEAL_A),
            None,
        )
        .await
        .unwrap();
        created.action_id
    }

    #[tokio::test]
    async fn report_reveal_confirmed_keeps_proposal_approved() {
        let repo = new_repo();
        let action_id = save_reveal_confirmed_approved(&repo, ACTION_HEX).await;

        let updated = report_broadcast_progress(
            &repo,
            "mock://asm-membership",
            SessionContext {
                authority: Authority::StrataAdmin,
                signer_pubkey: &sig_a().signer_pubkey,
            },
            &action_id,
            &mock_btc(),
            bitcoin::Network::Regtest,
            ReportBroadcastProgressRequest {
                broadcast_status: "reveal_confirmed".to_string(),
                proposal_status: None,
                commit_txid: Some(COMMIT_A.to_string()),
                reveal_txid: Some(REVEAL_A.to_string()),
                broadcast_error: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.status, ProposalStatus::Approved);
        assert_eq!(updated.broadcast_status, BroadcastStatus::RevealConfirmed);
    }

    #[tokio::test]
    async fn report_enacted_rejected_when_asm_not_enacted() {
        let repo = new_repo();
        let action_id = save_reveal_confirmed_approved(&repo, ACTION_HEX).await;

        let err = report_broadcast_progress(
            &repo,
            "mock://asm-membership",
            SessionContext {
                authority: Authority::StrataAdmin,
                signer_pubkey: &sig_a().signer_pubkey,
            },
            &action_id,
            &mock_btc(),
            bitcoin::Network::Regtest,
            ReportBroadcastProgressRequest {
                broadcast_status: "reveal_confirmed".to_string(),
                proposal_status: Some("enacted".to_string()),
                commit_txid: Some(COMMIT_A.to_string()),
                reveal_txid: Some(REVEAL_A.to_string()),
                broadcast_error: None,
            },
        )
        .await
        .unwrap_err();

        assert!(matches!(err, AppError::Conflict(_)));
    }

    /// Setup for the supersession tests: approved, quorum reached, nothing broadcast.
    async fn save_approved(repo: &InMemoryProposalRepository, seq_no: u64) -> ActionId {
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };
        let created = create_update_action(repo, session, seq_no, ACTION_HEX, &sig, 2, None)
            .await
            .unwrap();
        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        approve_action(repo, session_b.clone(), &created.action_id, &sig_b())
            .await
            .unwrap();
        transition_to_approved(repo, session_b, "mock://asm-membership", &created.action_id)
            .await
            .unwrap();
        created.action_id
    }

    /// The chain's sequence number has passed the proposal, so its transaction can never be
    /// accepted — whether or not it was ever broadcast.
    #[tokio::test]
    async fn reconcile_supersedes_a_proposal_the_chain_moved_past() {
        let repo = new_repo();
        let action_id = save_approved(&repo, 1).await;

        reconcile_enacted_for_authority(
            &repo,
            crate::infrastructure::asm_enactment::MOCK_SEQNO_AHEAD_URL,
            &mock_btc(),
            bitcoin::Network::Regtest,
            Authority::StrataAdmin,
        )
        .await
        .unwrap();

        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        assert_eq!(proposal.status, ProposalStatus::Superseded);
    }

    /// A height that failed once at RevealConfirmed must not stay null forever — GET retries it
    /// the same way it retries a missing `update_id_in_queue`.
    #[tokio::test]
    async fn reconcile_reveal_confirmed_facts_fills_a_null_height() {
        let repo = new_repo();
        let action_hex = test_fixture_action_hex();
        // Bypass `record_reveal_confirmed_facts` so the height stays null — the failure case.
        let action_id = save_reveal_confirmed_approved(&repo, &action_hex).await;

        reconcile_reveal_confirmed_facts(&repo, "mock://asm-membership", &mock_btc(), &action_id)
            .await
            .unwrap();

        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        // mock BTC height 800_000 + uniform mock lock period 2016 for strata admin update
        assert_eq!(proposal.activation_height, Some(802_016));
    }

    /// The desktop's confirmation task dies with its send screen. A reveal mined afterwards has to
    /// be noticed here, or the proposal is invisible to both the enactment check and the sweep.
    #[tokio::test]
    async fn reconcile_notices_a_reveal_that_confirmed_unwatched() {
        let repo = new_repo();
        // Seqno 6 is above the fixture's 5, so nothing supersedes it and the promotion stands alone.
        let action_id = save_approved(&repo, 6).await;
        repo.update_broadcast_status(
            &action_id,
            BroadcastStatus::RevealBroadcasted,
            None,
            Some(COMMIT_A),
            Some(REVEAL_A),
            None,
        )
        .await
        .unwrap();

        reconcile_enacted_for_authority(
            &repo,
            crate::infrastructure::asm_enactment::MOCK_SEQNO_AHEAD_URL,
            &mock_btc(),
            bitcoin::Network::Regtest,
            Authority::StrataAdmin,
        )
        .await
        .unwrap();

        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        assert_eq!(proposal.broadcast_status, BroadcastStatus::RevealConfirmed);
        assert_eq!(proposal.status, ProposalStatus::Approved);
    }

    /// #516: the desktop pre-registers both txids at `commit_broadcasted` before it broadcasts. If
    /// its later reports never land, the row stays there — so a mined reveal must be noticed from
    /// the earlier sub-statuses too, and the enactment check must then run on it.
    #[tokio::test]
    async fn reconcile_notices_a_mined_reveal_from_a_row_that_never_reached_reveal_broadcasted() {
        for status in [
            BroadcastStatus::CommitBroadcasted,
            BroadcastStatus::CommitConfirmed,
        ] {
            let repo = new_repo();
            let action_id = save_approved(&repo, 1).await;
            repo.update_broadcast_status(
                &action_id,
                status,
                None,
                Some(COMMIT_A),
                Some(REVEAL_A),
                None,
            )
            .await
            .unwrap();

            reconcile_enacted_for_authority(
                &repo,
                crate::infrastructure::asm_enactment::MOCK_ENACTED_AHEAD_URL,
                &mock_btc(),
                bitcoin::Network::Regtest,
                Authority::StrataAdmin,
            )
            .await
            .unwrap();

            let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
            assert_eq!(
                proposal.broadcast_status,
                BroadcastStatus::RevealConfirmed,
                "from {status}"
            );
            assert_eq!(proposal.status, ProposalStatus::Enacted, "from {status}");
        }
    }

    /// #516: a desktop may report `failed` for a bundle that later lands — a report it could not
    /// take back, or a node that answered "not found" for a reveal still propagating. The row keeps
    /// its reveal txid, so a mined reveal promotes it like any other sub-status. Otherwise the sweep
    /// would retire it as Superseded while it was its own reveal that consumed the seqno.
    #[tokio::test]
    async fn reconcile_promotes_a_failed_row_whose_reveal_was_mined() {
        let repo = new_repo();
        let action_id = save_approved(&repo, 1).await;
        repo.update_broadcast_status(
            &action_id,
            BroadcastStatus::Failed,
            None,
            Some(COMMIT_A),
            Some(REVEAL_A),
            Some("dropped: the bundle is no longer in the mempool or the chain"),
        )
        .await
        .unwrap();

        reconcile_enacted_for_authority(
            &repo,
            crate::infrastructure::asm_enactment::MOCK_ENACTED_AHEAD_URL,
            &mock_btc(),
            bitcoin::Network::Regtest,
            Authority::StrataAdmin,
        )
        .await
        .unwrap();

        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        assert_eq!(
            (proposal.broadcast_status, proposal.status),
            (BroadcastStatus::RevealConfirmed, ProposalStatus::Enacted),
            "a mined reveal wins over a mistaken `failed`, and the proposal is never Superseded"
        );
        assert_eq!(proposal.broadcast_error, None);
    }

    /// Upstream refuses `payload.seqno <= last_seqno`, so a proposal whose seqno *equals* the
    /// role's is as dead as one below it.
    #[tokio::test]
    async fn reconcile_supersedes_a_proposal_whose_seq_no_equals_the_role() {
        let repo = new_repo();
        let action_id = save_approved(&repo, 5).await;

        reconcile_enacted_for_authority(
            &repo,
            crate::infrastructure::asm_enactment::MOCK_SEQNO_AHEAD_URL,
            &mock_btc(),
            bitcoin::Network::Regtest,
            Authority::StrataAdmin,
        )
        .await
        .unwrap();

        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        assert_eq!(proposal.status, ProposalStatus::Superseded);
    }

    /// Enactment is decided first, so a proposal that did enact is never swept even though the
    /// same chain state would supersede it.
    #[tokio::test]
    async fn reconcile_prefers_enactment_over_supersession() {
        let repo = new_repo();
        let action_id = save_reveal_confirmed_approved(&repo, ACTION_HEX).await;

        reconcile_enacted_for_authority(
            &repo,
            crate::infrastructure::asm_enactment::MOCK_ENACTED_AHEAD_URL,
            &mock_btc(),
            bitcoin::Network::Regtest,
            Authority::StrataAdmin,
        )
        .await
        .unwrap();

        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        assert_eq!(proposal.status, ProposalStatus::Enacted);
    }

    #[tokio::test]
    async fn reconcile_promotes_reveal_confirmed_when_asm_enacted() {
        let repo = new_repo();
        let action_id = save_reveal_confirmed_approved(&repo, ACTION_HEX).await;

        reconcile_enacted_for_authority(
            &repo,
            "mock://asm-enacted",
            &mock_btc(),
            bitcoin::Network::Regtest,
            Authority::StrataAdmin,
        )
        .await
        .unwrap();

        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        assert_eq!(proposal.status, ProposalStatus::Enacted);
    }

    #[tokio::test]
    async fn reconcile_marks_target_canceled_when_cancel_enacted() {
        let repo = new_repo();
        let target = save_approved_proposal(&repo, Authority::StrataAdmin, 1).await;

        let cancel = create_cancel_proposal(
            &repo,
            "mock://asm-membership",
            SessionContext {
                authority: Authority::StrataAdmin,
                signer_pubkey: &sig_a().signer_pubkey,
            },
            target.action_id.clone(),
            2,
            "cafebabe",
            "cancel_sig",
        )
        .await
        .unwrap();

        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        approve_action(&repo, session_b.clone(), &cancel.action_id, &sig_b())
            .await
            .unwrap();
        transition_to_approved(&repo, session_b, "mock://asm-membership", &cancel.action_id)
            .await
            .unwrap();
        repo.update_broadcast_status(
            &cancel.action_id,
            BroadcastStatus::RevealConfirmed,
            None,
            Some("commit"),
            Some("reveal"),
            None,
        )
        .await
        .unwrap();

        reconcile_enacted_for_authority(
            &repo,
            "mock://asm-enacted",
            &mock_btc(),
            bitcoin::Network::Regtest,
            Authority::StrataAdmin,
        )
        .await
        .unwrap();

        let cancel = repo
            .find_by_action_id(&cancel.action_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(cancel.status, ProposalStatus::Enacted);

        let target = repo
            .find_by_action_id(&target.action_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(target.status, ProposalStatus::Canceled);
    }

    /// A cancel that reached the chain takes its target out of the ASM queue while the target's
    /// seqno is already consumed — the exact shape the supersession sweep reads as "dead". The
    /// target belongs to the cancel: it must stay put until the cancel's own enactment writes
    /// `Canceled`, or a cancelled Defcon 3 would end on `Superseded` and its cancel on `Expired`.
    /// See docs/specs/security-council-defcon-3.md Constraint 3 and AC 12.
    #[tokio::test]
    async fn reconcile_leaves_a_target_alone_once_its_cancel_reached_the_chain() {
        let repo = new_repo();
        let target = save_approved_proposal(&repo, Authority::StrataAdmin, 1).await;
        repo.update_broadcast_status(
            &target.action_id,
            BroadcastStatus::RevealConfirmed,
            None,
            Some("commit"),
            Some("reveal"),
            None,
        )
        .await
        .unwrap();

        let cancel = create_cancel_proposal(
            &repo,
            "mock://asm-membership",
            SessionContext {
                authority: Authority::StrataAdmin,
                signer_pubkey: &sig_a().signer_pubkey,
            },
            target.action_id.clone(),
            2,
            "cafebabe",
            "cancel_sig",
        )
        .await
        .unwrap();

        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        approve_action(&repo, session_b.clone(), &cancel.action_id, &sig_b())
            .await
            .unwrap();
        transition_to_approved(&repo, session_b, "mock://asm-membership", &cancel.action_id)
            .await
            .unwrap();
        repo.update_broadcast_status(
            &cancel.action_id,
            BroadcastStatus::RevealConfirmed,
            None,
            Some("commit"),
            Some("reveal"),
            None,
        )
        .await
        .unwrap();

        // A chain that has moved past both seqnos and shows neither in the queue: without the
        // guard this is what supersedes the target.
        reconcile_enacted_for_authority(
            &repo,
            crate::infrastructure::asm_enactment::MOCK_SEQNO_AHEAD_URL,
            &mock_btc(),
            bitcoin::Network::Regtest,
            Authority::StrataAdmin,
        )
        .await
        .unwrap();

        let target = repo
            .find_by_action_id(&target.action_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(target.status, ProposalStatus::Approved);
    }

    #[tokio::test]
    async fn reconcile_marks_cancel_expired_when_target_already_enacted() {
        let repo = new_repo();
        let target = save_approved_proposal(&repo, Authority::StrataAdmin, 1).await;

        let cancel = create_cancel_proposal(
            &repo,
            "mock://asm-membership",
            SessionContext {
                authority: Authority::StrataAdmin,
                signer_pubkey: &sig_a().signer_pubkey,
            },
            target.action_id.clone(),
            2,
            "cafebabe",
            "cancel_sig",
        )
        .await
        .unwrap();

        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        approve_action(&repo, session_b.clone(), &cancel.action_id, &sig_b())
            .await
            .unwrap();
        transition_to_approved(
            &repo,
            session_b.clone(),
            "mock://asm-membership",
            &cancel.action_id,
        )
        .await
        .unwrap();

        // Target reaches RevealConfirmed and gets enacted normally before the cancel does.
        repo.update_broadcast_status(
            &target.action_id,
            BroadcastStatus::RevealConfirmed,
            None,
            Some("commit"),
            Some("reveal"),
            None,
        )
        .await
        .unwrap();
        reconcile_enacted_for_authority(
            &repo,
            "mock://asm-enacted",
            &mock_btc(),
            bitcoin::Network::Regtest,
            Authority::StrataAdmin,
        )
        .await
        .unwrap();

        let target = repo
            .find_by_action_id(&target.action_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(target.status, ProposalStatus::Enacted);

        // The cancel reaches RevealConfirmed afterwards, but its target is no longer Approved —
        // the ASM would have rejected the cancel tx, so it should be marked Expired without
        // touching the already-enacted target.
        repo.update_broadcast_status(
            &cancel.action_id,
            BroadcastStatus::RevealConfirmed,
            None,
            Some("commit"),
            Some("reveal"),
            None,
        )
        .await
        .unwrap();
        reconcile_enacted_for_authority(
            &repo,
            "mock://asm-enacted",
            &mock_btc(),
            bitcoin::Network::Regtest,
            Authority::StrataAdmin,
        )
        .await
        .unwrap();

        let cancel = repo
            .find_by_action_id(&cancel.action_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(cancel.status, ProposalStatus::Expired);

        let target = repo
            .find_by_action_id(&target.action_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(target.status, ProposalStatus::Enacted);
    }

    // ---------------------------------------------------------------------------
    // create_cancel_proposal tests
    // ---------------------------------------------------------------------------

    /// An approved target for the cancel tests, with its reveal confirmed: a cancel names the
    /// update in the ASM queue, so only a queued update can be one. Its `action_hex` must decode:
    /// the cancel gate resolves the target's confirmation depth from it.
    async fn save_approved_proposal(
        repo: &InMemoryProposalRepository,
        authority: Authority,
        seq_no: SeqNo,
    ) -> Proposal {
        let action_hex = test_fixture_action_hex();
        save_approved_proposal_with_action(repo, authority, seq_no, &action_hex).await
    }

    async fn save_approved_proposal_with_action(
        repo: &InMemoryProposalRepository,
        authority: Authority,
        seq_no: SeqNo,
        action_hex: &str,
    ) -> Proposal {
        let sig = sig_a();
        let session = SessionContext {
            authority,
            signer_pubkey: &sig.signer_pubkey,
        };
        let created =
            create_update_action(repo, session.clone(), seq_no, action_hex, &sig, 2, None)
                .await
                .unwrap();
        let session_b = SessionContext {
            authority,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        approve_action(repo, session_b.clone(), &created.action_id, &sig_b())
            .await
            .unwrap();
        transition_to_approved(repo, session_b, "mock://asm-membership", &created.action_id)
            .await
            .unwrap();
        repo.update_broadcast_status(
            &created.action_id,
            BroadcastStatus::RevealConfirmed,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap()
        .unwrap()
    }

    #[tokio::test]
    async fn test_create_cancel_proposal_happy_path() {
        let repo = new_repo();
        // StrataAdmin has a mock threshold of 2; use it for cancel creation
        let target = save_approved_proposal(&repo, Authority::StrataAdmin, 1).await;

        let cancel = create_cancel_proposal(
            &repo,
            "mock://asm-membership",
            SessionContext {
                authority: Authority::StrataAdmin,
                signer_pubkey: &sig_a().signer_pubkey,
            },
            target.action_id.clone(),
            2,
            "cafebabe",
            "cancel_sig",
        )
        .await
        .unwrap();

        assert_eq!(cancel.status, ProposalStatus::Pending);
        assert_eq!(cancel.authority, Authority::StrataAdmin);
        assert_eq!(cancel.target_action_id, Some(target.action_id));
        assert_eq!(cancel.signatures.len(), 1);
        assert_eq!(cancel.action_hex, "cafebabe");
    }

    #[tokio::test]
    async fn test_create_cancel_proposal_idempotent() {
        let repo = new_repo();
        let target = save_approved_proposal(&repo, Authority::StrataAdmin, 1).await;

        let first = create_cancel_proposal(
            &repo,
            "mock://asm-membership",
            SessionContext {
                authority: Authority::StrataAdmin,
                signer_pubkey: &sig_a().signer_pubkey,
            },
            target.action_id.clone(),
            2,
            "cafebabe",
            "cancel_sig",
        )
        .await
        .unwrap();

        let second = create_cancel_proposal(
            &repo,
            "mock://asm-membership",
            SessionContext {
                authority: Authority::StrataAdmin,
                signer_pubkey: &sig_b().signer_pubkey,
            },
            target.action_id.clone(),
            2,
            "cafebabe",
            "cancel_sig_2",
        )
        .await
        .unwrap();

        assert_eq!(first.action_id, second.action_id);
        assert_eq!(
            second.signatures.len(),
            1,
            "idempotent: no new sig appended"
        );
    }

    #[tokio::test]
    async fn test_create_cancel_proposal_rejects_non_approved_target() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };
        let pending = create_update_action(&repo, session, 1, ACTION_HEX, &sig, 2, None)
            .await
            .unwrap();

        let err = create_cancel_proposal(
            &repo,
            "mock://asm-membership",
            SessionContext {
                authority: Authority::StrataAdmin,
                signer_pubkey: &sig_a().signer_pubkey,
            },
            pending.action_id,
            2,
            "cafebabe",
            "cancel_sig",
        )
        .await
        .unwrap_err();

        assert!(matches!(err, AppError::BadRequest(_)));
    }

    /// #562: an approved update is not in the ASM queue until its reveal confirms, and a cancel
    /// names it there. Before that — not sent, in the mempool, or failed — there is nothing to
    /// cancel, and a refused cancel must leave nothing behind.
    #[tokio::test]
    async fn test_create_cancel_proposal_rejects_target_whose_reveal_has_not_confirmed() {
        for broadcast_status in [
            BroadcastStatus::Idle,
            BroadcastStatus::RevealBroadcasted,
            BroadcastStatus::Failed,
        ] {
            let repo = new_repo();
            let target = save_approved_proposal(&repo, Authority::StrataAdmin, 1).await;
            repo.update_broadcast_status(
                &target.action_id,
                broadcast_status,
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();

            let err = create_cancel_proposal(
                &repo,
                "mock://asm-membership",
                SessionContext {
                    authority: Authority::StrataAdmin,
                    signer_pubkey: &sig_a().signer_pubkey,
                },
                target.action_id.clone(),
                2,
                "cafebabe",
                "cancel_sig",
            )
            .await
            .unwrap_err();

            let AppError::BadRequest(message) = err else {
                panic!("expected BadRequest for {broadcast_status}");
            };
            assert!(
                message.contains("reveal"),
                "the rejection must name the reveal: {message}"
            );
            assert!(repo
                .find_cancel_for_target(&target.action_id)
                .await
                .unwrap()
                .is_none());
        }
    }

    /// Tx types 15 (council rotation) and 14 (safe harbor address) reach into the council's world
    /// but belong to Strata Admin, and so do their cancels. Nothing in the cancel path knows about
    /// either type: this asserts the generic path lands on the right answer in both directions.
    /// The council fires the sweep and must not be able to stand down a change to who sits on it
    /// or where the sweep lands — and a refused cancel must leave nothing behind.
    #[tokio::test]
    async fn test_strata_admin_rotations_are_cancelled_only_by_the_strata_administrator() {
        for action_hex in [
            test_fixture_council_rotation_action_hex(),
            test_fixture_safe_harbor_address_action_hex(),
        ] {
            let repo = new_repo();
            let target =
                save_approved_proposal_with_action(&repo, Authority::StrataAdmin, 1, &action_hex)
                    .await;
            let signer = sig_a();
            let cancel_as = |authority| {
                create_cancel_proposal(
                    &repo,
                    "mock://asm-membership",
                    SessionContext {
                        authority,
                        signer_pubkey: &signer.signer_pubkey,
                    },
                    target.action_id.clone(),
                    2,
                    "cafebabe",
                    "cancel_sig",
                )
            };

            let err = cancel_as(Authority::SecurityCouncil).await.unwrap_err();
            assert!(matches!(err, AppError::Unauthorized), "{err:?}");
            assert!(
                repo.find_cancel_for_target(&target.action_id)
                    .await
                    .unwrap()
                    .is_none(),
                "a refused cancel must leave nothing behind"
            );

            let cancel = cancel_as(Authority::StrataAdmin).await.unwrap();
            assert_eq!(cancel.authority, Authority::StrataAdmin);
            assert_eq!(cancel.target_action_id, Some(target.action_id.clone()));
        }
    }

    /// AC 11: the gate is the action's confirmation depth, and the rejection names it. Defcon 1 is
    /// the zero-depth case — never enqueued, so an on-chain cancel would fail with `UnknownAction`.
    #[tokio::test]
    async fn test_create_cancel_proposal_rejects_zero_depth_action() {
        let repo = new_repo();
        let target = save_approved_proposal_with_action(
            &repo,
            Authority::SecurityCouncil,
            1,
            &test_fixture_defcon_1_action_hex(),
        )
        .await;

        let err = create_cancel_proposal(
            &repo,
            "mock://asm-membership",
            SessionContext {
                authority: Authority::SecurityCouncil,
                signer_pubkey: &sig_a().signer_pubkey,
            },
            target.action_id,
            99,
            "cafebabe",
            "cancel_sig",
        )
        .await
        .unwrap_err();

        let AppError::BadRequest(message) = err else {
            panic!("expected BadRequest");
        };
        assert!(
            message.contains("depth") && message.contains('0'),
            "the rejection must name the depth, not the authority: {message}"
        );
    }

    /// P-032 (race): concurrent duplicate approve results in exactly one signature stored.
    #[tokio::test]
    async fn test_concurrent_duplicate_approve_single_signature() {
        use std::sync::Arc;

        let repo = Arc::new(InMemoryProposalRepository::new());
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&*repo, session, 1, ACTION_HEX, &sig, 2, None)
            .await
            .unwrap();
        let action_id = created.action_id.clone();

        // Two signers: one is sig_b (first signer was sig_a who created the proposal).
        // We attempt two concurrent approvals from sig_b — only one should land.
        let sig_b = sig_b();
        let dup_sig = ProposalSignature {
            signer_pubkey: sig_b.signer_pubkey.clone(),
            signature_hex: "sig_b_dup".to_string(),
        };

        let repo1 = Arc::clone(&repo);
        let repo2 = Arc::clone(&repo);
        let action_id1 = action_id.clone();
        let action_id2 = action_id.clone();
        let session1 = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b.signer_pubkey,
        };
        let session2 = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &dup_sig.signer_pubkey,
        };

        let (r1, r2) = tokio::join!(
            approve_action(&*repo1, session1, &action_id1, &sig_b),
            approve_action(&*repo2, session2, &action_id2, &dup_sig),
        );

        let successes = [r1.is_ok(), r2.is_ok()].iter().filter(|&&ok| ok).count();
        let conflicts = [r1.is_err(), r2.is_err()]
            .iter()
            .filter(|&&err| err)
            .count();
        assert_eq!(successes, 1, "exactly one approve should succeed");
        assert_eq!(
            conflicts, 1,
            "exactly one approve should fail with conflict"
        );

        let final_proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        let b_sigs: Vec<_> = final_proposal
            .signatures
            .iter()
            .filter(|s| s.signer_pubkey.eq_ignore_ascii_case(&sig_b.signer_pubkey))
            .collect();
        assert_eq!(
            b_sigs.len(),
            1,
            "only one signature from sig_b must be stored"
        );
    }

    // ── #605: another signer's commit + reveal pair ─────────────────────────

    const COMMIT_A: &str = "aa00000000000000000000000000000000000000000000000000000000000000";
    const REVEAL_A: &str = "aa11111111111111111111111111111111111111111111111111111111111111";
    const COMMIT_B: &str = "bb00000000000000000000000000000000000000000000000000000000000000";
    const REVEAL_B: &str = "bb11111111111111111111111111111111111111111111111111111111111111";

    /// Mines only the txids it is given, each at its own height.
    struct MinedTxids(std::collections::HashMap<&'static str, u64>);

    #[async_trait::async_trait]
    impl BitcoinRpcClient for MinedTxids {
        async fn estimate_fee_rate_sats_per_vb(
            &self,
            _target_blocks: u16,
        ) -> Result<u64, AppError> {
            Ok(1)
        }

        async fn get_block_height_for_txid(&self, txid: &str) -> Result<u64, AppError> {
            self.0.get(txid).copied().ok_or_else(|| {
                AppError::BadRequest(format!(
                    "getrawtransaction: tx {txid} has no blockhash — not yet confirmed"
                ))
            })
        }

        async fn get_chain_tip(&self) -> Result<u64, AppError> {
            Ok(800_000)
        }
    }

    fn session_of(sig: &ProposalSignature) -> SessionContext<'_> {
        SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        }
    }

    fn report(
        status: &str,
        commit: Option<&str>,
        reveal: Option<&str>,
    ) -> ReportBroadcastProgressRequest {
        ReportBroadcastProgressRequest {
            broadcast_status: status.to_string(),
            proposal_status: None,
            commit_txid: commit.map(str::to_string),
            reveal_txid: reveal.map(str::to_string),
            broadcast_error: None,
        }
    }

    /// Signer A claimed, pre-registered its pair and sent it: the row is `reveal_broadcasted`.
    async fn save_with_pair_of_a(repo: &InMemoryProposalRepository) -> ActionId {
        save_with_pair(repo, 1, COMMIT_A, REVEAL_A).await
    }

    /// `save_with_pair_of_a` with signer A's pair given, at this sequence number.
    async fn save_with_pair(
        repo: &InMemoryProposalRepository,
        seq_no: u64,
        commit: &str,
        reveal: &str,
    ) -> ActionId {
        // A decodable action, so the confirmed reveal yields an activation height.
        let created = create_update_action(
            repo,
            session_of(&sig_a()),
            seq_no,
            &test_fixture_action_hex(),
            &sig_a(),
            2,
            None,
        )
        .await
        .unwrap();
        let action_id = created.action_id;
        approve_action(repo, session_of(&sig_b()), &action_id, &sig_b())
            .await
            .unwrap();
        transition_to_approved(
            repo,
            session_of(&sig_b()),
            "mock://asm-membership",
            &action_id,
        )
        .await
        .unwrap();
        claim_broadcast_coordination(
            repo,
            Authority::StrataAdmin,
            "mock://asm-membership",
            &action_id,
        )
        .await
        .unwrap();
        for status in ["commit_broadcasted", "reveal_broadcasted"] {
            report_broadcast_progress(
                repo,
                "mock://asm-membership",
                session_of(&sig_a()),
                &action_id,
                &mock_btc(),
                bitcoin::Network::Regtest,
                report(status, Some(commit), Some(reveal)),
            )
            .await
            .unwrap();
        }
        action_id
    }

    async fn register_pair_of_b(
        repo: &InMemoryProposalRepository,
        action_id: &ActionId,
    ) -> Result<Proposal, AppError> {
        register_additional_pair(
            repo,
            session_of(&sig_b()),
            "mock://asm-membership",
            &NodeView::default(),
            action_id,
            COMMIT_B,
            REVEAL_B,
        )
        .await
    }

    #[tokio::test]
    async fn the_claimers_pair_is_kept_with_its_broadcaster() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;

        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        assert_eq!(
            proposal.broadcast_attempts.len(),
            1,
            "one pair, reported twice"
        );
        let attempt = &proposal.broadcast_attempts[0];
        assert_eq!(
            attempt.broadcaster_pubkey.as_deref(),
            Some(sig_a().signer_pubkey.as_str())
        );
        assert_eq!(
            (attempt.commit_txid.as_str(), attempt.reveal_txid.as_str()),
            (COMMIT_A, REVEAL_A)
        );
    }

    #[tokio::test]
    async fn another_signer_sends_their_own_pair_beside_one_in_flight() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;

        let proposal = register_pair_of_b(&repo, &action_id).await.unwrap();

        assert_eq!(
            proposal.broadcast_status,
            BroadcastStatus::CommitBroadcasted
        );
        assert_eq!(proposal.commit_txid.as_deref(), Some(COMMIT_B));
        assert_eq!(proposal.reveal_txid.as_deref(), Some(REVEAL_B));
        let reveals: Vec<&str> = proposal
            .broadcast_attempts
            .iter()
            .map(|attempt| attempt.reveal_txid.as_str())
            .collect();
        assert_eq!(reveals, [REVEAL_A, REVEAL_B], "the first pair stays listed");
    }

    #[tokio::test]
    async fn a_signer_who_already_sent_a_pair_is_refused() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        // The earlier pair is still in the mempool, so it is not abandoned. The new reveal is not
        // in a block: `mock_btc` would refuse it as already mined.
        let still_live = NodeView {
            mempool: vec![COMMIT_A, COMMIT_B],
            ..NodeView::default()
        };

        let err = register_additional_pair(
            &repo,
            session_of(&sig_a()),
            "mock://asm-membership",
            &still_live,
            &action_id,
            COMMIT_B,
            REVEAL_B,
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, AppError::Conflict(ref m) if m == OWN_PAIR_EXISTS),
            "{err:?}"
        );

        register_pair_of_b(&repo, &action_id).await.unwrap();
        let err = register_additional_pair(
            &repo,
            session_of(&sig_b()),
            "mock://asm-membership",
            &still_live,
            &action_id,
            COMMIT_C,
            REVEAL_C,
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, AppError::Conflict(ref m) if m == OWN_PAIR_EXISTS),
            "a second pair of their own: {err:?}"
        );
    }

    /// A registration whose response was lost: the same signer sends the same pair again and gets
    /// the row back, unchanged, so the desktop can go on to broadcast it.
    #[tokio::test]
    async fn the_same_pair_registered_again_is_accepted_unchanged() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        let first = register_pair_of_b(&repo, &action_id).await.unwrap();

        let again = register_pair_of_b(&repo, &action_id).await.unwrap();

        assert_eq!(again.reveal_txid.as_deref(), Some(REVEAL_B));
        assert_eq!(again.broadcast_attempts, first.broadcast_attempts);
        assert_eq!(again.broadcast_claimed_at, first.broadcast_claimed_at);
    }

    /// Nothing on its way means nothing to send beside: `idle` and `failed` go through the claim,
    /// and an empty claim is a signer still at the device.
    #[tokio::test]
    async fn a_pair_is_refused_without_one_in_flight() {
        let repo = new_repo();
        let action_id = save_approved(&repo, 1).await;
        let err = register_pair_of_b(&repo, &action_id).await.unwrap_err();
        assert!(
            matches!(err, AppError::Conflict(ref m) if m == NO_PAIR_IN_FLIGHT),
            "idle: {err:?}"
        );

        claim_broadcast_coordination(
            &repo,
            Authority::StrataAdmin,
            "mock://asm-membership",
            &action_id,
        )
        .await
        .unwrap();
        let err = register_pair_of_b(&repo, &action_id).await.unwrap_err();
        assert!(
            matches!(err, AppError::Conflict(ref m) if m == NO_PAIR_IN_FLIGHT),
            "empty claim: {err:?}"
        );

        repo.stage_broadcast_claim(
            &action_id,
            BroadcastStatus::Failed,
            Some(COMMIT_A),
            Some(REVEAL_A),
            None,
            Some("rejected"),
        )
        .unwrap();
        let err = register_pair_of_b(&repo, &action_id).await.unwrap_err();
        assert!(
            matches!(err, AppError::Conflict(ref m) if m == NO_PAIR_IN_FLIGHT),
            "failed: {err:?}"
        );
    }

    #[tokio::test]
    async fn a_pair_needs_real_txids() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        let err = register_additional_pair(
            &repo,
            session_of(&sig_b()),
            "mock://asm-membership",
            &mock_btc(),
            &action_id,
            "commit",
            REVEAL_B,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)), "{err:?}");
    }

    /// The first pair's bundle is still being watched by its own desktop. Once a later pair is the
    /// one the row tracks, that desktop's `failed` must not mark the new pair failed.
    #[tokio::test]
    async fn a_report_about_a_replaced_pair_changes_nothing() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();

        let mut stale = report("failed", Some(COMMIT_A), Some(REVEAL_A));
        stale.broadcast_error = Some(DROPPED_FOR_TEST.to_string());
        let after = report_broadcast_progress(
            &repo,
            "mock://asm-membership",
            session_of(&sig_a()),
            &action_id,
            &mock_btc(),
            bitcoin::Network::Regtest,
            stale,
        )
        .await
        .unwrap();

        assert_eq!(after.broadcast_status, BroadcastStatus::CommitBroadcasted);
        assert_eq!(after.reveal_txid.as_deref(), Some(REVEAL_B));
        assert!(after.broadcast_error.is_none());
    }

    /// The race the read-time guard cannot close: the report read the row while it still tracked
    /// the first pair, and the registration commits before the report writes. The write itself
    /// must leave the new pair alone.
    #[tokio::test]
    async fn a_report_written_after_a_registration_it_did_not_see_changes_nothing() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();

        let after = repo
            .update_reported_pair_status(
                &action_id,
                BroadcastStatus::Failed,
                None,
                Some(COMMIT_A),
                Some(&REVEAL_A.to_uppercase()),
                Some(DROPPED_FOR_TEST),
                None,
                &[],
            )
            .await
            .unwrap()
            .unwrap();

        assert_eq!(after.broadcast_status, BroadcastStatus::CommitBroadcasted);
        assert_eq!(after.commit_txid.as_deref(), Some(COMMIT_B));
        assert_eq!(after.reveal_txid.as_deref(), Some(REVEAL_B));
        assert!(after.broadcast_error.is_none());
    }

    /// The import read an empty claim, then the claimer's pre-registration tagged the row before
    /// the import wrote. The write must not list the imported reveal: it may belong to another
    /// action, and the reconcile would confirm this proposal on it (#605).
    #[tokio::test]
    async fn a_report_the_write_rejects_is_not_recorded() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();

        let after = repo
            .update_reported_pair_status(
                &action_id,
                BroadcastStatus::CommitBroadcasted,
                None,
                Some(COMMIT_C),
                Some(REVEAL_C),
                None,
                Some(BroadcastAttempt {
                    broadcaster_pubkey: Some(sig_a().signer_pubkey),
                    commit_txid: COMMIT_C.to_string(),
                    reveal_txid: REVEAL_C.to_string(),
                    created_at: Utc::now(),
                }),
                &[],
            )
            .await
            .unwrap()
            .unwrap();

        assert_eq!(after.broadcast_status, BroadcastStatus::CommitBroadcasted);
        assert_eq!(after.reveal_txid.as_deref(), Some(REVEAL_B));
        let reveals: Vec<&str> = after
            .broadcast_attempts
            .iter()
            .map(|attempt| attempt.reveal_txid.as_str())
            .collect();
        assert_eq!(reveals, [REVEAL_A, REVEAL_B]);
    }

    const DROPPED_FOR_TEST: &str = "dropped";

    /// The first pair was mined after all: that is the one the ASM acted on, so it becomes the
    /// row's pair and the activation height is read from its block.
    #[tokio::test]
    async fn an_earlier_pair_mined_first_becomes_the_rows_pair() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();
        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        let btc = MinedTxids([(REVEAL_A, 800_010)].into_iter().collect());

        let confirmed = confirm_reveal_if_mined(
            &repo,
            proposal,
            &btc,
            bitcoin::Network::Regtest,
            "mock://asm-membership",
        )
        .await;

        assert_eq!(confirmed.broadcast_status, BroadcastStatus::RevealConfirmed);
        assert_eq!(confirmed.commit_txid.as_deref(), Some(COMMIT_A));
        assert_eq!(confirmed.reveal_txid.as_deref(), Some(REVEAL_A));
        let stored = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        // the reveal's block + the uniform mock lock period 2016
        assert_eq!(stored.activation_height, Some(802_026));
    }

    #[tokio::test]
    async fn of_two_mined_pairs_the_lower_block_wins() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();
        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        let btc = MinedTxids(
            [(REVEAL_A, 800_011), (REVEAL_B, 800_010)]
                .into_iter()
                .collect(),
        );

        let confirmed = confirm_reveal_if_mined(
            &repo,
            proposal,
            &btc,
            bitcoin::Network::Regtest,
            "mock://asm-membership",
        )
        .await;

        assert_eq!(
            confirmed.reveal_txid.as_deref(),
            Some(REVEAL_B),
            "the row's own pair, mined first"
        );
        let stored = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        assert_eq!(stored.activation_height, Some(802_026));
    }

    #[tokio::test]
    async fn no_mined_pair_leaves_the_row_in_flight() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();
        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        let btc = MinedTxids(std::collections::HashMap::new());

        let unchanged = confirm_reveal_if_mined(
            &repo,
            proposal,
            &btc,
            bitcoin::Network::Regtest,
            "mock://asm-membership",
        )
        .await;

        assert_eq!(
            unchanged.broadcast_status,
            BroadcastStatus::CommitBroadcasted
        );
        assert_eq!(unchanged.reveal_txid.as_deref(), Some(REVEAL_B));
    }

    const COMMIT_C: &str = "cc00000000000000000000000000000000000000000000000000000000000000";
    const REVEAL_C: &str = "cc11111111111111111111111111111111111111111111111111111111111111";

    /// A confirmation decided while the row tracked A must not land once the row tracks B. The
    /// next read sees both pairs and decides again (#605).
    #[tokio::test]
    async fn a_confirmation_does_not_apply_once_the_row_tracks_another_pair() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();

        let applied = repo
            .confirm_tracked_reveal(&action_id, Some(REVEAL_A), COMMIT_A, REVEAL_A)
            .await
            .unwrap();

        assert!(
            applied.is_none(),
            "the observed reveal is no longer the row's"
        );
        let stored = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        assert_eq!(stored.broadcast_status, BroadcastStatus::CommitBroadcasted);
        assert_eq!(stored.reveal_txid.as_deref(), Some(REVEAL_B));
        assert!(stored.activation_height.is_none());
    }

    /// The import screen reports any bundle against the open proposal, so an untracked pair may
    /// belong to another action. Recording it would let the reconcile confirm this proposal on a
    /// reveal mined for that other action.
    #[tokio::test]
    async fn a_report_about_an_untracked_pair_changes_nothing() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();

        let after = report_broadcast_progress(
            &repo,
            "mock://asm-membership",
            session_of(&sig_a()),
            &action_id,
            &mock_btc(),
            bitcoin::Network::Regtest,
            report("reveal_confirmed", Some(COMMIT_C), Some(REVEAL_C)),
        )
        .await
        .unwrap();
        assert_eq!(after.broadcast_status, BroadcastStatus::CommitBroadcasted);
        assert_eq!(after.reveal_txid.as_deref(), Some(REVEAL_B));
        assert_eq!(
            after.broadcast_attempts.len(),
            2,
            "the pair is not recorded"
        );

        let btc = MinedTxids([(REVEAL_C, 800_005)].into_iter().collect());
        let reconciled = confirm_reveal_if_mined(
            &repo,
            after,
            &btc,
            bitcoin::Network::Regtest,
            "mock://asm-membership",
        )
        .await;
        assert_eq!(
            reconciled.broadcast_status,
            BroadcastStatus::CommitBroadcasted
        );
    }

    /// The desktop reports `reveal_confirmed` for the pair the row tracks, but an earlier pair
    /// was mined in a lower block: that one is what the ASM acted on (#605).
    #[tokio::test]
    async fn a_confirmed_report_yields_to_a_pair_mined_earlier() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();
        let btc = MinedTxids(
            [(REVEAL_A, 800_010), (REVEAL_B, 800_011)]
                .into_iter()
                .collect(),
        );

        let confirmed = report_broadcast_progress(
            &repo,
            "mock://asm-membership",
            session_of(&sig_b()),
            &action_id,
            &btc,
            bitcoin::Network::Regtest,
            report("reveal_confirmed", Some(COMMIT_B), Some(REVEAL_B)),
        )
        .await
        .unwrap();

        assert_eq!(confirmed.broadcast_status, BroadcastStatus::RevealConfirmed);
        assert_eq!(confirmed.commit_txid.as_deref(), Some(COMMIT_A));
        assert_eq!(confirmed.reveal_txid.as_deref(), Some(REVEAL_A));
        let stored = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        // the earlier reveal's block + the uniform mock lock period 2016
        assert_eq!(stored.activation_height, Some(802_026));
    }

    /// Beside other pairs, a `reveal_confirmed` the node cannot see yet records the pair as sent;
    /// the next read confirms it.
    #[tokio::test]
    async fn a_confirmed_report_the_node_does_not_see_waits_for_the_reconcile() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();

        let after = report_broadcast_progress(
            &repo,
            "mock://asm-membership",
            session_of(&sig_b()),
            &action_id,
            &MinedTxids(std::collections::HashMap::new()),
            bitcoin::Network::Regtest,
            report("reveal_confirmed", Some(COMMIT_B), Some(REVEAL_B)),
        )
        .await
        .unwrap();

        assert_eq!(after.broadcast_status, BroadcastStatus::RevealBroadcasted);
        assert_eq!(after.reveal_txid.as_deref(), Some(REVEAL_B));
    }

    /// A lone pair has nothing to race: its `reveal_confirmed` is taken even when the node cannot
    /// find the reveal, as on a node without `-txindex` once the reveal left the mempool.
    #[tokio::test]
    async fn a_lone_pairs_confirmed_report_is_taken_without_the_node() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;

        let after = report_broadcast_progress(
            &repo,
            "mock://asm-membership",
            session_of(&sig_a()),
            &action_id,
            &MinedTxids(std::collections::HashMap::new()),
            bitcoin::Network::Regtest,
            report("reveal_confirmed", Some(COMMIT_A), Some(REVEAL_A)),
        )
        .await
        .unwrap();

        assert_eq!(after.broadcast_status, BroadcastStatus::RevealConfirmed);
        assert_eq!(after.reveal_txid.as_deref(), Some(REVEAL_A));
    }

    /// A report must name real txids: a malformed one would be listed and fail every lookup.
    #[tokio::test]
    async fn a_report_with_a_malformed_txid_is_refused() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;

        let err = report_broadcast_progress(
            &repo,
            "mock://asm-membership",
            session_of(&sig_a()),
            &action_id,
            &mock_btc(),
            bitcoin::Network::Regtest,
            report("reveal_broadcasted", Some(COMMIT_A), Some("reveal")),
        )
        .await
        .unwrap_err();

        assert!(matches!(err, AppError::BadRequest(_)), "{err:?}");
    }

    /// A malformed pair backfilled from before reports were validated is skipped by the
    /// reconcile instead of holding back the pair that was mined.
    #[tokio::test]
    async fn a_malformed_listed_pair_does_not_block_the_reconcile() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();
        let mut proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        proposal.broadcast_attempts.push(BroadcastAttempt {
            broadcaster_pubkey: None,
            commit_txid: "commit".to_string(),
            reveal_txid: "reveal".to_string(),
            created_at: Utc::now(),
        });
        let btc = RevealLookup {
            heights: [(REVEAL_B, 800_010)].into_iter().collect(),
            unreadable: "reveal",
        };

        let confirmed = confirm_reveal_if_mined(
            &repo,
            proposal,
            &btc,
            bitcoin::Network::Regtest,
            "mock://asm-membership",
        )
        .await;

        assert_eq!(confirmed.broadcast_status, BroadcastStatus::RevealConfirmed);
        assert_eq!(confirmed.reveal_txid.as_deref(), Some(REVEAL_B));
    }

    /// A report that does not name a reveal must not fail, or enact, the pair the row tracks.
    #[tokio::test]
    async fn a_report_that_names_no_reveal_leaves_a_tagged_row_alone() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();

        let mut unnamed = report("failed", None, None);
        unnamed.broadcast_error = Some(DROPPED_FOR_TEST.to_string());
        let after = report_broadcast_progress(
            &repo,
            "mock://asm-membership",
            session_of(&sig_a()),
            &action_id,
            &mock_btc(),
            bitcoin::Network::Regtest,
            unnamed,
        )
        .await
        .unwrap();

        assert_eq!(after.broadcast_status, BroadcastStatus::CommitBroadcasted);
        assert_eq!(after.reveal_txid.as_deref(), Some(REVEAL_B));
        assert!(after.broadcast_error.is_none());
        assert_eq!(after.broadcast_attempts.len(), 2);
    }

    /// The node failing to answer about the tracked reveal is not "that reveal lost". Confirming
    /// the other pair would pin the activation height to the wrong block.
    #[tokio::test]
    async fn an_unreadable_reveal_leaves_the_row_in_flight() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();
        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();

        let btc = RevealLookup {
            heights: [(REVEAL_A, 800_010)].into_iter().collect(),
            unreadable: REVEAL_B,
        };
        let unchanged = confirm_reveal_if_mined(
            &repo,
            proposal,
            &btc,
            bitcoin::Network::Regtest,
            "mock://asm-membership",
        )
        .await;

        assert_eq!(
            unchanged.broadcast_status,
            BroadcastStatus::CommitBroadcasted
        );
        assert_eq!(unchanged.reveal_txid.as_deref(), Some(REVEAL_B));
        assert!(unchanged.activation_height.is_none());
    }

    /// A Bitcoin Core node as the reconcile sees it (#605): mined txids with their height and place
    /// in the block, txids in the mempool, and the -5 answer for anything else — which, without
    /// `-txindex`, is also its answer for a mined txid it cannot index.
    #[derive(Default)]
    struct NodeView {
        mined: std::collections::HashMap<&'static str, (u64, usize)>,
        mempool: Vec<&'static str>,
        without_txindex: bool,
    }

    #[async_trait::async_trait]
    impl BitcoinRpcClient for NodeView {
        async fn estimate_fee_rate_sats_per_vb(
            &self,
            _target_blocks: u16,
        ) -> Result<u64, AppError> {
            Ok(1)
        }

        async fn get_block_height_for_txid(&self, txid: &str) -> Result<u64, AppError> {
            if self.without_txindex && !self.mempool.contains(&txid) {
                return Err(AppError::BadRequest(
                    "bitcoin rpc `getrawtransaction` failed (HTTP 500 Internal Server Error): No such \
                     mempool transaction. Use -txindex or provide a block hash to enable blockchain \
                     transaction queries. Use gettransaction for wallet transactions."
                        .to_string(),
                ));
            }
            if let Some((height, _)) = self.mined.get(txid) {
                return Ok(*height);
            }
            if self.mempool.contains(&txid) {
                return Err(AppError::BadRequest(format!(
                    "getrawtransaction: tx {txid} has no blockhash — not yet confirmed"
                )));
            }
            Err(AppError::BadRequest(
                "bitcoin rpc `getrawtransaction` failed (HTTP 500 Internal Server Error): No such \
                 mempool or blockchain transaction. Use gettransaction for wallet transactions."
                    .to_string(),
            ))
        }

        async fn get_tx_position_in_block(&self, txid: &str) -> Result<usize, AppError> {
            self.mined
                .get(txid)
                .map(|(_, position)| *position)
                .ok_or_else(|| AppError::BadRequest("not in a block".to_string()))
        }

        async fn get_chain_tip(&self) -> Result<u64, AppError> {
            Ok(800_000)
        }
    }

    /// Without `-txindex` the node cannot find a mined reveal. That is not "not mined": the
    /// reconcile must not skip it and confirm another pair in its place.
    #[tokio::test]
    async fn a_node_without_txindex_is_not_read_as_unmined() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();
        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        let btc = NodeView {
            without_txindex: true,
            ..NodeView::default()
        };

        let unchanged = confirm_reveal_if_mined(
            &repo,
            proposal,
            &btc,
            bitcoin::Network::Regtest,
            "mock://asm-membership",
        )
        .await;

        assert_eq!(
            unchanged.broadcast_status,
            BroadcastStatus::CommitBroadcasted
        );
    }

    /// Two reveals in one block: the ASM executed the one the block lists first, whichever pair
    /// the row tracked.
    #[tokio::test]
    async fn of_two_reveals_in_one_block_the_first_listed_wins() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();
        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        let btc = NodeView {
            mined: [(REVEAL_A, (800_010, 3)), (REVEAL_B, (800_010, 7))]
                .into_iter()
                .collect(),
            ..NodeView::default()
        };

        let confirmed = confirm_reveal_if_mined(
            &repo,
            proposal,
            &btc,
            bitcoin::Network::Regtest,
            "mock://asm-membership",
        )
        .await;

        assert_eq!(confirmed.broadcast_status, BroadcastStatus::RevealConfirmed);
        assert_eq!(
            confirmed.reveal_txid.as_deref(),
            Some(REVEAL_A),
            "listed first, though the row tracked B"
        );
    }

    /// A tie the node cannot order is left for the next read rather than guessed.
    #[tokio::test]
    async fn a_tie_the_node_cannot_order_leaves_the_row_in_flight() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();
        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        // `MinedTxids` gives heights only, like a client that cannot place a txid in its block.
        let btc = MinedTxids(
            [(REVEAL_A, 800_010), (REVEAL_B, 800_010)]
                .into_iter()
                .collect(),
        );

        let unchanged = confirm_reveal_if_mined(
            &repo,
            proposal,
            &btc,
            bitcoin::Network::Regtest,
            "mock://asm-membership",
        )
        .await;

        assert_eq!(
            unchanged.broadcast_status,
            BroadcastStatus::CommitBroadcasted
        );
    }

    /// A malformed txid on the row itself (stored before reports were validated) is skipped too.
    #[tokio::test]
    async fn a_malformed_tracked_reveal_does_not_block_the_reconcile() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();
        let mut proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        proposal.reveal_txid = Some("reveal".to_string());
        let btc = RevealLookup {
            heights: [(REVEAL_A, 800_010)].into_iter().collect(),
            unreadable: "reveal",
        };

        let winner = first_mined_pair(&proposal, &btc, bitcoin::Network::Regtest).await;

        assert_eq!(
            winner,
            MinedPair::First(COMMIT_A.to_string(), REVEAL_A.to_string())
        );
    }

    /// A signer whose earlier registration never reached the network (the session died before
    /// broadcasting) may register another pair; one whose pair is still in the mempool may not.
    #[tokio::test]
    async fn a_signer_whose_pair_is_nowhere_may_register_another() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();
        let register_a = |btc: NodeView| {
            let repo = &repo;
            let action_id = &action_id;
            async move {
                register_additional_pair(
                    repo,
                    session_of(&sig_a()),
                    "mock://asm-membership",
                    &btc,
                    action_id,
                    COMMIT_C,
                    REVEAL_C,
                )
                .await
            }
        };

        let err = register_a(NodeView {
            mempool: vec![COMMIT_A],
            ..NodeView::default()
        })
        .await
        .unwrap_err();
        assert!(
            matches!(err, AppError::Conflict(ref m) if m == OWN_PAIR_EXISTS),
            "still in the mempool: {err:?}"
        );

        let err = register_a(NodeView {
            without_txindex: true,
            ..NodeView::default()
        })
        .await
        .unwrap_err();
        assert!(
            matches!(err, AppError::Conflict(ref m) if m == OWN_PAIR_EXISTS),
            "a node that cannot tell: {err:?}"
        );

        let registered = register_a(NodeView::default()).await.unwrap();
        assert_eq!(registered.reveal_txid.as_deref(), Some(REVEAL_C));
        assert_eq!(registered.broadcast_attempts.len(), 3);
    }

    /// The SSZ payload the desktop puts in the envelope: this sequence number and action, signed
    /// by both fixture approvers (`sig_a` is 1·G, `sig_b` is 2·G).
    fn signed_payload(seq_no: u64, action_hex: &str) -> Vec<u8> {
        signed_payload_by(seq_no, action_hex, &[1, 2])
    }

    /// The same payload, signed by the fixture keys with these secret scalars.
    fn signed_payload_by(seq_no: u64, action_hex: &str, secrets: &[u8]) -> Vec<u8> {
        use ssz::{Decode, Encode};
        let action = strata_asm_txs_admin::actions::MultisigAction::from_ssz_bytes(
            &hex::decode(action_hex).unwrap(),
        )
        .unwrap();
        let sighash =
            strata_asm_txs_admin::signing_message::SigningMessage::for_action(&action, seq_no)
                .compute_sighash()
                .0;
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let message = bitcoin::secp256k1::Message::from_digest(sighash);
        let signatures = secrets
            .iter()
            .enumerate()
            .map(|(index, secret)| {
                let mut key = [0u8; 32];
                key[31] = *secret;
                let key = bitcoin::secp256k1::SecretKey::from_slice(&key).unwrap();
                let (recovery, compact) = secp
                    .sign_ecdsa_recoverable(&message, &key)
                    .serialize_compact();
                let mut bytes = [0u8; 65];
                bytes[0] = recovery.to_i32() as u8;
                bytes[1..].copy_from_slice(&compact);
                strata_crypto::threshold_signature::IndexedSignature::new(index as u8, bytes)
            })
            .collect();
        let signatures = strata_crypto::threshold_signature::SignatureSet::new(signatures).unwrap();
        strata_asm_txs_admin::parser::SignedPayload::new(seq_no, action, signatures).as_ssz_bytes()
    }

    /// The taproot envelope a commit pays to, built the way the desktop builds it.
    struct Envelope {
        keypair: bitcoin::key::Keypair,
        script: bitcoin::ScriptBuf,
        spend_info: bitcoin::taproot::TaprootSpendInfo,
    }

    impl Envelope {
        fn new(payload: &[u8]) -> Self {
            let secp = bitcoin::secp256k1::Secp256k1::new();
            let keypair = bitcoin::key::Keypair::from_seckey_slice(&secp, &[7u8; 32]).unwrap();
            let (internal_key, _) = keypair.x_only_public_key();
            let script = strata_l1_envelope_fmt::builder::EnvelopeScriptBuilder::with_pubkey(
                &internal_key.serialize(),
            )
            .unwrap()
            .add_envelope(payload)
            .unwrap()
            .build_without_min_check()
            .unwrap();
            let spend_info = bitcoin::taproot::TaprootBuilder::new()
                .add_leaf(0, script.clone())
                .unwrap()
                .finalize(&secp, internal_key)
                .unwrap();
            Self {
                keypair,
                script,
                spend_info,
            }
        }

        fn script_pubkey(&self) -> bitcoin::ScriptBuf {
            let secp = bitcoin::secp256k1::Secp256k1::new();
            let (internal_key, _) = self.keypair.x_only_public_key();
            bitcoin::ScriptBuf::new_p2tr(&secp, internal_key, self.spend_info.merkle_root())
        }

        /// The signed reveal spending `commit` (which holds `prevout`) to `to`.
        fn reveal(
            &self,
            commit: bitcoin::OutPoint,
            prevout: bitcoin::TxOut,
            to: bitcoin::ScriptBuf,
            fee: u64,
        ) -> bitcoin::Transaction {
            use bitcoin::hashes::Hash;
            use bitcoin::sighash::{Prevouts, SighashCache};
            use bitcoin::taproot::LeafVersion;
            let mut tx = bitcoin::Transaction {
                version: bitcoin::transaction::Version::TWO,
                lock_time: bitcoin::absolute::LockTime::ZERO,
                input: vec![bitcoin::TxIn {
                    previous_output: commit,
                    script_sig: bitcoin::ScriptBuf::new(),
                    sequence: bitcoin::Sequence::ENABLE_RBF_NO_LOCKTIME,
                    witness: bitcoin::Witness::new(),
                }],
                output: vec![bitcoin::TxOut {
                    value: prevout.value - bitcoin::Amount::from_sat(fee),
                    script_pubkey: to,
                }],
            };
            let leaf_hash = bitcoin::TapLeafHash::from_script(&self.script, LeafVersion::TapScript);
            let sighash = SighashCache::new(&tx)
                .taproot_script_spend_signature_hash(
                    0,
                    &Prevouts::All(&[prevout]),
                    leaf_hash,
                    bitcoin::TapSighashType::Default,
                )
                .unwrap();
            let message = bitcoin::secp256k1::Message::from_digest(sighash.to_byte_array());
            let signature = bitcoin::secp256k1::Secp256k1::new()
                .sign_schnorr_no_aux_rand(&message, &self.keypair);
            let control_block = self
                .spend_info
                .control_block(&(self.script.clone(), LeafVersion::TapScript))
                .unwrap();
            let mut witness = bitcoin::Witness::new();
            witness.push(signature.as_ref());
            witness.push(self.script.as_bytes());
            witness.push(control_block.serialize());
            tx.input[0].witness = witness;
            tx
        }
    }

    /// A reveal spending `commit:0` whose envelope carries `seq_no` and `action_hex`.
    fn envelope_reveal(commit: &str, seq_no: u64, action_hex: &str) -> bitcoin::Transaction {
        let envelope = Envelope::new(&signed_payload(seq_no, action_hex));
        let prevout = bitcoin::TxOut {
            value: bitcoin::Amount::from_sat(10_000),
            script_pubkey: envelope.script_pubkey(),
        };
        envelope.reveal(
            bitcoin::OutPoint::new(commit.parse().unwrap(), 0),
            prevout,
            bitcoin::ScriptBuf::new(),
            1_000,
        )
    }

    /// Mines the listed reveals as these transactions, and reports every other txid as in the
    /// mempool. The node reads transactions, like the real client.
    struct MinedReveals(Vec<(&'static str, u64, bitcoin::Transaction)>);

    #[async_trait::async_trait]
    impl BitcoinRpcClient for MinedReveals {
        async fn estimate_fee_rate_sats_per_vb(
            &self,
            _target_blocks: u16,
        ) -> Result<u64, AppError> {
            Ok(1)
        }

        async fn get_block_height_for_txid(&self, txid: &str) -> Result<u64, AppError> {
            self.get_mined_reveal(txid).await.map(|mined| mined.height)
        }

        async fn get_mined_reveal(&self, txid: &str) -> Result<MinedReveal, AppError> {
            if let Some((_, height, tx)) = self.0.iter().find(|(reveal, _, _)| *reveal == txid) {
                return Ok(MinedReveal {
                    height: *height,
                    tx: Some(tx.clone()),
                });
            }
            Err(AppError::BadRequest(format!(
                "getrawtransaction: tx {txid} has no blockhash — not yet confirmed"
            )))
        }

        async fn get_chain_tip(&self) -> Result<u64, AppError> {
            Ok(800_000)
        }
    }

    #[tokio::test]
    async fn a_reveal_carries_the_proposal_only_with_its_commit_seq_no_and_action() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        assert_eq!(proposal.seq_no, 1);
        let reveal = envelope_reveal(COMMIT_B, 1, &test_fixture_action_hex());
        assert!(reveal_carries_proposal(
            &reveal,
            COMMIT_B,
            &proposal,
            bitcoin::Network::Regtest
        ));
        assert!(
            !reveal_carries_proposal(&reveal, COMMIT_A, &proposal, bitcoin::Network::Regtest),
            "spends another commit"
        );
        let other_seq_no = envelope_reveal(COMMIT_B, 2, &test_fixture_action_hex());
        assert!(!reveal_carries_proposal(
            &other_seq_no,
            COMMIT_B,
            &proposal,
            bitcoin::Network::Regtest
        ));
        let other_action = envelope_reveal(
            COMMIT_B,
            1,
            &crate::infrastructure::action_codec::test_fixture_council_rotation_action_hex(),
        );
        assert!(!reveal_carries_proposal(
            &other_action,
            COMMIT_B,
            &proposal,
            bitcoin::Network::Regtest
        ));
        // The right action without the quorum the ASM needs: unsigned, or one approver of two, or
        // the approvers' keys signing twice.
        for secrets in [&[][..], &[1], &[1, 1], &[3, 4]] {
            let envelope =
                Envelope::new(&signed_payload_by(1, &test_fixture_action_hex(), secrets));
            let reveal = envelope.reveal(
                bitcoin::OutPoint::new(COMMIT_B.parse().unwrap(), 0),
                bitcoin::TxOut {
                    value: bitcoin::Amount::from_sat(10_000),
                    script_pubkey: envelope.script_pubkey(),
                },
                bitcoin::ScriptBuf::new(),
                1_000,
            );
            assert!(
                !reveal_carries_proposal(&reveal, COMMIT_B, &proposal, bitcoin::Network::Regtest),
                "signed by {secrets:?}"
            );
        }
        let plain = bitcoin::Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::absolute::LockTime::ZERO,
            input: vec![bitcoin::TxIn {
                previous_output: bitcoin::OutPoint::new(COMMIT_B.parse().unwrap(), 0),
                ..Default::default()
            }],
            output: vec![],
        };
        assert!(!reveal_carries_proposal(
            &plain,
            COMMIT_B,
            &proposal,
            bitcoin::Network::Regtest
        ));
    }

    /// Any txid can be registered. One mined after the registration that is not a commit + reveal
    /// of this proposal does not win, however low its block; the row stays in flight, and the real
    /// reveal mined later still confirms it.
    #[tokio::test]
    async fn an_unrelated_reveal_mined_after_registration_does_not_win() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();
        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        let unrelated = bitcoin::Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::absolute::LockTime::ZERO,
            input: vec![bitcoin::TxIn {
                previous_output: bitcoin::OutPoint::new(COMMIT_B.parse().unwrap(), 0),
                ..Default::default()
            }],
            output: vec![],
        };

        let unchanged = confirm_reveal_if_mined(
            &repo,
            proposal.clone(),
            &MinedReveals(vec![(REVEAL_B, 800_005, unrelated)]),
            bitcoin::Network::Regtest,
            "mock://asm-membership",
        )
        .await;
        assert_eq!(
            unchanged.broadcast_status,
            BroadcastStatus::CommitBroadcasted
        );
        assert_eq!(unchanged.reveal_txid.as_deref(), Some(REVEAL_B));

        let confirmed = confirm_reveal_if_mined(
            &repo,
            proposal,
            &MinedReveals(vec![(
                REVEAL_A,
                800_010,
                envelope_reveal(COMMIT_A, 1, &test_fixture_action_hex()),
            )]),
            bitcoin::Network::Regtest,
            "mock://asm-membership",
        )
        .await;
        assert_eq!(confirmed.broadcast_status, BroadcastStatus::RevealConfirmed);
        assert_eq!(confirmed.reveal_txid.as_deref(), Some(REVEAL_A));
    }

    /// A pair first reported after it was mined (the manual fallback, imported later) still wins:
    /// what counts is what the reveal carries, not when it was registered.
    #[tokio::test]
    async fn a_carrying_reveal_reported_after_it_was_mined_wins() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();
        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();

        let confirmed = confirm_reveal_if_mined(
            &repo,
            proposal,
            &MinedReveals(vec![(
                REVEAL_B,
                100,
                envelope_reveal(COMMIT_B, 1, &test_fixture_action_hex()),
            )]),
            bitcoin::Network::Regtest,
            "mock://asm-membership",
        )
        .await;
        assert_eq!(confirmed.broadcast_status, BroadcastStatus::RevealConfirmed);
        assert_eq!(confirmed.reveal_txid.as_deref(), Some(REVEAL_B));
    }

    /// The second pair failed before it was broadcast. The first one is still in the mempool, so
    /// the row goes back to it instead of `failed`.
    #[tokio::test]
    async fn a_failed_second_pair_puts_the_row_back_on_a_live_first() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();

        let mut failed = report("failed", Some(COMMIT_B), Some(REVEAL_B));
        failed.broadcast_error = Some("rejected".to_string());
        let after = report_broadcast_progress(
            &repo,
            "mock://asm-membership",
            session_of(&sig_b()),
            &action_id,
            &NodeView {
                mempool: vec![COMMIT_A],
                ..NodeView::default()
            },
            bitcoin::Network::Regtest,
            failed,
        )
        .await
        .unwrap();

        assert_eq!(after.broadcast_status, BroadcastStatus::CommitBroadcasted);
        assert_eq!(after.reveal_txid.as_deref(), Some(REVEAL_A));
        assert_eq!(after.commit_txid.as_deref(), Some(COMMIT_A));
        assert!(after.broadcast_error.is_none());
        assert_eq!(after.broadcast_attempts.len(), 2);

        // The only pair: `failed` still means failed.
        let lone = new_repo();
        let lone_id = save_with_pair_of_a(&lone).await;
        let mut failed = report("failed", Some(COMMIT_A), Some(REVEAL_A));
        failed.broadcast_error = Some("rejected".to_string());
        let after = report_broadcast_progress(
            &lone,
            "mock://asm-membership",
            session_of(&sig_a()),
            &lone_id,
            &mock_btc(),
            bitcoin::Network::Regtest,
            failed,
        )
        .await
        .unwrap();
        assert_eq!(after.broadcast_status, BroadcastStatus::Failed);
        assert_eq!(after.reveal_txid.as_deref(), Some(REVEAL_A));
    }

    /// The earlier pair is gone from the mempool too (or the node cannot say): the row is marked
    /// `failed`, and a fresh claim may send again, instead of tracking a pair nothing will mine.
    #[tokio::test]
    async fn a_failed_second_pair_beside_a_dead_first_marks_the_row_failed() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();

        let mut failed = report("failed", Some(COMMIT_B), Some(REVEAL_B));
        failed.broadcast_error = Some("dropped".to_string());
        let after = report_broadcast_progress(
            &repo,
            "mock://asm-membership",
            session_of(&sig_b()),
            &action_id,
            &NodeView::default(),
            bitcoin::Network::Regtest,
            failed,
        )
        .await
        .unwrap();

        assert_eq!(after.broadcast_status, BroadcastStatus::Failed);
        assert_eq!(after.reveal_txid.as_deref(), Some(REVEAL_B));
        let claimed = claim_broadcast_coordination(
            &repo,
            Authority::StrataAdmin,
            "mock://asm-membership",
            &action_id,
        )
        .await
        .unwrap();
        assert_eq!(claimed.broadcast_status, BroadcastStatus::CommitBroadcasted);
    }

    /// A node without `-txindex` cannot find any mined reveal, so a `reveal_confirmed` report
    /// beside other pairs is taken as reported rather than leaving the proposal in flight for
    /// good — as after a retry that follows a failed send.
    #[tokio::test]
    async fn a_node_without_txindex_takes_the_report_beside_other_pairs() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();

        let confirmed = report_broadcast_progress(
            &repo,
            "mock://asm-membership",
            session_of(&sig_b()),
            &action_id,
            &NodeView {
                without_txindex: true,
                ..NodeView::default()
            },
            bitcoin::Network::Regtest,
            report("reveal_confirmed", Some(COMMIT_B), Some(REVEAL_B)),
        )
        .await
        .unwrap();
        assert_eq!(confirmed.broadcast_status, BroadcastStatus::RevealConfirmed);
        assert_eq!(confirmed.reveal_txid.as_deref(), Some(REVEAL_B));
    }

    /// A signer whose pairs keep turning up nowhere may send again, but only up to
    /// `MAX_PAIRS_PER_SIGNER` pairs in all: registration takes any txids, and each pair takes the
    /// row over and costs every reconcile a node lookup.
    #[tokio::test]
    async fn a_signer_lists_at_most_max_pairs() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        let nowhere = NodeView {
            mempool: vec![COMMIT_A],
            ..NodeView::default()
        };
        for n in 0..MAX_PAIRS_PER_SIGNER {
            let (commit, reveal) = (
                format!("{n:x}").repeat(64),
                format!("{:x}", n + 8).repeat(64),
            );
            register_additional_pair(
                &repo,
                session_of(&sig_b()),
                "mock://asm-membership",
                &nowhere,
                &action_id,
                &commit,
                &reveal,
            )
            .await
            .unwrap();
        }
        let err = register_additional_pair(
            &repo,
            session_of(&sig_b()),
            "mock://asm-membership",
            &nowhere,
            &action_id,
            &"e".repeat(64),
            &"f".repeat(64),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, AppError::Conflict(ref m) if m == PAIR_LIMIT_REACHED),
            "{err:?}"
        );
    }

    /// The limit holds on the claim path too: a claim, a pre-registration with made-up txids and
    /// `failed`, again and again, lists no more than `MAX_PAIRS_PER_SIGNER` pairs for that signer.
    #[tokio::test]
    async fn claiming_again_and_again_lists_at_most_max_pairs() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        let mut refused = None;
        for n in 0..=MAX_PAIRS_PER_SIGNER {
            let mut failed = report("failed", Some(COMMIT_A), None);
            failed.reveal_txid = repo
                .find_by_action_id(&action_id)
                .await
                .unwrap()
                .unwrap()
                .reveal_txid;
            failed.broadcast_error = Some("dropped".to_string());
            report_broadcast_progress(
                &repo,
                "mock://asm-membership",
                session_of(&sig_a()),
                &action_id,
                &NodeView::default(),
                bitcoin::Network::Regtest,
                failed,
            )
            .await
            .unwrap();
            claim_broadcast_coordination(
                &repo,
                Authority::StrataAdmin,
                "mock://asm-membership",
                &action_id,
            )
            .await
            .unwrap();
            let (commit, reveal) = (
                format!("{n:x}").repeat(64),
                format!("{:x}", n + 8).repeat(64),
            );
            let registered = report_broadcast_progress(
                &repo,
                "mock://asm-membership",
                session_of(&sig_a()),
                &action_id,
                &NodeView::default(),
                bitcoin::Network::Regtest,
                report("commit_broadcasted", Some(&commit), Some(&reveal)),
            )
            .await;
            if let Err(err) = registered {
                refused = Some(err);
                break;
            }
        }
        let err = refused.expect("the limit refuses a pre-registration");
        assert!(
            matches!(err, AppError::Conflict(ref m) if m == PAIR_LIMIT_REACHED),
            "{err:?}"
        );
        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        assert_eq!(proposal.broadcast_attempts.len(), MAX_PAIRS_PER_SIGNER);
    }

    /// A report about the tracked reveal cannot change the row's commit: the reconcile checks
    /// that the mined reveal spends it.
    #[tokio::test]
    async fn a_report_cannot_change_the_tracked_pairs_commit() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        let after = report_broadcast_progress(
            &repo,
            "mock://asm-membership",
            session_of(&sig_b()),
            &action_id,
            &mock_btc(),
            bitcoin::Network::Regtest,
            report("reveal_broadcasted", Some(COMMIT_B), Some(REVEAL_A)),
        )
        .await
        .unwrap();
        assert_eq!(after.commit_txid.as_deref(), Some(COMMIT_A));
        assert_eq!(after.reveal_txid.as_deref(), Some(REVEAL_A));
    }

    /// A row tagged with its reveal only (a partial import) still confirms: the commit is the one
    /// the mined reveal spends.
    #[tokio::test]
    async fn a_reveal_with_no_commit_on_record_confirms_on_the_commit_it_spends() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        let mut proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        proposal.commit_txid = None;
        proposal.broadcast_attempts.clear();

        let winner = first_mined_pair(
            &proposal,
            &MinedReveals(vec![(
                REVEAL_A,
                800_010,
                envelope_reveal(COMMIT_A, 1, &test_fixture_action_hex()),
            )]),
            bitcoin::Network::Regtest,
        )
        .await;
        assert_eq!(
            winner,
            MinedPair::First(COMMIT_A.to_string(), REVEAL_A.to_string())
        );
    }

    /// A malformed report is a 400 even when it names a pair the row does not track.
    #[tokio::test]
    async fn a_report_about_an_untracked_pair_is_still_validated() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        for (status, proposal_status) in [("bogus", None), ("failed", Some("approved"))] {
            let mut body = report(status, Some(COMMIT_B), Some(REVEAL_B));
            body.proposal_status = proposal_status.map(str::to_string);
            let err = report_broadcast_progress(
                &repo,
                "mock://asm-membership",
                session_of(&sig_b()),
                &action_id,
                &mock_btc(),
                bitcoin::Network::Regtest,
                body,
            )
            .await
            .unwrap_err();
            assert!(matches!(err, AppError::BadRequest(_)), "{err:?}");
        }
    }

    /// The pre-check answers before anything is signed, with the registration's own refusals.
    #[tokio::test]
    async fn the_pre_check_refuses_before_signing() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        let still_live = NodeView {
            mempool: vec![COMMIT_A],
            ..NodeView::default()
        };
        let (a, b) = (sig_a(), sig_b());
        let check = |signer| {
            check_additional_pair(
                &repo,
                session_of(signer),
                "mock://asm-membership",
                &still_live,
                &action_id,
            )
        };

        let own = check(&a).await.unwrap_err();
        assert!(
            matches!(own, AppError::Conflict(ref m) if m == OWN_PAIR_EXISTS),
            "{own:?}"
        );
        let allowed = check(&b).await.unwrap();
        assert_eq!(allowed.reveal_txid.as_deref(), Some(REVEAL_A));

        // Nothing in flight any more: the desktop claims instead.
        let mut failed = report("failed", Some(COMMIT_A), Some(REVEAL_A));
        failed.broadcast_error = Some("rejected".to_string());
        report_broadcast_progress(
            &repo,
            "mock://asm-membership",
            session_of(&sig_a()),
            &action_id,
            &mock_btc(),
            bitcoin::Network::Regtest,
            failed,
        )
        .await
        .unwrap();
        let none = check(&b).await.unwrap_err();
        assert!(
            matches!(none, AppError::Conflict(ref m) if m == NO_PAIR_IN_FLIGHT),
            "{none:?}"
        );
    }

    // ── #605 against a real regtest bitcoind ───────────────────────────────
    //
    // `cargo test -p orchestrator-be -- --ignored regtest_` with `bitcoind` on PATH. Each test
    // starts its own node in a temporary datadir. Pairs are real commits + reveals whose envelope
    // carries the proposal, built the way the desktop builds them; plain wallet transactions stand
    // in for txids that are not a pair of this proposal.

    struct Regtest {
        child: std::process::Child,
        datadir: std::path::PathBuf,
        node: crate::infrastructure::bitcoin_rpc::HttpBitcoinRpcClient,
        wallet: crate::infrastructure::bitcoin_rpc::HttpBitcoinRpcClient,
        address: String,
    }

    impl Drop for Regtest {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
            let _ = std::fs::remove_dir_all(&self.datadir);
        }
    }

    impl Regtest {
        async fn start(txindex: bool) -> Self {
            use crate::infrastructure::bitcoin_rpc::HttpBitcoinRpcClient;
            let free_port = || {
                std::net::TcpListener::bind("127.0.0.1:0")
                    .unwrap()
                    .local_addr()
                    .unwrap()
                    .port()
            };
            let (rpc_port, p2p_port) = (free_port(), free_port());
            let datadir = std::env::temp_dir().join(format!(
                "orchestrator-605-{}",
                Utc::now().timestamp_nanos_opt().unwrap_or_default()
            ));
            std::fs::create_dir_all(&datadir).unwrap();
            let child = std::process::Command::new("bitcoind")
                .args([
                    "-regtest".to_string(),
                    format!("-datadir={}", datadir.display()),
                    format!("-rpcport={rpc_port}"),
                    format!("-port={p2p_port}"),
                    "-rpcuser=u".to_string(),
                    "-rpcpassword=p".to_string(),
                    "-listen=0".to_string(),
                    "-fallbackfee=0.0001".to_string(),
                    // Each send spends its own confirmed coin, so any one of them can be mined
                    // without the others.
                    "-spendzeroconfchange=0".to_string(),
                    format!("-txindex={}", u8::from(txindex)),
                ])
                .stdout(std::process::Stdio::null())
                .spawn()
                .expect("bitcoind on PATH");
            let url = format!("http://127.0.0.1:{rpc_port}");
            let node = HttpBitcoinRpcClient::new(&url, "u", "p");
            for _ in 0..100 {
                if node.rpc("getblockchaininfo", json!([])).await.is_ok() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            node.rpc("createwallet", json!(["w"])).await.unwrap();
            let wallet = HttpBitcoinRpcClient::new(&format!("{url}/wallet/w"), "u", "p");
            let address = wallet.rpc("getnewaddress", json!([])).await.unwrap();
            let address = address.as_str().unwrap().to_string();
            node.rpc("generatetoaddress", json!([120, address]))
                .await
                .unwrap();
            let regtest = Self {
                child,
                datadir,
                node,
                wallet,
                address,
            };
            if txindex {
                regtest.wait_for_txindex().await;
            }
            regtest
        }

        async fn wait_for_txindex(&self) {
            for _ in 0..100 {
                let info = self.node.rpc("getindexinfo", json!(["txindex"])).await;
                if info
                    .ok()
                    .and_then(|info| info.pointer("/txindex/synced").and_then(|v| v.as_bool()))
                    == Some(true)
                {
                    return;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            panic!("txindex never synced");
        }

        /// A commit + reveal pair of the fixture action at `seq_no`, both left in the mempool.
        async fn send_pair(&self, seq_no: u64) -> (String, String) {
            use std::str::FromStr;
            let envelope = Envelope::new(&signed_payload(seq_no, &test_fixture_action_hex()));
            let address =
                bitcoin::Address::from_script(&envelope.script_pubkey(), bitcoin::Network::Regtest)
                    .unwrap();
            let commit = self
                .wallet
                .rpc("sendtoaddress", json!([address.to_string(), 0.001]))
                .await
                .unwrap();
            let raw = self
                .node
                .rpc("getrawtransaction", json!([commit, true]))
                .await
                .unwrap();
            let commit_tx: bitcoin::Transaction =
                bitcoin::consensus::encode::deserialize_hex(raw["hex"].as_str().unwrap()).unwrap();
            let vout = commit_tx
                .output
                .iter()
                .position(|out| out.script_pubkey == envelope.script_pubkey())
                .unwrap();
            let to = bitcoin::Address::from_str(&self.address)
                .unwrap()
                .assume_checked()
                .script_pubkey();
            let reveal = envelope.reveal(
                bitcoin::OutPoint::new(commit_tx.compute_txid(), vout as u32),
                commit_tx.output[vout].clone(),
                to,
                1_000,
            );
            let reveal = self
                .node
                .rpc(
                    "sendrawtransaction",
                    json!([bitcoin::consensus::encode::serialize_hex(&reveal)]),
                )
                .await
                .unwrap();
            (
                commit.as_str().unwrap().to_string(),
                reveal.as_str().unwrap().to_string(),
            )
        }

        /// A wallet transaction left in the mempool.
        async fn send(&self) -> String {
            let txid = self
                .wallet
                .rpc("sendtoaddress", json!([self.address, 0.1]))
                .await
                .unwrap();
            txid.as_str().unwrap().to_string()
        }

        /// Mines one block holding exactly `txids`, in this order. Returns its height.
        async fn mine(&self, txids: &[&str]) -> u64 {
            self.node
                .rpc("generateblock", json!([self.address, txids]))
                .await
                .unwrap();
            if self
                .node
                .rpc("getindexinfo", json!(["txindex"]))
                .await
                .is_ok_and(|i| i.get("txindex").is_some())
            {
                self.wait_for_txindex().await;
            }
            self.node.get_chain_tip().await.unwrap()
        }
    }

    /// Signer A's pair is stuck in the mempool; signer B sends their own and B's reveal is mined.
    /// The row confirms on B's pair and the activation height comes from B's block.
    #[tokio::test]
    #[ignore = "needs bitcoind on PATH"]
    async fn regtest_the_second_signers_pair_mined_first_confirms() {
        let chain = Regtest::start(true).await;
        let (commit_a, reveal_a) = chain.send_pair(1).await;
        let (commit_b, reveal_b) = chain.send_pair(1).await;
        let repo = new_repo();
        let action_id = save_with_pair(&repo, 1, &commit_a, &reveal_a).await;
        register_additional_pair(
            &repo,
            session_of(&sig_b()),
            "mock://asm-membership",
            &chain.node,
            &action_id,
            &commit_b,
            &reveal_b,
        )
        .await
        .unwrap();

        let height = chain.mine(&[&commit_b, &reveal_b]).await;
        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        let confirmed = confirm_reveal_if_mined(
            &repo,
            proposal,
            &chain.node,
            bitcoin::Network::Regtest,
            "mock://asm-membership",
        )
        .await;

        assert_eq!(confirmed.broadcast_status, BroadcastStatus::RevealConfirmed);
        assert_eq!(confirmed.commit_txid.as_deref(), Some(commit_b.as_str()));
        assert_eq!(confirmed.reveal_txid.as_deref(), Some(reveal_b.as_str()));
        let stored = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        // B's block + the uniform mock lock period 2016
        assert_eq!(stored.activation_height, Some(height + 2016));
        let reveals: Vec<&str> = stored
            .broadcast_attempts
            .iter()
            .map(|attempt| attempt.reveal_txid.as_str())
            .collect();
        assert_eq!(reveals, [reveal_a.as_str(), reveal_b.as_str()]);
    }

    /// Signer B registered txids that are not a pair of this proposal, and they are mined first.
    /// The row stays in flight; A's real reveal, mined later, confirms it.
    #[tokio::test]
    #[ignore = "needs bitcoind on PATH"]
    async fn regtest_an_unrelated_mined_reveal_does_not_win() {
        let chain = Regtest::start(true).await;
        let (commit_a, reveal_a) = chain.send_pair(1).await;
        let (commit_x, reveal_x) = (chain.send().await, chain.send().await);
        let repo = new_repo();
        let action_id = save_with_pair(&repo, 1, &commit_a, &reveal_a).await;
        register_additional_pair(
            &repo,
            session_of(&sig_b()),
            "mock://asm-membership",
            &chain.node,
            &action_id,
            &commit_x,
            &reveal_x,
        )
        .await
        .unwrap();

        chain.mine(&[&commit_x, &reveal_x]).await;
        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        let unchanged = confirm_reveal_if_mined(
            &repo,
            proposal,
            &chain.node,
            bitcoin::Network::Regtest,
            "mock://asm-membership",
        )
        .await;
        assert_eq!(
            unchanged.broadcast_status,
            BroadcastStatus::CommitBroadcasted
        );

        let height = chain.mine(&[&commit_a, &reveal_a]).await;
        let confirmed = confirm_reveal_if_mined(
            &repo,
            unchanged,
            &chain.node,
            bitcoin::Network::Regtest,
            "mock://asm-membership",
        )
        .await;
        assert_eq!(confirmed.broadcast_status, BroadcastStatus::RevealConfirmed);
        assert_eq!(confirmed.reveal_txid.as_deref(), Some(reveal_a.as_str()));
        let stored = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        assert_eq!(stored.activation_height, Some(height + 2016));
    }

    /// Both reveals in one block: the one the block lists first is the one the ASM executed.
    #[tokio::test]
    #[ignore = "needs bitcoind on PATH"]
    async fn regtest_two_reveals_in_one_block_follow_the_block_order() {
        let chain = Regtest::start(true).await;
        let (commit_a, reveal_a) = chain.send_pair(1).await;
        let (commit_b, reveal_b) = chain.send_pair(1).await;
        let repo = new_repo();
        let action_id = save_with_pair(&repo, 1, &commit_a, &reveal_a).await;
        register_additional_pair(
            &repo,
            session_of(&sig_b()),
            "mock://asm-membership",
            &chain.node,
            &action_id,
            &commit_b,
            &reveal_b,
        )
        .await
        .unwrap();

        chain
            .mine(&[&commit_a, &commit_b, &reveal_a, &reveal_b])
            .await;
        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        let confirmed = confirm_reveal_if_mined(
            &repo,
            proposal,
            &chain.node,
            bitcoin::Network::Regtest,
            "mock://asm-membership",
        )
        .await;

        assert_eq!(confirmed.broadcast_status, BroadcastStatus::RevealConfirmed);
        assert_eq!(
            confirmed.reveal_txid.as_deref(),
            Some(reveal_a.as_str()),
            "listed first in the block, though the row tracked B"
        );
    }

    /// Without `-txindex` the node cannot find B's mined reveal once it left the mempool. The row
    /// stays in flight instead of confirming A's unmined pair or skipping B's; a lone pair's
    /// report is still taken.
    #[tokio::test]
    #[ignore = "needs bitcoind on PATH"]
    async fn regtest_a_node_without_txindex_leaves_several_pairs_in_flight() {
        let chain = Regtest::start(false).await;
        let (commit_a, reveal_a) = chain.send_pair(1).await;
        let (commit_b, reveal_b) = chain.send_pair(1).await;
        let repo = new_repo();
        let action_id = save_with_pair(&repo, 1, &commit_a, &reveal_a).await;
        register_additional_pair(
            &repo,
            session_of(&sig_b()),
            "mock://asm-membership",
            &chain.node,
            &action_id,
            &commit_b,
            &reveal_b,
        )
        .await
        .unwrap();

        chain.mine(&[&commit_b, &reveal_b]).await;
        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        let unchanged = confirm_reveal_if_mined(
            &repo,
            proposal,
            &chain.node,
            bitcoin::Network::Regtest,
            "mock://asm-membership",
        )
        .await;
        assert_eq!(
            unchanged.broadcast_status,
            BroadcastStatus::CommitBroadcasted
        );
        assert_eq!(chain.node.txindex().await.unwrap(), TxIndex::Missing);

        // A lone pair on the same node: its desktop's report confirms it.
        let (commit_c, reveal_c) = chain.send_pair(2).await;
        // Its own repository: one broadcast in flight per authority.
        let lone_repo = new_repo();
        let lone = save_with_pair(&lone_repo, 2, &commit_c, &reveal_c).await;
        chain.mine(&[&commit_c, &reveal_c]).await;
        let confirmed = report_broadcast_progress(
            &lone_repo,
            "mock://asm-membership",
            session_of(&sig_a()),
            &lone,
            &chain.node,
            bitcoin::Network::Regtest,
            report("reveal_confirmed", Some(&commit_c), Some(&reveal_c)),
        )
        .await
        .unwrap();
        assert_eq!(confirmed.broadcast_status, BroadcastStatus::RevealConfirmed);
    }

    /// A registration whose session died before broadcasting leaves a commit the node finds
    /// nowhere; that signer may register again. While it is in the mempool, they may not.
    #[tokio::test]
    #[ignore = "needs bitcoind on PATH"]
    async fn regtest_a_pair_that_never_reached_the_network_can_be_replaced() {
        let chain = Regtest::start(true).await;
        assert_eq!(chain.node.txindex().await.unwrap(), TxIndex::Synced);
        let (commit_a, reveal_a) = chain.send_pair(1).await;
        let repo = new_repo();
        let action_id = save_with_pair(&repo, 1, &commit_a, &reveal_a).await;
        // B registered this pair and died before broadcasting it.
        let never_sent = ("ab".repeat(32), "cd".repeat(32));
        register_additional_pair(
            &repo,
            session_of(&sig_b()),
            "mock://asm-membership",
            &chain.node,
            &action_id,
            &never_sent.0,
            &never_sent.1,
        )
        .await
        .unwrap();

        let (commit_b, reveal_b) = chain.send_pair(1).await;
        let replaced = register_additional_pair(
            &repo,
            session_of(&sig_b()),
            "mock://asm-membership",
            &chain.node,
            &action_id,
            &commit_b,
            &reveal_b,
        )
        .await
        .unwrap();
        assert_eq!(replaced.reveal_txid.as_deref(), Some(reveal_b.as_str()));

        // A's commit is in the mempool: A's pair is still on its way.
        let err = register_additional_pair(
            &repo,
            session_of(&sig_a()),
            "mock://asm-membership",
            &chain.node,
            &action_id,
            &"ef".repeat(32),
            &"fe".repeat(32),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, AppError::Conflict(ref m) if m == OWN_PAIR_EXISTS),
            "{err:?}"
        );
    }

    /// Mines the txids it is given. One txid is a node failure rather than "not in a block".
    struct RevealLookup {
        heights: std::collections::HashMap<&'static str, u64>,
        unreadable: &'static str,
    }

    #[async_trait::async_trait]
    impl BitcoinRpcClient for RevealLookup {
        async fn estimate_fee_rate_sats_per_vb(
            &self,
            _target_blocks: u16,
        ) -> Result<u64, AppError> {
            Ok(1)
        }

        async fn get_block_height_for_txid(&self, txid: &str) -> Result<u64, AppError> {
            if txid == self.unreadable {
                return Err(AppError::BadRequest(
                    "bitcoin rpc `getrawtransaction` request failed: connection refused"
                        .to_string(),
                ));
            }
            self.heights.get(txid).copied().ok_or_else(|| {
                AppError::BadRequest(format!(
                    "getrawtransaction: tx {txid} has no blockhash — not yet confirmed"
                ))
            })
        }

        async fn get_chain_tip(&self) -> Result<u64, AppError> {
            Ok(800_000)
        }
    }

    /// Answers every lookup with a failed call: a node that is down.
    struct NodeDown;

    #[async_trait::async_trait]
    impl BitcoinRpcClient for NodeDown {
        async fn estimate_fee_rate_sats_per_vb(
            &self,
            _target_blocks: u16,
        ) -> Result<u64, AppError> {
            Ok(1)
        }

        async fn get_block_height_for_txid(&self, _txid: &str) -> Result<u64, AppError> {
            Err(AppError::BadRequest(
                "Bitcoin RPC request timed out".to_string(),
            ))
        }

        async fn get_chain_tip(&self) -> Result<u64, AppError> {
            Ok(800_000)
        }
    }

    /// A node that cannot answer does not make the earlier pair dead: the row goes back to it
    /// rather than `failed`, which would stop its watchers and reopen the claim gate.
    #[tokio::test]
    async fn a_node_that_cannot_answer_keeps_the_earlier_pair() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        register_pair_of_b(&repo, &action_id).await.unwrap();

        let mut failed = report("failed", Some(COMMIT_B), Some(REVEAL_B));
        failed.broadcast_error = Some("rejected".to_string());
        let after = report_broadcast_progress(
            &repo,
            "mock://asm-membership",
            session_of(&sig_b()),
            &action_id,
            &NodeDown,
            bitcoin::Network::Regtest,
            failed,
        )
        .await
        .unwrap();
        assert_eq!(after.broadcast_status, BroadcastStatus::CommitBroadcasted);
        assert_eq!(after.reveal_txid.as_deref(), Some(REVEAL_A));
    }

    /// A pair already listed under another signer is refused as such, not as the caller's own.
    #[tokio::test]
    async fn a_pair_listed_under_another_signer_is_refused_as_listed() {
        let repo = new_repo();
        let action_id = save_with_pair_of_a(&repo).await;
        let err = register_additional_pair(
            &repo,
            session_of(&sig_b()),
            "mock://asm-membership",
            &NodeView::default(),
            &action_id,
            COMMIT_A,
            REVEAL_A,
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, AppError::Conflict(ref m) if m == PAIR_ALREADY_LISTED),
            "{err:?}"
        );
    }
}
