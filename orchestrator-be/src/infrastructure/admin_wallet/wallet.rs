//! Admin wallet infrastructure — error types and hardware wallet adapter.

use bitcoin::Network;
use thiserror::Error;

/// Errors specific to admin wallet operations.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Error)]
pub(crate) enum AdminWalletError {
    #[error("wallet is read-only — no signer attached")]
    ReadOnly,
    #[error("signer not allowed on network {network:?}")]
    SignerNotAllowedOnNetwork { network: Network },
    #[error("PSBT build failed: {0}")]
    PsbtBuild(String),
    #[error("signing failed: {0}")]
    SigningFailed(String),
    #[error("PSBT finalize failed: {0}")]
    FinalizeFailed(String),
    #[error("tx extract failed: {0}")]
    ExtractFailed(String),
}
