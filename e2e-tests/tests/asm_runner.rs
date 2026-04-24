//! E2E integration test for ASM runner JSON-RPC basic methods.
//!
//! Run only this test with:
//! `cargo test -p alpen-multisig-e2e-tests --test asm_runner -- --nocapture`
//!
//! Important:
//! - This test does NOT start `asm-runner` automatically.
//! - Start `asm-runner` manually before running this test.

use alpen_multisig_e2e_tests::asm_runner::utils::{assert_expected_admin_keys, rpc_call, RPC_URL};
use serde_json::json;

pub const EXPECTED_STRATA_ADMINISTRATOR_KEY_HEX: &str =
    "028c0ea5beee14a1aedeb7b6139f506321015708310eb686d1010477ef80fb6f3e";
pub const EXPECTED_STRATA_SEQUENCER_MANAGER_KEY_HEX: &str =
    "028c0ea5beee14a1aedeb7b6139f506321015708310eb686d1010477ef80fb6f3e";

#[tokio::test(flavor = "multi_thread")]
async fn e2e_asm_runner_jsonrpc_basic_methods() {
    let status_result = rpc_call("strata_asm_getStatus", json!([]))
        .await
        .unwrap_or_else(|err| {
            panic!(
                "Could not reach asm-runner at {RPC_URL}: {err}. \
                 Start asm-runner manually before running this test."
            )
        });
    assert!(
        status_result.is_object() || status_result.is_array() || status_result.is_null(),
        "status result should be valid JSON value, got: {status_result}"
    );
    assert_expected_admin_keys(
        &status_result,
        EXPECTED_STRATA_ADMINISTRATOR_KEY_HEX,
        EXPECTED_STRATA_SEQUENCER_MANAGER_KEY_HEX,
    )
    .expect(
        "decoded admin state should contain the expected keys for StrataAdministrator and StrataSequencerManager",
    );
}
