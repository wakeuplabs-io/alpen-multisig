use std::str::FromStr;

use crate::application::traits::ProposalRepository;
use crate::domain::authority::Authority;
use crate::domain::proposal::{
    ActionId, BroadcastStatus, Proposal, ProposalSignature, ProposalStatus,
};
use crate::error::AppError;
use sqlx::{PgPool, Row};

#[derive(Clone)]
pub(crate) struct PostgresProposalRepository {
    pool: PgPool,
}

impl PostgresProposalRepository {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn authority_to_db(authority: Authority) -> &'static str {
    match authority {
        Authority::AlpenAdmin => "alpen_admin",
        Authority::StrataAdmin => "strata_admin",
        Authority::SequencerManager => "sequencer_manager",
        Authority::SecurityCouncil => "security_council",
        Authority::PayoutAdmin => "payout_admin",
    }
}

fn authority_from_db(authority: &str) -> Result<Authority, AppError> {
    match authority {
        "alpen_admin" => Ok(Authority::AlpenAdmin),
        "strata_admin" => Ok(Authority::StrataAdmin),
        "sequencer_manager" => Ok(Authority::SequencerManager),
        "security_council" => Ok(Authority::SecurityCouncil),
        "payout_admin" => Ok(Authority::PayoutAdmin),
        _ => Err(AppError::Internal(anyhow::anyhow!(
            "invalid authority in database: {authority}"
        ))),
    }
}

fn status_to_db(status: ProposalStatus) -> &'static str {
    match status {
        ProposalStatus::Pending => "pending",
        ProposalStatus::Approved => "approved",
        ProposalStatus::Enacted => "enacted",
        ProposalStatus::Canceled => "canceled",
        ProposalStatus::Expired => "expired",
    }
}

fn status_from_db(status: &str) -> Result<ProposalStatus, AppError> {
    match status {
        "pending" => Ok(ProposalStatus::Pending),
        "approved" => Ok(ProposalStatus::Approved),
        "enacted" => Ok(ProposalStatus::Enacted),
        "canceled" => Ok(ProposalStatus::Canceled),
        "expired" => Ok(ProposalStatus::Expired),
        _ => Err(AppError::Internal(anyhow::anyhow!(
            "invalid status in database: {status}"
        ))),
    }
}

async fn load_signatures(
    pool: &PgPool,
    action_id: &str,
) -> Result<Vec<ProposalSignature>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT signer_pubkey, signature_hex
        FROM proposal_signatures
        WHERE action_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(action_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to load signatures: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|row| ProposalSignature {
            signer_pubkey: row.get("signer_pubkey"),
            signature_hex: row.get("signature_hex"),
        })
        .collect())
}

fn row_to_proposal_no_sigs(
    row: &sqlx::postgres::PgRow,
    action_id: String,
) -> Result<Proposal, AppError> {
    let authority: String = row.get("authority");
    let status: String = row.get("status");
    let broadcast_status: String = row.get("broadcast_status");
    let target_action_id: Option<String> = row.get("target_action_id");
    let activation_height: Option<i64> = row.get("activation_height");
    let update_id_in_queue: Option<i32> = row.get("update_id_in_queue");
    Ok(Proposal {
        action_id: ActionId(action_id),
        seq_no: row.get::<i64, _>("seq_no") as u64,
        authority: authority_from_db(&authority)?,
        status: status_from_db(&status)?,
        required_signatures: row.get::<i16, _>("required_signatures") as u16,
        action_hex: row.get("action_hex"),
        signatures: Vec::new(),
        broadcast_status: BroadcastStatus::from_str(&broadcast_status)?,
        commit_txid: row.get("commit_txid"),
        reveal_txid: row.get("reveal_txid"),
        broadcast_error: row.get("broadcast_error"),
        target_action_id: target_action_id.map(ActionId),
        activation_height: activation_height.map(|h| h as u64),
        update_id_in_queue: update_id_in_queue.map(|id| id as u32),
    })
}

const SELECT_PROPOSAL_COLS: &str = r#"
    action_id, seq_no, authority, status, action_hex, required_signatures,
    broadcast_status, commit_txid, reveal_txid, broadcast_error,
    target_action_id, activation_height, update_id_in_queue
