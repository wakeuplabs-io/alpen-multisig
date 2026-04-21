use alpen_multisig_e2e_tests::test_harness::AsmTestHarnessBuilder;
use std::process::Command;

#[tokio::test(flavor = "multi_thread")]
async fn test_harness_hello_world_mine_one_block() {
    if Command::new("bitcoind").arg("--version").output().is_err() {
        eprintln!(
            "Skipping test_harness_hello_world_mine_one_block: bitcoind is not available in PATH"
        );
        return;
    }

    let harness = AsmTestHarnessBuilder::default()
        .build()
        .await
        .expect("harness should build");

    let before = harness
        .get_processed_height()
        .expect("processed height should exist");

    let _hash = harness
        .mine_block(None)
        .await
        .expect("should mine and process one block");

    let after = harness
        .get_processed_height()
        .expect("processed height should exist after mining");

    assert!(
        after > before,
        "processed height should advance by at least 1"
    );

    let latest = harness
        .get_latest_asm_state()
        .expect("state query should succeed");

    assert!(latest.is_some(), "latest ASM state should exist");
}
