use std::str::FromStr;

use crate::application::proposals::{
    broadcast_write_for_report, decide_additional_pair, report_effect, AdditionalPair,
    AUTHORITY_IN_FLIGHT,
};
use crate::application::traits::ProposalRepository;
use crate::domain::authority::Authority;
use crate::domain::proposal::{
    ActionId, BroadcastAttempt, BroadcastStatus, Proposal, ProposalSignature, ProposalStatus,
};
use crate::error::AppError;
use chrono::{DateTime, Utc};
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

/// Broadcast sub-status write shared by `update_broadcast_status` and
/// `update_reported_pair_status`. Binds: status, proposal status, commit, reveal, error, action id.
const UPDATE_BROADCAST_STATUS: &str = r#"
    UPDATE proposals
    SET
        broadcast_status  = $1,
        status            = COALESCE($2, status),
        commit_txid       = COALESCE($3, commit_txid),
        reveal_txid       = COALESCE($4, reveal_txid),
        broadcast_error   = $5,
        updated_at        = NOW()
    WHERE action_id = $6
"#;

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
        ProposalStatus::Superseded => "superseded",
    }
}

fn status_from_db(status: &str) -> Result<ProposalStatus, AppError> {
    match status {
        "pending" => Ok(ProposalStatus::Pending),
        "approved" => Ok(ProposalStatus::Approved),
        "enacted" => Ok(ProposalStatus::Enacted),
        "canceled" => Ok(ProposalStatus::Canceled),
        "expired" => Ok(ProposalStatus::Expired),
        "superseded" => Ok(ProposalStatus::Superseded),
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

async fn load_broadcast_attempts<'e, E>(
    executor: E,
    action_id: &str,
) -> Result<Vec<BroadcastAttempt>, AppError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let rows = sqlx::query(
        r#"
        SELECT broadcaster_pubkey, commit_txid, reveal_txid, created_at
        FROM broadcast_attempts
        WHERE action_id = $1
        ORDER BY created_at ASC, reveal_txid ASC
        "#,
    )
    .bind(action_id)
    .fetch_all(executor)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to load broadcast attempts: {e}")))?;

    Ok(rows.iter().map(row_to_broadcast_attempt).collect())
}

/// Every pair of the listed proposals in one query, grouped by action id, each list oldest first.
async fn load_broadcast_attempts_for(
    pool: &PgPool,
    action_ids: &[String],
) -> Result<std::collections::HashMap<String, Vec<BroadcastAttempt>>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT action_id, broadcaster_pubkey, commit_txid, reveal_txid, created_at
        FROM broadcast_attempts
        WHERE action_id = ANY($1)
        ORDER BY created_at ASC, reveal_txid ASC
        "#,
    )
    .bind(action_ids)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to load broadcast attempts: {e}")))?;

    let mut by_action: std::collections::HashMap<String, Vec<BroadcastAttempt>> =
        std::collections::HashMap::new();
    for row in &rows {
        by_action
            .entry(row.get("action_id"))
            .or_default()
            .push(row_to_broadcast_attempt(row));
    }
    Ok(by_action)
}

fn row_to_broadcast_attempt(row: &sqlx::postgres::PgRow) -> BroadcastAttempt {
    BroadcastAttempt {
        broadcaster_pubkey: row.get("broadcaster_pubkey"),
        commit_txid: row.get("commit_txid"),
        reveal_txid: row.get("reveal_txid"),
        created_at: row.get("created_at"),
    }
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
    let created_at: DateTime<Utc> = row.get("created_at");
    let updated_at: DateTime<Utc> = row.get("updated_at");
    let broadcast_claimed_at: Option<DateTime<Utc>> = row.get("broadcast_claimed_at");
    Ok(Proposal {
        action_id: ActionId(action_id),
        seq_no: row.get::<i64, _>("seq_no") as u64,
        authority: authority_from_db(&authority)?,
        status: status_from_db(&status)?,
        required_signatures: row.get::<i16, _>("required_signatures") as u16,
        action_hex: row.get("action_hex"),
        title: row.get("title"),
        signatures: Vec::new(),
        broadcast_status: BroadcastStatus::from_str(&broadcast_status)?,
        commit_txid: row.get("commit_txid"),
        reveal_txid: row.get("reveal_txid"),
        broadcast_error: row.get("broadcast_error"),
        target_action_id: target_action_id.map(ActionId),
        activation_height: activation_height.map(|h| h as u64),
        update_id_in_queue: update_id_in_queue.map(|id| id as u32),
        created_at,
        updated_at,
        broadcast_claimed_at,
        broadcast_attempts: Vec::new(),
    })
}

