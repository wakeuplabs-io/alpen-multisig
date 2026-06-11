/// Whether the in-process ASM mock (`mock://` RPC URLs) is compiled into this build.
/// `true` only for test builds and builds with the `dev-mocks` feature.
pub(crate) const ASM_MOCKS_COMPILED: bool = cfg!(any(test, feature = "dev-mocks"));

/// Reject `mock://` ASM RPC URLs unless the in-process mock is compiled in.
///
/// The desktop ASM RPC URL comes from the user-editable node config (Custom mode),
/// so this guards against a `mock://` URL being entered in a production build and
/// silently bypassing on-chain authorization checks.
pub(crate) fn reject_mock_asm_url_in_prod(rpc_url: &str) -> Result<(), String> {
    check_mock_asm_url(rpc_url, ASM_MOCKS_COMPILED)
}

fn check_mock_asm_url(rpc_url: &str, mocks_compiled: bool) -> Result<(), String> {
    if rpc_url.starts_with("mock://") && !mocks_compiled {
        return Err(
            "the configured ASM RPC URL uses a `mock://` scheme, which bypasses on-chain \
             authorization and is only available in dev/test builds. Set a real ASM RPC URL \
             in node settings."
                .to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod mock_guard_tests {
    use super::check_mock_asm_url;

    #[test]
    fn mock_url_rejected_only_when_mocks_not_compiled() {
        assert!(check_mock_asm_url("mock://asm-membership", false).is_err());
        assert!(check_mock_asm_url("mock://asm-enacted", false).is_err());
        assert!(check_mock_asm_url("mock://asm-membership", true).is_ok());
        assert!(check_mock_asm_url("http://127.0.0.1:8080", false).is_ok());
    }
}

pub mod action_codec;
pub mod admin_wallet;
pub mod asm_enactment;
pub mod asm_role_membership;
pub mod asm_status_rpc;
pub mod bitcoin_rpc;
pub mod broadcast_env;
pub mod broadcast_tx;
pub mod challenge_verifier;
pub mod dev_secrets;
pub mod electrum_broadcaster;
pub mod electrum_fee_estimator;
pub mod env_loader;
pub mod hw_wallet;
pub mod node_broadcaster;
pub mod node_config_store;
pub mod node_fee_estimator;
pub mod orchestrator_client;
pub mod rpc_timeout;
pub mod signing;
