use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::domain::authority::Authority;

/// An ephemeral session bound to a single signer + authority.
///
/// Auth flow:
///   1. Client generates ephemeral keypair
///   2. Signer signs attestation with canonical admin key (binds ephemeral key, authority, nonce/expiry)
///   3. Backend verifies against ASM canonical signer set
///   4. All subsequent requests signed with ephemeral key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
	pub id: Uuid,
	/// The ephemeral public key used for request signing.
	pub ephemeral_pubkey: String,
	/// The canonical signer public key that attested this session.
	pub signer_pubkey: String,
	pub authority: Authority,
	pub nonce: String,
	pub expires_at: DateTime<Utc>,
	pub created_at: DateTime<Utc>,
}

impl Session {
	pub fn is_expired(&self) -> bool {
		Utc::now() > self.expires_at
	}
}

/// Challenge nonce issued to a client before authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthChallenge {
	pub nonce: String,
	pub expires_at: DateTime<Utc>,
}
