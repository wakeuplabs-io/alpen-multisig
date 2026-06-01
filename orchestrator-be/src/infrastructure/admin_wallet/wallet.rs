//! Admin wallet infrastructure — error types and hardware wallet adapter.

use thiserror::Error;

/// Errors specific to admin wallet operations.
#[derive(Debug, Error)]
pub(crate) enum AdminWalletError {
    #[error("wallet is read-only — no signer attached")]
    ReadOnly,
}