"#;

#[async_trait::async_trait]
impl ProposalRepository for PostgresProposalRepository {
    async fn save_proposal(&self, proposal: Proposal) -> Result<(), AppError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to begin tx: {e}")))?;

        let proposal_insert = sqlx::query(
            r#"
            INSERT INTO proposals(action_id, seq_no, authority, status, action_hex, required_signatures, target_action_id, activation_height, update_id_in_queue)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(&proposal.action_id.0)
        .bind(proposal.seq_no as i64)
        .bind(authority_to_db(proposal.authority))
        .bind(status_to_db(proposal.status))
        .bind(&proposal.action_hex)
        .bind(proposal.required_signatures as i16)
        .bind(proposal.target_action_id.as_ref().map(|id| &id.0))
        .bind(proposal.activation_height.map(|h| h as i64))
        .bind(proposal.update_id_in_queue.map(|id| id as i32))
        .execute(&mut *tx)
        .await;

        if let Err(error) = proposal_insert {
            if let sqlx::Error::Database(db_error) = &error {
                if db_error.is_unique_violation() {
                    return Err(AppError::Conflict("proposal already exists".to_string()));
                }
            }
            return Err(AppError::Internal(anyhow::anyhow!(
                "failed to insert proposal: {error}"
            )));
        }

        for signature in proposal.signatures {
            let signature_insert = sqlx::query(
                r#"
                INSERT INTO proposal_signatures(action_id, signer_pubkey, signature_hex)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(&proposal.action_id.0)
            .bind(signature.signer_pubkey)
            .bind(signature.signature_hex)
            .execute(&mut *tx)
            .await;

            if let Err(error) = signature_insert {
                if let sqlx::Error::Database(db_error) = &error {
                    if db_error.is_unique_violation() {
                        return Err(AppError::Conflict("signer already signed".to_string()));
                    }
                }
                return Err(AppError::Internal(anyhow::anyhow!(
                    "failed to insert proposal signature: {error}"
                )));
            }
        }

        tx.commit()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to commit tx: {e}")))?;
        Ok(())
    }

    async fn find_by_action_id(&self, action_id: &ActionId) -> Result<Option<Proposal>, AppError> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_PROPOSAL_COLS} FROM proposals WHERE action_id = $1"
        ))
        .bind(&action_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to fetch proposal: {e}")))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let action_id_str: String = row.get("action_id");
        let signatures = load_signatures(&self.pool, &action_id_str).await?;
        let mut proposal = row_to_proposal_no_sigs(&row, action_id_str)?;
        proposal.signatures = signatures;
        Ok(Some(proposal))
    }

    async fn add_signature(
        &self,
        action_id: &ActionId,
        signer_pubkey: &str,
        signature_hex: &str,
    ) -> Result<Option<Proposal>, AppError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to begin tx: {e}")))?;

        let exists = sqlx::query_scalar::<_, i32>("SELECT 1 FROM proposals WHERE action_id = $1")
            .bind(&action_id.0)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to check proposal: {e}")))?;

        if exists.is_none() {
            tx.rollback()
                .await
                .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to rollback tx: {e}")))?;
            return Ok(None);
        }

        let insert_result = sqlx::query(
            r#"
            INSERT INTO proposal_signatures(action_id, signer_pubkey, signature_hex)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(&action_id.0)
        .bind(signer_pubkey)
        .bind(signature_hex)
        .execute(&mut *tx)
        .await;

        if let Err(error) = insert_result {
            if let sqlx::Error::Database(db_error) = &error {
                if db_error.is_unique_violation() {
                    return Err(AppError::Conflict("signer already signed".to_string()));
                }
            }
            return Err(AppError::Internal(anyhow::anyhow!(
                "failed to insert signature: {error}"
            )));
        }

        tx.commit()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to commit tx: {e}")))?;

        self.find_by_action_id(action_id).await
    }

    async fn list_by_status(
        &self,
        authority: Authority,
        status: Option<ProposalStatus>,
    ) -> Result<Vec<Proposal>, AppError> {
        let rows = if let Some(status) = status {
            sqlx::query(&format!(
                "SELECT {SELECT_PROPOSAL_COLS} FROM proposals WHERE authority = $1 AND status = $2 ORDER BY created_at DESC"
            ))
            .bind(authority_to_db(authority))
            .bind(status_to_db(status))
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query(&format!(
                "SELECT {SELECT_PROPOSAL_COLS} FROM proposals WHERE authority = $1 ORDER BY created_at DESC"
            ))
            .bind(authority_to_db(authority))
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to list proposals: {e}")))?;

        let mut proposals = Vec::with_capacity(rows.len());
        for row in rows {
            let action_id_str: String = row.get("action_id");
            let signatures = load_signatures(&self.pool, &action_id_str).await?;
            let mut proposal = row_to_proposal_no_sigs(&row, action_id_str)?;
            proposal.signatures = signatures;
            proposals.push(proposal);
        }

        Ok(proposals)
    }

    async fn claim_broadcast(&self, action_id: &ActionId) -> Result<Proposal, AppError> {
        let row = sqlx::query(&format!(
            r#"
            UPDATE proposals
            SET broadcast_status = 'commit_broadcasted', updated_at = NOW()
            WHERE action_id = $1 AND broadcast_status IN ('idle', 'failed')
            RETURNING {SELECT_PROPOSAL_COLS}
            "#
        ))
        .bind(&action_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to claim broadcast: {e}")))?;

        let Some(row) = row else {
            return Err(AppError::Conflict(
                "broadcast already in progress or completed".to_string(),
            ));
        };

        let action_id_str: String = row.get("action_id");
        let signatures = load_signatures(&self.pool, &action_id_str).await?;
        let mut proposal = row_to_proposal_no_sigs(&row, action_id_str)?;
        proposal.signatures = signatures;
        Ok(proposal)
    }

    async fn update_broadcast_status(
        &self,
        action_id: &ActionId,
        status: BroadcastStatus,
        proposal_status: Option<ProposalStatus>,
        commit_txid: Option<&str>,
        reveal_txid: Option<&str>,
        error: Option<&str>,
    ) -> Result<Option<Proposal>, AppError> {
        let proposal_status_db = proposal_status.map(status_to_db);

        sqlx::query(
            r#"
            UPDATE proposals
            SET
                broadcast_status  = $1,
                status            = COALESCE($2, status),
                commit_txid       = COALESCE($3, commit_txid),
                reveal_txid       = COALESCE($4, reveal_txid),
                broadcast_error   = $5,
                updated_at        = NOW()
            WHERE action_id = $6
            "#,
        )
        .bind(status.to_string())
        .bind(proposal_status_db)
        .bind(commit_txid)
        .bind(reveal_txid)
        .bind(error)
        .bind(&action_id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::Internal(anyhow::anyhow!("failed to update broadcast status: {e}"))
        })?;

        self.find_by_action_id(action_id).await
    }

    async fn find_cancel_for_target(
        &self,
        target: &ActionId,
    ) -> Result<Option<Proposal>, AppError> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_PROPOSAL_COLS} FROM proposals WHERE target_action_id = $1 LIMIT 1"
        ))
        .bind(&target.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to find cancel proposal: {e}")))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let action_id_str: String = row.get("action_id");
        let signatures = load_signatures(&self.pool, &action_id_str).await?;
        let mut proposal = row_to_proposal_no_sigs(&row, action_id_str)?;
        proposal.signatures = signatures;
        Ok(Some(proposal))
    }

    async fn update_activation_height(
        &self,
        action_id: &ActionId,
        height: u64,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE proposals SET activation_height = $1, updated_at = NOW() WHERE action_id = $2",
        )
        .bind(height as i64)
        .bind(&action_id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::Internal(anyhow::anyhow!("failed to update activation height: {e}"))
        })?;

        Ok(())
    }

    async fn update_update_id_in_queue(
        &self,
        action_id: &ActionId,
        update_id: u32,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE proposals SET update_id_in_queue = $1, updated_at = NOW() WHERE action_id = $2",
        )
        .bind(update_id as i32)
        .bind(&action_id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::Internal(anyhow::anyhow!("failed to update update_id_in_queue: {e}"))
        })?;

        Ok(())
    }
}