const SELECT_PROPOSAL_COLS: &str = r#"
    action_id, seq_no, authority, status, action_hex, required_signatures,
    broadcast_status, commit_txid, reveal_txid, broadcast_error,
    target_action_id, activation_height, update_id_in_queue, created_at, updated_at, title,
    broadcast_claimed_at
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
            INSERT INTO proposals(action_id, seq_no, authority, status, action_hex, required_signatures, target_action_id, activation_height, update_id_in_queue, title)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
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
        .bind(proposal.title.as_deref())
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
        proposal.broadcast_attempts =
            load_broadcast_attempts(&self.pool, &proposal.action_id.0).await?;
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

        let action_ids: Vec<String> = rows.iter().map(|row| row.get("action_id")).collect();
        let mut attempts = load_broadcast_attempts_for(&self.pool, &action_ids).await?;

        let mut proposals = Vec::with_capacity(rows.len());
        for row in rows {
            let action_id_str: String = row.get("action_id");
            let signatures = load_signatures(&self.pool, &action_id_str).await?;
            let mut proposal = row_to_proposal_no_sigs(&row, action_id_str)?;
            proposal.signatures = signatures;
            proposal.broadcast_attempts =
                attempts.remove(&proposal.action_id.0).unwrap_or_default();
            proposals.push(proposal);
        }

        Ok(proposals)
    }

    async fn claim_broadcast(&self, action_id: &ActionId) -> Result<Proposal, AppError> {
        // Same predicate as `broadcast_claim_allowed` / `CLAIM_STALE_AFTER` (600s).
        // Lock every row of this authority first: two claims of different proposals
        // do not share a row, so `NOT EXISTS` alone can let both through.
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to begin tx: {e}")))?;

        sqlx::query(
            r#"
            SELECT action_id FROM proposals
            WHERE authority = (SELECT authority FROM proposals WHERE action_id = $1)
            ORDER BY action_id
            FOR UPDATE
            "#,
        )
        .bind(&action_id.0)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to lock authority rows: {e}")))?;

        let row = sqlx::query(&format!(
            r#"
            UPDATE proposals AS p
            SET broadcast_status = 'commit_broadcasted',
                broadcast_claimed_at = NOW(),
                broadcast_error = NULL,
                commit_txid = NULL,
                reveal_txid = NULL,
                updated_at = NOW()
            WHERE p.action_id = $1
              AND (
                    p.broadcast_status IN ('idle', 'failed')
                 OR (
                        p.broadcast_status = 'commit_broadcasted'
                    AND p.commit_txid IS NULL
                    AND p.reveal_txid IS NULL
                    AND p.broadcast_claimed_at IS NOT NULL
                    AND p.broadcast_claimed_at <= NOW() - INTERVAL '600 seconds'
                    )
                  )
              AND NOT EXISTS (
                    SELECT 1 FROM proposals AS o
                    WHERE o.authority = p.authority
                      AND o.action_id <> p.action_id
                      AND o.status = 'approved'
                      AND (
                            o.broadcast_status IN ('commit_confirmed', 'reveal_broadcasted')
                         OR (
                                o.broadcast_status = 'commit_broadcasted'
                            AND NOT (
                                    o.commit_txid IS NULL
                                AND o.reveal_txid IS NULL
                                AND o.broadcast_claimed_at IS NOT NULL
                                AND o.broadcast_claimed_at <= NOW() - INTERVAL '600 seconds'
                                )
                            )
                          )
                  )
            RETURNING {SELECT_PROPOSAL_COLS}
            "#
        ))
        .bind(&action_id.0)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to claim broadcast: {e}")))?;

        let Some(row) = row else {
            return Err(AppError::Conflict(AUTHORITY_IN_FLIGHT.to_string()));
        };

        tx.commit()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to commit claim: {e}")))?;

        let action_id_str: String = row.get("action_id");
        let signatures = load_signatures(&self.pool, &action_id_str).await?;
        let mut proposal = row_to_proposal_no_sigs(&row, action_id_str)?;
        proposal.signatures = signatures;
        proposal.broadcast_attempts =
            load_broadcast_attempts(&self.pool, &proposal.action_id.0).await?;
        Ok(proposal)
    }

    async fn register_additional_pair(
        &self,
        action_id: &ActionId,
        attempt: BroadcastAttempt,
        abandoned: &[String],
    ) -> Result<Proposal, AppError> {
        // Lock the row so two registrations — or one beside a claim or a report — are decided
        // one after the other on what the previous one wrote.
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to begin tx: {e}")))?;

        let row = sqlx::query(&format!(
            "SELECT {SELECT_PROPOSAL_COLS} FROM proposals WHERE action_id = $1 FOR UPDATE"
        ))
        .bind(&action_id.0)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to lock proposal: {e}")))?;
        let Some(row) = row else {
            return Err(AppError::NotFound);
        };
        let mut current = row_to_proposal_no_sigs(&row, action_id.0.clone())?;
        current.broadcast_attempts = load_broadcast_attempts(&mut *tx, &action_id.0).await?;

        match decide_additional_pair(&current, &attempt, abandoned) {
            Err(refusal) => return Err(AppError::Conflict(refusal.to_string())),
            // Dropping the transaction releases the lock; the row is left as it is.
            Ok(AdditionalPair::AlreadyRegistered) => {
                drop(tx);
                return self
                    .find_by_action_id(action_id)
                    .await?
                    .ok_or(AppError::NotFound);
            }
            Ok(AdditionalPair::Register) => {}
        }

        sqlx::query(
            r#"
            INSERT INTO broadcast_attempts(action_id, broadcaster_pubkey, commit_txid, reveal_txid, created_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(&action_id.0)
        .bind(attempt.broadcaster_pubkey.as_deref())
        .bind(&attempt.commit_txid)
        .bind(&attempt.reveal_txid)
        .bind(attempt.created_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            AppError::Internal(anyhow::anyhow!("failed to register broadcast attempt: {e}"))
        })?;

        sqlx::query(
            r#"
            UPDATE proposals
            SET broadcast_status = 'commit_broadcasted',
                commit_txid = $2,
                reveal_txid = $3,
                broadcast_error = NULL,
                broadcast_claimed_at = NOW(),
                updated_at = NOW()
            WHERE action_id = $1
            "#,
        )
        .bind(&action_id.0)
        .bind(&attempt.commit_txid)
        .bind(&attempt.reveal_txid)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to track the new pair: {e}")))?;

        tx.commit()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to commit tx: {e}")))?;

        self.find_by_action_id(action_id)
            .await?
            .ok_or(AppError::NotFound)
    }

    async fn enact_if_approved(&self, action_id: &ActionId) -> Result<bool, AppError> {
        let updated = sqlx::query(
            r#"
            UPDATE proposals
            SET status = 'enacted', updated_at = NOW()
            WHERE action_id = $1
              AND status = 'approved'
              AND broadcast_status = 'reveal_confirmed'
            "#,
        )
        .bind(&action_id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to enact proposal: {e}")))?;
        Ok(updated.rows_affected() > 0)
    }

    async fn expire_if_approved(&self, action_id: &ActionId) -> Result<bool, AppError> {
        let updated = sqlx::query(
            r#"
            UPDATE proposals
            SET status = 'expired', updated_at = NOW()
            WHERE action_id = $1
              AND status = 'approved'
              AND broadcast_status = 'reveal_confirmed'
            "#,
        )
        .bind(&action_id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to expire proposal: {e}")))?;
        Ok(updated.rows_affected() > 0)
    }

    async fn supersede_if_unchanged(
        &self,
        action_id: &ActionId,
        observed: BroadcastStatus,
    ) -> Result<bool, AppError> {
        let updated = sqlx::query(
            r#"
            UPDATE proposals
            SET status = 'superseded', updated_at = NOW()
            WHERE action_id = $1
              AND status = 'approved'
              AND broadcast_status = $2
            "#,
        )
        .bind(&action_id.0)
        .bind(observed.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to supersede proposal: {e}")))?;
        Ok(updated.rows_affected() > 0)
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

        sqlx::query(UPDATE_BROADCAST_STATUS)
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

    #[allow(clippy::too_many_arguments)]
    async fn update_reported_pair_status(
        &self,
        action_id: &ActionId,
        status: BroadcastStatus,
        proposal_status: Option<ProposalStatus>,
        commit_txid: Option<&str>,
        reveal_txid: Option<&str>,
        error: Option<&str>,
        attempt: Option<BroadcastAttempt>,
        live: &[String],
    ) -> Result<Option<Proposal>, AppError> {
        let proposal_status_db = proposal_status.map(status_to_db);

        // One transaction, under the row lock: what the report does is decided against the row as
        // it stands now (#605).
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to begin tx: {e}")))?;

        let row = sqlx::query(&format!(
            "SELECT {SELECT_PROPOSAL_COLS} FROM proposals WHERE action_id = $1 FOR UPDATE"
        ))
        .bind(&action_id.0)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to lock proposal: {e}")))?;
        let Some(row) = row else {
            return Ok(None);
        };
        let mut current = row_to_proposal_no_sigs(&row, action_id.0.clone())?;
        current.broadcast_attempts = load_broadcast_attempts(&mut *tx, &action_id.0).await?;
        let effect = match report_effect(
            &current,
            reveal_txid,
            status,
            proposal_status,
            attempt.as_ref(),
        ) {
            Ok(effect) => effect,
            Err(refusal) => {
                drop(tx);
                return Err(AppError::Conflict(refusal.to_string()));
            }
        };
        if let Some(attempt) = attempt.filter(|_| effect.lists_pair) {
            // Txids arrive lowercase. The case-sensitive primary key keeps a pair once.
            sqlx::query(
                r#"
                INSERT INTO broadcast_attempts(action_id, broadcaster_pubkey, commit_txid, reveal_txid, created_at)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (action_id, reveal_txid) DO NOTHING
                "#,
            )
            .bind(&action_id.0)
            .bind(attempt.broadcaster_pubkey.as_deref())
            .bind(&attempt.commit_txid)
            .bind(&attempt.reveal_txid)
            .bind(attempt.created_at)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                AppError::Internal(anyhow::anyhow!("failed to record broadcast attempt: {e}"))
            })?;
            let kept = current
                .broadcast_attempts
                .iter()
                .any(|kept| kept.reveal_txid.eq_ignore_ascii_case(&attempt.reveal_txid));
            if !kept {
                current.broadcast_attempts.push(attempt);
            }
        }
        if !effect.moves_row {
            tx.commit()
                .await
                .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to commit tx: {e}")))?;
            return self.find_by_action_id(action_id).await;
        }

        // A failed report for the tracked pair puts the row back on an earlier live one (#605).
        let write =
            broadcast_write_for_report(&current, status, commit_txid, reveal_txid, error, live);
        sqlx::query(UPDATE_BROADCAST_STATUS)
            .bind(write.status.to_string())
            .bind(proposal_status_db)
            .bind(write.commit_txid.as_deref())
            .bind(write.reveal_txid.as_deref())
            .bind(write.error.as_deref())
            .bind(&action_id.0)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                AppError::Internal(anyhow::anyhow!("failed to update broadcast status: {e}"))
            })?;

        tx.commit()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to commit tx: {e}")))?;

        self.find_by_action_id(action_id).await
    }

    async fn confirm_tracked_reveal(
        &self,
        action_id: &ActionId,
        observed_reveal: Option<&str>,
        commit_txid: &str,
        reveal_txid: &str,
    ) -> Result<Option<Proposal>, AppError> {
        // One UPDATE: the winner's txids are written only while the row still tracks the reveal
        // the reconcile observed, and the broadcast has not already confirmed.
        let updated = sqlx::query(
            r#"
            UPDATE proposals
            SET broadcast_status = 'reveal_confirmed',
                status = 'approved',
                commit_txid = LOWER($2),
                reveal_txid = LOWER($3),
                broadcast_error = NULL,
                updated_at = NOW()
            WHERE action_id = $1
              AND status IN ('approved', 'superseded')
              AND (
                    status = 'superseded'
                    OR broadcast_status IN (
                        'commit_broadcasted', 'commit_confirmed', 'reveal_broadcasted', 'failed'
                    )
                  )
              AND (
                    ($4::text IS NULL AND reveal_txid IS NULL)
                    OR ($4::text IS NOT NULL AND LOWER(reveal_txid) = LOWER($4))
                  )
            "#,
        )
        .bind(&action_id.0)
        .bind(commit_txid)
        .bind(reveal_txid)
        .bind(observed_reveal)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::Internal(anyhow::anyhow!("failed to confirm tracked reveal: {e}"))
        })?;

        if updated.rows_affected() == 0 {
            return Ok(None);
        }
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
        proposal.broadcast_attempts =
            load_broadcast_attempts(&self.pool, &proposal.action_id.0).await?;
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

    async fn enact_executed_cancel(
        &self,
        cancel_action_id: &ActionId,
        target_action_id: &ActionId,
    ) -> Result<bool, AppError> {
        let mut tx =
            self.pool.begin().await.map_err(|e| {
                AppError::Internal(anyhow::anyhow!("failed to begin transaction: {e}"))
            })?;

        let result = sqlx::query(
            "UPDATE proposals SET status = $1, updated_at = NOW() \
             WHERE action_id = $2 AND status IN ($3, $4, $5)",
        )
        .bind(status_to_db(ProposalStatus::Canceled))
        .bind(&target_action_id.0)
        .bind(status_to_db(ProposalStatus::Approved))
        .bind(status_to_db(ProposalStatus::Superseded))
        .bind(status_to_db(ProposalStatus::Enacted))
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            AppError::Internal(anyhow::anyhow!("failed to mark target as canceled: {e}"))
        })?;

        if result.rows_affected() == 0 {
            tx.commit().await.map_err(|e| {
                AppError::Internal(anyhow::anyhow!("failed to commit transaction: {e}"))
            })?;
            return Ok(false);
        }

        sqlx::query("UPDATE proposals SET status = $1, updated_at = NOW() WHERE action_id = $2")
            .bind(status_to_db(ProposalStatus::Enacted))
            .bind(&cancel_action_id.0)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                AppError::Internal(anyhow::anyhow!("failed to mark cancel as enacted: {e}"))
            })?;

        tx.commit().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("failed to commit transaction: {e}"))
        })?;

        Ok(true)
    }
}

