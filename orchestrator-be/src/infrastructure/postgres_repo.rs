use crate::application::traits::ProposalRepository;
use crate::domain::authority::Authority;
use crate::domain::proposal::{ActionId, Proposal, ProposalSignature, ProposalStatus};
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
            INSERT INTO proposals(action_id, seq_no, authority, status, action_hex, required_signatures)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(&proposal.action_id.0)
        .bind(proposal.seq_no as i64)
        .bind(authority_to_db(proposal.authority))
        .bind(status_to_db(proposal.status))
        .bind(&proposal.action_hex)
        .bind(proposal.required_signatures as i16)
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
        let row = sqlx::query(
            r#"
            SELECT action_id, seq_no, authority, status, action_hex, required_signatures
            FROM proposals
            WHERE action_id = $1
            "#,
        )
        .bind(&action_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to fetch proposal: {e}")))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let action_id: String = row.get("action_id");
        let signatures = load_signatures(&self.pool, &action_id).await?;
        let authority: String = row.get("authority");
        let status: String = row.get("status");

        Ok(Some(Proposal {
            action_id: ActionId(action_id),
            seq_no: row.get::<i64, _>("seq_no") as u64,
            authority: authority_from_db(&authority)?,
            status: status_from_db(&status)?,
            required_signatures: row.get::<i16, _>("required_signatures") as u16,
            action_hex: row.get("action_hex"),
            signatures,
        }))
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
        status: Option<ProposalStatus>,
    ) -> Result<Vec<Proposal>, AppError> {
        let rows = if let Some(status) = status {
            sqlx::query(
                r#"
                SELECT action_id, seq_no, authority, status, action_hex, required_signatures
                FROM proposals
                WHERE status = $1
                ORDER BY created_at DESC
                "#,
            )
            .bind(status_to_db(status))
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query(
                r#"
                SELECT action_id, seq_no, authority, status, action_hex, required_signatures
                FROM proposals
                ORDER BY created_at DESC
                "#,
            )
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to list proposals: {e}")))?;

        let mut proposals = Vec::with_capacity(rows.len());
        for row in rows {
            let action_id: String = row.get("action_id");
            let authority: String = row.get("authority");
            let status: String = row.get("status");
            let signatures = load_signatures(&self.pool, &action_id).await?;
            proposals.push(Proposal {
                action_id: ActionId(action_id),
                seq_no: row.get::<i64, _>("seq_no") as u64,
                authority: authority_from_db(&authority)?,
                status: status_from_db(&status)?,
                required_signatures: row.get::<i16, _>("required_signatures") as u16,
                action_hex: row.get("action_hex"),
                signatures,
            });
        }

        Ok(proposals)
    }
}