/// #605 against a real Postgres: run with
/// `TEST_DATABASE_URL=postgres://… cargo test -p orchestrator-be -- --ignored postgres_`.
/// The in-memory repository covers the same rules in every run; this exercises the SQL itself.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::proposals::OWN_PAIR_EXISTS;

    const COMMIT_A: &str = "aa00000000000000000000000000000000000000000000000000000000000000";
    const REVEAL_A: &str = "aa11111111111111111111111111111111111111111111111111111111111111";
    const COMMIT_B: &str = "bb00000000000000000000000000000000000000000000000000000000000000";
    const REVEAL_B: &str = "bb11111111111111111111111111111111111111111111111111111111111111";
    const COMMIT_C: &str = "cc00000000000000000000000000000000000000000000000000000000000000";
    const REVEAL_C: &str = "cc11111111111111111111111111111111111111111111111111111111111111";
    const SIGNER_A: &str = "02aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const SIGNER_B: &str = "03bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    async fn repo() -> Option<PostgresProposalRepository> {
        let url = std::env::var("TEST_DATABASE_URL").ok()?;
        let pool = PgPool::connect(&url)
            .await
            .expect("connect TEST_DATABASE_URL");
        sqlx::migrate!().run(&pool).await.expect("migrations");
        Some(PostgresProposalRepository::new(pool))
    }

    fn attempt(signer: &str, commit: &str, reveal: &str) -> BroadcastAttempt {
        BroadcastAttempt {
            broadcaster_pubkey: Some(signer.to_string()),
            commit_txid: commit.to_string(),
            reveal_txid: reveal.to_string(),
            created_at: Utc::now(),
        }
    }

    /// An approved proposal whose claimer (signer A) reported its pair as sent.
    async fn save_with_pair_of_a(repo: &PostgresProposalRepository) -> ActionId {
        let nanos = Utc::now().timestamp_nanos_opt().unwrap_or_default();
        let action_id = ActionId(format!("pg-605-{nanos}"));
        repo.save_proposal(Proposal {
            action_id: action_id.clone(),
            seq_no: u64::try_from(nanos).unwrap_or_default(),
            authority: Authority::StrataAdmin,
            status: ProposalStatus::Approved,
            required_signatures: 2,
            action_hex: "deadbeef".to_string(),
            title: None,
            signatures: vec![ProposalSignature {
                signer_pubkey: SIGNER_A.to_string(),
                signature_hex: "sig0".to_string(),
            }],
            broadcast_status: BroadcastStatus::CommitBroadcasted,
            commit_txid: None,
            reveal_txid: None,
            broadcast_error: None,
            target_action_id: None,
            activation_height: None,
            update_id_in_queue: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            broadcast_claimed_at: Some(Utc::now()),
            broadcast_attempts: Vec::new(),
        })
        .await
        .unwrap();
        repo.update_reported_pair_status(
            &action_id,
            BroadcastStatus::RevealBroadcasted,
            None,
            Some(COMMIT_A),
            Some(REVEAL_A),
            None,
            Some(attempt(SIGNER_A, COMMIT_A, REVEAL_A)),
            &[],
        )
        .await
        .unwrap();
        action_id
    }

    fn reveals(proposal: &Proposal) -> Vec<&str> {
        proposal
            .broadcast_attempts
            .iter()
            .map(|attempt| attempt.reveal_txid.as_str())
            .collect()
    }

    #[tokio::test]
    #[ignore = "needs TEST_DATABASE_URL"]
    async fn postgres_registers_reports_and_confirms_pairs() {
        let Some(repo) = repo().await else {
            return;
        };
        let action_id = save_with_pair_of_a(&repo).await;

        // Another signer's pair becomes the tracked one; a retry of it changes nothing.
        let registered = repo
            .register_additional_pair(&action_id, attempt(SIGNER_B, COMMIT_B, REVEAL_B), &[])
            .await
            .unwrap();
        assert_eq!(registered.reveal_txid.as_deref(), Some(REVEAL_B));
        assert_eq!(
            registered.broadcast_status,
            BroadcastStatus::CommitBroadcasted
        );
        assert_eq!(reveals(&registered), [REVEAL_A, REVEAL_B]);
        let again = repo
            .register_additional_pair(&action_id, attempt(SIGNER_B, COMMIT_B, REVEAL_B), &[])
            .await
            .unwrap();
        assert_eq!(again.broadcast_attempts, registered.broadcast_attempts);

        // A second pair of their own is refused, unless the earlier one is nowhere.
        let err = repo
            .register_additional_pair(&action_id, attempt(SIGNER_A, COMMIT_C, REVEAL_C), &[])
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::Conflict(ref m) if m == OWN_PAIR_EXISTS),
            "{err:?}"
        );

        // A report about the replaced pair does not move the row.
        let after = repo
            .update_reported_pair_status(
                &action_id,
                BroadcastStatus::Failed,
                None,
                Some(COMMIT_A),
                Some(REVEAL_A),
                Some("dropped"),
                Some(attempt(SIGNER_A, COMMIT_A, REVEAL_A)),
                &[],
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after.broadcast_status, BroadcastStatus::CommitBroadcasted);
        assert_eq!(after.reveal_txid.as_deref(), Some(REVEAL_B));
        assert!(after.broadcast_error.is_none());
        let after = repo
            .update_reported_pair_status(
                &action_id,
                BroadcastStatus::RevealBroadcasted,
                None,
                Some(COMMIT_C),
                Some(REVEAL_C),
                None,
                Some(attempt(SIGNER_A, COMMIT_C, REVEAL_C)),
                &[],
            )
            .await
            .unwrap()
            .unwrap();
        // The row stays on B; the reported pair is listed beside it.
        assert_eq!(after.reveal_txid.as_deref(), Some(REVEAL_B));
        assert_eq!(reveals(&after), [REVEAL_A, REVEAL_B, REVEAL_C]);

        // The listing carries every pair.
        let listed = repo
            .list_by_status(Authority::StrataAdmin, Some(ProposalStatus::Approved))
            .await
            .unwrap();
        let mine = listed
            .iter()
            .find(|proposal| proposal.action_id == action_id)
            .expect("listed");
        assert_eq!(reveals(mine), [REVEAL_A, REVEAL_B, REVEAL_C]);

        // A confirmation decided against a row that moved on does not apply; one that did, does.
        let stale = repo
            .confirm_tracked_reveal(&action_id, Some(REVEAL_A), COMMIT_A, REVEAL_A)
            .await
            .unwrap();
        assert!(stale.is_none());
        let confirmed = repo
            .confirm_tracked_reveal(
                &action_id,
                Some(&REVEAL_B.to_uppercase()),
                &COMMIT_A.to_uppercase(),
                &REVEAL_A.to_uppercase(),
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(confirmed.broadcast_status, BroadcastStatus::RevealConfirmed);
        assert_eq!(confirmed.commit_txid.as_deref(), Some(COMMIT_A));
        assert_eq!(confirmed.reveal_txid.as_deref(), Some(REVEAL_A));

        // Once confirmed, nothing is registered beside it.
        let err = repo
            .register_additional_pair(&action_id, attempt(SIGNER_A, COMMIT_C, REVEAL_C), &[])
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)), "{err:?}");
    }

    /// A `failed` report for the tracked pair goes back to an earlier pair the node still has, and
    /// is marked `failed` when none is left.
    #[tokio::test]
    #[ignore = "needs TEST_DATABASE_URL"]
    async fn postgres_a_failed_pair_falls_back_only_to_a_live_one() {
        let Some(repo) = repo().await else {
            return;
        };
        let action_id = save_with_pair_of_a(&repo).await;
        repo.register_additional_pair(&action_id, attempt(SIGNER_B, COMMIT_B, REVEAL_B), &[])
            .await
            .unwrap();
        let fail = |commit, reveal, live: Vec<String>| {
            let repo = &repo;
            let action_id = &action_id;
            async move {
                repo.update_reported_pair_status(
                    action_id,
                    BroadcastStatus::Failed,
                    None,
                    Some(commit),
                    Some(reveal),
                    Some("rejected"),
                    None,
                    &live,
                )
                .await
                .unwrap()
                .unwrap()
            }
        };

        let back = fail(COMMIT_B, REVEAL_B, vec![REVEAL_A.to_string()]).await;
        assert_eq!(back.broadcast_status, BroadcastStatus::CommitBroadcasted);
        assert_eq!(back.reveal_txid.as_deref(), Some(REVEAL_A));
        assert!(back.broadcast_error.is_none());

        let failed = fail(COMMIT_A, REVEAL_A, Vec::new()).await;
        assert_eq!(failed.broadcast_status, BroadcastStatus::Failed);
        assert_eq!(failed.reveal_txid.as_deref(), Some(REVEAL_A));
    }

    #[tokio::test]
    #[ignore = "needs TEST_DATABASE_URL"]
    async fn postgres_lets_a_signer_replace_a_pair_that_is_nowhere() {
        let Some(repo) = repo().await else {
            return;
        };
        let action_id = save_with_pair_of_a(&repo).await;
        repo.register_additional_pair(&action_id, attempt(SIGNER_B, COMMIT_B, REVEAL_B), &[])
            .await
            .unwrap();

        let replaced = repo
            .register_additional_pair(
                &action_id,
                attempt(SIGNER_A, COMMIT_C, REVEAL_C),
                &[REVEAL_A.to_string()],
            )
            .await
            .unwrap();

        assert_eq!(replaced.reveal_txid.as_deref(), Some(REVEAL_C));
        assert_eq!(reveals(&replaced), [REVEAL_A, REVEAL_B, REVEAL_C]);
    }
}
